use async_trait::async_trait;

use reqwest::StatusCode;
use reqwest::Url;

use serde::{Deserialize, Serialize};

use crate::{
    interface::blockchain_interface::{check_status, Balance, BlockchainInterface, Utxo},
    messages::{BlockHeader, Tx},
    network::Network,
    util::{ChainGangError, Serializable},
};

/// Status information reported by a UaaS node.
#[derive(Debug, Deserialize)]
pub struct UaaSStatus {
    /// UaaS software version, if reported.
    pub version: Option<String>,
    /// Network the node is running on (e.g. `main`, `test`).
    pub network: String,
    /// Timestamp of the most recent block.
    #[serde(alias = "last block time")]
    pub last_block_time: String,
    /// Height of the chain tip.
    #[serde(alias = "block height")]
    pub block_height: u64,
    /// Total number of transactions known to the node.
    #[serde(alias = "number of txs")]
    pub number_of_txs: u64,
    /// Number of entries in the UTXO set.
    #[serde(alias = "number of utxo entries")]
    pub number_of_utxo_entries: u64,
    /// Number of entries in the mempool.
    #[serde(alias = "number of mempool entries")]
    pub number_of_mempool_entries: u64,
}

/// Response wrapper for the UaaS `/status` endpoint.
#[derive(Debug, Deserialize)]
pub struct UaaSStatusResponse {
    /// The node status payload.
    pub status: UaaSStatus,
}

/// Decoded fields of a block header as returned by UaaS.
#[allow(non_snake_case, dead_code)]
#[derive(Debug, Deserialize)]
pub struct HeaderFields {
    hash: String,
    version: String,
    hashPrevBlock: String,
    hashMerkleRoot: String,
    nTime: String,
    nBits: String,
    nNonce: String,
}

/// A block header together with associated block metadata.
#[derive(Debug, Deserialize)]
pub struct HeaderFormat {
    /// Height of the block.
    pub height: u64,
    /// Decoded block header fields.
    pub header: HeaderFields,
    /// Size of the block in bytes.
    pub blocksize: u64,
    /// Number of transactions in the block.
    #[serde(alias = "number of tx")]
    pub number_of_tx: u64,
}

/// Response wrapper for a list of block headers.
#[derive(Debug, Deserialize)]
pub struct BlockHeadersResponse {
    /// The returned block headers.
    pub blocks: Vec<HeaderFormat>,
}

/// Response wrapper carrying a hex-encoded block header.
#[derive(Debug, Deserialize)]
pub struct BlockHeaderHexResponse {
    /// Hex-encoded block header.
    pub block: String,
}

/// Response wrapper carrying a hex-encoded transaction.
#[derive(Debug, Deserialize)]
pub struct TxResponse {
    /// Hex-encoded transaction.
    pub result: String,
}

/// Request body for broadcasting a transaction to UaaS.
#[derive(Debug, Serialize, Deserialize)]
pub struct UaaSBroadcastTxType {
    /// Hex-encoded transaction to broadcast.
    pub tx: String,
}

/// Blockchain interface backed by a UaaS (UTXO-as-a-Service) node.
#[derive(Debug, Clone)]
pub struct UaaSInterface {
    url: Url,
    network_type: Network,
}

// This represents an address or locking script monitor
/// A UaaS monitor tracking an address or locking script pattern.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Monitor {
    /// Name identifying the monitor.
    pub name: String,
    /// Whether to also track descendant transactions.
    pub track_descendants: bool,
    /// Address to monitor, if monitoring by address.
    pub address: Option<String>,
    /// Locking script pattern to monitor, if monitoring by script.
    pub locking_script_pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetMonitorResponse {
    pub collections: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GetUtxoResponse {
    pub utxo: Utxo,
}

/// UaaS specific funtionality
impl UaaSInterface {
    /// Create a new `UaaSInterface` targeting the UaaS node at `input_url`.
    pub fn new(input_url: &str) -> Result<Self, ChainGangError> {
        // Check this is a valid URL
        let url = Url::parse(input_url)?;

        Ok(UaaSInterface {
            url,
            network_type: Network::BSV_Testnet,
        })
    }

    // Return Ok(UaaSStatusResponse) if UaaS responds...
    /// Query the node's `/status` endpoint and return its status.
    pub async fn get_uaas_status(&self) -> Result<UaaSStatusResponse, ChainGangError> {
        log::debug!("status");

        let status_url = self.url.join("/status").unwrap();
        let response = reqwest::get(status_url.clone()).await?;
        let response = check_status(response, &status_url)?;
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(err) => {
                return Err(ChainGangError::ResponseError(format!(
                    "response.text() = {}",
                    err
                )))
            }
        };

        let status: UaaSStatusResponse = serde_json::from_str(&txt)?;
        Ok(status)
    }

    /// Fetch the latest block headers known to the node.
    pub async fn get_uaas_block_headers(&self) -> Result<BlockHeadersResponse, ChainGangError> {
        log::debug!("get_uaas_block_headers");

        let status_url = self.url.join("/block/latest").unwrap();
        let response = reqwest::get(status_url.clone()).await?;
        let response = check_status(response, &status_url)?;

        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                return Err(ChainGangError::ResponseError(format!(
                    "response.text() = {}",
                    x
                )))
            }
        };

        let blockheaders: BlockHeadersResponse = serde_json::from_str(&txt)?;

        Ok(blockheaders)
    }

    /// Return the names of the monitor collections configured on the node.
    pub async fn get_monitors(&self) -> Result<Vec<String>, ChainGangError> {
        log::debug!("get_monitors");

        let collection_url = self.url.join("/collection").unwrap();
        let response = reqwest::get(collection_url.clone()).await?;
        let response = check_status(response, &collection_url)?;

        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                return Err(ChainGangError::ResponseError(format!(
                    "response.text() = {}",
                    x
                )))
            }
        };

        let monitors: GetMonitorResponse = serde_json::from_str(&txt)?;
        Ok(monitors.collections)
    }

    /// Register a new monitor on the node.
    pub async fn add_monitor(&self, monitor: &Monitor) -> Result<(), ChainGangError> {
        log::debug!("add_monitor");
        // check the input is valid
        if monitor.address.is_none() && monitor.locking_script_pattern.is_none() {
            return Err(ChainGangError::BadArgument(
                "monitor requires address or locking_script pattern".to_string(),
            ));
        }

        let add_monitor_url = self.url.join("/collection/monitor").unwrap();
        let client = reqwest::Client::new();
        let response = client
            .post(add_monitor_url.clone())
            .json(&monitor)
            .send()
            .await?;

        check_status(response, &add_monitor_url)?;
        Ok(())
    }

    /// Remove the monitor with the given name from the node.
    pub async fn delete_monitor(&self, monitor_name: &str) -> Result<(), ChainGangError> {
        log::debug!("delete_monitor");

        let delete_url = format!("/collection/monitor?monitor_name={}", monitor_name);
        let delete_monitor_url = self.url.join(&delete_url).unwrap();
        let client = reqwest::Client::new();

        let response = client.delete(delete_monitor_url.clone()).send().await?;

        check_status(response, &delete_monitor_url)?;
        Ok(())
    }
}

#[async_trait]
impl BlockchainInterface for UaaSInterface {
    fn set_network(&mut self, network: &Network) {
        self.network_type = *network;
    }

    // Return Ok(()) if UaaS responds...
    async fn status(&self) -> Result<(), ChainGangError> {
        log::debug!("status");

        let status_url = self.url.join("/status").unwrap();
        let response = reqwest::get(status_url.clone()).await?;
        let response = check_status(response, &status_url)?;
        match response.text().await {
            Ok(_txt) => Ok(()),
            Err(err) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                err
            ))),
        }
    }

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainGangError> {
        log::debug!("get_balance");
        let get_utxo_balance_url = format!("/utxo/balance?address={}", address);

        let url = self.url.join(&get_utxo_balance_url).unwrap();

        let response = reqwest::get(url.clone()).await?;
        let response = check_status(response, &url)?;

        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                log::debug!("address = {}", &address);
                return Err(ChainGangError::ResponseError(format!(
                    "response.text() = {}",
                    x
                )));
            }
        };
        let data: Balance = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::debug!("address = {}", &address);
                log::warn!("txt = {}", &txt);
                return Err(ChainGangError::JSONParseError(format!(
                    "json parse error = {}",
                    x
                )));
            }
        };
        Ok(data)
    }

    /// Get UXTO associated with address
    async fn get_utxo(&self, address: &str) -> Result<Utxo, ChainGangError> {
        log::debug!("get_utxo");

        let get_utxo_url = format!("/utxo/get?address={}", address);

        let url = self.url.join(&get_utxo_url).unwrap();

        let response = reqwest::get(url.clone()).await?;
        let response = check_status(response, &url)?;

        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                return Err(ChainGangError::ResponseError(format!(
                    "response.text() = {}",
                    x
                )));
            }
        };
        let data: GetUtxoResponse = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::warn!("txt = {}", &txt);
                return Err(ChainGangError::JSONParseError(format!(
                    "json parse error = {}",
                    x
                )));
            }
        };
        Ok(data.utxo)
    }

    /// Broadcast Tx
    ///
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String, ChainGangError> {
        log::debug!("broadcast_tx");

        let url = self.url.join("/tx/hex").unwrap();

        let data_for_broadcast = UaaSBroadcastTxType { tx: tx.as_hexstr() };

        let client = reqwest::Client::new();
        let response = client
            .post(url.clone())
            .json(&data_for_broadcast)
            .send()
            .await?;
        let status = response.status();

        // Assume a response of 200 means broadcast tx success
        match status {
            StatusCode::OK => {
                let res = response.text().await?;
                let hash = res.trim();
                let txid = hash.trim_matches('"');
                Ok(txid.to_string())
            }
            _ => {
                log::debug!("url = {}", &url);
                Err(ChainGangError::ResponseError(format!(
                    "response.status() = {}",
                    status
                )))
            }
        }
    }

    async fn get_tx(&self, txid: &str) -> Result<Tx, ChainGangError> {
        log::debug!("get_tx");

        let get_tx_url = format!("/tx/hex?hash={}", txid);
        let url = self.url.join(&get_tx_url).unwrap();

        let response = reqwest::get(url.clone()).await?;
        let response = check_status(response, &url)?;
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                return Err(ChainGangError::ResponseError(format!(
                    "response.text() = {}",
                    x
                )));
            }
        };

        let data: TxResponse = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::warn!("txt = {}", &txt);
                return Err(ChainGangError::JSONParseError(format!(
                    "json parse error = {}",
                    x
                )));
            }
        };

        let bytes = hex::decode(data.result)?;
        let mut byte_slice = &bytes[..];
        let tx: Tx = Tx::read(&mut byte_slice)?;
        Ok(tx)
    }

    async fn get_latest_block_header(&self) -> Result<BlockHeader, ChainGangError> {
        log::debug!("get_latest_block_header");

        let url = self.url.join("/block/last/hex").unwrap();

        let response = reqwest::get(url.clone()).await?;
        let response = check_status(response, &url)?;
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                return Err(ChainGangError::ResponseError(format!(
                    "response.text() = {}",
                    x
                )));
            }
        };

        let data: BlockHeaderHexResponse = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::warn!("txt = {}", &txt);
                return Err(ChainGangError::JSONParseError(format!(
                    "json parse error = {}",
                    x
                )));
            }
        };

        let bytes = hex::decode(data.block)?;
        let mut byte_slice = &bytes[..];
        let blockheader: BlockHeader = BlockHeader::read(&mut byte_slice)?;
        Ok(blockheader)
    }

    async fn get_block_headers(&self) -> Result<String, ChainGangError> {
        log::debug!("get_block_headers");

        let status_url = self.url.join("/block/latest").unwrap();
        let response = reqwest::get(status_url.clone()).await?;
        let response = check_status(response, &status_url)?;

        return match response.text().await {
            Ok(headers) => Ok(headers),
            Err(x) => Err(ChainGangError::JSONParseError(format!(
                "response.text() = {}",
                x
            ))),
        };
    }
}
