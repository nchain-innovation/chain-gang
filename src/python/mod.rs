use pyo3::{prelude::*, types::PyBytes};
use std::io::Cursor;

mod base58_checksum;
mod hashes;
mod op_code_names;
mod py_script;
mod py_tx;
mod py_wallet;

use crate::{
    python::{
        hashes::hash160,
        py_script::PyScript,
        py_tx::{PyTx, PyTxIn, PyTxOut},
        py_wallet::{address_to_public_key_hash, p2pkh_pyscript, PyWallet, public_key_to_address},
    },
    script::{
        stack::{decode_num, encode_num, Stack},
        Script, TransactionlessChecker, ZChecker, NO_FLAGS,
    },
    util::{Hash256, Serializable, Error},
    network::Network,
};

pub type Bytes = Vec<u8>;

#[pyfunction]
fn py_encode_num(val: i64) -> PyResult<Bytes> {
    Ok(encode_num(val)?)
}

#[pyfunction]
fn py_decode_num(s: &[u8]) -> PyResult<i64> {
    Ok(decode_num(s)?)
}

#[pyfunction(name = "p2pkh_script")]
fn py_p2pkh_pyscript(h160: &[u8]) -> PyScript {
    p2pkh_pyscript(h160)
}

#[pyfunction(name = "hash160")]
pub fn py_hash160(py: Python, data: &[u8]) -> PyObject {
    let result = hash160(data);
    PyBytes::new_bound(py, &result).into()
}

#[pyfunction(name = "address_to_public_key_hash")]
pub fn py_address_to_public_key_hash(py: Python, address: &str) -> PyResult<PyObject> {
    let result = address_to_public_key_hash(address)?;
    Ok(PyBytes::new_bound(py, &result).into())
}

#[pyfunction(name = "public_key_to_address")]
pub fn py_public_key_to_address(public_key: &[u8], network: &str) -> PyResult<String> {
    // network conversion
    let network_type = match network {
        "BSV_Mainnet" => Network::BSV_Mainnet,
        "BSV_Testnet" => Network::BSV_Testnet,
        _ => {
            let msg = format!("Unknown network: {}", network);
            return Err(Error::BadData(msg).into());
        }
    };
    Ok(public_key_to_address(public_key, network_type)?)
}

/// py_script_eval evaluates bitcoin script
/// Where
///  * py_script - the script to execute
///  * break_at - the instruction to stop at, or None
///  * z - the sig_hash of the transaction as bytes, or None
#[pyfunction]
fn py_script_eval(
    py_script: &[u8],
    break_at: Option<usize>,
    z: Option<&[u8]>,
) -> PyResult<(Stack, Stack)> {
    let mut script = Script::new();
    script.append_slice(py_script);
    // Pick the appropriate transaction checker
    match z {
        Some(sig_hash) => {
            let z = Hash256::read(&mut Cursor::new(sig_hash))?;
            Ok(script.eval_with_stack(&mut ZChecker { z }, NO_FLAGS, break_at)?)
        }
        None => Ok(script.eval_with_stack(&mut TransactionlessChecker {}, NO_FLAGS, break_at)?),
    }
}

/// A Python module for interacting with the Rust chain-gang BSV script interpreter
#[pymodule]
#[pyo3(name = "tx_engine")]
fn chain_gang(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_encode_num, m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_num, m)?)?;
    m.add_function(wrap_pyfunction!(py_script_eval, m)?)?;
    m.add_function(wrap_pyfunction!(py_p2pkh_pyscript, m)?)?;
    m.add_function(wrap_pyfunction!(py_hash160, m)?)?;
    m.add_function(wrap_pyfunction!(py_address_to_public_key_hash, m)?)?;
    m.add_function(wrap_pyfunction!(py_public_key_to_address, m)?)?;

    // Script
    m.add_class::<PyScript>()?;

    // Tx classes
    m.add_class::<PyTxIn>()?;
    m.add_class::<PyTxOut>()?;
    m.add_class::<PyTx>()?;
    // Wallet class
    m.add_class::<PyWallet>()?;
    Ok(())
}
