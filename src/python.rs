use pyo3::prelude::*;
use std::io::{Cursor, Write};

use crate::{
    python_tx::{Bytes, PyTx, PyTxIn, PyTxOut},
    script::{
        stack::{decode_num, encode_num, Stack},
        Script, TransactionlessChecker, ZChecker, NO_FLAGS,
    },
    util::{var_int, Hash256, Serializable},
};

/*
// Convert errors to PyErr
impl std::convert::From<crate::util::Error> for PyErr {
    fn from(err: crate::util::Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}
*/

#[pyfunction]
fn py_encode_num(val: i64) -> PyResult<Bytes> {
    Ok(encode_num(val)?)
}

#[pyfunction]
fn py_decode_num(s: &[u8]) -> PyResult<i64> {
    Ok(decode_num(s)?)
}

#[pyfunction]
fn py_encode_varint(n: u64) -> PyResult<Bytes> {
    let mut v = Vec::new();
    var_int::write(n, &mut v)?;
    Ok(v)
}

#[pyfunction]
fn py_script_serialise(py_script: &[u8]) -> PyResult<Bytes> {
    let mut script = Script::new();
    script.append_slice(py_script);

    let mut v = Vec::new();
    v.write_all(&script.0)?;
    Ok(v)
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
#[pyo3(name = "chain_gang")]
fn chain_gang(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_encode_num, m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_num, m)?)?;
    m.add_function(wrap_pyfunction!(py_encode_varint, m)?)?;
    m.add_function(wrap_pyfunction!(py_script_serialise, m)?)?;
    m.add_function(wrap_pyfunction!(py_script_eval, m)?)?;
    // Tx classes
    m.add_class::<PyTxIn>()?;
    m.add_class::<PyTxOut>()?;
    m.add_class::<PyTx>()?;

    Ok(())
}
