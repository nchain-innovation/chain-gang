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

// Disabled this warning as would probably break too much other code to fix it
//warn: module has the same name as its containing module
#[allow(clippy::module_inception)]
mod network;
mod seed_iter;

pub use self::network::Network;
pub use self::seed_iter::SeedIter;
