use async_trait::async_trait;
use reqwest::StatusCode;

use crate::util::Serializable;
use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::{
    interface::blockchain_interface::{Balance, BlockchainInterface, Utxo},
    messages::Tx,
    network::Network,
};

/// Structure for json serialisation for broadcast_tx
#[derive(Debug, Serialize)]
pub struct BroadcastTxType {
    pub txhex: String,
}

#[derive(Debug, Clone)]
pub struct WocInterface {
    network_type: Network,
}

impl Default for WocInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl WocInterface {
    pub fn new() -> Self {
        WocInterface {
            network_type: Network::BSV_Testnet,
        }
    }

    /// Return the current network as a string
    fn get_network_str(&self) -> &'static str {
        match self.network_type {
            Network::BSV_Mainnet => "main",
            Network::BSV_Testnet => "test",
            Network::BSV_STN => "stn",
            _ => panic!("unknown network {}", &self.network_type),
        }
    }
}

#[async_trait]
impl BlockchainInterface for WocInterface {
    fn set_network(&mut self, network: &Network) {
        self.network_type = *network;
    }

    // Return Ok(()) if connection is good
    async fn status(&self) -> Result<()> {
        log::debug!("status");

        let network = self.get_network_str();
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/woc");
        let response = reqwest::get(&url).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        match response.text().await {
            Ok(txt) if txt == "Whats On Chain" => Ok(()),
            Ok(txt) => std::result::Result::Err(anyhow!("Unexpected txt = {}", txt)),
            Err(err) => std::result::Result::Err(anyhow!("response.text() = {}", err)),
        }
    }

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance> {
        log::debug!("get_balance");

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
    }

    /// Get UXTO associated with address
    async fn get_utxo(&self, address: &str) -> Result<Utxo> {
        log::debug!("get_utxo");
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
    }

    /// Broadcast Tx
    ///
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String> {
        log::debug!("broadcast_tx");
        let network = self.get_network_str();
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/tx/raw");
        log::debug!("url = {}", &url);
        let data_for_broadcast = BroadcastTxType {
            txhex: tx.as_hexstr(),
        };
        //let data = serde_json::to_string(&data_for_broadcast).unwrap();
        let client = reqwest::Client::new();
        let response = client.post(&url).json(&data_for_broadcast).send().await?;
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

        let network = self.get_network_str();

        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/tx/{txid}/hex");
        let response = reqwest::get(&url).await?;
        if response.status() != 200 {
            log::warn!("url = {}", &url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        match response.text().await {
            Ok(txt) => {
                let bytes = hex::decode(txt)?;
                let mut byte_slice = &bytes[..];
                let tx: Tx = Tx::read(&mut byte_slice)?;
                Ok(tx)
            }
            Err(x) => std::result::Result::Err(anyhow!("response.text() = {}", x)),
        }
    }
}
