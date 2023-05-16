use crate::network::Network;
use anyhow::Result;
use serde::Deserialize;

//#[allow(unused_must_use)]

/// Balance returned from WoC
#[derive(Debug, Default, Deserialize, Clone, Copy)]
pub struct Balance {
    pub confirmed: u64,
    pub unconfirmed: u64,
}

/// Type to represent UTXO Entry
#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UtxoEntry {
    pub height: u32,
    pub tx_pos: u32,
    pub tx_hash: String,
    pub value: u64,
}
/// Type to represent UTXO set
pub type Utxo = Vec<UtxoEntry>;

pub struct BroadcastResponse {
    pub txid: String,
}

/// Trait of the blockchain interface
///
pub trait BlockchainInterface {
    fn set_network(&mut self, network: &Network);

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance>;

    /// Get UXTO associated with address
    async fn get_utxo(&self, address: &str) -> Result<Utxo>;

    /// Broadcast Tx
    async fn broadcast_tx(&self, tx: &str) -> Result<String>;
}
