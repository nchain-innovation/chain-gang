use crate::{python::op_code_names::OP_CODE_NAMES, script::Script, util::var_int};
use pyo3::{
    prelude::*,
    types::{PyBytes, PyType},
};
use regex::Regex;
use std::io::Write;

#[derive(FromPyObject, Debug)]
pub enum Command {
    Int(u8),
    Bytes(Vec<u8>),
}

// Convert Vec<Commands> to Vec<u8>
fn commands_as_vec(cmds: Vec<Command>) -> Vec<u8> {
    let mut script: Vec<u8> = Vec::new();
    for x in cmds {
        match x {
            Command::Int(value) => script.push(value),
            Command::Bytes(list) => script.extend_from_slice(&list),
        }
    }
    script
}

/*
def decode_op(op: str) -> Union[int, bytes]:
    # Given an op as string convert it to parsable value
    #   e.g. "OP_2" -> 0x52
    op = op.strip()
    if op[:2] == "0x":
        b: bytes = bytes.fromhex(op[2:])
        return b

    elif op in ALL_OPS:
        n: int = ALL_OPS[op]
        return n

    else:
        n = eval(op)
        if isinstance(n, int):
            x: bytes = encode_num(n)
            return x
        elif isinstance(n, str):
            y = n.encode("utf-8")
            return y
        elif isinstance(n, bytes):
            return n
        else:
            # have not captured conversion
            assert 1 == 2  # should not get here

*/

fn decode_op(op: &str) -> Command {
    let op = op.trim();
    if let Some(val) = OP_CODE_NAMES.get(op) {
        return Command::Int(*val);
    }
    // dbg!(&op);
    if op[..2] == *"0x" {
        if op.len() == 4 {
            return Command::Int(u8::from_str_radix(&op[2..], 16).unwrap());
        } else {
            return Command::Bytes(hex::decode(&op[2..]).unwrap());
        }
    }
    if op[..1] == *"b" {
        let bytes: Vec<u8> = op[2..op.len() - 1].chars().map(|c| c as u8).collect();
        Command::Bytes(bytes)
    } else {
        println!("other");
        Command::Bytes(op.as_bytes().to_vec())
    }
}

#[pyclass(name = "Script", get_all, set_all)]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PyScript {
    pub script: Vec<u8>,
}

impl PyScript {
    pub fn new(script: &[u8]) -> Self {
        PyScript {
            script: script.to_vec(),
        }
    }

    pub fn as_script(&self) -> Script {
        Script(self.script.clone())
    }
}

#[pymethods]
impl PyScript {
    #[new]
    pub fn py_new(cmds: Vec<Command>) -> PyScript {
        // Convert Vec<Commands> to Vec<u8>
        let script = commands_as_vec(cmds);
        PyScript { script }
    }

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

    fn get_commands(&self) -> PyResult<Vec<u8>> {
        Ok(self.script.clone())
    }

    /// Converts a string to a Script
    #[classmethod]
    fn parse_string(_cls: &Bound<'_, PyType>, in_string: &str) -> PyResult<Self> {
        let stripped = in_string.trim();
        let separator = Regex::new(r"[ ,\n]+").unwrap();
        let splits: Vec<_> = separator
            .split(stripped)
            .filter(|x| x.trim() != "")
            .collect();
        let decoded: Vec<_> = splits.into_iter().map(decode_op).collect();

        let script = commands_as_vec(decoded);

        Ok(PyScript { script })
    }
}
