//! A foundation for building applications on Bitcoin SV using Rust.
#![cfg(feature = "interface")]
#![feature(async_fn_in_trait)]

//#[cfg(feature = "RUSTC_IS_NIGHTLY")]

extern crate byteorder;
extern crate digest;
extern crate dns_lookup;
extern crate hex;
#[macro_use]
extern crate log;
extern crate linked_hash_map;
extern crate murmur3;
extern crate rand;
extern crate ring;
extern crate ripemd160;
extern crate rust_base58;
extern crate secp256k1;
extern crate snowflake;

pub mod address;
pub mod messages;
pub mod network;
pub mod peer;
pub mod script;
pub mod transaction;
pub mod util;
pub mod wallet;

// Only include interface if nightly build as we are dependent on async_fn_in_trait feature
#[cfg(feature = "interface")]
pub mod interface;
