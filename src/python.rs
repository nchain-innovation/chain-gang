use pyo3::{
    exceptions::PyRuntimeError,
    prelude::*,
};

use crate::{
    script::{
        Script, TransactionlessChecker, NO_FLAGS,
        stack::{Stack, encode_num, decode_num}
    }
};


// Convert errors to PyErr
impl std::convert::From<crate::util::Error> for PyErr {
    fn from(err: crate::util::Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}


#[pyfunction]
fn py_encode_num(val: i64) -> PyResult<Vec<u8>> {
    Ok(encode_num(val)?)
}

#[pyfunction]
fn py_decode_num(s: &[u8]) -> PyResult<i64> {
    Ok(decode_num(s)?)
}


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
    m.add_function(wrap_pyfunction!(py_encode_num, m)?)?;
    m.add_function(wrap_pyfunction!(py_decode_num, m)?)?;
    Ok(())
}
