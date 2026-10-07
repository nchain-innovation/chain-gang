use pyo3::{
    prelude::*,
    types::{PyBytes, PyInt, PyType},
};
use std::{
    fmt,
    io::{Cursor, Read, Write},
};

use crate::{
    script::{op_codes, stack::encode_bigint, Script},
    util::{var_int, ChainGangError},
};

use crate::util::read_exact_vec;
use num_bigint::BigInt;
use num_traits::ToPrimitive;

#[derive(FromPyObject, Debug, Clone)]
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

/// Returns `data` preceded by the opcodes that push it onto the stack
fn pushdata_bytes(data: &[u8]) -> Vec<u8> {
    let len = data.len();
    let mut retval: Vec<u8> = Vec::with_capacity(len + 5);
    match len {
        0 => retval.push(op_codes::OP_0),
        1..=75 => retval.push(op_codes::OP_PUSH + len as u8),
        76..=255 => {
            retval.push(op_codes::OP_PUSHDATA1);
            retval.push(len as u8);
        }
        256..=65535 => {
            retval.push(op_codes::OP_PUSHDATA2);
            retval.extend_from_slice(&(len as u16).to_le_bytes());
        }
        _ => {
            retval.push(op_codes::OP_PUSHDATA4);
            retval.extend_from_slice(&(len as u32).to_le_bytes());
        }
    }
    retval.extend_from_slice(data);
    retval
}

/// The bytes a quoted token stands for: `'text'` or `b'text'`, one byte per
/// character. None when `token` is not quoted.
///
/// A character above U+00FF has no single byte, so it is an error; it used to
/// be cut to its low byte.
fn quoted_bytes(token: &str) -> Option<Result<Vec<u8>, ChainGangError>> {
    let inner = token
        .strip_prefix("b'")
        .or_else(|| token.strip_prefix('\''))?
        .strip_suffix('\'')?;
    Some(
        inner
            .chars()
            .map(|c| {
                u8::try_from(c).map_err(|_| {
                    ChainGangError::BadData(format!(
                        "Unable to parse '{token}': '{c}' is not a single byte"
                    ))
                })
            })
            .collect(),
    )
}

/// The bytes a `0x...` or quoted token stands for. None when it is neither.
fn literal_bytes(token: &str) -> Option<Result<Vec<u8>, ChainGangError>> {
    if let Some(hex_digits) = token.strip_prefix("0x") {
        return Some(hex::decode(hex_digits).map_err(|e| {
            ChainGangError::BadData(format!("Unable to parse '{token}' as hex: {e}"))
        }));
    }
    quoted_bytes(token)
}

/// The script bytes for one token that is not an opcode name: a number, hex
/// or quoted text, each pushed onto the stack. Anything else is an error; an
/// unknown word such as `OP_DUPP` used to become raw bytes.
fn decode_token(token: &str) -> Result<Vec<u8>, ChainGangError> {
    // Parsed as a BigInt so numbers outside i32 are encoded rather than
    // rejected, as Script.append_big_integer does.
    if let Ok(val) = token.parse::<BigInt>() {
        return Ok(match val.to_i64() {
            Some(-1) => vec![op_codes::OP_1NEGATE],
            Some(0) => vec![op_codes::OP_0],
            Some(small @ 1..=16) => vec![small as u8 + 0x50], // 1 => OP_1
            _ => pushdata_bytes(&encode_bigint(val)),
        });
    }
    match literal_bytes(token) {
        Some(bytes) => Ok(pushdata_bytes(&bytes?)),
        None => Err(ChainGangError::BadData(format!(
            "Unable to parse '{token}': not an opcode, number, hex or quoted text"
        ))),
    }
}

/// The length field and data that follow an explicit `OP_PUSHDATA1`, `2` or
/// `4`, written as the two tokens after it.
///
/// The length is a decimal number, written in the opcode's 1, 2 or 4 bytes,
/// little-endian, or hex of exactly that many bytes. The data is hex, quoted
/// text or a number (as a script number), and must be as long as the length
/// says. A decimal length of 1 to 16 used to become OP_1 to OP_16, one of 128
/// or more gained a sign byte, and after OP_PUSHDATA2 and 4 the parser took
/// three and five tokens as raw bytes rather than two.
fn explicit_push_bytes(
    op: u8,
    length_token: Option<&str>,
    data_token: Option<&str>,
) -> Result<Vec<u8>, ChainGangError> {
    let name = op_codes::opcode_name(op).unwrap_or("OP_PUSHDATA");
    let width = match op {
        op_codes::OP_PUSHDATA1 => 1,
        op_codes::OP_PUSHDATA2 => 2,
        _ => 4,
    };
    let (Some(length_token), Some(data_token)) = (length_token, data_token) else {
        return Err(ChainGangError::BadData(format!(
            "{name} must be followed by a length and data"
        )));
    };
    let length_field = if let Ok(length) = length_token.parse::<u32>() {
        let bytes = length.to_le_bytes();
        if bytes[width..].iter().any(|&b| b != 0) {
            return Err(ChainGangError::BadData(format!(
                "{name} length {length} does not fit in {width} bytes"
            )));
        }
        bytes[..width].to_vec()
    } else {
        match length_token.strip_prefix("0x").map(hex::decode) {
            Some(Ok(bytes)) if bytes.len() == width => bytes,
            _ => {
                return Err(ChainGangError::BadData(format!(
                    "{name} length '{length_token}' must be a number or {width} bytes of hex"
                )))
            }
        }
    };
    let length = length_field
        .iter()
        .rev()
        .fold(0usize, |acc, &b| (acc << 8) | b as usize);
    let data = match literal_bytes(data_token) {
        Some(bytes) => bytes?,
        None => match data_token.parse::<BigInt>() {
            Ok(val) => encode_bigint(val),
            Err(_) => {
                return Err(ChainGangError::BadData(format!(
                    "{name} data '{data_token}' must be hex, quoted text or a number"
                )))
            }
        },
    };
    if data.len() != length {
        return Err(ChainGangError::BadData(format!(
            "{name} says {length} bytes but '{data_token}' is {}",
            data.len()
        )));
    }
    let mut bytes = vec![op];
    bytes.extend(length_field);
    bytes.extend(data);
    Ok(bytes)
}

/// `Script.parse_string`: opcode names, numbers, hex and quoted text,
/// separated by whitespace or commas. Tabs used not to separate, so
/// `OP_1<tab>OP_2` was one unknown token.
fn parse_script_string(in_string: &str) -> Result<Vec<u8>, ChainGangError> {
    let mut tokens = in_string
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|token| !token.is_empty());
    let mut script = Vec::new();
    while let Some(token) = tokens.next() {
        match op_codes::name_to_byte(token) {
            Some(
                op @ (op_codes::OP_PUSHDATA1 | op_codes::OP_PUSHDATA2 | op_codes::OP_PUSHDATA4),
            ) => script.extend(explicit_push_bytes(op, tokens.next(), tokens.next())?),
            Some(op) => script.push(op),
            None => script.extend(decode_token(token)?),
        }
    }
    Ok(script)
}

#[pyclass(name = "Script", get_all, set_all, from_py_object)]
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

    fn read(reader: &mut dyn Read) -> Result<Self, ChainGangError> {
        let script_len = var_int::read(reader)?;
        let script: Vec<u8> = read_exact_vec(reader, script_len, "script")?;
        Ok(PyScript { cmds: script })
    }
}

impl fmt::Debug for PyScript {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let script = self.as_script();
        let ret = script.string_representation(false);
        f.write_str(&ret)
    }
}

impl fmt::Display for PyScript {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let script = self.as_script();
        let ret = script.string_representation(false);
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
    fn raw_serialize(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let mut v: Vec<u8> = Vec::new();
        v.write_all(&self.cmds)?;

        let bytes = PyBytes::new(py, &v);
        Ok(bytes.into())
    }

    /// Return the serialised script with the length prepended
    pub fn serialize(&self, py: Python) -> PyResult<Py<PyAny>> {
        let mut script: Vec<u8> = Vec::new();
        script.write_all(&self.cmds)?;
        let length = script.len();
        let mut a: Vec<u8> = Vec::new();
        var_int::write(length.try_into()?, &mut a)?;
        a.append(&mut script);

        let bytes = PyBytes::new(py, &a);
        Ok(bytes.into())
    }

    /// Return a copy of the commands in this script
    fn get_commands(&self, py: Python<'_>) -> Py<PyAny> {
        PyBytes::new(py, &self.cmds).into()
    }

    /// Return a string presentation of the script
    fn __repr__(&self) -> String {
        format!("{}", self)
    }

    fn __getitem__(&self, index: usize) -> PyResult<u8> {
        match self.cmds.get(index) {
            Some(value) => Ok(*value),
            None => {
                let msg = format!("Index '{}' out of range", index);
                Err(ChainGangError::BadData(msg).into())
            }
        }
    }

    /// append integers
    fn append_integer(&mut self, int_val: i64) {
        match int_val {
            -1 => self.cmds.push(op_codes::OP_1NEGATE),
            0 => self.cmds.push(op_codes::OP_0),
            1..=16 => self.cmds.push((int_val + 0x50).try_into().unwrap()),
            17..=75 => {
                let retval: Vec<u8> = vec![1, int_val.try_into().unwrap()];
                self.cmds.extend(&retval);
            }
            _ => {
                let retval = encode_bigint(BigInt::from(int_val));
                self.cmds.extend(pushdata_bytes(&retval));
            }
        }
    }

    #[allow(clippy::inherent_to_string_shadow_display)]
    fn to_string(&self) -> String {
        self.__repr__()
    }

    fn to_debug_parser_string(&self) -> String {
        let script = self.as_script();
        script.string_representation(true)
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
        self.cmds.extend(pushdata_bytes(data));
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

    /// Add an integer to a string (but handle big ints)
    fn append_big_integer(&mut self, int_rep: &Bound<'_, PyAny>) -> PyResult<bool> {
        // Use with_gil to get a reference to the Python interpreter
        //Python::with_gil(|_cls| {
        // Use the bound reference to access the PyAny
        // Downcast the PyAny reference to PyInt
        let py_long: &Bound<'_, PyInt> = int_rep
            .cast::<PyInt>()
            .map_err(|_| pyo3::exceptions::PyTypeError::new_err("Expected a PyInt"))?;

        // Convert the PyInt into a BigInt using to_string
        let big_int_str = py_long.str()?.to_str()?.to_owned();

        // Convert the string to a Rust BigInt (assumption is base-10)
        let big_int = BigInt::parse_bytes(big_int_str.as_bytes(), 10)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to parse BigInt"))?;

        match big_int {
            ref n if *n == BigInt::from(-1) => self.cmds.push(op_codes::OP_1NEGATE),
            ref n if *n == BigInt::from(0) => self.cmds.push(op_codes::OP_0),
            ref n if *n >= BigInt::from(1) && *n <= BigInt::from(16) => {
                let opcode: u8 = (n + BigInt::from(0x50)).to_u64().unwrap() as u8;
                self.cmds.push(opcode);
            }
            ref n if *n >= BigInt::from(17) && *n <= BigInt::from(75) => {
                let retval: Vec<u8> = vec![1, n.to_u8().unwrap()];
                self.cmds.extend(&retval);
            }
            _ => {
                let retval = encode_bigint(big_int.clone());
                self.cmds.extend(pushdata_bytes(&retval));
            }
        }
        Ok(true)
    }

    /// These functions were added for the debugger
    /// Shortens the script by removing amount number of bytes from the vec.
    pub fn shorten(&mut self, amount: usize) {
        if amount >= self.cmds.len() {
            self.cmds.clear();
        } else {
            self.cmds.drain(0..amount);
        }
    }

    /// sets the script to a shorter script between start & end
    pub fn sub_script(&mut self, start: usize, end: usize) {
        if start < end && end <= self.cmds.len() {
            self.cmds = self.cmds[start..end].to_vec();
        }
    }

    /// Converts a String to a Script
    #[classmethod]
    fn parse_string(_cls: &Bound<'_, PyType>, in_string: &str) -> PyResult<Self> {
        Ok(PyScript {
            cmds: parse_script_string(in_string)?,
        })
    }

    /// Converts bytes to a Script:
    #[classmethod]
    fn parse(_cls: &Bound<'_, PyType>, bytes: &[u8]) -> PyResult<Self> {
        let script = PyScript::read(&mut Cursor::new(&bytes))?;
        Ok(script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(op: &str) -> Result<Vec<u8>, ChainGangError> {
        decode_token(op)
    }

    fn parse(script: &str) -> Vec<u8> {
        parse_script_string(script).unwrap_or_else(|e| panic!("{script:?}: {e}"))
    }

    fn parse_err(script: &str) -> String {
        match parse_script_string(script) {
            Ok(bytes) => panic!("{script:?} should fail, got {}", hex::encode(bytes)),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn small_numbers_are_unchanged() {
        assert_eq!(decode("-1").unwrap(), vec![op_codes::OP_1NEGATE]);
        assert_eq!(decode("0").unwrap(), vec![op_codes::OP_0]);
        assert_eq!(decode("16").unwrap(), vec![op_codes::OP_16]);
        assert_eq!(decode("17").unwrap(), vec![1, 17]);
        assert_eq!(decode("76").unwrap(), vec![1, 76]);
        assert_eq!(decode("-128").unwrap(), vec![2, 0x80, 0x80]);
    }

    #[test]
    fn numbers_outside_i32_are_encoded() {
        assert_eq!(decode("2147483648").unwrap(), vec![5, 0, 0, 0, 0x80, 0]);
        assert_eq!(decode("-2147483648").unwrap(), vec![5, 0, 0, 0, 0x80, 0x80]);
        // Past i64 these used to fall through to the string branch
        let big = decode("18446744073709551616").unwrap();
        assert_eq!(big, vec![9, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn long_numbers_use_pushdata() {
        // 2^700 encodes to 88 bytes, too many for a single length byte
        let n: BigInt = BigInt::from(1) << 700usize;
        let encoded = decode(&n.to_string()).unwrap();
        assert_eq!(&encoded[..2], &[op_codes::OP_PUSHDATA1, 88]);
        assert_eq!(encoded.len(), 90);
    }

    #[test]
    fn bad_tokens_are_errors() {
        for op in ["0xZZ", "0x123", "x", "'", "b'", "é", "éa", "'abc", "abc'"] {
            assert!(decode(op).is_err(), "{op}");
        }
    }

    /// An unknown word used to become raw bytes, its first and last
    /// characters dropped: `OP_DUPP` gave `P_DUP`, `DUP` gave `U`.
    #[test]
    fn unknown_words_are_errors() {
        for script in ["OP_DUPP", "DUP", "hello", "OP_1 OP_ADDD"] {
            assert!(parse_err(script).contains("not an opcode"), "{script}");
        }
    }

    #[test]
    fn non_ascii_tokens_do_not_panic() {
        assert_eq!(decode("'é'").unwrap(), vec![1, 0xe9]);
        // No single byte: it used to be cut to its low byte, 0xac.
        assert!(decode("'€'")
            .unwrap_err()
            .to_string()
            .contains("single byte"));
    }

    /// Quoted text is pushed, like hex, and like `'...'` in bitcoin-sv's
    /// script format. It used to be inserted as raw script bytes.
    #[test]
    fn quoted_text_is_pushed() {
        assert_eq!(parse("'abc'"), vec![3, b'a', b'b', b'c']);
        assert_eq!(parse("b'abc'"), vec![3, b'a', b'b', b'c']);
        assert_eq!(parse("''"), vec![op_codes::OP_0]);
    }

    #[test]
    fn whitespace_and_commas_separate() {
        let expected = vec![op_codes::OP_1, op_codes::OP_2, op_codes::OP_ADD];
        for script in [
            "OP_1 OP_2 OP_ADD",
            "OP_1\tOP_2\tOP_ADD",
            " OP_1,OP_2\r\n OP_ADD ",
        ] {
            assert_eq!(parse(script), expected, "{script:?}");
        }
    }

    /// The two tokens after OP_PUSHDATA1, 2 or 4 are its length field and its
    /// data, written as they are. The length can be decimal or hex of the
    /// field's width (#8).
    #[test]
    fn explicit_pushdata() {
        use op_codes::{OP_ADD, OP_PUSHDATA1, OP_PUSHDATA2, OP_PUSHDATA4};
        let data = [1u8, 2, 3];
        let push = |op: u8, field: &[u8]| {
            let mut bytes = vec![op];
            bytes.extend_from_slice(field);
            bytes.extend_from_slice(&data);
            bytes
        };
        for (script, expected) in [
            ("OP_PUSHDATA1 0x03 0x010203", push(OP_PUSHDATA1, &[3])),
            ("OP_PUSHDATA1 3 0x010203", push(OP_PUSHDATA1, &[3])),
            ("OP_PUSHDATA2 0x0300 0x010203", push(OP_PUSHDATA2, &[3, 0])),
            ("OP_PUSHDATA2 3 0x010203", push(OP_PUSHDATA2, &[3, 0])),
            (
                "OP_PUSHDATA4 0x03000000 0x010203",
                push(OP_PUSHDATA4, &[3, 0, 0, 0]),
            ),
            ("OP_PUSHDATA4 3 0x010203", push(OP_PUSHDATA4, &[3, 0, 0, 0])),
        ] {
            assert_eq!(parse(script), expected, "{script}");
        }

        // Decimal lengths in full: 16 is not OP_16, 200 has no sign byte.
        assert_eq!(
            &parse(&format!("OP_PUSHDATA1 16 0x{}", "aa".repeat(16)))[..2],
            &[OP_PUSHDATA1, 16]
        );
        assert_eq!(
            &parse(&format!("OP_PUSHDATA1 200 0x{}", "aa".repeat(200)))[..2],
            &[OP_PUSHDATA1, 200]
        );
        assert_eq!(
            &parse(&format!("OP_PUSHDATA2 300 0x{}", "aa".repeat(300)))[..3],
            &[OP_PUSHDATA2, 0x2c, 1]
        );
        // Empty data, as Script.to_string writes it.
        assert_eq!(parse("OP_PUSHDATA1 0x00 0x"), vec![OP_PUSHDATA1, 0]);
        // Quoted text as data.
        assert_eq!(
            parse("OP_PUSHDATA1 3 'abc'"),
            vec![OP_PUSHDATA1, 3, b'a', b'b', b'c']
        );

        // After the data, tokens are ordinary again: pushed, not raw.
        for op in [
            "OP_PUSHDATA1 0x03",
            "OP_PUSHDATA2 0x0300",
            "OP_PUSHDATA4 0x03000000",
        ] {
            let script = format!("{op} 0x010203 0x0405 20 OP_ADD");
            let parsed = parse(&script);
            assert_eq!(
                &parsed[parsed.len() - 6..],
                &[2, 4, 5, 1, 20, OP_ADD],
                "{script}"
            );
        }
    }

    #[test]
    fn explicit_pushdata_errors() {
        for (script, reason) in [
            ("OP_PUSHDATA1", "must be followed by a length and data"),
            ("OP_PUSHDATA1 0x03", "must be followed by a length and data"),
            ("OP_PUSHDATA1 256 0x01", "does not fit in 1 bytes"),
            ("OP_PUSHDATA2 65536 0x01", "does not fit in 2 bytes"),
            ("OP_PUSHDATA1 -1 0x01", "must be a number or 1 bytes of hex"),
            (
                "OP_PUSHDATA2 0x03 0x010203",
                "must be a number or 2 bytes of hex",
            ),
            (
                "OP_PUSHDATA1 OP_ADD 0x01",
                "must be a number or 1 bytes of hex",
            ),
            (
                "OP_PUSHDATA1 0x03 OP_ADD",
                "must be hex, quoted text or a number",
            ),
            ("OP_PUSHDATA1 0x03 0x0102", "says 3 bytes but '0x0102' is 2"),
            (
                "OP_PUSHDATA4 2 0x010203",
                "says 2 bytes but '0x010203' is 3",
            ),
        ] {
            let err = parse_err(script);
            assert!(err.contains(reason), "{script}: {err}");
        }
    }

    /// What Script.to_string writes, parse_string reads back.
    #[test]
    fn to_string_round_trips() {
        let mut script = Script::new();
        script.append_data(&[0xaa; 3]);
        script.append_data(&[0xbb; 80]);
        script.append_data(&[0xcc; 300]);
        script.append(op_codes::OP_PUSHDATA4);
        script.append_slice(&5u32.to_le_bytes());
        script.append_slice(&[0xdd; 5]);
        script.append(op_codes::OP_ADD);
        let text = PyScript::new(&script.0).to_string();
        assert_eq!(parse(&text), script.0, "{text}");
    }

    #[test]
    fn pushdata_lengths() {
        assert_eq!(pushdata_bytes(&[]), vec![op_codes::OP_0]);
        assert_eq!(pushdata_bytes(&[7; 75])[0], 75);
        assert_eq!(
            &pushdata_bytes(&[7; 76])[..2],
            &[op_codes::OP_PUSHDATA1, 76]
        );
        assert_eq!(
            &pushdata_bytes(&[7; 256])[..3],
            &[op_codes::OP_PUSHDATA2, 0, 1]
        );
        assert_eq!(
            &pushdata_bytes(&[7; 65536])[..5],
            &[op_codes::OP_PUSHDATA4, 0, 0, 1, 0]
        );
    }
}
