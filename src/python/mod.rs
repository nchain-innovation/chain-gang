use pyo3::prelude::*;
use std::io::Cursor;

mod op_code_names;
mod py_script;
mod py_tx;

use crate::{
    python::{
        py_script::PyScript,
        py_tx::{PyTx, PyTxIn, PyTxOut},
    },
    script::{
        stack::{decode_num, encode_num, Stack},
        Script, TransactionlessChecker, ZChecker, NO_FLAGS,
    },
    util::{Hash256, Serializable},
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
    // Script
    m.add_class::<PyScript>()?;

    // Tx classes
    m.add_class::<PyTxIn>()?;
    m.add_class::<PyTxOut>()?;
    m.add_class::<PyTx>()?;

    Ok(())
}
