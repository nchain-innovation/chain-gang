// This module provides the blockchain interface

pub mod blockchain_interface;
pub mod woc_interface;

//#[cfg(test)]
pub mod test_interface;

pub use blockchain_interface::{Balance, BlockchainInterface, Utxo, UtxoEntry};
pub use woc_interface::WocInterface;

//#[cfg(test)]
pub use test_interface::TestInterface;
