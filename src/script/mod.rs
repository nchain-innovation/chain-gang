//! Script opcodes and interpreter
//!
//! # Examples
//!
//! Evaluate a script that divides two numbers:
//!
//! ```rust
//! use chain_gang::script::op_codes::*;
//! use chain_gang::script::{Script, TransactionlessChecker, NO_FLAGS};
//!
//! let mut script = Script::new();
//! script.append(OP_10);
//! script.append(OP_5);
//! script.append(OP_DIV);
//!
//! script.eval(&mut TransactionlessChecker {}, NO_FLAGS).unwrap();
//! ```

use crate::util::ChainGangError;
use std::fmt;

mod checker;
mod format;
mod interpreter;
#[allow(dead_code)]
pub mod op_codes;
pub mod stack;

pub use self::checker::{
    Checker, TransactionChecker, TransactionlessChecker, TxVersionChecker, ZChecker,
    ZVersionChecker,
};
pub(crate) use self::interpreter::next_op;
pub use self::interpreter::{
    eval_two_phase, eval_two_phase_with_stack, is_push_only, max_script_num_length,
    uses_relaxed_malleability, uses_two_phase_eval, NO_FLAGS, PREGENESIS_RULES,
};
pub use self::stack::{
    check_script_num_length, MAX_SCRIPT_NUM_LENGTH_CHRONICLE, MAX_SCRIPT_NUM_LENGTH_GENESIS,
    MAX_SCRIPT_NUM_LENGTH_PREGENESIS,
};
pub use self::stack::Stack;

use self::format::{format_script, ScriptFormatStyle};

/// Transaction script
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct Script(pub Vec<u8>);

impl Script {
    /// Creates a new empty script
    pub fn new() -> Script {
        Script(vec![])
    }

    /// Appends a single opcode or data byte
    pub fn append(&mut self, byte: u8) {
        self.0.push(byte);
    }

    /// Appends a slice of data
    pub fn append_slice(&mut self, slice: &[u8]) {
        self.0.extend_from_slice(slice);
    }

    /// Appends the opcodes and provided data that push it onto the stack
    pub fn append_data(&mut self, data: &[u8]) {
        let len = data.len();
        match len {
            0 => self.0.push(op_codes::OP_0),
            1..=75 => {
                self.0.push(op_codes::OP_PUSH + len as u8);
                self.0.extend_from_slice(data);
            }
            76..=255 => {
                self.0.push(op_codes::OP_PUSHDATA1);
                self.0.push(len as u8);
                self.0.extend_from_slice(data);
            }
            256..=65535 => {
                self.0.push(op_codes::OP_PUSHDATA2);
                self.0.push(len as u8);
                self.0.push((len >> 8) as u8);
                self.0.extend_from_slice(data);
            }
            _ => {
                self.0.push(op_codes::OP_PUSHDATA4);
                self.0.push(len as u8);
                self.0.push((len >> 8) as u8);
                self.0.push((len >> 16) as u8);
                self.0.push((len >> 24) as u8);
                self.0.extend_from_slice(data);
            }
        }
    }

    /// Appends the opcodes to push a number to the stack
    ///
    /// The number must be in the range [2^-31+1,2^31-1].
    pub fn append_num(&mut self, n: i32) -> Result<(), ChainGangError> {
        self.append_data(&stack::encode_num(n as i64)?);
        Ok(())
    }

    /// Evaluates a script using the provided checker
    pub fn eval<T: Checker>(&self, checker: &mut T, flags: u32) -> Result<(), ChainGangError> {
        self::interpreter::eval(&self.0, checker, flags)
    }

    /// Evaluates a script using the provided checker, returning the stacks for inspection
    pub fn eval_with_stack<T: Checker>(
        &self,
        checker: &mut T,
        flags: u32,
        start_at: Option<usize>,
        break_at: Option<usize>,
        stack_val: Option<Stack>,
        alt_stack_val: Option<Stack>,
    ) -> Result<(Stack, Stack, Option<usize>), ChainGangError> {
        self::interpreter::core_eval(
            &self.0,
            checker,
            flags,
            start_at,
            break_at,
            stack_val,
            alt_stack_val,
            None,
        )
    }

    // Used by PyScript
    pub fn string_representation(&self, include_byte_offsets: bool) -> String {
        format_script(
            &self.0,
            ScriptFormatStyle::StringRep {
                include_byte_offsets,
            },
            "",
            "",
        )
    }
}

impl fmt::Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&format_script(
            &self.0,
            ScriptFormatStyle::Debug,
            "[",
            "]",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::op_codes::*;
    use super::*;
    use crate::script::stack::encode_num;

    #[test]
    fn append_data() {
        let mut s = Script::new();
        s.append_data(&[]);
        assert!(s.0.len() == 1);

        let mut s = Script::new();
        s.append_data(&[0; 1]);
        assert!(s.0[0] == OP_PUSH + 1 && s.0.len() == 2);

        let mut s = Script::new();
        s.append_data(&[0; 75]);
        assert!(s.0[0] == OP_PUSH + 75 && s.0.len() == 76);

        let mut s = Script::new();
        s.append_data(&[0; 76]);
        assert!(s.0[0] == OP_PUSHDATA1 && s.0[1] == 76 && s.0.len() == 78);

        let mut s = Script::new();
        s.append_data(&vec![0; 255]);
        assert!(s.0[0] == OP_PUSHDATA1 && s.0[1] == 255 && s.0.len() == 257);

        let mut s = Script::new();
        s.append_data(&vec![0; 256]);
        assert!(s.0[0] == OP_PUSHDATA2 && s.0[1] == 0 && s.0[2] == 1 && s.0.len() == 259);

        let mut s = Script::new();
        s.append_data(&vec![0; 65535]);
        assert!(s.0[0] == OP_PUSHDATA2 && s.0[1] == 255 && s.0[2] == 255 && s.0.len() == 65538);

        let mut s = Script::new();
        s.append_data(&vec![0; 65536]);
        assert!(s.0[0] == OP_PUSHDATA4 && s.0[1] == 0 && s.0[2] == 0 && s.0[3] == 1);
        assert!(s.0.len() == 65541);
    }

    #[test]
    fn eval_with_stack_start_at_skips_prefix() {
        use crate::script::op_codes::*;
        let script = [OP_1, OP_2, OP_3];
        let (stack, _, _) = Script(script.to_vec())
            .eval_with_stack(
                &mut TransactionlessChecker {},
                NO_FLAGS,
                Some(2),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(stack.len(), 1);
        assert_eq!(stack[0], encode_num(3).unwrap());
    }

    #[test]
    fn test_debug() {
        let mut script = Script::new();
        script.append_slice(&[OP_10, OP_5, OP_DIV]);
        let result = script.eval_with_stack(
            &mut TransactionlessChecker {},
            NO_FLAGS,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());

        if let Ok((stack, _alt_stack, _script_counter)) = result {
            assert_eq!(stack[0][0], 2);
        }
    }
}
