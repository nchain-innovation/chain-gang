//! A foundation for building applications on Bitcoin SV using Rust.
#![doc = include_str!("../docs/README-chain-gang.md")]

#![warn(missing_docs)]

#[macro_use]
extern crate log;

#[cfg(feature = "python")]
extern crate lazy_static;

pub mod address;
pub mod chronicle;
pub mod messages;
pub mod network;
pub mod peer;
pub mod script;
pub mod transaction;
pub mod util;
pub mod wallet;

#[cfg(feature = "interface")]
pub mod interface;

/// Python (PyO3) bindings, compiled only with the `python` feature.
#[cfg(feature = "python")]
pub mod python;
