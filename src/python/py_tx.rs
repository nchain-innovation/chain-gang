use core::hash::Hash;
use std::io::Cursor;

use crate::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    python::py_script::PyScript,
    util::{Hash256, Serializable},
};
use pyo3::{
    exceptions::PyRuntimeError,
    prelude::*,
    types::{PyBytes, PyType},
};

// Convert errors to PyErr
impl std::convert::From<crate::util::Error> for PyErr {
    fn from(err: crate::util::Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

/// TxIn - This represents is a bitcoin transaction input
//
// #[pyclass(name = "TxIn")]
#[pyclass(name = "TxIn", get_all, set_all)]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PyTxIn {
    pub prev_tx: String,
    pub prev_index: u32,
    pub sequence: u32,
    pub script_sig: PyScript,
}

impl PyTxIn {
    fn as_txin(&self) -> TxIn {
        // convert hexstr to bytes and reverse
        let hash = Hash256::decode(&self.prev_tx).expect("Error decoding hexstr prev outpoint");

        TxIn {
            prev_output: OutPoint {
                hash,
                index: self.prev_index,
            },
            sequence: self.sequence,
            unlock_script: self.script_sig.as_script(),
        }
    }
}

#[pymethods]
impl PyTxIn {
    #[new]
    #[pyo3(signature = (prev_tx, prev_index, script=vec![], sequence=0xFFFFFFFF))]
    fn new(prev_tx: &str, prev_index: u32, script: Vec<u8>, sequence: u32) -> Self {
        let script_sig = PyScript::new(&script);
        PyTxIn {
            prev_tx: prev_tx.to_string(),
            prev_index,
            sequence,
            script_sig,
        }
    }
}

/// TxOut - This represents a bitcoin transaction output

//
//#[pyclass(name = "TxOut")]
#[pyclass(name = "TxOut", get_all, set_all)]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PyTxOut {
    pub amount: i64,
    pub script_pubkey: PyScript,
}

impl PyTxOut {
    fn as_txout(&self) -> TxOut {
        TxOut {
            satoshis: self.amount,
            lock_script: self.script_pubkey.as_script(),
        }
    }
}

#[pymethods]
impl PyTxOut {
    #[new]
    fn new(amount: i64, script_pubkey: &[u8]) -> Self {
        PyTxOut {
            amount,
            script_pubkey: PyScript::new(script_pubkey),
        }
    }
}

// Conversion functions
fn txin_as_pytxin(txin: &TxIn) -> PyTxIn {
    let prev_tx = txin.prev_output.hash.encode();
    PyTxIn {
        prev_tx,
        prev_index: txin.prev_output.index,
        sequence: txin.sequence,
        script_sig: PyScript::new(&txin.unlock_script.0),
    }
}

fn txout_as_pytxout(txout: &TxOut) -> PyTxOut {
    PyTxOut {
        amount: txout.satoshis,
        script_pubkey: PyScript::new(&txout.lock_script.0),
    }
}

/// Convert from Rust Tx to PyTx
pub fn tx_as_pytx(tx: &Tx) -> PyTx {
    PyTx {
        version: tx.version,
        tx_ins: tx
            .inputs
            .clone()
            .into_iter()
            .map(|x| txin_as_pytxin(&x))
            .collect(),
        tx_outs: tx
            .outputs
            .clone()
            .into_iter()
            .map(|x| txout_as_pytxout(&x))
            .collect(),
        locktime: tx.lock_time,
    }
}

/// Tx - This represents a bitcoin transaction
/// We need this to
/// * parse a bytestream - python
/// * serialise a transaction - rust
/// * sign tx - rust
/// * verify tx - rust
#[pyclass(name = "Tx", get_all, set_all)]
#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct PyTx {
    pub version: u32,
    pub tx_ins: Vec<PyTxIn>,
    pub tx_outs: Vec<PyTxOut>,
    pub locktime: u32,
}

impl PyTx {
    pub fn as_tx(&self) -> Tx {
        Tx {
            version: self.version,
            inputs: self
                .tx_ins
                .clone()
                .into_iter()
                .map(|x| x.as_txin())
                .collect(),
            outputs: self
                .tx_outs
                .clone()
                .into_iter()
                .map(|x| x.as_txout())
                .collect(),
            lock_time: self.locktime,
        }
    }
}

#[pymethods]
impl PyTx {
    #[new]
    #[pyo3(signature = (version, tx_ins, tx_outs, locktime=0))]
    fn new(version: u32, tx_ins: Vec<PyTxIn>, tx_outs: Vec<PyTxOut>, locktime: u32) -> Self {
        PyTx {
            version,
            tx_ins,
            tx_outs,
            locktime,
        }
    }

    fn clone_py(&self) -> Self {
        self.clone()
    }

    /// def id(self) -> str:
    /// Human-readable hexadecimal of the transaction hash"""
    fn id(&self) -> PyResult<String> {
        let tx = self.as_tx();
        let hash = tx.hash();
        Ok(hash.encode())
    }

    /// Binary hash of the serialization
    /// def hash(self) -> bytes:
    fn hash(&self, py: Python<'_>) -> PyResult<PyObject> {
        let tx = self.as_tx();
        let hash = tx.hash();
        let bytes = PyBytes::new_bound(py, &hash.0);
        Ok(bytes.into())
    }

    /// Returns true if it is a coinbase transaction
    fn is_coinbase(&self) -> PyResult<bool> {
        let tx = self.as_tx();
        Ok(tx.coinbase())
    }

    /// Note that we return PyResult<PyObject> and not PyResult<PyBytes>
    fn serialize(&self, py: Python<'_>) -> PyResult<PyObject> {
        let mut v = Vec::new();
        let tx = self.as_tx();
        tx.write(&mut v)?;
        let bytes = PyBytes::new_bound(py, &v);
        Ok(bytes.into())
    }

    /// Add a TxIn to a transaction
    fn add_tx_in(&mut self, txin: PyTxIn) -> PyResult<bool>{
        self.tx_ins.push(txin);
        Ok(true)
    }

    /// Add a TxOut to a transaction
    fn add_tx_out(&mut self, txout: PyTxOut) -> PyResult<bool>{
        self.tx_outs.push(txout);
        Ok(true)
    }

    /// Parse Bytes to produce Tx
    // #[new]
    #[classmethod]
    fn parse(_cls: &Bound<'_, PyType>, bytes: &[u8]) -> PyResult<Self> {
        let tx = Tx::read(&mut Cursor::new(&bytes))?;
        let pytx = tx_as_pytx(&tx);
        Ok(pytx)
    }
}
