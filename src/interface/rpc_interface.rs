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

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use crate::{network::Network, util::ChainGangError};

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
