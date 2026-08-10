//! Blockchain interface for querying live chain state (balances, UTXOs, broadcast)
//! via backends such as WhatsOnChain and UaaS.
//!
//! Enabled by the `interface` feature flag.

/// Core `BlockchainInterface` trait and shared balance/UTXO types.
pub mod blockchain_interface;
/// UaaS (UTXO-as-a-Service) blockchain query backend.
pub mod uaas_interface;
/// WhatsOnChain blockchain query backend.
pub mod woc_interface;

//#[cfg(test)]
/// In-memory blockchain interface used for testing.
pub mod test_interface;

pub use blockchain_interface::{Balance, BlockchainInterface, Utxo, UtxoEntry};
pub use uaas_interface::{Monitor, UaaSInterface};
pub use woc_interface::WocInterface;

//#[cfg(test)]
pub use test_interface::TestInterface;
