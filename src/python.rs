use pyo3::{exceptions::PyRuntimeError, prelude::*};
use std::io::Write;

use crate::{
    util::var_int,
    script::{
        stack::{decode_num, encode_num, Stack},
        Script, TransactionlessChecker, NO_FLAGS,
    }
};

pub type Bytes = Vec<u8>;


// Convert errors to PyErr
impl std::convert::From<crate::util::Error> for PyErr {
    fn from(err: crate::util::Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

#[pyfunction]
fn py_encode_num(val: i64) -> PyResult<Bytes> {
    Ok(encode_num(val)?)
}

#[pyfunction]
fn py_decode_num(s: &[u8]) -> PyResult<i64> {
    Ok(decode_num(s)?)
}


#[pyfunction]
fn py_encode_varint(n :u64) -> PyResult<Bytes> {
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

#[pyfunction]
fn py_script_eval(py_script: &[u8]) -> PyResult<(Stack, Stack)> {
    let mut script = Script::new();
    script.append_slice(py_script);
    Ok(script.eval_with_stack(&mut TransactionlessChecker {}, NO_FLAGS)?)
}



/// A Python module for interacting with the Rust chain-gang BSV script interpreter implemented in Rust.
#[pymodule]
fn chain_gang(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_encode_num, m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_num, m)?)?;
    m.add_function(wrap_pyfunction!(py_encode_varint, m)?)?;
    m.add_function(wrap_pyfunction!(py_script_serialise, m)?)?;
    m.add_function(wrap_pyfunction!(py_script_eval, m)?)?;
    Ok(())
}
