//use linked_hash_map::LinkedHashMap;
// use std::collections::HashSet;
use core::hash::Hash;
// use std::collections::HashMap;

use crate::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    script::Script,
    transaction::{
        generate_signature,
        sighash::{sighash, SigHashCache},
    },
    util::{Hash256, Serializable},
};
use pyo3::{exceptions::PyRuntimeError, prelude::*, types::PyBytes};

pub type Bytes = Vec<u8>;

// Convert errors to PyErr
impl std::convert::From<crate::util::Error> for PyErr {
    fn from(err: crate::util::Error) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

/// TxIn - This represents is a bitcoin transaction input
#[pyclass(name = "TxIn")]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PyTxIn {
    pub prev_tx: [u8; 32],
    pub prev_index: u32,
    pub sequence: u32,
    pub sig_script: Script,
}

impl PyTxIn {
    fn as_txin(&self) -> TxIn {
        TxIn {
            prev_output: OutPoint {
                hash: Hash256(self.prev_tx),
                index: self.prev_index,
            },
            sequence: self.sequence,
            unlock_script: self.sig_script.clone(),
        }
    }
}

#[pymethods]
impl PyTxIn {
    #[new]
    fn new(prev_tx: [u8; 32], prev_index: u32, sig_script: &[u8], sequence: u32) -> Self {
        let script = Script(sig_script.to_vec());
        PyTxIn {
            prev_tx,
            prev_index,
            sequence,
            sig_script: script,
        }
    }
}

/// TxIn - This represents a bitcoin transaction output

#[pyclass(name = "TxOut")]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PyTxOut {
    pub amount: i64,
    pub script_pubkey: Script,
}

impl PyTxOut {
    fn as_txout(&self) -> TxOut {
        TxOut {
            satoshis: self.amount,
            lock_script: self.script_pubkey.clone(),
        }
    }
}

#[pymethods]
impl PyTxOut {
    #[new]
    fn new(amount: i64, script_pubkey: &[u8]) -> Self {
        PyTxOut {
            amount,
            script_pubkey: Script(script_pubkey.to_vec()),
        }
    }
}

/// Tx - This represents a bitcoin transaction
/// We need this to
/// * parse a bytestream - python
/// * serialise a transaction - rust
/// * sign tx - rust
/// * verify tx - rust

#[pyclass(name = "Tx")]
#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct PyTx {
    pub version: u32,
    pub tx_ins: Vec<PyTxIn>,
    pub tx_outs: Vec<PyTxOut>,
    pub locktime: u32,
}

impl PyTx {
    fn as_tx(&self) -> Tx {
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

/*
struct PyMap<K, V>(LinkedHashMap<K, V>);

impl<'a, K, V> FromPyObject<'a> for PyMap<K, V> where K: FromPyObject<'a> + Hash + Eq, V: FromPyObject<'a> + Hash + Eq {
    fn extract(ob: &'a PyAny) -> PyResult<Self> {
        if let Ok(dict) = ob.downcast::<PyDict>() {
            Ok(PyMap(dict.items().extract::<Vec<(K, V)>>()?.into_iter().collect::<LinkedHashMap<_,_>>()))
        } else {
            Err(PyTypeError::new_err("dict expected"))
        }
    }
}
*/

#[pymethods]
impl PyTx {
    #[new]
    fn new(version: u32, tx_ins: Vec<PyTxIn>, tx_outs: Vec<PyTxOut>, locktime: u32) -> Self {
        PyTx {
            version,
            tx_ins,
            tx_outs,
            locktime,
        }
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
    /// def is_coinbase(self) -> bool:
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

    /// sign the transaction input to spend it
    /// * self - Spending transaction
    /// * n_input - Spending input index
    /// * script_code - The lock_script of the output being spent. This may be a subset of the lock_script if OP_CODESEPARATOR is used.
    /// * satoshis - The satoshi amount in the output being spent
    /// * sighash_type - Sighash flags
    fn generate_signature_for_input(
        &self,
        py: Python<'_>,
        n_input: usize,
        script_code: &[u8],
        satoshis: i64,
        private_key: &[u8],
        sighash_type: u8,
    ) -> PyResult<PyObject> {
        let mut cache = SigHashCache::new();
        let tx = self.as_tx();
        // Create sighash
        let tx_sighash = sighash(
            &tx,
            n_input,
            script_code,
            satoshis,
            sighash_type,
            &mut cache,
        )?;

        let private_key: &[u8; 32] = private_key.try_into()?;
        // generate_signature
        let signature = generate_signature(private_key, &tx_sighash, sighash_type)?;
        let bytes = PyBytes::new_bound(py, &signature);
        Ok(bytes.into())
    }

    /*
    /// Validates a non-coinbase transaction
    //pub fn validate(&self, require_sighash_forkid: bool,   use_genesis_rules: bool,  utxos: LinkedHashMap<OutPoint, TxOut>,  pregenesis_outputs: &HashSet<OutPoint>) -> PyResult<()> {
        pub fn validate(&self, py: Python<'_>, require_sighash_forkid: bool,   use_genesis_rules: bool,  utxos: PyObject,  pregenesis_outputs: PyObject) -> PyResult<()> {
        let tx = self.as_tx();
        let linked_utxo: HashMap<u8, u8> = utxos.extract(py)?;
        //let linked_utxo: HashMap<OutPoint, TxOut> = utxos.extract(py)?;
        //let p_outputs: HashSet<OutPoint> =  pregenesis_outputs.extract(py)?;
        //Ok(tx.validate(require_sighash_forkid, use_genesis_rules, &linked_utxo, &p_outputs)?)
        Ok(())
    }
    */
}
