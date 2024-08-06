//! A foundation for building applications on Bitcoin SV using Rust.

//#![cfg(feature = "interface")]

extern crate byteorder;
extern crate dns_lookup;
extern crate hex;
#[macro_use]
extern crate log;
extern crate hmac;
extern crate linked_hash_map;
extern crate murmur3;
extern crate rand;

extern crate base58;
extern crate k256;
extern crate ripemd;
extern crate snowflake;

#[cfg(feature = "python")]
extern crate lazy_static;

pub mod address;
pub mod messages;
pub mod network;
pub mod peer;
pub mod script;
pub mod transaction;
pub mod util;
pub mod wallet;

#[cfg(feature = "interface")]
pub mod interface;

#[cfg(feature = "python")]
pub mod python;
