use pyo3::{
    prelude::*,
    types::{PyBytes, PyType},
};
use regex::Regex;
use std::{
    fmt,
    io::{Cursor, Read, Write},
};

use crate::{
    python::op_code_names::OP_CODE_NAMES,
    script::{op_codes, stack::encode_num, Script},
    util::{var_int, Error, Result},
};

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

fn is_pushdata_operation(cmd: &Command) -> Option<usize> {
    match cmd {
        #[allow(clippy::match_ref_pats)]
        Command::Int(v) => match v {
            &op_codes::OP_PUSHDATA1 => Some(2),
            &op_codes::OP_PUSHDATA2 => Some(3),
            &op_codes::OP_PUSHDATA4 => Some(5),
            _ => None,
        },
        _ => None,
    }
}

fn handle_pushdata(cmd: &Command, is_pushdata: usize) -> usize {
    match is_pushdata_operation(cmd) {
        Some(val) => val,
        None => {
            if is_pushdata > 0 {
                is_pushdata - 1
            } else {
                0
            }
        }
    }
}

fn decode_op(op: &str, is_pushdata: usize) -> Command {
    let op = op.trim();
    // println!("decode_op({:?})", &op);
    // Command
    if let Some(val) = OP_CODE_NAMES.get(op) {
        return Command::Int(*val);
    }
    // Is an int
    if let Ok(val) = op.parse::<i64>() {
        match val {
            -1 => return Command::Int(op_codes::OP_1NEGATE),
            0 => return Command::Int(op_codes::OP_0),
            1..=16 => return Command::Int((val + 0x50).try_into().unwrap()), // 1 => OP_1, => 0x81
            17..=75 => {
                if is_pushdata > 0 {
                    return Command::Int(val.try_into().unwrap());
                } else {
                    let retval: Vec<u8> = vec![1, val.try_into().unwrap()];
                    return Command::Bytes(retval);
                }
            }
            _ => {
                if is_pushdata > 0 {
                    let retval = encode_num(val).unwrap();
                    return Command::Bytes(retval);
                } else {
                    let mut retval = encode_num(val).unwrap();
                    let len: u8 = retval.len().try_into().unwrap();
                    retval.insert(0, len);
                    return Command::Bytes(retval);
                }
            }
        }
    }
    // Hex digit, digits
    if op[..2] == *"0x" {
        if is_pushdata > 0 {
            let retval: Vec<u8> = hex::decode(&op[2..]).unwrap();
            return Command::Bytes(retval);
        } else {
            let len = op[2..].len() / 2;
            let data: Vec<u8> = hex::decode(&op[2..]).unwrap();
            let mut retval: Vec<u8> = Vec::new();
            match len {
                0 => {
                    retval.push(op_codes::OP_0);
                }
                1..=75 => {
                    retval.push(op_codes::OP_PUSH + len as u8);
                    retval.extend(data);
                }
                76..=255 => {
                    retval.push(op_codes::OP_PUSHDATA1);
                    retval.push(len as u8);
                    retval.extend(data);
                }
                256..=65535 => {
                    retval.push(op_codes::OP_PUSHDATA2);
                    retval.push(len as u8);
                    retval.push((len >> 8) as u8);
                    retval.extend(data);
                }
                _ => {
                    retval.push(op_codes::OP_PUSHDATA4);
                    retval.push(len as u8);
                    retval.push((len >> 8) as u8);
                    retval.push((len >> 16) as u8);
                    retval.push((len >> 24) as u8);
                    retval.extend(data);
                }
            }
            return Command::Bytes(retval);
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
#[derive(PartialEq, Eq, Hash, Clone)]
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

impl fmt::Debug for PyScript {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let script = self.as_script();
        let ret = script.string_representation();
        f.write_str(&ret)
    }
}

impl fmt::Display for PyScript {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let script = self.as_script();
        let ret = script.string_representation();
        f.write_str(&ret)
    }
}

#[pymethods]
impl PyScript {
    #[new]
    #[pyo3(signature = (cmds=vec![]))]
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
    pub fn serialize(&self, py: Python) -> PyResult<PyObject> {
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
    fn get_commands(&self, py: Python<'_>) -> PyObject {
        PyBytes::new_bound(py, &self.cmds).into()
    }

    /// Return a string presentation of the script
    fn __repr__(&self) -> String {
        format!("{}", &self)
    }

    fn __getitem__(&self, index: usize) -> PyResult<u8> {
        match self.cmds.get(index) {
            Some(value) => Ok(*value),
            None => {
                let msg = format!("Index '{}' out of range", index);
                Err(Error::BadData(msg).into())
            }
        }
    }

    #[allow(clippy::inherent_to_string_shadow_display)]
    fn to_string(&self) -> String {
        self.__repr__()
    }

    /// Add two scripts together to produce a new script
    ///  c_script = a_script + b_script
    fn __add__(&self, other: &Self) -> Self {
        let mut script = self.cmds.clone();
        script.extend(other.cmds.clone());
        PyScript { cmds: script }
    }

    // a_script == b_script
    fn __eq__(&self, other: &Self) -> bool {
        self.cmds == other.cmds
    }

    /// Appends a single opcode or data byte
    fn append_byte(&mut self, byte: u8) {
        self.cmds.push(byte);
    }

    /// Appends data
    fn append_data(&mut self, data: &[u8]) {
        self.cmds.extend_from_slice(data);
    }

    /// Appends the opcodes and provided data that push it onto the stack
    fn append_pushdata(&mut self, data: &[u8]) {
        let len = data.len();
        match len {
            0 => self.cmds.push(op_codes::OP_0),
            1..=75 => {
                self.cmds.push(op_codes::OP_PUSH + len as u8);
                self.cmds.extend_from_slice(data);
            }
            76..=255 => {
                self.cmds.push(op_codes::OP_PUSHDATA1);
                self.cmds.push(len as u8);
                self.cmds.extend_from_slice(data);
            }
            256..=65535 => {
                self.cmds.push(op_codes::OP_PUSHDATA2);
                self.cmds.push((len) as u8);
                self.cmds.push((len >> 8) as u8);
                self.cmds.extend_from_slice(data);
            }
            _ => {
                self.cmds.push(op_codes::OP_PUSHDATA4);
                self.cmds.push((len) as u8);
                self.cmds.push((len >> 8) as u8);
                self.cmds.push((len >> 16) as u8);
                self.cmds.push((len >> 24) as u8);
                self.cmds.extend_from_slice(data);
            }
        }
    }

    /// Return true if p2pkh
    fn is_p2pkh(&self) -> bool {
        let len = self.cmds.len();
        len == 25
            && self.cmds[0] == op_codes::OP_DUP
            && self.cmds[1] == op_codes::OP_HASH160
            && self.cmds[len - 2] == op_codes::OP_EQUALVERIFY
            && self.cmds[len - 1] == op_codes::OP_CHECKSIG
    }

    /// Converts a String to a Script
    #[classmethod]
    fn parse_string(_cls: &Bound<'_, PyType>, in_string: &str) -> PyResult<Self> {
        let stripped = in_string.trim();
        let separator = Regex::new(r"[ ,\n]+").unwrap();

        let splits: Vec<_> = separator
            .split(stripped)
            .filter(|x| x.trim() != "")
            .collect();
        let mut decoded: Vec<Command> = Vec::new();
        let mut is_pushdata: usize = 0;
        for s in splits {
            let op = decode_op(s, is_pushdata);
            is_pushdata = handle_pushdata(&op, is_pushdata);
            decoded.push(op);
        }
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
