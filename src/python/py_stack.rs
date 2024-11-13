use crate::script::stack::{decode_num, decode_number_combined, encode_bigint, Stack};
use num_bigint::BigInt;
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyLong};
#[pyclass(name = "Stack", get_all, set_all, dict)]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PyStack {
    inner: Stack, // internal representation of py_stack
}

#[pymethods]
impl PyStack {
    #[new] // creates a new empty stack
    #[pyo3(signature =(items=vec![]))]
    fn new(items: Vec<Vec<u8>>) -> Self {
        PyStack { inner: items }
    }

    // push a list of bytes (Vec<u8>)
    fn push(&mut self, item: Vec<u8>) {
        self.inner.push(item);
    }

    /// Push bytes onto the stack
    fn push_bytes_integer(&mut self, _py: Python, item: &Bound<'_, PyList>) -> PyResult<()> {
        for val in item.iter() {
            let py_long = val
                .downcast::<PyLong>()
                .map_err(|_| pyo3::exceptions::PyTypeError::new_err("Expected a PyLong"))?
                .as_ref();

            let big_int_str = py_long.str()?.to_str()?.to_owned();
            // Convert the string to a Rust BigInt (assumption is base-10)
            let big_int = BigInt::parse_bytes(big_int_str.as_bytes(), 10)
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to parse BigInt"))?;

            self.inner.push(encode_bigint(big_int));
        }
        Ok(())
    }

    // pop an utem of bytes from the stack
    fn pop(&mut self) -> PyResult<Vec<u8>> {
        self.inner
            .pop()
            .ok_or_else(|| PyValueError::new_err("Cannot pop from an empty stack"))
    }

    // decode stack
    fn decode_stack(&mut self) -> PyResult<Vec<i64>> {
        let mut decoded_values = Vec::new();
        for item in &self.inner {
            decoded_values.push(decode_num(item)?)
        }
        Ok(decoded_values)
    }

    #[pyo3(signature = (index=None))]
    fn decode_element(&mut self, py: Python<'_>, index: Option<usize>) -> PyResult<Py<PyLong>> {
        // Set default index to 0 if not provided (i.e., the top element)
        let idx = index.unwrap_or(0);
        // Check if the index is within bounds
        if idx >= self.inner.len() {
            return Err(PyValueError::new_err("Index out of bounds"));
        }

        let elem = self
            .inner
            .get_mut(idx)
            .ok_or_else(|| PyValueError::new_err("Cannot pop from an empty stack"));
        let elem_num = decode_number_combined(elem?.as_mut_slice())?;

        // Convert the large integer to a string (Python handles large integers from strings well)
        let result_str = elem_num.to_string();
        // Create a new PyDict for globals
        let globals = PyDict::new_bound(py);
        // Use Python's built-in int() constructor to convert the string to a Python integer
        let py_int = py.eval_bound(&format!("int('{}')", result_str), Some(&globals), None)?;
        // Cast to PyLong and return
        Ok((*py_int.downcast::<PyLong>()?).clone().into())
    }

    // Display the Stack contents as a string
    fn __repr__(&self) -> String {
        let items: Vec<String> = self.inner.iter().map(|vec| format!("{:?}", vec)).collect();
        format!("Stack({})", items.join(", "))
    }

    // Adds a size method to return the length of the inner vector
    fn size(&self) -> usize {
        self.inner.len()
    }

    pub fn to_stack(&self) -> Stack {
        self.inner.clone()
    }

    #[staticmethod] // Secondary constructor to create PyStack from an existing Stack
    pub fn from_stack(stack: Stack) -> Self {
        PyStack { inner: stack }
    }

    #[staticmethod]
    fn single_from_array_nums(_py: Python, array: &Bound<'_, PyList>) -> PyResult<Self> {
        // Convert each Python list of bytes into a Vec<u8>
        let mut inner_stack = Vec::new();
        //if let Ok(item) = array.iter() {
        for item in array.iter() {
            let py_long = item
                .downcast::<PyLong>()
                .map_err(|_| pyo3::exceptions::PyTypeError::new_err("Expected a PyLong"))?
                .as_ref();

            // Convert PyLong to BigInt
            //let big_int = BigInt::from_py(py_long);
            // Convert the PyLong into a BigInt using to_string
            let big_int_str = py_long.str()?.to_str()?.to_owned();
            // Convert the string to a Rust BigInt (assumption is base-10)
            let big_int = BigInt::parse_bytes(big_int_str.as_bytes(), 10)
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to parse BigInt"))?;
            let (sign, magnitude) = big_int.to_bytes_le();
            println!("from_array -> {:?} {:?}", sign, magnitude);
            inner_stack.extend(encode_bigint(big_int));
        }
        Ok(PyStack {
            inner: vec![inner_stack],
        })
    }

    #[staticmethod]
    fn single_from_array_bytes(_py: Python, array: &Bound<'_, PyList>) -> PyResult<Self> {
        let mut inner_stack = Vec::new();

        for item in array.iter() {
            let py_long = item
                .downcast::<PyLong>()
                .map_err(|_| pyo3::exceptions::PyTypeError::new_err("Expected a PyLong"))?
                .as_ref();
            let val: u8 = py_long.extract()?;
            inner_stack.push(val);
        }

        Ok(PyStack {
            inner: vec![inner_stack],
        })
    }

    // Enable subscript access (self.inner[index])
    fn __getitem__(&self, index: usize) -> PyResult<Vec<u8>> {
        self.inner
            .get(index)
            .cloned() // Clone the Vec<u8> to return an owned value
            .ok_or_else(|| PyIndexError::new_err("Index out of range"))
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[pyfunction]
pub fn decode_num_stack(s: &[u8]) -> PyResult<i64> {
    decode_num(s).map_err(|e| PyValueError::new_err(format!("Decode error: {:?}", e)))
}
