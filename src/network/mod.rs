//! Configuration for mainnet and testnet
//!
//! # Examples
//!
//! Iterate through seed nodes:
//!
//! ```no_run, rust
//! use chain_gang::network::Network;
//!
//! for (ip, port) in Network::BSV_Mainnet.seed_iter() {
//!     println!("Seed node {:?}:{}", ip, port);
//! }
//! ```

mod network;
mod seed_iter;

pub use self::network::Network;
pub use self::seed_iter::SeedIter;
