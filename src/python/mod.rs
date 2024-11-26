use pyo3::Bound;
use pyo3::{prelude::*, types::PyBytes};

mod op_code_names;
mod py_script;
mod py_stack;
mod py_tx;
mod py_wallet;

use crate::{
    messages::Tx,
    network::Network,
    python::{
        py_script::PyScript,
        py_stack::{decode_num_stack, PyStack},
        py_tx::{PyTx, PyTxIn, PyTxOut},
        py_wallet::{
            address_to_public_key_hash, bytes_to_wif, generate_wif, p2pkh_pyscript, wif_to_bytes,
            PyWallet,
        },
    },
    script::{stack::Stack, Script, TransactionlessChecker, ZChecker, NO_FLAGS},
    transaction::sighash::{sig_hash_preimage, SigHashCache},
    util::{hash160, sha256d, Error, Hash256},
    wallet::{create_sighash, public_key_to_address, MAIN_PRIVATE_KEY, TEST_PRIVATE_KEY},
};

pub type Bytes = Vec<u8>;

#[pyfunction(name = "p2pkh_script")]
fn py_p2pkh_pyscript(h160: &[u8]) -> PyScript {
    p2pkh_pyscript(h160)
}

#[pyfunction(name = "hash160")]
pub fn py_hash160(py: Python, data: &[u8]) -> PyObject {
    let result = hash160(data).0;
    PyBytes::new_bound(py, &result).into()
}

#[pyfunction(name = "hash256d")]
pub fn py_hash256d(py: Python, data: &[u8]) -> PyObject {
    let result = sha256d(data).0;
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
#[pyo3(signature = (py_script, break_at=None, z=None))]
fn py_script_eval(
    py_script: &[u8],
    break_at: Option<usize>,
    z: Option<&[u8]>,
) -> PyResult<(Stack, Stack, Option<usize>)> {
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
            Ok(
                script.eval_with_stack(
                    &mut ZChecker { z },
                    NO_FLAGS,
                    None,
                    break_at,
                    None,
                    None,
                )?,
            )
        }
        None => Ok(script.eval_with_stack(
            &mut TransactionlessChecker {},
            NO_FLAGS,
            None,
            break_at,
            None,
            None,
        )?),
    }
}

#[pyfunction]
#[pyo3(signature = (py_script, start_at=None, break_at=None, z=None, stack_param=None, alt_stack_param=None))]
fn py_script_eval_pystack(
    py_script: &[u8],
    start_at: Option<usize>,
    break_at: Option<usize>,
    z: Option<&[u8]>,
    stack_param: Option<PyStack>,
    alt_stack_param: Option<PyStack>,
) -> PyResult<(PyStack, PyStack, Option<usize>)> {
    let mut script = Script::new();
    script.append_slice(py_script);
    // Handle stack and alt_stack parameters with match
    let main_stack = stack_param.map(|py_stack_main| py_stack_main.to_stack());

    let alternative_stack = alt_stack_param.map(|alt_stack_param| alt_stack_param.to_stack());

    // Pick the appropriate transaction checker
    let (main_stack, alt_stack, prog_counter) = match z {
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
            script.eval_with_stack(
                &mut ZChecker { z },
                NO_FLAGS,
                break_at,
                start_at,
                main_stack,
                alternative_stack,
            )?
        }
        None => script.eval_with_stack(
            &mut TransactionlessChecker {},
            NO_FLAGS,
            start_at,
            break_at,
            main_stack,
            alternative_stack,
        )?,
    };

    let optional_i = match break_at {
        Some(_) => prog_counter,
        None => None,
    };

    Ok((
        PyStack::from_stack(main_stack),
        PyStack::from_stack(alt_stack),
        optional_i,
    ))
}

/// Return the transaction data prior to the hash function
///
/// # Arguments
///
/// * `tx` - Spending transaction
/// * `index` - Spending input index
/// * `script_pubkey` - The lock_script of the output being spent. This may be a subset of the
///   lock_script if OP_CODESEPARATOR is used.
/// * `satoshis` - The satoshi amount in the output being spent
/// * `sighash_flags` - Sighash flags

#[pyfunction(name = "sig_hash_preimage")]
pub fn py_sig_hash_preimage(
    _py: Python,
    tx: &PyTx,
    index: usize,
    script_pubkey: PyScript,
    prev_amount: i64,
    sighash_flags: u8,
) -> PyResult<PyObject> {
    let input_tx: Tx = tx.as_tx();
    let prev_lock_script: Script = script_pubkey.as_script();

    let mut cache = SigHashCache::new();
    let sigh_hash = sig_hash_preimage(
        &input_tx,
        index,
        &prev_lock_script.0,
        prev_amount,
        sighash_flags,
        &mut cache,
    );
    let bytes = PyBytes::new_bound(_py, &sigh_hash.unwrap());
    Ok(bytes.into())
}

/// Generates a transaction digest
///
/// # Arguments
///
/// * `tx` - Spending transaction
/// * `index` - Spending input index
/// * `script_pubkey` - The lock_script of the output being spent. This may be a subset of the
///   lock_script if OP_CODESEPARATOR is used.
/// * `satoshis` - The satoshi amount in the output being spent
/// * `sighash_flags` - Sighash flags
#[pyfunction(name = "sig_hash")]
pub fn py_sig_hash(
    _py: Python,
    tx: &PyTx,
    index: usize,
    script_pubkey: PyScript,
    prev_amount: i64,
    sighash_flags: u8,
) -> PyResult<PyObject> {
    let input_tx = tx.as_tx();
    let prev_lock_script = script_pubkey.as_script();

    let full_sig_hash = create_sighash(
        &input_tx,
        index,
        &prev_lock_script,
        prev_amount,
        sighash_flags,
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
#[pyo3(signature = (password, nonce, network=None))]
pub fn py_generate_wif_from_pw_nonce(
    _py: Python,
    password: &str,
    nonce: &str,
    network: Option<&str>,
) -> String {
    // Provide default value if `network` is None
    let network = network.unwrap_or("BSV_Testnet");

    // Example logic: derive WIF based on password, nonce, and network
    match network {
        "BSV_Mainnet" => generate_wif(password, nonce, "BSV_Mainnet"),
        _ => generate_wif(password, nonce, "BSV_Testnet"), // Default to "testnet"
    }
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
    m.add_function(wrap_pyfunction!(decode_num_stack, m)?)?;
    m.add_function(wrap_pyfunction!(py_script_eval_pystack, m)?)?;
    // Script
    m.add_class::<PyScript>()?;

    // Tx classes
    m.add_class::<PyTxIn>()?;
    m.add_class::<PyTxOut>()?;
    m.add_class::<PyTx>()?;
    // Wallet class
    m.add_class::<PyWallet>()?;
    // stack class
    m.add_class::<PyStack>()?;
    Ok(())
}
