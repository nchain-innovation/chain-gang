//! A native Rust JSON-RPC interface to a bitcoind / BSV node.
//!
//! This mirrors the capability of the pure-Python `RPCInterface`
//! (`python/src/tx_engine/interface/rpc_interface.py`) for native Rust clients.
//! It implements the shared [`BlockchainInterface`] trait plus a set of
//! read-only query helpers. Wallet/regtest mutation calls are intentionally
//! out of scope.
//!
//! Transport is JSON-RPC 1.0 over HTTP Basic auth, hand-rolled on `reqwest`
//! for consistency with [`WocInterface`](crate::interface::WocInterface) and
//! [`UaaSInterface`](crate::interface::UaaSInterface).
//!
//! # Behavioural caveats
//!
//! - `get_balance` / `get_utxo` are **wallet-scoped**, not chain-scoped. RPC
//!   `listunspent` only sees addresses the node's wallet knows; an arbitrary
//!   address returns empty until `importaddress` + rescan. This differs from
//!   `WocInterface`, which indexes the whole chain.
//! - `get_block_headers` has no direct RPC equivalent and returns an error.

use std::time::Duration;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    interface::blockchain_interface::{Balance, BlockchainInterface, Utxo, UtxoEntry},
    messages::{BlockHeader, Tx},
    network::Network,
    util::{ChainGangError, Serializable},
};

/// Number of confirmations at or above which a UTXO counts as confirmed.
/// Matches the default used by the Python `RPCInterface.get_balance`.
const CONFIRMED_DEPTH: i64 = 6;
/// One BSV/BTC expressed in satoshis.
const SATOSHIS_PER_COIN: f64 = 100_000_000.0;

/// Default number of retries on transient connection failures.
const DEFAULT_MAX_RETRIES: u32 = 5;
/// Default delay between retries.
const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(250);

/// JSON-RPC 1.0 request envelope.
#[derive(Debug, Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    id: &'a str,
    method: &'a str,
    params: Value,
}

/// JSON-RPC error object, as returned in the `error` field of a response.
#[derive(Debug, Deserialize)]
struct RpcErrorObject {
    code: i32,
    message: String,
}

/// JSON-RPC 1.0 response envelope.
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcErrorObject>,
}

/// A native Rust JSON-RPC interface to a bitcoind / BSV node.
#[derive(Debug, Clone)]
pub struct RpcInterface {
    client: reqwest::Client,
    url: String,
    user: String,
    password: String,
    network_type: Network,
    max_retries: u32,
    retry_delay: Duration,
}

impl RpcInterface {
    /// Create a new RPC interface.
    ///
    /// `address` is the node endpoint, e.g. `"http://127.0.0.1:8332"`. If no
    /// scheme is supplied, `http://` is assumed. Credentials are sent as HTTP
    /// Basic auth on every request.
    pub fn new(
        address: &str,
        user: &str,
        password: &str,
        network: Network,
    ) -> Result<Self, ChainGangError> {
        // Normalise and validate the endpoint.
        let url = if address.contains("://") {
            address.to_string()
        } else {
            format!("http://{address}")
        };
        // Validate by parsing; discard the parsed value (reqwest re-parses).
        let _ = reqwest::Url::parse(&url)?;

        Ok(RpcInterface {
            client: reqwest::Client::new(),
            url,
            user: user.to_string(),
            password: password.to_string(),
            network_type: network,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_delay: DEFAULT_RETRY_DELAY,
        })
    }

    /// Override the bounded retry policy for transient connection failures.
    pub fn with_retries(mut self, max_retries: u32, retry_delay: Duration) -> Self {
        self.max_retries = max_retries;
        self.retry_delay = retry_delay;
        self
    }

    /// Perform a single JSON-RPC call and deserialize the `result` into `T`.
    ///
    /// Retries up to `max_retries` times on transient connection/timeout
    /// errors (with `retry_delay` between attempts). A JSON-RPC `error` object
    /// is mapped to [`ChainGangError::RpcError`].
    async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, ChainGangError> {
        let request = RpcRequest {
            jsonrpc: "1.0",
            id: "chain-gang",
            method,
            params,
        };

        let mut attempt: u32 = 0;
        let response = loop {
            let result = self
                .client
                .post(&self.url)
                .basic_auth(&self.user, Some(&self.password))
                .json(&request)
                .send()
                .await;

            match result {
                Ok(resp) => break resp,
                Err(err) if (err.is_connect() || err.is_timeout()) && attempt < self.max_retries => {
                    log::warn!("rpc {method}: transient error (attempt {attempt}): {err}");
                    attempt += 1;
                    tokio::time::sleep(self.retry_delay).await;
                }
                Err(err) => return Err(err.into()),
            }
        };

        let status = response.status();
        let text = response.text().await?;

        // bitcoind returns the JSON-RPC envelope even on HTTP error statuses
        // (e.g. 500 with an `error` object), so parse the body first and prefer
        // the RPC error message; fall back to the HTTP status.
        let parsed: RpcResponse<T> = match serde_json::from_str(&text) {
            Ok(parsed) => parsed,
            Err(err) => {
                if !status.is_success() {
                    return Err(ChainGangError::ResponseError(format!(
                        "rpc {method}: http status {status}, body: {text}"
                    )));
                }
                return Err(ChainGangError::JSONParseError(format!(
                    "rpc {method}: {err}, body: {text}"
                )));
            }
        };

        if let Some(rpc_err) = parsed.error {
            return Err(ChainGangError::RpcError {
                code: rpc_err.code,
                message: rpc_err.message,
            });
        }

        parsed.result.ok_or_else(|| {
            ChainGangError::ResponseError(format!("rpc {method}: missing result in response"))
        })
    }
}

/// A single entry from `listunspent`. Only the fields we consume are modelled;
/// serde ignores the rest.
#[derive(Debug, Deserialize)]
struct ListUnspentEntry {
    txid: String,
    vout: u32,
    #[serde(default)]
    address: Option<String>,
    amount: f64,
    confirmations: i64,
}

impl RpcInterface {
    /// Fetch all wallet UTXOs, optionally filtered to a single address.
    ///
    /// Note: `listunspent` is wallet-scoped — it only returns outputs for
    /// addresses the node's wallet is watching.
    async fn list_unspent(
        &self,
        address: Option<&str>,
    ) -> Result<Vec<ListUnspentEntry>, ChainGangError> {
        // listunspent minconf=0 to include unconfirmed outputs.
        let unspent: Vec<ListUnspentEntry> = self.call("listunspent", json!([0])).await?;
        Ok(match address {
            None => unspent,
            Some(addr) => unspent
                .into_iter()
                .filter(|entry| entry.address.as_deref() == Some(addr))
                .collect(),
        })
    }
}

#[async_trait]
impl BlockchainInterface for RpcInterface {
    fn set_network(&mut self, network: &Network) {
        self.network_type = *network;
    }

    /// Return `Ok(())` if the node responds to `getblockchaininfo`.
    async fn status(&self) -> Result<(), ChainGangError> {
        log::debug!("status");
        let _: Value = self.call("getblockchaininfo", json!([])).await?;
        Ok(())
    }

    /// Confirmed/unconfirmed balance (in satoshis) for a wallet-known address.
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainGangError> {
        log::debug!("get_balance");
        let unspent = self.list_unspent(Some(address)).await?;

        let mut confirmed = 0.0_f64;
        let mut unconfirmed = 0.0_f64;
        for entry in &unspent {
            if entry.confirmations >= CONFIRMED_DEPTH {
                confirmed += entry.amount;
            } else {
                unconfirmed += entry.amount;
            }
        }
        Ok(Balance {
            confirmed: (confirmed * SATOSHIS_PER_COIN) as i64,
            unconfirmed: (unconfirmed * SATOSHIS_PER_COIN) as i64,
        })
    }

    /// Ordered list of UTXOs for a wallet-known address.
    async fn get_utxo(&self, address: &str) -> Result<Utxo, ChainGangError> {
        log::debug!("get_utxo");
        let unspent = self.list_unspent(Some(address)).await?;
        let block_count: i32 = self.call("getblockcount", json!([])).await?;

        let mut utxo: Utxo = unspent
            .into_iter()
            .map(|entry| {
                // True block height the output was mined at: for N confirmations
                // at tip height H, the output is in block H - N + 1. (This
                // deliberately differs from the Python RPCInterface, which uses
                // H - N - 1 and is off by two.) Unconfirmed outputs get 0.
                let height = if entry.confirmations == 0 {
                    0
                } else {
                    block_count - entry.confirmations as i32 + 1
                };
                UtxoEntry {
                    height,
                    tx_pos: entry.vout,
                    tx_hash: entry.txid,
                    value: (entry.amount * SATOSHIS_PER_COIN) as i64,
                }
            })
            .collect();
        utxo.sort_by_key(|entry| entry.height);
        Ok(utxo)
    }

    /// Broadcast a transaction via `sendrawtransaction`, returning the txid.
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String, ChainGangError> {
        log::debug!("broadcast_tx");
        let txid: String = self
            .call("sendrawtransaction", json!([tx.as_hexstr()]))
            .await?;
        Ok(txid)
    }

    /// Fetch a transaction by txid via `getrawtransaction` (hex form).
    async fn get_tx(&self, txid: &str) -> Result<Tx, ChainGangError> {
        log::debug!("get_tx");
        let hex_tx: String = self.call("getrawtransaction", json!([txid])).await?;
        let bytes = hex::decode(hex_tx)?;
        let mut byte_slice = &bytes[..];
        let tx = Tx::read(&mut byte_slice)?;
        Ok(tx)
    }

    /// Fetch the header of the current tip.
    ///
    /// There is no single "latest header" RPC, so this chains
    /// `getblockcount` → `getblockhash` → `getblockheader` (hex form).
    async fn get_latest_block_header(&self) -> Result<BlockHeader, ChainGangError> {
        log::debug!("get_latest_block_header");
        let block_count: u64 = self.call("getblockcount", json!([])).await?;
        let block_hash: String = self.call("getblockhash", json!([block_count])).await?;
        // getblockheader with verbose=false returns the serialized header hex.
        let header_hex: String = self
            .call("getblockheader", json!([block_hash, false]))
            .await?;
        let bytes = hex::decode(header_hex)?;
        let mut byte_slice = &bytes[..];
        let header = BlockHeader::read(&mut byte_slice)?;
        Ok(header)
    }

    /// Not supported over JSON-RPC — bitcoind exposes no bulk-headers call.
    async fn get_block_headers(&self) -> Result<String, ChainGangError> {
        Err(ChainGangError::InvalidOperation(
            "get_block_headers is not available over the RPC interface".to_string(),
        ))
    }
}

/// Read-only query helpers beyond the shared trait. These mirror the read
/// calls of the Python `RPCInterface`; loosely-shaped node responses are
/// returned as [`serde_json::Value`] for the caller to interpret.
impl RpcInterface {
    /// Current block height (`getblockcount`).
    pub async fn get_block_count(&self) -> Result<u64, ChainGangError> {
        self.call("getblockcount", json!([])).await
    }

    /// Best (tip) block hash (`getbestblockhash`).
    pub async fn get_best_block_hash(&self) -> Result<String, ChainGangError> {
        self.call("getbestblockhash", json!([])).await
    }

    /// Block hash at the given height (`getblockhash`).
    pub async fn get_block_hash(&self, index: u64) -> Result<String, ChainGangError> {
        self.call("getblockhash", json!([index])).await
    }

    /// Raw transaction hex for a txid (`getrawtransaction`).
    pub async fn get_raw_transaction(&self, txid: &str) -> Result<String, ChainGangError> {
        self.call("getrawtransaction", json!([txid])).await
    }

    /// Unspent output details (`gettxout`).
    pub async fn get_tx_out(&self, txid: &str, index: u32) -> Result<Value, ChainGangError> {
        self.call("gettxout", json!([txid, index])).await
    }

    /// Full block for a block hash (`getblock`).
    pub async fn get_block(&self, block_hash: &str) -> Result<Value, ChainGangError> {
        self.call("getblock", json!([block_hash])).await
    }

    /// Verbose block header for a block hash (`getblockheader`).
    pub async fn get_block_header(&self, block_hash: &str) -> Result<Value, ChainGangError> {
        self.call("getblockheader", json!([block_hash])).await
    }

    /// Current mempool contents (`getrawmempool`).
    pub async fn get_raw_mempool(&self) -> Result<Value, ChainGangError> {
        self.call("getrawmempool", json!([])).await
    }

    /// Merkle proof (hex) that a txid is in a block (`gettxoutproof`).
    pub async fn get_merkle_proof(
        &self,
        block_hash: &str,
        tx_id: &str,
    ) -> Result<String, ChainGangError> {
        self.call("gettxoutproof", json!([[tx_id], block_hash])).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    fn rpc(url: &str) -> RpcInterface {
        RpcInterface::new(url, "user", "pass", Network::BSV_Testnet).unwrap()
    }

    #[tokio::test]
    async fn get_block_count_ok() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).body_contains("getblockcount");
                then.status(200).json_body(json!({
                    "result": 12345, "error": null, "id": "chain-gang"
                }));
            })
            .await;

        let count = rpc(&server.base_url()).get_block_count().await.unwrap();
        assert_eq!(count, 12345);
    }

    #[tokio::test]
    async fn rpc_error_is_mapped() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).body_contains("getrawtransaction");
                // bitcoind reports RPC errors with HTTP 500 + an error object.
                then.status(500).json_body(json!({
                    "result": null,
                    "error": {"code": -5, "message": "No such mempool transaction"},
                    "id": "chain-gang"
                }));
            })
            .await;

        let err = rpc(&server.base_url())
            .get_raw_transaction("deadbeef")
            .await
            .unwrap_err();
        match err {
            ChainGangError::RpcError { code, message } => {
                assert_eq!(code, -5);
                assert!(message.contains("No such mempool"));
            }
            other => panic!("expected RpcError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn basic_auth_header_is_sent() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST)
                    .header_exists("authorization")
                    .body_contains("getbestblockhash");
                then.status(200).json_body(json!({
                    "result": "00ff", "error": null, "id": "chain-gang"
                }));
            })
            .await;

        let hash = rpc(&server.base_url())
            .get_best_block_hash()
            .await
            .unwrap();
        assert_eq!(hash, "00ff");
    }

    #[tokio::test]
    async fn get_utxo_maps_filters_and_sorts() {
        let server = MockServer::start_async().await;
        server
            .mock_async(|when, then| {
                when.method(POST).body_contains("listunspent");
                then.status(200).json_body(json!({
                    "result": [
                        {"txid":"aa","vout":0,"address":"addr","amount":1.0,"confirmations":10},
                        {"txid":"bb","vout":2,"address":"addr","amount":0.5,"confirmations":2},
                        {"txid":"cc","vout":1,"address":"other","amount":9.0,"confirmations":3}
                    ],
                    "error": null, "id": "chain-gang"
                }));
            })
            .await;
        server
            .mock_async(|when, then| {
                when.method(POST).body_contains("getblockcount");
                then.status(200).json_body(json!({
                    "result": 100, "error": null, "id": "chain-gang"
                }));
            })
            .await;

        let utxo = rpc(&server.base_url()).get_utxo("addr").await.unwrap();

        // "other" address is filtered out; results sorted by ascending height.
        assert_eq!(utxo.len(), 2);
        // aa: height = 100 - 10 + 1 = 91, value = 1.0 * 1e8
        assert_eq!(utxo[0].tx_hash, "aa");
        assert_eq!(utxo[0].height, 91);
        assert_eq!(utxo[0].tx_pos, 0);
        assert_eq!(utxo[0].value, 100_000_000);
        // bb: height = 100 - 2 + 1 = 99, value = 0.5 * 1e8
        assert_eq!(utxo[1].tx_hash, "bb");
        assert_eq!(utxo[1].height, 99);
        assert_eq!(utxo[1].value, 50_000_000);
    }

    #[tokio::test]
    async fn connection_error_surfaces_after_retries_exhausted() {
        // Port 1 refuses connections; with zero retries this returns promptly.
        let iface = RpcInterface::new("http://127.0.0.1:1", "u", "p", Network::BSV_Testnet)
            .unwrap()
            .with_retries(0, Duration::from_millis(1));
        let err = iface.get_block_count().await.unwrap_err();
        assert!(matches!(err, ChainGangError::ReqwestError(_)));
    }
}
