use pyo3::{prelude::*, types::PyBytes};
use pyo3::types::PyLong;
use pyo3::Bound;
use num_bigint::BigInt;

mod base58_checksum;
mod hashes;
mod op_code_names;
mod py_script;
mod py_tx;
mod py_wallet;

use crate::{
    messages::Tx,
    network::Network,
    python::{
        hashes::{hash160, sha256d},
        py_script::PyScript,
        py_tx::{PyTx, PyTxIn, PyTxOut},
        py_wallet::{address_to_public_key_hash, p2pkh_pyscript, public_key_to_address, wif_to_bytes, bytes_to_wif, generate_wif, PyWallet, wallet_from_int, MAIN_PRIVATE_KEY, TEST_PRIVATE_KEY},
    },
    script::{stack::Stack, Script, TransactionlessChecker, ZChecker, NO_FLAGS},
    transaction::sighash::{sig_hash_preimage, sighash, SigHashCache},
    util::{Error, Hash256},
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
pub fn py_hash256d(py: Python, data: &[u8]) -> PyObject {
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
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "z_bytes must be exactly 32 bytes long",
                    ));
                }
            };

            let z = Hash256(z_array);
            Ok(script.eval_with_stack(&mut ZChecker { z }, NO_FLAGS, break_at)?)
        }
        None => Ok(script.eval_with_stack(&mut TransactionlessChecker {}, NO_FLAGS, break_at)?),
    }
}

#[pyfunction(name = "sig_hash_preimage")]
pub fn py_sig_hash_preimage(
    _py: Python,
    tx: &PyTx,
    index: usize,
    script_pubkey: PyScript,
    prev_amount: i64,
    sighash_value: Option<u8>,
) -> PyResult<PyObject> {
    let input_tx: Tx = tx.as_tx();
    let prev_lock_script: Script = script_pubkey.as_script();
    let mut cache = SigHashCache::new();
    let sigh_hash = sig_hash_preimage(
        &input_tx,
        index,
        &prev_lock_script.0,
        prev_amount,
        sighash_value.unwrap(),
        &mut cache,
    );
    let bytes = PyBytes::new_bound(_py, &sigh_hash.unwrap());
    Ok(bytes.into())
}

#[pyfunction(name = "sig_hash")]
pub fn py_sig_hash(
    _py: Python,
    tx: &PyTx,
    index: usize,
    script_pubkey: PyScript,
    prev_amount: i64,
    sighash_value: Option<u8>,
) -> PyResult<PyObject> {
    let input_tx = tx.as_tx();
    let prev_lock_script = script_pubkey.as_script();
    let mut cache = SigHashCache::new();
    let full_sig_hash = sighash(
        &input_tx,
        index,
        &prev_lock_script.0,
        prev_amount,
        sighash_value.unwrap(),
        &mut cache,
    );
    let bytes = PyBytes::new_bound(_py, &full_sig_hash.unwrap().0);
    Ok(bytes.into())
}

#[pyfunction(name = "wif_to_bytes")]
pub fn py_wif_to_bytes(py: Python, wif: &str) -> PyResult<PyObject> {
    let key_bytes = wif_to_bytes(wif)?;
    let bytes = PyBytes::new_bound(py, &key_bytes);
    Ok(bytes.into())
}

#[pyfunction(name = "bytes_to_wif")]
pub fn py_bytes_to_wif(key_bytes: &[u8], network: &str) -> PyResult<String> {
    // network conversion
    let network_prefix = match network {
        "BSV_Mainnet" => MAIN_PRIVATE_KEY,
        "BSV_Testnet" => TEST_PRIVATE_KEY,
        _ => {
            let msg = format!("Unknown network: {}", network);
            return Err(Error::BadData(msg).into());
        }
    };
    Ok(bytes_to_wif(key_bytes, network_prefix))
}

#[pyfunction(name = "wif_from_pw_nonce")]
pub fn py_generate_wif_from_pw_nonce(_py: Python, password: &str, nonce: &str, network: Option<&str>) -> String {
    // Provide default value if `network` is None
    let network = network.unwrap_or("testnet");

    // Example logic: derive WIF based on password, nonce, and network
    let wif = match network {
        "mainnet" => generate_wif(password, nonce, "mainnet"),
        _ => generate_wif(password, nonce, "testnet"), // Default to "testnet"
    };

    wif
}


#[pyfunction(name = "wallet_from_int")]
fn py_wallet_from_int(int_rep: &Bound<'_, PyAny>, network: &str) -> PyResult<PyWallet> {
    // Use with_gil to get a reference to the Python interpreter
    Python::with_gil(|_py| {
        // Use the bound reference to access the PyAny
        let py_any = int_rep.as_ref();
        // Downcast the PyAny reference to PyLong
        let py_long = py_any.downcast::<PyLong>().map_err(|_| pyo3::exceptions::PyTypeError::new_err("Expected a PyLong"))?.as_ref();

        // Convert the PyLong into a BigInt using to_string
        let big_int_str = py_long.str()?.to_str()?.to_owned();
        
        // Convert the string to a Rust BigInt (assumption is base-10)
        let big_int = BigInt::parse_bytes(big_int_str.as_bytes(), 10)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to parse BigInt"))?;

        let test_wallet = wallet_from_int(network, big_int)?;
        Ok(test_wallet)
   })
}

/// A Python module for interacting with the Rust chain-gang BSV script interpreter
#[pymodule]
#[pyo3(name = "tx_engine")]
fn chain_gang(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_script_eval, m)?)?;
    m.add_function(wrap_pyfunction!(py_p2pkh_pyscript, m)?)?;
    m.add_function(wrap_pyfunction!(py_hash160, m)?)?;
    m.add_function(wrap_pyfunction!(py_hash256d, m)?)?;
    m.add_function(wrap_pyfunction!(py_address_to_public_key_hash, m)?)?;
    m.add_function(wrap_pyfunction!(py_public_key_to_address, m)?)?;
    m.add_function(wrap_pyfunction!(py_sig_hash_preimage, m)?)?;
    m.add_function(wrap_pyfunction!(py_sig_hash, m)?)?;
    m.add_function(wrap_pyfunction!(py_wif_to_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(py_bytes_to_wif, m)?)?;
    m.add_function(wrap_pyfunction!(py_generate_wif_from_pw_nonce, m)?)?;
    m.add_function(wrap_pyfunction!(py_wallet_from_int, m)?)?;
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
