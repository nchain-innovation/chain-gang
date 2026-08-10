use async_trait::async_trait;

use crate::{
    messages::{BlockHeader, Tx},
    network::Network,
    util::ChainGangError,
};
use serde::Deserialize;

//#[allow(unused_must_use)]

/// Balance returned from WoC
#[derive(Debug, Default, Deserialize, Clone, Copy)]
pub struct Balance {
    /// Confirmed balance in satoshis
    pub confirmed: i64,
    /// Unconfirmed balance in satoshis
    pub unconfirmed: i64,
}

/// Type to represent UTXO Entry
#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UtxoEntry {
    /// Block height at which the UTXO was confirmed (negative if unconfirmed)
    pub height: i32,
    /// Output index within the transaction
    pub tx_pos: u32,
    /// Hex-encoded transaction ID containing the output
    pub tx_hash: String,
    /// Output value in satoshis
    pub value: i64,
}
/// Type to represent UTXO set
pub type Utxo = Vec<UtxoEntry>;

/// Trait of the blockchain interface
///
#[async_trait]
pub trait BlockchainInterface: Send + Sync {
    /// Set the network this interface operates on
    fn set_network(&mut self, network: &Network);

    /// Return `Ok(())` if the connection to the blockchain service is good
    async fn status(&self) -> Result<(), ChainGangError>;

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainGangError>;

    /// Get UXTO associated with address
    async fn get_utxo(&self, address: &str) -> Result<Utxo, ChainGangError>;

    /// Broadcast Tx, return the txid
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String, ChainGangError>;

    /// Get the transaction identified by `txid`
    async fn get_tx(&self, txid: &str) -> Result<Tx, ChainGangError>;

    /// Get the most recent block header
    async fn get_latest_block_header(&self) -> Result<BlockHeader, ChainGangError>;

    /// Get the block headers
    async fn get_block_headers(&self) -> Result<String, ChainGangError>;
}
