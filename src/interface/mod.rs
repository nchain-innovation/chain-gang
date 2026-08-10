//! Blockchain interface for querying live chain state (balances, UTXOs, broadcast)
//! via backends such as WhatsOnChain and UaaS.
//!
//! Enabled by the `interface` feature flag.

pub mod blockchain_interface;
pub mod uaas_interface;
pub mod woc_interface;

//#[cfg(test)]
pub mod test_interface;

pub use blockchain_interface::{Balance, BlockchainInterface, Utxo, UtxoEntry};
pub use uaas_interface::{Monitor, UaaSInterface};
pub use woc_interface::WocInterface;

//#[cfg(test)]
pub use test_interface::TestInterface;
