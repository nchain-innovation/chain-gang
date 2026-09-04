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

/// Height reported for a UTXO that has not been confirmed in a block.
///
/// Any negative height means unconfirmed; this is the value the interfaces in
/// this crate emit, and the one the parallel Python clients emit too.
pub const UNCONFIRMED_HEIGHT: i32 = -1;

// The balance filters distinguish unconfirmed outputs by testing for a negative
// height, so the sentinel has to stay negative
const _: () = assert!(UNCONFIRMED_HEIGHT < 0);

/// Type to represent UTXO Entry
#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UtxoEntry {
    /// Block height at which the UTXO was confirmed, or a negative height if it
    /// is unconfirmed. See [`UNCONFIRMED_HEIGHT`].
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

/// Returns `response` unchanged if its HTTP status is 200, otherwise logs the
/// request URL and returns a [`ChainGangError::ResponseError`].
///
/// Shared by the HTTP-backed blockchain interfaces so the status check lives in
/// one place.
pub(crate) fn check_status(
    response: reqwest::Response,
    url: impl std::fmt::Display,
) -> Result<reqwest::Response, ChainGangError> {
    if response.status() != 200 {
        log::warn!("url = {}", url);
        return Err(ChainGangError::ResponseError(format!(
            "response.status() = {}",
            response.status()
        )));
    }
    Ok(response)
}

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
