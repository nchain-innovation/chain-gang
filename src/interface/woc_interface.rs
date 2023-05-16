use anyhow::Result;
use serde::Serialize;

use crate::{
    interface::blockchain_interface::{
        Balance,
        //BroadcastResponse,
        BlockchainInterface,
        Utxo,
    },
    network::Network,
};

/// Structure for json serialisation for broadcast_tx
#[derive(Debug, Serialize)]
pub struct BroadcastTxType {
    pub txhex: String,
}

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
            network_type: Network::BCH_Testnet,
        }
    }

    /// Return the current network as a string
    fn get_network_str(&self) -> &'static str {
        match self.network_type {
            Network::BSV_Mainnet => "main",
            Network::BSV_Testnet => "test",
            Network::BSV_STN => "stn",
            _ => "unknown",
        }
    }
}

impl BlockchainInterface for WocInterface {
    fn set_network(&mut self, network: &Network) {
        self.network_type = *network;
    }

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance> {
        let network = self.get_network_str();
        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}/balance");
        let response = reqwest::get(url).await?;
        let data = response.json::<Balance>().await?;
        dbg!(&address);
        dbg!(&data);
        Ok(data)
    }

    /// Get UXTO associated with address
    async fn get_utxo(&self, address: &str) -> Result<Utxo> {
        let network = self.get_network_str();

        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}/unspent");
        let response = reqwest::get(url).await?;
        let data = response.json::<Utxo>().await?;
        dbg!(&address);
        dbg!(&data);
        Ok(data)
    }

    /// Broadcast Tx
    async fn broadcast_tx(&self, tx: &str) -> Result<String> {
        println!("broadcast_tx");
        let network = self.get_network_str();

        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/tx/raw");
        dbg!(&url);
        let data_for_broadcast = BroadcastTxType {
            txhex: tx.to_string(),
        };
        let data = serde_json::to_string(&data_for_broadcast).unwrap();
        dbg!(&data);
        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(&data)
            .send()
            .await
            .expect("failedto get a response")
            .text()
            .await
            .expect("failedto get a payload");
        // TODO change this to a BroadcastResponse later

        Ok(response)
    }
}
