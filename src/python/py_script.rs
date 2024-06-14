
use std::io::Write;
use pyo3::{
    prelude::*,
    types::PyBytes,
};
use crate::{
    script::Script,
    util::var_int,
};

#[pyclass(name = "Script", get_all, set_all)]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PyScript{
    pub script: Vec<u8>,
}

impl PyScript { 

    pub fn new(script: &[u8]) -> PyScript {
        PyScript{
            script: script.to_vec()
        }
    }

    pub fn as_script(&self) -> Script {
        Script ( self.script.clone() )
    }
    
    
}

#[pymethods]
impl PyScript { 

    // Return the serialised script without the length prepended
    fn raw_serialize(&self, py: Python<'_>) -> PyResult<PyObject> {
        let mut v: Vec<u8> = Vec::new();
        v.write_all(&self.script)?;
     
        let bytes = PyBytes::new_bound(py, &v);
        Ok(bytes.into())
    }


    // Return the serialised script with the length prepended
    fn serialize(&self, py: Python<'_>) -> PyResult<PyObject> {
        let mut script: Vec<u8> = Vec::new();
        script.write_all(&self.script)?;
        let length = script.len();
        
        let mut a: Vec<u8> = Vec::new();
        
        var_int::write(length.try_into()?, &mut a)?;
        a.append(&mut script);
        
        let bytes = PyBytes::new_bound(py, &a);
        Ok(bytes.into())
    }
    
}




