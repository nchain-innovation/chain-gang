use async_trait::async_trait;

use async_mutex::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    interface::blockchain_interface::{Balance, BlockchainInterface, Utxo},
    messages::{BlockHeader, Tx},
    network::Network,
    util::ChainGangError,
};

/// TestData - is the data used to set up a a test fixture and can be used to capture broadcast transactions
#[derive(Debug, Default, Clone)]
pub struct TestData {
    utxo: HashMap<String, Utxo>,
    height: u32,
    broadcast: Vec<String>,
}

/// Number of confirmations required before a UTXO is treated as confirmed
const REQUIRED_CONFIRMATIONS: u32 = 6;

/// Highest block height that is still deemed confirmed at the given chain height.
///
/// The result is signed so that a chain shorter than `REQUIRED_CONFIRMATIONS`
/// yields a negative threshold — no UTXO can be confirmed yet — rather than
/// underflowing. Heights beyond `i32::MAX` saturate instead of wrapping.
fn confirmation_height(height: u32) -> i32 {
    i32::try_from(height)
        .unwrap_or(i32::MAX)
        .saturating_sub(REQUIRED_CONFIRMATIONS as i32)
}

/// Mock `BlockchainInterface` implementation backed by in-memory `TestData`
#[derive(Debug, Clone)]
pub struct TestInterface {
    network_type: Network,
    /// TestData  is separated and enclosed in a Mutex to provide interior mutablity.
    test_data: Arc<Mutex<TestData>>,
}

impl Default for TestInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl TestInterface {
    /// Create a new `TestInterface` with empty test data on BCH testnet
    pub fn new() -> Self {
        TestInterface {
            network_type: Network::BCH_Testnet,
            test_data: Arc::new(Mutex::new(TestData::default())),
        }
    }

    /// Populate the interface with the UTXO set and block height from `test_data`
    pub async fn set_test_data(&mut self, test_data: &TestData) {
        // Check there is no broadcast data
        assert!(test_data.broadcast.is_empty());

        for (addr, utxo) in &test_data.utxo {
            self.set_utxo(addr, utxo).await;
        }
        self.set_height(test_data.height).await;
    }

    /// Set the UTXO set associated with `address`
    pub async fn set_utxo(&self, address: &str, utxo: &Utxo) {
        let mut test_data = self.test_data.lock().await;
        test_data.utxo.insert(address.to_string(), utxo.to_vec());
    }

    /// Set the current block height used for confirmation calculations
    pub async fn set_height(&self, height: u32) {
        let mut test_data = self.test_data.lock().await;
        test_data.height = height;
    }
}

#[async_trait]
impl BlockchainInterface for TestInterface {
    fn set_network(&mut self, network: &Network) {
        self.network_type = *network;
    }

    // Return Ok(()) if connection is good
    async fn status(&self) -> Result<(), ChainGangError> {
        Ok(())
    }

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainGangError> {
        debug!("get_balance");

        let utxo: Utxo = self.get_utxo(address).await?;
        let test_data = self.test_data.lock().await;

        let confirmation_height: i32 = confirmation_height(test_data.height);

        let confirmed: i64 = utxo
            .iter()
            .filter(|x| x.height >= 0 && x.height <= confirmation_height)
            .map(|x| x.value)
            .sum();

        let unconfirmed: i64 = utxo
            .iter()
            .filter(|x| x.height < 0 || x.height > confirmation_height)
            .map(|x| x.value)
            .sum();

        let balance = Balance {
            confirmed,
            unconfirmed,
        };
        Ok(balance)
    }

    /// Get UXTO associated with address
    async fn get_utxo(&self, address: &str) -> Result<Utxo, ChainGangError> {
        debug!("get_utxo");

        let test_data = self.test_data.lock().await;

        match test_data.utxo.get(address) {
            Some(value) => Ok(value.to_vec()),
            None => Ok(Vec::new()),
        }
    }

    /// Broadcast Tx
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String, ChainGangError> {
        debug!("broadcast_tx");
        let mut test_data = self.test_data.lock().await;

        // Record tx
        test_data.broadcast.push(tx.as_hexstr());

        // Return hex
        let txid = tx.hash().encode();
        Ok(txid)
    }

    async fn get_tx(&self, _txid: &str) -> Result<Tx, ChainGangError> {
        debug!("get_tx");
        std::unimplemented!();
    }

    async fn get_latest_block_header(&self) -> Result<BlockHeader, ChainGangError> {
        debug!("get_latest_block_header");
        std::unimplemented!();
    }

    async fn get_block_headers(&self) -> Result<String, ChainGangError> {
        debug!("get_block_headers");
        std::unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_height_below_required_confirmations_is_negative() {
        // A chain shorter than six blocks has nothing confirmed yet. The
        // threshold must go negative rather than underflow (issue #139).
        assert_eq!(confirmation_height(0), -6);
        assert_eq!(confirmation_height(1), -5);
        assert_eq!(confirmation_height(5), -1);
    }

    #[test]
    fn confirmation_height_at_and_above_required_confirmations() {
        assert_eq!(confirmation_height(6), 0);
        assert_eq!(confirmation_height(7), 1);
        assert_eq!(confirmation_height(100), 94);
    }

    #[test]
    fn confirmation_height_saturates_instead_of_wrapping() {
        assert_eq!(confirmation_height(i32::MAX as u32), i32::MAX - 6);
        assert_eq!(confirmation_height(u32::MAX), i32::MAX - 6);
    }

    #[test]
    fn nothing_is_confirmed_on_a_shallow_chain() {
        // At height 0 the filters in get_balance must treat every UTXO as
        // unconfirmed, including one mined in the genesis block.
        let threshold = confirmation_height(0);
        for height in [-1_i32, 0, 1] {
            assert!(
                !(height >= 0 && height <= threshold),
                "height {height} confirmed"
            );
            assert!(
                height < 0 || height > threshold,
                "height {height} not unconfirmed"
            );
        }
    }
}
