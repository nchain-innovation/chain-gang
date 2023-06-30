use async_trait::async_trait;

use crate::{messages::Tx, network::Network};
use anyhow::Result;
use serde::Deserialize;

//#[allow(unused_must_use)]

/// Balance returned from WoC
#[derive(Debug, Default, Deserialize, Clone, Copy)]
pub struct Balance {
    pub confirmed: i64,
    pub unconfirmed: i64,
}

/// Type to represent UTXO Entry
#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UtxoEntry {
    pub height: u32,
    pub tx_pos: u32,
    pub tx_hash: String,
    pub value: i64,
}
/// Type to represent UTXO set
pub type Utxo = Vec<UtxoEntry>;

/// Trait of the blockchain interface
///
#[async_trait]
pub trait BlockchainInterface: Send + Sync {
    fn set_network(&mut self, network: &Network);

    // Return Ok(()) if connection is good
    async fn status(&self) -> Result<()>;

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance>;

    /// Get UXTO associated with address
    async fn get_utxo(&self, address: &str) -> Result<Utxo>;

    /// Broadcast Tx, return the txid
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String>;

    async fn get_tx(&self, txid: &str) -> Result<Tx>;
}
