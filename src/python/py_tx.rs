use crate::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    python::py_script::PyScript,
    util::{Error, Hash256, Serializable},
};
use core::hash::Hash;
use linked_hash_map::LinkedHashMap;
use pyo3::{
    exceptions::PyRuntimeError,
    prelude::*,
    types::{PyBytes, PyType},
};
use std::{
    collections::{HashMap, HashSet},
    fmt,
    io::Cursor,
};

// Convert errors to PyErr
impl std::convert::From<crate::util::Error> for PyErr {
    fn from(err: crate::util::Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

/// TxIn - This represents a bitcoin transaction input
//
// #[pyclass(name = "TxIn")]
#[pyclass(name = "TxIn", get_all, set_all, dict)]
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

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }

    fn __repr__(&self) -> String {
        format!("{:?}", &self)
    }
}

/// TxOut - This represents a bitcoin transaction output
//
//#[pyclass(name = "TxOut")]
#[pyclass(name = "TxOut", get_all, set_all, dict)]
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

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }

    fn __repr__(&self) -> String {
        format!("{:?}", &self)
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
#[pyclass(name = "Tx", get_all, set_all, dict)]
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

impl fmt::Display for PyTx {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ret = format!("{:?}", &self);
        f.write_str(&ret)
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

    fn copy(&self) -> Self {
        self.clone()
    }

    /// Human-readable hexadecimal of the transaction hash"""
    /// def id(self) -> str:
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
    fn is_coinbase(&self) -> bool {
        let tx = self.as_tx();
        tx.coinbase()
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
    fn add_tx_in(&mut self, txin: PyTxIn) {
        self.tx_ins.push(txin);
    }

    /// Add a TxOut to a transaction
    fn add_tx_out(&mut self, txout: PyTxOut) {
        self.tx_outs.push(txout);
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }

    fn __repr__(&self) -> String {
        format!("{}", &self)
    }

    #[allow(clippy::inherent_to_string_shadow_display)]
    fn to_string(&self) -> String {
        self.__repr__()
    }

    // This will only work on post genesis txs
    // This will only work for non coinbase transactions
    fn validate(&self, utxos: Vec<PyTx>) -> PyResult<()> {
        let tx = self.as_tx();
        if tx.coinbase() {
            let msg = "Validate can not check coinbase transactions.".to_string();
            return Err(Error::BadData(msg).into());
        }

        // Get the tx input OutPoints
        let outpoints: Vec<OutPoint> = tx.inputs.iter().map(|x| x.prev_output.clone()).collect();

        // Speed up the OutPoint lookups by preparing HashMap
        let utxo_as_tx: HashMap<Hash256, Tx> = utxos
            .iter()
            .map(|x| x.as_tx())
            .map(|tx| (tx.hash(), tx))
            .collect();

        // Convert input utxos into processed_utxo: &LinkedHashMap<OutPoint, TxOut>,
        let mut processed_utxo: LinkedHashMap<OutPoint, TxOut> = LinkedHashMap::new();
        for op in outpoints {
            match utxo_as_tx.get(&op.hash) {
                Some(tx) => match tx.outputs.get(op.index as usize) {
                    Some(txout) => processed_utxo.insert(op, txout.clone()),
                    None => {
                        let msg = format!("Invalid Outpoint index {}", op.index);
                        return Err(Error::BadData(msg).into());
                    }
                },
                None => {
                    let msg = format!("Unable to find hash {:?}", op.hash);
                    return Err(Error::BadData(msg).into());
                }
            };
        }
        // Empty HashSet
        let pregenesis_outputs: HashSet<OutPoint> = HashSet::new();

        // Call validate
        let result = tx.validate(true, true, &processed_utxo, &pregenesis_outputs);
        Ok(result?)
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
