//! A foundation for building applications on Bitcoin SV using Rust.

use pyo3::{
    //types::{PyBytes, PyTuple, PyList},
    exceptions::PyRuntimeError,
    prelude::*,
};

//#![cfg(feature = "interface")]
//#![feature(async_fn_in_trait)]

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

use crate::script::{Script, TransactionlessChecker, NO_FLAGS};

// Convert errors to PyErr
impl std::convert::From<crate::util::Error> for PyErr {
    fn from(err: crate::util::Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

type Stack = Vec<Vec<u8>>;

#[pyfunction]
fn script_eval(py_script: &[u8]) -> PyResult<(Stack, Stack)> {
    let mut script = Script::new();
    script.append_slice(py_script);

    Ok(script.eval_with_stack(&mut TransactionlessChecker {}, NO_FLAGS)?)
}

/// A Python module for interacting with the Rust chain-gang BSV script interpreter implemented in Rust.
#[pymodule]
fn chain_gang(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(script_eval, m)?)?;
    Ok(())
}

// Only include interface if nightly build as we are dependent on async_fn_in_trait feature
#[cfg(feature = "interface")]
pub mod interface;
