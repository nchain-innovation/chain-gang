use crate::{
    python::PyTx,
    util::Hash256,
    transaction::sighash::{partial_sig_hash, SigHashCache, sighash},
    python::py_script::PyScript,
    script::Script
};
use crate::messages::Tx;

use pyo3::{
    exceptions::PyRuntimeError,
    prelude::*,
};


pub fn compute_partial_pre_image(input_tx: Tx, index: usize, sighashflag: Option<u8>) -> PyResult<PySigHash>{

    match partial_sig_hash(&input_tx, index, sighashflag) {
        Ok(sighash_cache) => {
            Ok(sighashcache_as_pysighashcache(&sighash_cache))
        }
        Err(e) => {
            println!("Error occurred: {:?}", e);
            Err(PyRuntimeError::new_err(format!("Error occurred: {:?}", e)))
        }
    }
}


#[pyclass(name = "SigHashCache")]
//#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[derive(Debug)]
pub struct PySigHash {
    pub hash_prevouts:  Option<[u8; 32]>,
    pub hash_sequence:   Option<[u8; 32]>,
    pub hash_outputs:  Option<[u8; 32]>,
}

impl PySigHash {
    //#[allow(dead_code)]
    fn as_sighashcache(&self) -> SigHashCache {
        let mut cache = SigHashCache::new();
        cache.set_hash_prevouts(self.hash_prevouts.map(Hash256).unwrap_or_else(|| Hash256([0; 32])));
        cache.set_hash_sequence(self.hash_sequence.map(Hash256).unwrap_or_else(|| Hash256([0; 32])));
        cache.set_hash_outputs(self.hash_outputs.map(Hash256).unwrap_or_else(||Hash256([0; 32])));
        cache
    }
}

    // Convert Option<Hash256> to Option<[u8; 32]>
    pub fn sighashcache_as_pysighashcache(sighash_cache: &SigHashCache) -> PySigHash{
        PySigHash {
            hash_prevouts: sighash_cache.hash_prevouts().map(|hash| hash.0),
            hash_sequence: sighash_cache.hash_sequence().map(|hash| hash.0),
            hash_outputs: sighash_cache.hash_outputs().map(|hash| hash.0),
        }
    }

#[pymethods]
impl PySigHash {
    #[new]
    pub fn new() -> PyResult<Self> {
        Ok(PySigHash {
            hash_prevouts: None,
            hash_sequence: None,
            hash_outputs: None,
        })
    }
    
    // Convert hash_prevouts to a hex string
    pub fn hash_prevouts_to_hex(&self) -> PyResult<Option<String>> {
        Ok(self.hash_prevouts.map(|h| hex::encode(h)))
    }

     // Convert hash_sequence to a hex string
     pub fn hash_sequence_to_hex(&self) -> PyResult<Option<String>> {
        Ok(self.hash_sequence.map(|h| hex::encode(h)))
    }

    // Convert hash_outputs to a hex string
    pub fn hash_outputs_to_hex(&self) -> PyResult<Option<String>> {
        Ok(self.hash_outputs.map(|h| hex::encode(h)))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", &self)
    }
}

#[pyfunction(name = "partial_sig_hash")]
pub fn py_partial_sig_hash(_py: Python, tx: &PyTx, index: usize, sighash_value: Option<u8>) -> PyResult<PySigHash> {

    let input_tx: Tx = tx.as_tx();
    compute_partial_pre_image(input_tx, index, sighash_value)
}

#[pyfunction(name = "full_sig_hash")]
pub fn py_full_sig_hash(_py: Python, tx: &PyTx, index: usize, script_pubkey: PyScript, prev_amount: i64, sighash_value: Option<u8>) -> PyResult<[u8; 32]>{
    let input_tx: Tx = tx.as_tx();
    let prev_lock_script: Script = script_pubkey.as_script();
    let mut cache = SigHashCache::new();

    let sig_hash = sighash(
        &input_tx,
        index,
        &prev_lock_script.0,
        prev_amount,
        sighash_value.unwrap(),
        &mut cache,
    );
    Ok(sig_hash.map(|hash| hash.0)?)
}



