
use async_trait::async_trait;
use reqwest::StatusCode;

use anyhow::{
    Result, anyhow,
};
use serde::Serialize;

use crate::{
    interface::blockchain_interface::{
        Balance,
        BlockchainInterface,
        Utxo,
    },
    network::Network,
    messages::Tx,
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

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance> {
        debug!("get_balance");

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
                debug!("address = {}", &address);
                return std::result::Result::Err(anyhow!("response.text() = {}", x));
            },
        };
        let data: Balance = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                debug!("address = {}", &address);
                warn!("txt = {}", &txt);
                return std::result::Result::Err(anyhow!("json parse error = {}", x));
            },
        };
        Ok(data)
    }

    /// Get UXTO associated with address
    async fn get_utxo(&self, address: &str) -> Result<Utxo> {
        debug!("get_utxo");
        let network = self.get_network_str();

        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}/unspent");
        let response = reqwest::get(&url).await?;
        if response.status() != 200 {
            warn!("url = {}", &url);
            return std::result::Result::Err(anyhow!("response.status() = {}", response.status()));
        };
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                return std::result::Result::Err(anyhow!("response.text() = {}", x));
            },
        };
        let data: Utxo = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                warn!("txt = {}", &txt);
                return std::result::Result::Err(anyhow!("json parse error = {}", x));
            },
        };
        Ok(data)
    }

    /// Broadcast Tx
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String> {
        debug!("broadcast_tx");
        let network = self.get_network_str();
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/tx/raw");
        debug!("url = {}", &url);
        let data_for_broadcast = BroadcastTxType {
            txhex: tx.as_hexstr(),
        };
        let data = serde_json::to_string(&data_for_broadcast).unwrap();
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&data)
            .send()
            .await?;
        
        // Assume a response of 200 means broadcast tx success
        match response.status() {
            StatusCode::OK => {
                let txid = tx.hash().encode();
                Ok(txid)
            },
            _ => {
                debug!("url = {}", &url);
                std::result::Result::Err(anyhow!("response.status() = {}", response.status()))
            },
        }
    }
}
