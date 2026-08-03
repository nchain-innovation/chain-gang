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
                // Height derived from confirmations, matching the Python client.
                let height = if entry.confirmations == 0 {
                    0
                } else {
                    block_count - entry.confirmations as i32 - 1
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
