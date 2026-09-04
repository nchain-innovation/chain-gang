//! WhatsOnChain blockchain interface (Rust implementation).
//!
//! A parallel, independent pure-Python client lives in
//! `python/src/tx_engine/interface/woc.py` and `woc_interface.py`. The
//! duplication is deliberate: it lets the Python package talk to WhatsOnChain
//! without depending on this crate's `interface` feature. When changing
//! endpoint paths or the network -> `main`/`test`/`stn` mapping, update both
//! implementations so they stay in sync.

use async_trait::async_trait;
use reqwest::StatusCode;

use crate::util::Serializable;
use serde::Serialize;

use crate::{
    interface::blockchain_interface::{
        check_status, Balance, BlockchainInterface, Utxo, UNCONFIRMED_HEIGHT,
    },
    messages::{BlockHeader, Tx},
    network::Network,
    util::ChainGangError,
};

/// Structure for json serialisation for broadcast_tx
#[derive(Debug, Serialize)]
struct BroadcastTxType {
    pub txhex: String,
}

/// Blockchain interface backed by the WhatsOnChain API.
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
    /// Create a new `WocInterface` defaulting to the BSV testnet.
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
            // WhatsOnChain serves no other chain. Listed rather than caught so
            // a new network has to choose; the panic itself is #148.
            Network::BSV_Regtest
            | Network::BTC_Mainnet
            | Network::BTC_Testnet
            | Network::BCH_Mainnet
            | Network::BCH_Testnet => {
                panic!("unknown network {}", self.network_type)
            }
        }
    }
}

/// Rewrites WhatsOnChain's unconfirmed marker to this crate's.
///
/// WhatsOnChain reports a mempool UTXO as `height: 0`, while
/// [`UtxoEntry::height`] defines a negative height as meaning unconfirmed, so
/// the value is translated on the way in rather than left for every caller to
/// special-case. A height of 0 cannot mean the genesis block here: the genesis
/// coinbase is unspendable, so it never appears in an unspent set.
fn normalise_unconfirmed(utxo: &mut Utxo) {
    for entry in utxo.iter_mut() {
        if entry.height == 0 {
            entry.height = UNCONFIRMED_HEIGHT;
        }
    }
}

#[async_trait]
impl BlockchainInterface for WocInterface {
    fn set_network(&mut self, network: &Network) {
        self.network_type = *network;
    }

    // Return Ok(()) if connection is good
    async fn status(&self) -> Result<(), ChainGangError> {
        log::debug!("status");

        let network = self.get_network_str();
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/woc");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        match response.text().await {
            Ok(txt) if txt == "Whats On Chain" => Ok(()),
            Ok(txt) => Err(ChainGangError::ResponseError(format!(
                "Unexpected txt = {}",
                txt
            ))),
            Err(err) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                err
            ))),
        }
    }

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainGangError> {
        log::debug!("get_balance");

        let network = self.get_network_str();
        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}/balance");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                log::debug!("address = {}", address);
                return Err(ChainGangError::ResponseError(format!(
                    "response.text() = {}",
                    x
                )));
            }
        };
        let data: Balance = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::debug!("address = {}", address);
                log::warn!("txt = {}", txt);
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
        let network = self.get_network_str();

        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}/unspent");
        let response = reqwest::get(&url).await?;
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
        let mut data: Utxo = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::warn!("txt = {}", txt);
                return Err(ChainGangError::JSONParseError(format!(
                    "json parse error = {}",
                    x
                )));
            }
        };
        normalise_unconfirmed(&mut data);
        Ok(data)
    }

    /// Broadcast Tx
    ///
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String, ChainGangError> {
        log::debug!("broadcast_tx");
        let network = self.get_network_str();
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/tx/raw");
        log::debug!("url = {}", url);
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
                log::debug!("url = {}", url);
                Err(ChainGangError::ResponseError(format!(
                    "response.status() = {}",
                    status
                )))
            }
        }
    }

    async fn get_tx(&self, txid: &str) -> Result<Tx, ChainGangError> {
        log::debug!("get_tx");

        let network = self.get_network_str();
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/tx/{txid}/hex");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        match response.text().await {
            Ok(txt) => {
                let bytes = hex::decode(txt)?;
                let mut byte_slice = &bytes[..];
                let tx: Tx = Tx::read(&mut byte_slice)?;
                Ok(tx)
            }
            Err(x) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                x
            ))),
        }
    }

    async fn get_latest_block_header(&self) -> Result<BlockHeader, ChainGangError> {
        log::debug!("get_latest_block_header");
        let network = self.get_network_str();
        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/block/headers/latest?count=1");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        match response.text().await {
            Ok(txt) => {
                let bytes = hex::decode(txt)?;
                let mut byte_slice = &bytes[..];
                let blockheader: BlockHeader = BlockHeader::read(&mut byte_slice)?;
                Ok(blockheader)
            }
            Err(x) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                x
            ))),
        }
    }

    async fn get_block_headers(&self) -> Result<String, ChainGangError> {
        log::debug!("get_block_headers");
        let network = self.get_network_str();
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/block/headers");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        match response.text().await {
            Ok(headers) => Ok(headers),
            Err(x) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                x
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::blockchain_interface::UtxoEntry;

    /// A trimmed response from `/address/{addr}/unspent`, with the mempool
    /// entry WhatsOnChain reports as height 0. Taken from a real response:
    /// tx fa7f15f8..d860 was in WhatsOnChain's own mempool/raw list, with no
    /// blockhash, blockheight or confirmations.
    fn woc_response() -> &'static str {
        r#"[
            {"height": 964754, "tx_pos": 1, "tx_hash": "aa", "value": 1},
            {"height": 0,      "tx_pos": 1, "tx_hash": "fa7f15f8", "value": 2954693}
        ]"#
    }

    #[test]
    fn woc_height_zero_becomes_the_unconfirmed_sentinel() {
        let mut utxo: Utxo = serde_json::from_str(woc_response()).unwrap();
        assert_eq!(utxo[1].height, 0, "as WhatsOnChain sends it");

        normalise_unconfirmed(&mut utxo);

        assert_eq!(utxo[1].height, UNCONFIRMED_HEIGHT);
        assert!(utxo[1].height < 0, "the balance filters test for negative");
        // and the confirmed entry is untouched
        assert_eq!(utxo[0].height, 964754);
        assert_eq!(utxo[1].value, 2954693, "only the height is rewritten");
    }

    #[test]
    fn normalising_is_idempotent() {
        let mut utxo = vec![UtxoEntry {
            height: UNCONFIRMED_HEIGHT,
            tx_pos: 0,
            tx_hash: "aa".to_string(),
            value: 1,
        }];
        normalise_unconfirmed(&mut utxo);
        assert_eq!(utxo[0].height, UNCONFIRMED_HEIGHT);
    }

    #[test]
    fn an_empty_utxo_set_is_fine() {
        let mut utxo: Utxo = Vec::new();
        normalise_unconfirmed(&mut utxo);
        assert!(utxo.is_empty());
    }
}
