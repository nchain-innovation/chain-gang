use crate::{
    python::op_code_names::OP_CODE_NAMES,
    script::Script,
    util::{var_int, Result},
};
use pyo3::{
    prelude::*,
    types::{PyBytes, PyType},
};
use regex::Regex;
use std::io::{Cursor, Read, Write};

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
    // println!("decode_op({:?})", &op);
    // Command
    if let Some(val) = OP_CODE_NAMES.get(op) {
        return Command::Int(*val);
    }
    // One char
    if op.len() == 1 {
        let mut val = op.parse::<u8>().unwrap();
        if let 1..=16 = val {
            val += 0x50; // OP_1,
        }
        return Command::Int(val);
    }
    // Hex digit, digits
    if op[..2] == *"0x" {
        if op.len() == 4 {
            return Command::Int(u8::from_str_radix(&op[2..], 16).unwrap());
        } else {
            return Command::Bytes(hex::decode(&op[2..]).unwrap());
        }
    }
    // Byte array
    if op[..1] == *"b" {
        let bytes: Vec<u8> = op[2..op.len() - 1].chars().map(|c| c as u8).collect();
        Command::Bytes(bytes)
    } else {
        // String
        let bytes: Vec<u8> = op[1..op.len() - 1].chars().map(|c| c as u8).collect();
        Command::Bytes(bytes)
    }
}

#[pyclass(name = "Script", get_all, set_all)]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct PyScript {
    pub cmds: Vec<u8>,
}

impl PyScript {
    pub fn new(script: &[u8]) -> Self {
        PyScript {
            cmds: script.to_vec(),
        }
    }

    pub fn as_script(&self) -> Script {
        Script(self.cmds.clone())
    }

    fn read(reader: &mut dyn Read) -> Result<Self> {
        let script_len = var_int::read(reader)?;
        let mut script: Vec<u8> = vec![0; script_len as usize];
        reader.read_exact(&mut script)?;
        Ok(PyScript { cmds: script })
    }
}

#[pymethods]
impl PyScript {
    #[new]
    pub fn py_new(cmds: Vec<Command>) -> PyScript {
        // Convert Vec<Commands> to Vec<u8>
        let script = commands_as_vec(cmds);
        PyScript { cmds: script }
    }

    // Return the serialised script without the length prepended
    fn raw_serialize(&self, py: Python<'_>) -> PyResult<PyObject> {
        let mut v: Vec<u8> = Vec::new();
        v.write_all(&self.cmds)?;

        let bytes = PyBytes::new_bound(py, &v);
        Ok(bytes.into())
    }

    /// Return the serialised script with the length prepended
    fn serialize(&self, py: Python<'_>) -> PyResult<PyObject> {
        let mut script: Vec<u8> = Vec::new();
        script.write_all(&self.cmds)?;
        let length = script.len();
        let mut a: Vec<u8> = Vec::new();
        var_int::write(length.try_into()?, &mut a)?;
        a.append(&mut script);

        let bytes = PyBytes::new_bound(py, &a);
        Ok(bytes.into())
    }

    /// Return a copy of the commands in this script
    fn get_commands(&self) -> PyResult<Vec<u8>> {
        Ok(self.cmds.clone())
    }

    // c_script = a_script + b_script
    fn __add__(&self, other: &Self) -> Self {
        let mut script = self.cmds.clone();
        script.extend(other.cmds.clone());
        PyScript { cmds: script }
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

        Ok(PyScript { cmds: script })
    }

    /// Converts bytes to a Script:
    #[classmethod]
    fn parse(_cls: &Bound<'_, PyType>, bytes: &[u8]) -> PyResult<Self> {
        let script = PyScript::read(&mut Cursor::new(&bytes))?;
        Ok(script)
    }
}
