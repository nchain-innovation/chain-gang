use pyo3::{prelude::*, types::PyBytes};
//use std::io::Cursor;

mod base58_checksum;
mod hashes;
mod op_code_names;
mod py_script;
mod py_tx;
mod py_wallet;

use crate::{
    network::Network,
    python::{
        hashes::{hash160,sha256d},
        py_script::PyScript,
        py_tx::{PyTx, PyTxIn, PyTxOut},
        py_wallet::{address_to_public_key_hash, p2pkh_pyscript, public_key_to_address, PyWallet}
    },
    messages::Tx,
    script::{stack::Stack, Script, TransactionlessChecker, ZChecker, NO_FLAGS},
    util::{Error, Hash256},
    transaction::sighash::{sig_hash_preimage,sighash,SigHashCache}
};

pub type Bytes = Vec<u8>;

#[pyfunction(name = "p2pkh_script")]
fn py_p2pkh_pyscript(h160: &[u8]) -> PyScript {
    p2pkh_pyscript(h160)
}

#[pyfunction(name = "hash160")]
pub fn py_hash160(py: Python, data: &[u8]) -> PyObject {
    let result = hash160(data);
    PyBytes::new_bound(py, &result).into()
}

#[pyfunction(name = "hash256d")]
pub fn py_hash256d(py: Python, data: &[u8]) -> PyObject{
    let result = sha256d(data);
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
            // Ensure the slice is exactly 32 bytes long
            let z_bytes = sig_hash;
            let z_array: [u8; 32] = match z_bytes.try_into() {
                Ok(array) => array,
                Err(_) => {
                    // Handle the error if `z_bytes` is not 32 bytes long
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("z_bytes must be exactly 32 bytes long"));
                }
            };

            let z = Hash256(z_array);
            Ok(script.eval_with_stack(&mut ZChecker { z }, NO_FLAGS, break_at)?)
        }
        None => Ok(script.eval_with_stack(&mut TransactionlessChecker {}, NO_FLAGS, break_at)?),
    }
}

#[pyfunction(name="sig_hash_preimage")]
pub fn py_sig_hash_preimage(_py: Python, tx: &PyTx, index:usize, script_pubkey: PyScript, prev_amount: i64, sighash_value: Option<u8>) ->  PyResult<PyObject> {
    let input_tx: Tx = tx.as_tx();
    let prev_lock_script: Script = script_pubkey.as_script();
    let mut cache = SigHashCache::new();
    let sigh_hash = sig_hash_preimage(&input_tx, index, &prev_lock_script.0, prev_amount, sighash_value.unwrap(), &mut cache); 
    let bytes = PyBytes::new_bound(_py, &sigh_hash.unwrap());
    Ok(bytes.into())
}

#[pyfunction(name="sig_hash")]
pub fn py_sig_hash(_py: Python, tx: &PyTx, index:usize, script_pubkey: PyScript, prev_amount: i64, sighash_value: Option<u8>) -> PyResult<PyObject>{
    let input_tx = tx.as_tx();
    let prev_lock_script = script_pubkey.as_script();
    let mut cache = SigHashCache::new(); 
    let full_sig_hash = sighash(&input_tx, index, &prev_lock_script.0, prev_amount, sighash_value.unwrap(), &mut cache);
    let bytes = PyBytes::new_bound(_py, &full_sig_hash.unwrap().0);
    Ok(bytes.into())
}

/// A Python module for interacting with the Rust chain-gang BSV script interpreter
#[pymodule]
#[pyo3(name = "tx_engine")]
fn chain_gang(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_script_eval, m)?)?;
    m.add_function(wrap_pyfunction!(py_p2pkh_pyscript, m)?)?;
    m.add_function(wrap_pyfunction!(py_hash160, m)?)?;
    m.add_function(wrap_pyfunction!(py_hash256d,m)?)?;
    m.add_function(wrap_pyfunction!(py_address_to_public_key_hash, m)?)?;
    m.add_function(wrap_pyfunction!(py_public_key_to_address, m)?)?;
    m.add_function(wrap_pyfunction!(py_sig_hash_preimage,m)?)?;
    m.add_function(wrap_pyfunction!(py_sig_hash,m)?)?;

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
