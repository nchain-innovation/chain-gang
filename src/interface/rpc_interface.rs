//! JSON-RPC blockchain interface, for talking to a local node (Rust implementation).
//!
//! This is the natural backend for regtest, where there is no public explorer
//! API to point [`crate::interface::WocInterface`] at.
//!
//! A parallel, independent client lives in
//! `python/src/tx_engine/interface/rpc_interface.py`. The duplication is
//! deliberate: it lets the Python package reach a node without depending on
//! this crate's `interface` feature. When changing RPC method names or the
//! network mapping, update both implementations so they stay in sync.
//!
//! One divergence is intentional: an unconfirmed UTXO reports height -1 here,
//! per the [`UtxoEntry::height`] contract, and 0 in the Python client, which
//! predates it.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    interface::blockchain_interface::{Balance, BlockchainInterface, Utxo, UtxoEntry},
    messages::{BlockHeader, Tx},
    network::Network,
    util::{ChainGangError, Serializable},
};

/// Satoshis in one bitcoin, for converting the node's BTC-denominated amounts
const SATOSHIS_PER_BITCOIN: f64 = 100_000_000.0;

/// Confirmations at which a UTXO counts towards the confirmed balance
const CONFIRMATIONS: u32 = 6;

/// Number of recent headers [`RpcInterface::get_block_headers`] returns
const RECENT_HEADER_COUNT: u32 = 10;

/// Highest `maxconf` accepted by `listunspent`, used to mean "no upper bound"
const MAX_CONFIRMATIONS: u32 = 9_999_999;

/// One entry of a `listunspent` response, with only the fields used here
#[derive(Debug, Deserialize)]
struct Unspent {
    txid: String,
    vout: u32,
    /// Value in bitcoin, as the node reports it
    amount: f64,
    confirmations: i64,
}

/// A JSON-RPC error object as returned by the node
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

/// The envelope a JSON-RPC response arrives in
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

/// Blockchain interface backed by a node's JSON-RPC endpoint.
///
/// Intended for a local regtest or private node. Every call is a POST carrying
/// HTTP basic auth, so the credentials go to whatever host is configured — point
/// this at a node you control.
#[derive(Debug, Clone)]
pub struct RpcInterface {
    url: String,
    user: String,
    password: String,
    network_type: Network,
}

impl RpcInterface {
    /// Creates an interface for the node reachable at `address`.
    ///
    /// `address` is a host and port such as `127.0.0.1:18443`, optionally with a
    /// scheme; without one, `http://` is assumed, which is the usual case for a
    /// node on the local machine.
    pub fn new(address: &str, user: &str, password: &str, network: Network) -> Self {
        let url = if address.starts_with("http://") || address.starts_with("https://") {
            address.to_string()
        } else {
            format!("http://{address}")
        };
        RpcInterface {
            url,
            user: user.to_string(),
            password: password.to_string(),
            network_type: network,
        }
    }

    /// The network this interface is configured for
    pub fn network(&self) -> Network {
        self.network_type
    }

    /// Calls `method` and deserialises the `result` field of the response.
    ///
    /// A JSON-RPC error from the node, which arrives with HTTP 200 alongside a
    /// populated `error` field, becomes a [`ChainGangError::ResponseError`].
    async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, ChainGangError> {
        log::debug!("rpc call {method}");

        let body = json!({
            "jsonrpc": "1.0",
            "id": "chain-gang",
            "method": method,
            "params": params,
        });

        let response = reqwest::Client::new()
            .post(&self.url)
            .basic_auth(&self.user, Some(&self.password))
            .json(&body)
            .send()
            .await?;

        // A node reports a bad method or bad arguments as HTTP 500 with a
        // JSON-RPC error in the body, so read the body before judging the status
        let status = response.status();
        let text = response.text().await?;

        let parsed: RpcResponse<T> = serde_json::from_str(&text).map_err(|e| {
            ChainGangError::JSONParseError(format!(
                "{method} returned status {status} and a body that is not a JSON-RPC response: {e}"
            ))
        })?;

        if let Some(err) = parsed.error {
            return Err(ChainGangError::ResponseError(format!(
                "{method} failed with code {}: {}",
                err.code, err.message
            )));
        }

        parsed.result.ok_or_else(|| {
            ChainGangError::ResponseError(format!(
                "{method} returned neither a result nor an error"
            ))
        })
    }

    /// Returns the unspent outputs the node holds for `address`
    async fn list_unspent(&self, address: &str) -> Result<Vec<Unspent>, ChainGangError> {
        // The node filters by address, so unlike the Python client there is no
        // need to fetch the whole wallet's UTXO set and discard most of it
        self.call("listunspent", json!([0, MAX_CONFIRMATIONS, [address]]))
            .await
    }

    /// Returns the height of the most recent block
    async fn block_count(&self) -> Result<u32, ChainGangError> {
        self.call("getblockcount", json!([])).await
    }

    /// Converts a BTC-denominated amount from the node into satoshis
    fn as_satoshis(amount: f64) -> i64 {
        (amount * SATOSHIS_PER_BITCOIN).round() as i64
    }

    /// Height at which a UTXO with `confirmations` was mined, given the current
    /// `block_count`.
    ///
    /// An unconfirmed output gets -1, matching the [`UtxoEntry::height`] contract
    /// that a negative height means unconfirmed.
    fn utxo_height(block_count: u32, confirmations: i64) -> i32 {
        if confirmations <= 0 {
            return -1;
        }
        // A UTXO in the tip block has one confirmation, so it sits at block_count
        (i64::from(block_count) - confirmations + 1) as i32
    }
}

#[async_trait]
impl BlockchainInterface for RpcInterface {
    fn set_network(&mut self, network: &Network) {
        self.network_type = *network;
    }

    /// Returns `Ok(())` if the node answers a `getblockchaininfo` call
    async fn status(&self) -> Result<(), ChainGangError> {
        log::debug!("status");
        let _: Value = self.call("getblockchaininfo", json!([])).await?;
        Ok(())
    }

    /// Get balance associated with address
    ///
    /// An output counts as confirmed once it has [`CONFIRMATIONS`]
    /// confirmations. Amounts are converted to satoshis individually and summed
    /// as integers, so the totals do not accumulate floating point error.
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainGangError> {
        log::debug!("get_balance");

        let unspent = self.list_unspent(address).await?;
        let mut balance = Balance::default();
        for entry in &unspent {
            let satoshis = Self::as_satoshis(entry.amount);
            if entry.confirmations >= i64::from(CONFIRMATIONS) {
                balance.confirmed += satoshis;
            } else {
                balance.unconfirmed += satoshis;
            }
        }
        Ok(balance)
    }

    /// Get UXTO associated with address, ordered by height
    async fn get_utxo(&self, address: &str) -> Result<Utxo, ChainGangError> {
        log::debug!("get_utxo");

        let unspent = self.list_unspent(address).await?;
        if unspent.is_empty() {
            return Ok(Vec::new());
        }
        let block_count = self.block_count().await?;

        let mut utxo: Utxo = unspent
            .into_iter()
            .map(|entry| UtxoEntry {
                height: Self::utxo_height(block_count, entry.confirmations),
                tx_pos: entry.vout,
                tx_hash: entry.txid,
                value: Self::as_satoshis(entry.amount),
            })
            .collect();
        utxo.sort_by_key(|entry| entry.height);
        Ok(utxo)
    }

    /// Broadcast Tx, return the txid
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String, ChainGangError> {
        log::debug!("broadcast_tx");
        self.call("sendrawtransaction", json!([tx.as_hexstr()]))
            .await
    }

    /// Get the transaction identified by `txid`
    async fn get_tx(&self, txid: &str) -> Result<Tx, ChainGangError> {
        log::debug!("get_tx");

        let hexstr: String = self.call("getrawtransaction", json!([txid, false])).await?;
        let bytes = hex::decode(hexstr)?;
        let mut byte_slice = &bytes[..];
        Ok(Tx::read(&mut byte_slice)?)
    }

    /// Get the most recent block header
    async fn get_latest_block_header(&self) -> Result<BlockHeader, ChainGangError> {
        log::debug!("get_latest_block_header");

        let hash: String = self.call("getbestblockhash", json!([])).await?;
        let hexstr: String = self.call("getblockheader", json!([hash, false])).await?;
        let bytes = hex::decode(hexstr)?;
        let mut byte_slice = &bytes[..];
        Ok(BlockHeader::read(&mut byte_slice)?)
    }

    /// Get the block headers, as a JSON array of the most recent
    /// [`RECENT_HEADER_COUNT`] headers, tip last.
    ///
    /// There is no single RPC call for this, so it walks back from the tip. The
    /// WhatsOnChain backend returns whatever that service considers recent; the
    /// count here is this crate's choice.
    async fn get_block_headers(&self) -> Result<String, ChainGangError> {
        log::debug!("get_block_headers");

        let block_count = self.block_count().await?;
        let first = block_count.saturating_sub(RECENT_HEADER_COUNT.saturating_sub(1));

        let mut headers: Vec<Value> = Vec::new();
        for height in first..=block_count {
            let hash: String = self.call("getblockhash", json!([height])).await?;
            let header: Value = self.call("getblockheader", json!([hash, true])).await?;
            headers.push(header);
        }
        Ok(serde_json::to_string(&headers)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_adds_a_scheme_only_when_missing() {
        let plain = RpcInterface::new("127.0.0.1:18443", "u", "p", Network::BSV_Regtest);
        assert_eq!(plain.url, "http://127.0.0.1:18443");

        let with_http = RpcInterface::new("http://node:18443", "u", "p", Network::BSV_Regtest);
        assert_eq!(with_http.url, "http://node:18443");

        let with_https = RpcInterface::new("https://node:18443", "u", "p", Network::BSV_Regtest);
        assert_eq!(with_https.url, "https://node:18443");
    }

    #[test]
    fn set_network_replaces_the_configured_network() {
        let mut rpc = RpcInterface::new("127.0.0.1:18443", "u", "p", Network::BSV_Regtest);
        assert_eq!(rpc.network(), Network::BSV_Regtest);
        rpc.set_network(&Network::BSV_Testnet);
        assert_eq!(rpc.network(), Network::BSV_Testnet);
    }

    #[test]
    fn amounts_convert_to_whole_satoshis() {
        assert_eq!(RpcInterface::as_satoshis(0.0), 0);
        assert_eq!(RpcInterface::as_satoshis(1.0), 100_000_000);
        assert_eq!(RpcInterface::as_satoshis(0.000_000_01), 1);
        // 0.1 has no exact binary representation, so this rounds rather than
        // truncating to 9999999
        assert_eq!(RpcInterface::as_satoshis(0.1), 10_000_000);
        assert_eq!(
            RpcInterface::as_satoshis(21_000_000.0),
            2_100_000_000_000_000
        );
    }

    #[test]
    fn utxo_height_counts_back_from_the_tip() {
        // One confirmation means the output is in the tip block
        assert_eq!(RpcInterface::utxo_height(100, 1), 100);
        assert_eq!(RpcInterface::utxo_height(100, 2), 99);
        assert_eq!(RpcInterface::utxo_height(100, 100), 1);
        // The coinbase of the genesis block, on a chain of height 100
        assert_eq!(RpcInterface::utxo_height(100, 101), 0);
    }

    #[test]
    fn unconfirmed_utxo_height_is_negative() {
        // UtxoEntry documents a negative height as meaning unconfirmed, and the
        // in-memory interface's balance filters rely on it
        assert_eq!(RpcInterface::utxo_height(100, 0), -1);
        assert_eq!(RpcInterface::utxo_height(0, 0), -1);
    }

    #[test]
    fn rpc_error_response_is_reported_as_an_error() {
        // The shape a node returns for an unknown method
        let body = r#"{"result":null,"error":{"code":-32601,"message":"Method not found"},"id":"chain-gang"}"#;
        let parsed: RpcResponse<Value> = serde_json::from_str(body).unwrap();
        let err = parsed.error.expect("error field should be populated");
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }

    #[test]
    fn listunspent_entries_deserialise() {
        // Trimmed from a real regtest listunspent response
        let body = r#"[{
            "txid": "9d1a5b0e6c9a1d5e2f3a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90",
            "vout": 1,
            "address": "mwxr2K5Xn3nkVEbVmVLNTZiVeJ8nSHkZFR",
            "amount": 0.5,
            "confirmations": 3,
            "spendable": true
        }]"#;
        let unspent: Vec<Unspent> = serde_json::from_str(body).unwrap();
        assert_eq!(unspent.len(), 1);
        assert_eq!(unspent[0].vout, 1);
        assert_eq!(unspent[0].confirmations, 3);
        assert_eq!(RpcInterface::as_satoshis(unspent[0].amount), 50_000_000);
    }
}
