use async_trait::async_trait;

use reqwest::StatusCode;
use reqwest::Url;

//use crate::util::Serializable;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::{
    interface::blockchain_interface::{Balance, BlockchainInterface, Utxo},
    // interface::woc_interface::BroadcastTxType,
    messages::{BlockHeader, Tx},
    network::Network,
    util::Serializable,
};

#[derive(Debug, Deserialize)]
pub struct UaaSStatus {
    pub version: Option<String>,
    pub network: String,
    #[serde(alias = "last block time")]
    pub last_block_time: String,
    #[serde(alias = "block height")]
    pub block_height: u64,
    #[serde(alias = "number of txs")]
    pub number_of_txs: u64,
    #[serde(alias = "number of utxo entries")]
    pub number_of_utxo_entries: u64,
    #[serde(alias = "number of mempool entries")]
    pub number_of_mempool_entries: u64,
}

#[derive(Debug, Deserialize)]
pub struct UaaSStatusResponse {
    pub status: UaaSStatus,
}


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

#[derive(Debug, Deserialize)]
pub struct HeaderFormat {
    pub height: u64,
    pub header: HeaderFields,
    pub blocksize: u64,
    #[serde(alias = "number of tx")]
    pub number_of_tx: u64,
}

#[derive(Debug, Deserialize)]
pub struct BlockHeadersResponse {
    pub blocks: Vec<HeaderFormat>,
}

#[derive(Debug, Deserialize)]
pub struct BlockHeaderHexResponse {
    pub block: String,
}

#[derive(Debug, Deserialize)]
pub struct TxResponse {
    pub result: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UaaSBroadcastTxType {
    pub tx: String,
}



#[derive(Debug, Clone)]
pub struct UaaSInterface {
    url: Url,
    network_type: Network,
}

/// UaaS specific funtionality
impl UaaSInterface {
    pub fn new(input_url: &str) -> Result<Self> {
        // Check this is a valid URL
        let url = Url::parse(input_url)?;

        Ok(UaaSInterface {
            url,
            network_type: Network::BSV_Testnet,
        })
    }

    // Return Ok(UaaSStatusResponse) if UaaS responds...
    pub async fn get_uaas_status(&self) -> Result<UaaSStatusResponse> {
        log::debug!("status");

        let status_url = self.url.join("/status").unwrap();
        let response = reqwest::get(status_url.clone()).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &status_url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(err) => return std::result::Result::Err(anyhow!("response.text() = {}", err)),
        };

        let status: UaaSStatusResponse = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::warn!("txt = {}", &txt);
                return std::result::Result::Err(anyhow!("json parse error = {}", x));
            }
        };
        Ok(status)
    }

    pub async fn get_uaas_block_headers(&self) -> Result<BlockHeadersResponse> {
        log::debug!("get_uaas_block_headers");

        let status_url = self.url.join("/block/latest").unwrap();
        let response = reqwest::get(status_url.clone()).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &status_url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };

        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => return std::result::Result::Err(anyhow!("response.text() = {}", x)),
        };

        let blockheaders: BlockHeadersResponse = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::warn!("txt = {}", &txt);
                return std::result::Result::Err(anyhow!("json parse error = {}", x));
            }
        };

        Ok(blockheaders)
    }
}

#[async_trait]
impl BlockchainInterface for UaaSInterface {
    fn set_network(&mut self, network: &Network) {
        self.network_type = *network;
    }

    // Return Ok(()) if UaaS responds...
    async fn status(&self) -> Result<()> {
        log::debug!("status");

        let status_url = self.url.join("/status").unwrap();
        let response = reqwest::get(status_url.clone()).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &status_url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        match response.text().await {
            Ok(_txt) => Ok(()),
            Err(err) => std::result::Result::Err(anyhow!("response.text() = {}", err)),
        }
    }

    /// Get balance associated with address
    async fn get_balance(&self, _address: &str) -> Result<Balance> {
        log::debug!("get_balance");
        std::unimplemented!();
        /*
        let status_url = self.url.join("/status").unwrap();

        let response = reqwest::get(status_url.clone()).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &status_url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        */

        /*
        let network = self.get_network_str();
        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}/balance");
        let response = reqwest::get(&url).await?;
        if response.status() != 200 {
            warn!("url = {}", &url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                log::debug!("address = {}", &address);
                return std::result::Result::Err(anyhow!("response.text() = {}", x));
            }
        };
        let data: Balance = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::debug!("address = {}", &address);
                log::warn!("txt = {}", &txt);
                return std::result::Result::Err(anyhow!("json parse error = {}", x));
            }
        };
        Ok(data)
         */
    }

    /// Get UXTO associated with address
    async fn get_utxo(&self, _address: &str) -> Result<Utxo> {
        log::debug!("get_utxo");
        std::unimplemented!();
        /*
        let network = self.get_network_str();

        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}/unspent");
        let response = reqwest::get(&url).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                return std::result::Result::Err(anyhow!("response.text() = {}", x));
            }
        };
        let data: Utxo = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::warn!("txt = {}", &txt);
                return std::result::Result::Err(anyhow!("json parse error = {}", x));
            }
        };
        Ok(data)
        */
    }

    /// Broadcast Tx
    ///
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String> {
        log::debug!("broadcast_tx");

        let url = self.url.join(&"/tx/hex").unwrap();

        let data_for_broadcast = UaaSBroadcastTxType {
            tx: tx.as_hexstr(),
        };

        let client = reqwest::Client::new();
        let response = client.post(url.clone()).json(&data_for_broadcast).send().await?;
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
                std::result::Result::Err(anyhow!("response.status() = {}", status))
            }
        }
    }

    async fn get_tx(&self, txid: &str) -> Result<Tx> {
        log::debug!("get_tx");

        let get_tx_url = format!("/collection/tx/hex?hash={}", txid);
        let url = self.url.join(&get_tx_url).unwrap();

        let response = reqwest::get(url.clone()).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                return std::result::Result::Err(anyhow!("response.text() = {}", x));
            }
        };

        let data: TxResponse = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::warn!("txt = {}", &txt);
                return std::result::Result::Err(anyhow!("json parse error = {}", x));
            }
        };

        let bytes = hex::decode(data.result)?;
        let mut byte_slice = &bytes[..];
        let tx: Tx = Tx::read(&mut byte_slice)?;
        Ok(tx)
    }

    async fn get_latest_block_header(&self) -> Result<BlockHeader> {
        log::debug!("get_latest_block_header");

        let url = self.url.join("/block/last/hex").unwrap();

        let response = reqwest::get(url.clone()).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                return std::result::Result::Err(anyhow!("response.text() = {}", x));
            }
        };

        let data: BlockHeaderHexResponse = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::warn!("txt = {}", &txt);
                return std::result::Result::Err(anyhow!("json parse error = {}", x));
            }
        };

        let bytes = hex::decode(data.block)?;
        let mut byte_slice = &bytes[..];
        let blockheader: BlockHeader = BlockHeader::read(&mut byte_slice)?;
        Ok(blockheader)
    }

    async fn get_block_headers(&self) -> Result<String> {
        log::debug!("get_block_headers");

        let status_url = self.url.join("/block/latest").unwrap();
        let response = reqwest::get(status_url.clone()).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &status_url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };

        return match response.text().await {
            Ok(headers) => Ok(headers),
            Err(x) => std::result::Result::Err(anyhow!("response.text() = {}", x)),
        };
    }
}
