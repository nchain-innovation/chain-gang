use crate::script::op_codes::*;
use crate::script::stack::{
    check_script_num_length, decode_bigint, decode_bool, encode_bigint, encode_num,
    is_minimally_encoded, pop_bigint_checked, pop_bool, pop_bool_minimal, pop_num_minimal,
    push_bigint_checked, Stack, MAX_SCRIPT_NUM_LENGTH_CHRONICLE, MAX_SCRIPT_NUM_LENGTH_GENESIS,
    MAX_SCRIPT_NUM_LENGTH_PREGENESIS,
};
use crate::script::Checker;
use crate::transaction::sighash::SIGHASH_FORKID;
use crate::util::{hash160, lshift, rshift, sha1::sha1, sha256::sha256, sha256d, ChainGangError};

use num_bigint::BigInt;
use num_traits::{One, ToPrimitive, Zero};
use ripemd::{Digest, Ripemd160};

// Stack capacity defaults, which may exceeded
const STACK_CAPACITY: usize = 100;
const ALT_STACK_CAPACITY: usize = 10;

/// Execute the script with genesis rules
pub const NO_FLAGS: u32 = 0x00;

/// Flag to execute the script with pre-genesis rules
pub const PREGENESIS_RULES: u32 = 0x01;

/// Whether script inputs are evaluated in separate unlock/lock phases (Chronicle).
pub fn uses_two_phase_eval(tx_version: u32) -> bool {
    tx_version > 1
}

/// Whether malleability-related script rules are relaxed (Chronicle).
pub fn uses_relaxed_malleability(tx_version: u32) -> bool {
    tx_version > 1
}

/// Maximum encoded script number length for the current evaluation context.
pub fn max_script_num_length<T: Checker>(checker: &T, flags: u32) -> usize {
    if flags & PREGENESIS_RULES != 0 {
        return MAX_SCRIPT_NUM_LENGTH_PREGENESIS;
    }
    if let Ok(version) = checker.tx_version() {
        if version as u32 > 1 {
            return MAX_SCRIPT_NUM_LENGTH_CHRONICLE;
        }
    }
    MAX_SCRIPT_NUM_LENGTH_GENESIS
}

/// True when the script contains only push operations.
pub fn is_push_only(script: &[u8]) -> bool {
    let mut i = 0;
    while i < script.len() {
        match script[i] {
            OP_0 | OP_1NEGATE | OP_1..=OP_16 | 1..=75 | OP_PUSHDATA1 | OP_PUSHDATA2 | OP_PUSHDATA4 => {}
            _ => return false,
        }
        i = next_op(i, script);
    }
    true
}

fn tx_enforces_malleability_rules<T: Checker>(checker: &T) -> bool {
    match checker.tx_version() {
        Ok(version) => !uses_relaxed_malleability(version as u32),
        Err(_) => false,
    }
}

fn pop_num_for_eval<T: Checker>(stack: &mut Stack, checker: &T) -> Result<i32, ChainGangError> {
    pop_num_minimal(stack, tx_enforces_malleability_rules(checker))
}

fn pop_bool_for_if<T: Checker>(stack: &mut Stack, checker: &T) -> Result<bool, ChainGangError> {
    pop_bool_minimal(stack, tx_enforces_malleability_rules(checker))
}

fn check_canonical_push(i: usize, script: &[u8]) -> Result<(), ChainGangError> {
    let op = script[i];
    match op {
        OP_0 => Ok(()),
        1..=75 => {
            let len = op as usize;
            if len == 0 {
                return Err(ChainGangError::ScriptError(
                    "Non-minimal push".to_string(),
                ));
            }
            if len == 1 {
                match script[i + 1] {
                    0 | 1..=16 | OP_1NEGATE => {
                        return Err(ChainGangError::ScriptError(
                            "Non-minimal push".to_string(),
                        ));
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        OP_PUSHDATA1 => {
            if i + 1 >= script.len() {
                return Ok(());
            }
            if (script[i + 1] as usize) < 76 {
                Err(ChainGangError::ScriptError(
                    "Non-minimal push".to_string(),
                ))
            } else {
                Ok(())
            }
        }
        OP_PUSHDATA2 => {
            if i + 2 >= script.len() {
                return Ok(());
            }
            let len = (script[i + 1] as usize) + ((script[i + 2] as usize) << 8);
            if len <= 255 {
                Err(ChainGangError::ScriptError(
                    "Non-minimal push".to_string(),
                ))
            } else {
                Ok(())
            }
        }
        OP_PUSHDATA4 => {
            if i + 4 >= script.len() {
                return Ok(());
            }
            let len = (script[i + 1] as usize)
                + ((script[i + 2] as usize) << 8)
                + ((script[i + 3] as usize) << 16)
                + ((script[i + 4] as usize) << 24);
            if len <= 65535 {
                Err(ChainGangError::ScriptError(
                    "Non-minimal push".to_string(),
                ))
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

fn validate_final_stack<T: Checker>(stack: &Stack, checker: &T) -> Result<(), ChainGangError> {
    if stack.is_empty() {
        return Err(ChainGangError::ScriptError(
            "Stack empty".to_string(),
        ));
    }
    if !decode_bool(&stack[stack.len() - 1]) {
        return Err(ChainGangError::ScriptError(
            "Top of stack is false".to_string(),
        ));
    }
    if tx_enforces_malleability_rules(checker) && stack.len() != 1 {
        return Err(ChainGangError::ScriptError(
            "Clean stack violation".to_string(),
        ));
    }
    Ok(())
}

/// Phase of a two-phase unlock/lock script evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwoPhasePhase {
    Unlock,
    Lock,
}

/// Context for Chronicle two-phase script evaluation (`tx.version > 1`).
pub struct TwoPhaseEvalContext<'a> {
    pub lock_script: &'a [u8],
    pub phase: TwoPhasePhase,
}

fn verif_branch_exec<T: Checker>(
    checker: &T,
    comparison: BigInt,
    invert: bool,
) -> Result<bool, ChainGangError> {
    let version = BigInt::from(checker.tx_version()?);
    let execute = version >= comparison;
    Ok(if invert { !execute } else { execute })
}

fn substr_error(msg: &str) -> ChainGangError {
    ChainGangError::ScriptError(msg.to_string())
}

fn strip_code_separators(script: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(script.len());
    let mut i = 0;
    while i < script.len() {
        let next = next_op(i, script);
        if script[i] != OP_CODESEPARATOR {
            result.extend_from_slice(&script[i..next]);
        }
        i = next;
    }
    result
}

fn checksig_script_code(
    script: &[u8],
    check_index: usize,
    sig: &[u8],
    two_phase: Option<&TwoPhaseEvalContext>,
) -> Vec<u8> {
    let mut cleaned_script = match two_phase {
        None => script[check_index..].to_vec(),
        Some(ctx) if ctx.phase == TwoPhasePhase::Unlock => {
            let mut code = strip_code_separators(&script[check_index..]);
            code.extend(strip_code_separators(ctx.lock_script));
            code
        }
        Some(_) => strip_code_separators(&script[check_index..]),
    };
    if prefork(sig) {
        cleaned_script = remove_sig(sig, &cleaned_script);
    }
    cleaned_script
}

fn multisig_script_code(
    script: &[u8],
    check_index: usize,
    two_phase: Option<&TwoPhaseEvalContext>,
) -> Vec<u8> {
    match two_phase {
        None => script[check_index..].to_vec(),
        Some(ctx) if ctx.phase == TwoPhasePhase::Unlock => {
            let mut code = strip_code_separators(&script[check_index..]);
            code.extend(strip_code_separators(ctx.lock_script));
            code
        }
        Some(_) => strip_code_separators(&script[check_index..]),
    }
}

/// Core of the script evaluation - split out for debugging
pub fn core_eval<T: Checker>(
    script: &[u8],
    checker: &mut T,
    flags: u32,
    start_at: Option<usize>,
    break_at: Option<usize>,
    stack_param: Option<Stack>,
    alt_stack_param: Option<Stack>,
    two_phase: Option<&TwoPhaseEvalContext>,
) -> Result<(Stack, Stack, Option<usize>), ChainGangError> {
    let mut stack: Stack = stack_param.unwrap_or_else(|| Vec::with_capacity(STACK_CAPACITY));
    let mut alt_stack: Stack =
        alt_stack_param.unwrap_or_else(|| Vec::with_capacity(ALT_STACK_CAPACITY));

    // True if executing current if/else branch, false if next else
    let mut branch_exec: Vec<bool> = Vec::new();
    let mut check_index = 0;
    let mut i = start_at.unwrap_or(0);
    let max_num_len = max_script_num_length(checker, flags);

    'outer: while i < script.len() {
        if !branch_exec.is_empty() && !branch_exec[branch_exec.len() - 1] {
            i = skip_branch(script, i);
            if i >= script.len() {
                break;
            }
        }
        if let Some(val) = break_at {
            // hit our breakpoint
            if i >= val {
                break;
            }
        }
        match script[i] {
            OP_0 => stack.push(encode_num(0)?),
            OP_1NEGATE => stack.push(encode_num(-1)?),
            OP_1 => stack.push(encode_num(1)?),
            OP_2 => stack.push(encode_num(2)?),
            OP_3 => stack.push(encode_num(3)?),
            OP_4 => stack.push(encode_num(4)?),
            OP_5 => stack.push(encode_num(5)?),
            OP_6 => stack.push(encode_num(6)?),
            OP_7 => stack.push(encode_num(7)?),
            OP_8 => stack.push(encode_num(8)?),
            OP_9 => stack.push(encode_num(9)?),
            OP_10 => stack.push(encode_num(10)?),
            OP_11 => stack.push(encode_num(11)?),
            OP_12 => stack.push(encode_num(12)?),
            OP_13 => stack.push(encode_num(13)?),
            OP_14 => stack.push(encode_num(14)?),
            OP_15 => stack.push(encode_num(15)?),
            OP_16 => stack.push(encode_num(16)?),
            len @ 1..=75 => {
                remains(i + 1, len as usize, script)?;
                if tx_enforces_malleability_rules(checker) {
                    check_canonical_push(i, script)?;
                }
                let data = &script[i + 1..i + 1 + len as usize];
                if tx_enforces_malleability_rules(checker) && !is_minimally_encoded(data) {
                    return Err(ChainGangError::ScriptError(
                        "Non-minimal push data".to_string(),
                    ));
                }
                stack.push(data.to_vec());
            }
            OP_PUSHDATA1 => {
                remains(i + 1, 1, script)?;
                let len = script[i + 1] as usize;
                remains(i + 2, len, script)?;
                if tx_enforces_malleability_rules(checker) {
                    check_canonical_push(i, script)?;
                }
                let data = &script[i + 2..i + 2 + len];
                if tx_enforces_malleability_rules(checker) && !is_minimally_encoded(data) {
                    return Err(ChainGangError::ScriptError(
                        "Non-minimal push data".to_string(),
                    ));
                }
                stack.push(data.to_vec());
            }
            OP_PUSHDATA2 => {
                remains(i + 1, 2, script)?;
                let len = (script[i + 1] as usize) + ((script[i + 2] as usize) << 8);
                remains(i + 3, len, script)?;
                if tx_enforces_malleability_rules(checker) {
                    check_canonical_push(i, script)?;
                }
                let data = &script[i + 3..i + 3 + len];
                if tx_enforces_malleability_rules(checker) && !is_minimally_encoded(data) {
                    return Err(ChainGangError::ScriptError(
                        "Non-minimal push data".to_string(),
                    ));
                }
                stack.push(data.to_vec());
            }
            OP_PUSHDATA4 => {
                remains(i + 1, 4, script)?;
                let len = (script[i + 1] as usize)
                    + ((script[i + 2] as usize) << 8)
                    + ((script[i + 3] as usize) << 16)
                    + ((script[i + 4] as usize) << 24);
                remains(i + 5, len, script)?;
                if tx_enforces_malleability_rules(checker) {
                    check_canonical_push(i, script)?;
                }
                let data = &script[i + 5..i + 5 + len];
                if tx_enforces_malleability_rules(checker) && !is_minimally_encoded(data) {
                    return Err(ChainGangError::ScriptError(
                        "Non-minimal push data".to_string(),
                    ));
                }
                stack.push(data.to_vec());
            }
            OP_NOP => {}
            OP_VER => {
                stack.push(encode_num(checker.tx_version()? as i64)?);
            }
            OP_IF => branch_exec.push(pop_bool_for_if(&mut stack, checker)?),
            OP_NOTIF => branch_exec.push(!pop_bool_for_if(&mut stack, checker)?),
            OP_VERIF => {
                let comparison = pop_bigint_checked(&mut stack, max_num_len)?;
                branch_exec.push(verif_branch_exec(checker, comparison, false)?);
            }
            OP_VERNOTIF => {
                let comparison = pop_bigint_checked(&mut stack, max_num_len)?;
                branch_exec.push(verif_branch_exec(checker, comparison, true)?);
            }
            OP_ELSE => {
                let len = branch_exec.len();
                if len == 0 {
                    let msg = "ELSE found without matching IF".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                branch_exec[len - 1] = !branch_exec[len - 1];
            }
            OP_ENDIF => {
                if branch_exec.is_empty() {
                    let msg = "ENDIF found without matching IF".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                branch_exec.pop().unwrap();
            }
            OP_VERIFY => {
                if !pop_bool(&mut stack)? {
                    return Err(ChainGangError::ScriptError("OP_VERIFY failed".to_string()));
                }
            }
            OP_RETURN => {
                if flags & PREGENESIS_RULES == PREGENESIS_RULES {
                    return Err(ChainGangError::ScriptError("Hit OP_RETURN".to_string()));
                } else {
                    break 'outer;
                }
            }
            OP_TOALTSTACK => {
                check_stack_size(1, &stack)?;
                alt_stack.push(stack.pop().unwrap());
            }
            OP_FROMALTSTACK => {
                check_stack_size(1, &alt_stack)?;
                stack.push(alt_stack.pop().unwrap());
            }
            OP_IFDUP => {
                check_stack_size(1, &stack)?;
                if decode_bool(&stack[stack.len() - 1]) {
                    let copy = stack[stack.len() - 1].clone();
                    stack.push(copy);
                }
            }
            OP_DEPTH => {
                let depth = stack.len() as i64;
                stack.push(encode_num(depth)?);
            }
            OP_DROP => {
                check_stack_size(1, &stack)?;
                stack.pop().unwrap();
            }
            OP_DUP => {
                check_stack_size(1, &stack)?;
                let copy = stack[stack.len() - 1].clone();
                stack.push(copy);
            }
            OP_NIP => {
                check_stack_size(2, &stack)?;
                let index = stack.len() - 2;
                stack.remove(index);
            }
            OP_OVER => {
                check_stack_size(2, &stack)?;
                let copy = stack[stack.len() - 2].clone();
                stack.push(copy);
            }
            OP_PICK => {
                let n = pop_num_for_eval(&mut stack, checker)?;
                if n < 0 {
                    let msg = "OP_PICK failed, n negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                check_stack_size(n as usize + 1, &stack)?;
                let copy = stack[stack.len() - n as usize - 1].clone();
                stack.push(copy);
            }
            OP_ROLL => {
                let n = pop_num_for_eval(&mut stack, checker)?;
                if n < 0 {
                    let msg = "OP_ROLL failed, n negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                check_stack_size(n as usize + 1, &stack)?;
                let index = stack.len() - n as usize - 1;
                let item = stack.remove(index);
                stack.push(item);
            }
            OP_ROT => {
                check_stack_size(3, &stack)?;
                let index = stack.len() - 3;
                let third = stack.remove(index);
                stack.push(third);
            }
            OP_SWAP => {
                check_stack_size(2, &stack)?;
                let index = stack.len() - 2;
                let second = stack.remove(index);
                stack.push(second);
            }
            OP_TUCK => {
                check_stack_size(2, &stack)?;
                let len = stack.len();
                let top = stack[len - 1].clone();
                stack.insert(len - 2, top);
            }
            OP_2DROP => {
                check_stack_size(2, &stack)?;
                stack.pop().unwrap();
                stack.pop().unwrap();
            }
            OP_2DUP => {
                check_stack_size(2, &stack)?;
                let len = stack.len();
                let top = stack[len - 1].clone();
                let second = stack[len - 2].clone();
                stack.push(second);
                stack.push(top);
            }
            OP_3DUP => {
                check_stack_size(3, &stack)?;
                let len = stack.len();
                let top = stack[len - 1].clone();
                let second = stack[len - 2].clone();
                let third = stack[len - 3].clone();
                stack.push(third);
                stack.push(second);
                stack.push(top);
            }
            OP_2OVER => {
                check_stack_size(4, &stack)?;
                let len = stack.len();
                let third = stack[len - 3].clone();
                let fourth = stack[len - 4].clone();
                stack.push(fourth);
                stack.push(third);
            }
            OP_2ROT => {
                check_stack_size(6, &stack)?;
                let index = stack.len() - 6;
                let sixth = stack.remove(index);
                let fifth = stack.remove(index);
                stack.push(sixth);
                stack.push(fifth);
            }
            OP_2SWAP => {
                check_stack_size(4, &stack)?;
                let index = stack.len() - 4;
                let fourth = stack.remove(index);
                let third = stack.remove(index);
                stack.push(fourth);
                stack.push(third);
            }
            OP_CAT => {
                check_stack_size(2, &stack)?;
                let top = stack.pop().unwrap();
                let mut second = stack.pop().unwrap();
                second.extend_from_slice(&top);
                stack.push(second);
            }
            OP_SPLIT => {
                check_stack_size(2, &stack)?;
                let n = pop_num_for_eval(&mut stack, checker)?;
                let x = stack.pop().unwrap();
                if n < 0 {
                    let msg = "OP_SPLIT failed, n negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                } else if n > x.len() as i32 {
                    let msg = "OP_SPLIT failed, n out of range".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                } else if n == 0 {
                    stack.push(encode_num(0)?);
                    stack.push(x);
                } else if n as usize == x.len() {
                    stack.push(x);
                    stack.push(encode_num(0)?);
                } else {
                    stack.push(x[..n as usize].to_vec());
                    stack.push(x[n as usize..].to_vec());
                }
            }
            OP_SUBSTR => {
                check_stack_size(3, &stack)?;
                let length = pop_num_for_eval(&mut stack, checker)?;
                let start = pop_num_for_eval(&mut stack, checker)?;
                let s = stack.pop().unwrap();
                if s.is_empty() {
                    return Err(substr_error("OP_SUBSTR failed, zero-length source"));
                }
                if length < 0 || start < 0 {
                    return Err(substr_error("OP_SUBSTR failed, negative index or length"));
                }
                let start = start as usize;
                let length = length as usize;
                if start + length > s.len() {
                    return Err(substr_error("OP_SUBSTR failed, length out of range"));
                }
                stack.push(s[start..start + length].to_vec());
            }
            OP_LEFT => {
                check_stack_size(2, &stack)?;
                let length = pop_num_for_eval(&mut stack, checker)?;
                let s = stack.pop().unwrap();
                if length < 0 {
                    return Err(substr_error("OP_LEFT failed, negative length"));
                }
                let length = length as usize;
                if length > s.len() {
                    return Err(substr_error("OP_LEFT failed, length out of range"));
                }
                stack.push(s[..length].to_vec());
            }
            OP_RIGHT => {
                check_stack_size(2, &stack)?;
                let length = pop_num_for_eval(&mut stack, checker)?;
                let s = stack.pop().unwrap();
                if length < 0 {
                    return Err(substr_error("OP_RIGHT failed, negative length"));
                }
                let length = length as usize;
                if length > s.len() {
                    return Err(substr_error("OP_RIGHT failed, length out of range"));
                }
                let start = s.len() - length;
                stack.push(s[start..].to_vec());
            }
            OP_SIZE => {
                check_stack_size(1, &stack)?;
                let len = stack[stack.len() - 1].len();
                stack.push(encode_num(len as i64)?);
            }
            OP_AND => {
                check_stack_size(2, &stack)?;
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                if a.len() != b.len() {
                    let msg = "OP_AND failed, different sizes".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let mut result = Vec::with_capacity(a.len());
                for i in 0..a.len() {
                    result.push(a[i] & b[i]);
                }
                stack.push(result);
            }
            OP_OR => {
                check_stack_size(2, &stack)?;
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                if a.len() != b.len() {
                    let msg = "OP_OR failed, different sizes".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let mut result = Vec::with_capacity(a.len());
                for i in 0..a.len() {
                    result.push(a[i] | b[i]);
                }
                stack.push(result);
            }
            OP_XOR => {
                check_stack_size(2, &stack)?;
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                if a.len() != b.len() {
                    let msg = "OP_XOR failed, different sizes".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let mut result = Vec::with_capacity(a.len());
                for i in 0..a.len() {
                    result.push(a[i] ^ b[i]);
                }
                stack.push(result);
            }
            OP_INVERT => {
                check_stack_size(1, &stack)?;
                let input_val = stack.pop().unwrap();
                // Invert each byte in the input
                let output_val: Vec<u8> = input_val.iter().map(|x| !x).collect();
                stack.push(output_val);
            }
            OP_LSHIFT => {
                check_stack_size(2, &stack)?;
                let n = pop_num_for_eval(&mut stack, checker)?;
                if n < 0 {
                    let msg = "n must be non-negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let v = stack.pop().unwrap();
                stack.push(lshift(&v, n as usize));
            }
            OP_RSHIFT => {
                check_stack_size(2, &stack)?;
                let n = pop_num_for_eval(&mut stack, checker)?;
                if n < 0 {
                    let msg = "n must be non-negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let v = stack.pop().unwrap();
                stack.push(rshift(&v, n as usize));
            }
            OP_EQUAL => {
                check_stack_size(2, &stack)?;
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                if a == b && a.len() == b.len() {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_EQUALVERIFY => {
                check_stack_size(2, &stack)?;
                let a = stack.pop().unwrap();
                let b = stack.pop().unwrap();
                if a != b || a.len() != b.len() {
                    let msg = "OP_EQUALVERIFY operands are not equal".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
            }
            OP_1ADD => {
                let mut x = pop_bigint_checked(&mut stack, max_num_len)?;
                x += 1;
                push_bigint_checked(&mut stack, x, max_num_len)?;
            }
            OP_1SUB => {
                let mut x = pop_bigint_checked(&mut stack, max_num_len)?;
                x -= 1;
                push_bigint_checked(&mut stack, x, max_num_len)?;
            }
            OP_NEGATE => {
                let mut x = pop_bigint_checked(&mut stack, max_num_len)?;
                x = -x;
                push_bigint_checked(&mut stack, x, max_num_len)?;
            }
            OP_ABS => {
                let mut x = pop_bigint_checked(&mut stack, max_num_len)?;
                if x < BigInt::zero() {
                    x = -x;
                }
                push_bigint_checked(&mut stack, x, max_num_len)?;
            }
            OP_NOT => {
                let mut x = pop_bigint_checked(&mut stack, max_num_len)?;
                if x == BigInt::zero() {
                    x = BigInt::one();
                } else {
                    x = BigInt::zero();
                }
                push_bigint_checked(&mut stack, x, max_num_len)?;
            }
            OP_0NOTEQUAL => {
                let mut x = pop_bigint_checked(&mut stack, max_num_len)?;
                if x == BigInt::zero() {
                    x = BigInt::zero();
                } else {
                    x = BigInt::one();
                }
                push_bigint_checked(&mut stack, x, max_num_len)?;
            }
            OP_ADD => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                let sum = a + b;
                push_bigint_checked(&mut stack, sum, max_num_len)?;
            }
            OP_SUB => {
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let difference = b - a;
                push_bigint_checked(&mut stack, difference, max_num_len)?;
            }
            OP_MUL => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                let product = a * b;
                push_bigint_checked(&mut stack, product, max_num_len)?;
            }
            OP_2MUL => {
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                let two = BigInt::from(2);
                let product = a * two;
                push_bigint_checked(&mut stack, product, max_num_len)?;
            }
            OP_DIV => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if b == BigInt::zero() {
                    let msg = "OP_DIV failed, divide by 0".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let quotient = a / b;
                push_bigint_checked(&mut stack, quotient, max_num_len)?;
            }
            OP_2DIV => {
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                let b = BigInt::from(2);

                let quotient = a / b;
                push_bigint_checked(&mut stack, quotient, max_num_len)?;
            }
            OP_MOD => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if b == BigInt::zero() {
                    let msg = "OP_MOD failed, divide by 0".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let remainder = a % b;
                push_bigint_checked(&mut stack, remainder, max_num_len)?;
            }
            OP_BOOLAND => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a != BigInt::zero() && b != BigInt::zero() {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_BOOLOR => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a != BigInt::zero() || b != BigInt::zero() {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_NUMEQUAL => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a == b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_NUMEQUALVERIFY => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a != b {
                    let msg = "Numbers are not equal".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
            }
            OP_NUMNOTEQUAL => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a != b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_LESSTHAN => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a < b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_GREATERTHAN => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a > b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_LESSTHANOREQUAL => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a <= b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_GREATERTHANOREQUAL => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a >= b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_MIN => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a < b {
                    push_bigint_checked(&mut stack, a, max_num_len)?;
                } else {
                    push_bigint_checked(&mut stack, b, max_num_len)?;
                }
            }
            OP_MAX => {
                let b = pop_bigint_checked(&mut stack, max_num_len)?;
                let a = pop_bigint_checked(&mut stack, max_num_len)?;
                if a > b {
                    push_bigint_checked(&mut stack, a, max_num_len)?;
                } else {
                    push_bigint_checked(&mut stack, b, max_num_len)?;
                }
            }
            OP_WITHIN => {
                let max = pop_bigint_checked(&mut stack, max_num_len)?;
                let min = pop_bigint_checked(&mut stack, max_num_len)?;
                let x = pop_bigint_checked(&mut stack, max_num_len)?;
                if x >= min && x < max {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_NUM2BIN => {
                check_stack_size(2, &stack)?;
                let m = pop_bigint_checked(&mut stack, max_num_len)?;
                let mut n = stack.pop().unwrap();
                if m < BigInt::one() {
                    let msg = format!("OP_NUM2BIN failed. m too small: {m}");
                    return Err(ChainGangError::ScriptError(msg));
                }
                let nlen = n.len();
                if m < BigInt::from(nlen) {
                    let msg = "OP_NUM2BIN failed. n longer than m".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                if m > BigInt::from(max_num_len) {
                    let msg = "OP_NUM2BIN failed. m too big".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                check_script_num_length(nlen, max_num_len)?;
                let mut v = Vec::with_capacity(m.to_usize().unwrap());
                let mut neg = 0;
                if nlen > 0 {
                    neg = n[nlen - 1] & 128;
                    n[nlen - 1] &= 127;
                }
                // Add zeros
                let diff = m.to_usize().unwrap() - n.len();
                v.extend(std::iter::repeat_n(0, diff));
                // Prepend the value
                for b in n.iter().rev() {
                    v.insert(0, *b);
                }
                // Add the sign
                v[0] |= neg;
                check_script_num_length(v.len(), max_num_len)?;
                stack.push(v);
            }
            OP_BIN2NUM => {
                check_stack_size(1, &stack)?;
                let mut v = stack.pop().unwrap();
                check_script_num_length(v.len(), max_num_len)?;
                let n = decode_bigint(&mut v);
                let e = encode_bigint(n);
                check_script_num_length(e.len(), max_num_len)?;
                stack.push(e);
            }
            OP_RIPEMD160 => {
                check_stack_size(1, &stack)?;
                let v = stack.pop().unwrap();
                let result = Ripemd160::digest(&v).to_vec();

                stack.push(result);
            }
            OP_SHA1 => {
                check_stack_size(1, &stack)?;
                let v = stack.pop().unwrap();
                let result = sha1(&v);
                stack.push(result);
            }
            OP_SHA256 => {
                check_stack_size(1, &stack)?;
                let v = stack.pop().unwrap();
                let result = sha256(&v);
                stack.push(result);
            }
            OP_HASH160 => {
                check_stack_size(1, &stack)?;
                let v = stack.pop().unwrap();
                let hash160 = hash160(&v).0;
                stack.push(hash160.to_vec());
            }
            OP_HASH256 => {
                check_stack_size(1, &stack)?;
                let v = stack.pop().unwrap();
                let result = sha256d(&v).0;
                stack.push(result.as_ref().to_vec());
            }
            OP_CODESEPARATOR => {
                check_index = i + 1;
            }
            OP_CHECKSIG => {
                check_stack_size(2, &stack)?;
                let pubkey = stack.pop().unwrap();
                let sig = stack.pop().unwrap();
                let cleaned_script =
                    checksig_script_code(script, check_index, &sig, two_phase);

                let success = checker.check_sig(&sig, &pubkey, &cleaned_script)?;
                if tx_enforces_malleability_rules(checker) && !success && !sig.is_empty() {
                    return Err(ChainGangError::ScriptError(
                        "OP_CHECKSIG NULLFAIL".to_string(),
                    ));
                }
                match success {
                    true => stack.push(encode_num(1)?),
                    false => stack.push(encode_num(0)?),
                }
            }
            OP_CHECKSIGVERIFY => {
                check_stack_size(2, &stack)?;
                let pubkey = stack.pop().unwrap();
                let sig = stack.pop().unwrap();
                let cleaned_script =
                    checksig_script_code(script, check_index, &sig, two_phase);
                let success = checker.check_sig(&sig, &pubkey, &cleaned_script)?;
                if tx_enforces_malleability_rules(checker) && !success && !sig.is_empty() {
                    return Err(ChainGangError::ScriptError(
                        "OP_CHECKSIG NULLFAIL".to_string(),
                    ));
                }
                if !success {
                    return Err(ChainGangError::ScriptError(
                        "OP_CHECKSIGVERIFY failed".to_string(),
                    ));
                }
            }
            OP_CHECKMULTISIG => {
                let cleaned_script = multisig_script_code(script, check_index, two_phase);
                match check_multisig(&mut stack, checker, &cleaned_script)? {
                    true => stack.push(encode_num(1)?),
                    false => stack.push(encode_num(0)?),
                }
            }
            OP_CHECKMULTISIGVERIFY => {
                let cleaned_script = multisig_script_code(script, check_index, two_phase);
                if !check_multisig(&mut stack, checker, &cleaned_script)? {
                    let msg = "OP_CHECKMULTISIGVERIFY failed".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
            }
            OP_CHECKLOCKTIMEVERIFY => {
                if flags & PREGENESIS_RULES == PREGENESIS_RULES {
                    let locktime = pop_num_for_eval(&mut stack, checker)?;
                    if !checker.check_locktime(locktime)? {
                        let msg = "OP_CHECKLOCKTIMEVERIFY failed".to_string();
                        return Err(ChainGangError::ScriptError(msg));
                    }
                }
            }
            OP_CHECKSEQUENCEVERIFY => {
                if flags & PREGENESIS_RULES == PREGENESIS_RULES {
                    let sequence = pop_num_for_eval(&mut stack, checker)?;
                    if !checker.check_sequence(sequence)? {
                        let msg = "OP_CHECKSEQUENCEVERIFY failed".to_string();
                        return Err(ChainGangError::ScriptError(msg));
                    }
                }
            }
            OP_NOP1 => {}
            OP_LSHIFTNUM => {
                check_stack_size(2, &stack)?;
                let n = pop_num_for_eval(&mut stack, checker)?;
                if n < 0 {
                    let msg = "n must be non-negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let v = stack.pop().unwrap();
                stack.push(lshift(&v, n as usize));
            }
            OP_RSHIFTNUM => {
                check_stack_size(2, &stack)?;
                let n = pop_num_for_eval(&mut stack, checker)?;
                if n < 0 {
                    let msg = "n must be non-negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let v = stack.pop().unwrap();
                stack.push(rshift(&v, n as usize));
            }
            OP_NOP9 => {}
            OP_NOP10 => {}
            _ => {
                let msg = format!("Bad opcode: {}, index {}", script[i], i);
                return Err(ChainGangError::ScriptError(msg));
            }
        }
        i = next_op(i, script);
    }

    if !branch_exec.is_empty() {
        return Err(ChainGangError::ScriptError("ENDIF missing".to_string()));
    }

    let optional_i = break_at.map(|_| i);
    Ok((stack, alt_stack, optional_i))
}

/// Executes a script
pub fn eval<T: Checker>(script: &[u8], checker: &mut T, flags: u32) -> Result<(), ChainGangError> {
    match core_eval(script, checker, flags, None, None, None, None, None) {
        Ok((stack, _alt_stack, _script_counter)) => validate_final_stack(&stack, checker),
        Err(x) => Err(x),
    }
}

/// Evaluates unlock and lock scripts in separate phases (Chronicle, `tx.version > 1`).
///
/// The main stack is carried from unlock to lock; conditional and alt stacks are cleared
/// between phases. CHECKSIG scriptCode in the unlock phase spans from the last
/// OP_CODESEPARATOR in the unlock script through the end of the lock script.
pub fn eval_two_phase<T: Checker>(
    unlock: &[u8],
    lock: &[u8],
    checker: &mut T,
    flags: u32,
) -> Result<(), ChainGangError> {
    let ctx_unlock = TwoPhaseEvalContext {
        lock_script: lock,
        phase: TwoPhasePhase::Unlock,
    };
    let (stack, _, _) = core_eval(
        unlock,
        checker,
        flags,
        None,
        None,
        None,
        None,
        Some(&ctx_unlock),
    )?;

    let ctx_lock = TwoPhaseEvalContext {
        lock_script: lock,
        phase: TwoPhasePhase::Lock,
    };
    let (stack, _, _) = core_eval(
        lock,
        checker,
        flags,
        None,
        None,
        Some(stack),
        None,
        Some(&ctx_lock),
    )?;

    validate_final_stack(&stack, checker)
}

/// Like [`eval_two_phase`], but returns the final main and alt stacks after validation.
pub fn eval_two_phase_with_stack<T: Checker>(
    unlock: &[u8],
    lock: &[u8],
    checker: &mut T,
    flags: u32,
) -> Result<(Stack, Stack), ChainGangError> {
    let ctx_unlock = TwoPhaseEvalContext {
        lock_script: lock,
        phase: TwoPhasePhase::Unlock,
    };
    let (stack, _, _) = core_eval(
        unlock,
        checker,
        flags,
        None,
        None,
        None,
        None,
        Some(&ctx_unlock),
    )?;

    let ctx_lock = TwoPhaseEvalContext {
        lock_script: lock,
        phase: TwoPhasePhase::Lock,
    };
    let (stack, alt_stack, _) = core_eval(
        lock,
        checker,
        flags,
        None,
        None,
        Some(stack),
        None,
        Some(&ctx_lock),
    )?;

    validate_final_stack(&stack, checker)?;
    Ok((stack, alt_stack))
}

#[inline]
fn check_multisig<T: Checker>(
    stack: &mut Stack,
    checker: &mut T,
    script: &[u8],
) -> Result<bool, ChainGangError> {
    // Pop the keys
    let total = pop_num_for_eval(stack, checker)?;
    if total < 0 {
        return Err(ChainGangError::ScriptError(
            "total out of range".to_string(),
        ));
    }
    check_stack_size(total as usize, stack)?;
    let mut keys = Vec::with_capacity(total as usize);
    for _i in 0..total {
        keys.push(stack.pop().unwrap());
    }

    // Pop the sigs
    let required = pop_num_for_eval(stack, checker)?;
    if required < 0 || required > total {
        return Err(ChainGangError::ScriptError(
            "required out of range".to_string(),
        ));
    }
    check_stack_size(required as usize, stack)?;
    let mut sigs = Vec::with_capacity(required as usize);
    for _i in 0..required {
        sigs.push(stack.pop().unwrap());
    }

    // Pop one more off. This isn't used and can't be changed.
    check_stack_size(1, stack)?;
    let dummy = stack.pop().unwrap();
    if tx_enforces_malleability_rules(checker) && !dummy.is_empty() {
        return Err(ChainGangError::ScriptError(
            "OP_CHECKMULTISIG NULLDUMMY".to_string(),
        ));
    }

    // Remove signature for pre-fork scripts
    let mut cleaned_script = script.to_vec();
    for sig in sigs.iter() {
        if prefork(sig) {
            cleaned_script = remove_sig(sig, &cleaned_script);
        }
    }

    let mut key = 0;
    let mut sig = 0;
    while sig < sigs.len() {
        if key == keys.len() {
            return Ok(false);
        }
        if checker.check_sig(&sigs[sig], &keys[key], &cleaned_script)? {
            sig += 1;
        }
        key += 1;
    }
    let success = sig == sigs.len();
    if !success && tx_enforces_malleability_rules(checker) {
        for remaining in &sigs {
            if !remaining.is_empty() {
                return Err(ChainGangError::ScriptError(
                    "OP_CHECKMULTISIG NULLFAIL".to_string(),
                ));
            }
        }
    }
    Ok(success)
}

fn prefork(sig: &[u8]) -> bool {
    !sig.is_empty() && sig[sig.len() - 1] & SIGHASH_FORKID == 0
}

/// Removes any instances of the signature from the lock_script in pre-fork transactions
fn remove_sig(sig: &[u8], script: &[u8]) -> Vec<u8> {
    if sig.is_empty() {
        return script.to_vec();
    }
    let mut result = Vec::with_capacity(script.len());
    let mut i = 0;
    let mut start = 0;
    while i + sig.len() <= script.len() {
        if script[i..i + sig.len()] == *sig {
            result.extend_from_slice(&script[start..i]);
            start = i + sig.len();
            i = start;
        } else {
            i = next_op(i, script);
        }
    }
    result.extend_from_slice(&script[start..]);
    result
}

#[inline]
fn check_stack_size(minsize: usize, stack: &Stack) -> Result<(), ChainGangError> {
    if stack.len() < minsize {
        return Err(ChainGangError::ScriptError(format!(
            "Stack too small: {minsize}"
        )));
    }
    Ok(())
}

#[inline]
fn remains(i: usize, len: usize, script: &[u8]) -> Result<(), ChainGangError> {
    if i + len > script.len() {
        Err(ChainGangError::ScriptError(
            "Not enough data remaining".to_string(),
        ))
    } else {
        Ok(())
    }
}

/// Gets the next operation index in the script, or the script length if at the end
pub fn next_op(i: usize, script: &[u8]) -> usize {
    if i >= script.len() {
        return script.len();
    }
    let next = match script[i] {
        len @ 1..=75 => i + 1 + len as usize,
        OP_PUSHDATA1 => {
            if i + 2 > script.len() {
                return script.len();
            }
            i + 2 + script[i + 1] as usize
        }
        OP_PUSHDATA2 => {
            if i + 3 > script.len() {
                return script.len();
            }
            i + 3 + (script[i + 1] as usize) + ((script[i + 2] as usize) << 8)
        }
        OP_PUSHDATA4 => {
            if i + 5 > script.len() {
                return script.len();
            }
            let len = (script[i + 1] as usize)
                + ((script[i + 2] as usize) << 8)
                + ((script[i + 3] as usize) << 16)
                + ((script[i + 4] as usize) << 24);
            i + 5 + len
        }
        _ => i + 1,
    };
    let overflow = next > script.len();
    if overflow {
        script.len()
    } else {
        next
    }
}

/// Skips over a branch of if/else and return the index of the next else or endif opcode
fn skip_branch(script: &[u8], mut i: usize) -> usize {
    let mut sub = 0;
    while i < script.len() {
        match script[i] {
            OP_IF => sub += 1,
            OP_NOTIF => sub += 1,
            OP_VERIF => sub += 1,
            OP_VERNOTIF => sub += 1,
            OP_ELSE => {
                if sub == 0 {
                    return i;
                }
            }
            OP_ENDIF => {
                if sub == 0 {
                    return i;
                }
                sub -= 1;
            }
            _ => {}
        }
        i = next_op(i, script);
    }
    script.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::stack::{
        MAX_SCRIPT_NUM_LENGTH_CHRONICLE, MAX_SCRIPT_NUM_LENGTH_GENESIS,
        MAX_SCRIPT_NUM_LENGTH_PREGENESIS,
    };
    use crate::script::Script;
    use hex;
    use std::cell::RefCell;

    #[test]
    fn valid() {
        pass(&[OP_TRUE]);
        pass(&[OP_16]);
        pass(&[OP_PUSH + 1, 1]);
        pass(&[OP_PUSHDATA1, 2, 0, 1]);
        pass(&[OP_PUSHDATA2, 2, 0, 0, 1]);
        pass(&[OP_PUSHDATA4, 2, 0, 0, 0, 0, 1]);
        pass(&[OP_NOP, OP_NOP, OP_NOP, OP_1]);
        pass(&[OP_1, OP_1, OP_IF, OP_ELSE, OP_ENDIF]);
        pass(&[OP_1, OP_1, OP_1, OP_IF, OP_IF, OP_ENDIF, OP_ENDIF]);
        pass(&[OP_1, OP_IF, OP_1, OP_ELSE, OP_0, OP_ENDIF]);
        pass(&[OP_0, OP_IF, OP_0, OP_ELSE, OP_1, OP_ENDIF]);
        pass(&[OP_1, OP_IF, OP_0, OP_1, OP_ENDIF]);
        pass(&[OP_1, OP_IF, OP_0, OP_IF, OP_ELSE, OP_1, OP_ENDIF, OP_ENDIF]);
        pass(&[OP_1, OP_IF, OP_PUSHDATA1, 1, 0, OP_1, OP_ENDIF]);
        pass(&[OP_1, OP_IF, OP_ELSE, OP_ELSE, OP_1, OP_ENDIF]);
        pass(&[
            OP_1, OP_IF, OP_ELSE, OP_ELSE, OP_ELSE, OP_ELSE, OP_1, OP_ENDIF,
        ]);
        pass(&[OP_1, OP_VERIFY, OP_1]);
        pass(&[OP_1, OP_RETURN]);
        pass(&[OP_FALSE, OP_TRUE, OP_RETURN]);
        pass(&[OP_1, OP_0, OP_TOALTSTACK]);
        pass(&[OP_1, OP_TOALTSTACK, OP_FROMALTSTACK]);
        pass(&[OP_1, OP_IFDUP, OP_DROP, OP_DROP, OP_1]);
        pass(&[OP_DEPTH, OP_1]);
        pass(&[OP_0, OP_DEPTH]);
        pass(&[OP_1, OP_0, OP_DROP]);
        pass(&[OP_0, OP_DUP, OP_DROP, OP_DROP, OP_1]);
        pass(&[OP_1, OP_0, OP_0, OP_NIP, OP_DROP]);
        pass(&[OP_1, OP_0, OP_OVER]);
        pass(&[OP_1, OP_0, OP_PICK]);
        pass(&[OP_1, OP_0, OP_0, OP_0, OP_0, OP_4, OP_PICK]);
        pass(&[OP_1, OP_0, OP_ROLL]);
        pass(&[OP_1, OP_0, OP_0, OP_ROLL, OP_DROP]);
        pass(&[OP_1, OP_0, OP_0, OP_0, OP_0, OP_4, OP_ROLL]);
        pass(&[OP_1, OP_0, OP_0, OP_ROT]);
        pass(&[OP_0, OP_1, OP_0, OP_ROT, OP_ROT]);
        pass(&[OP_0, OP_0, OP_1, OP_ROT, OP_ROT, OP_ROT]);
        pass(&[OP_1, OP_0, OP_SWAP]);
        pass(&[OP_0, OP_1, OP_TUCK, OP_DROP, OP_DROP]);
        pass(&[OP_1, OP_0, OP_0, OP_2DROP]);
        pass(&[OP_0, OP_1, OP_2DUP]);
        pass(&[OP_0, OP_1, OP_2DUP, OP_DROP, OP_DROP]);
        pass(&[OP_0, OP_0, OP_1, OP_3DUP]);
        pass(&[OP_0, OP_0, OP_1, OP_3DUP, OP_DROP, OP_DROP, OP_DROP]);
        pass(&[OP_0, OP_1, OP_0, OP_0, OP_2OVER]);
        pass(&[OP_0, OP_0, OP_0, OP_1, OP_2OVER, OP_DROP, OP_DROP]);
        pass(&[OP_0, OP_1, OP_0, OP_0, OP_0, OP_0, OP_2ROT]);
        pass(&[OP_0, OP_0, OP_0, OP_1, OP_0, OP_0, OP_2ROT, OP_2ROT]);
        pass(&[
            OP_0, OP_0, OP_0, OP_0, OP_0, OP_1, OP_2ROT, OP_2ROT, OP_2ROT,
        ]);
        pass(&[OP_1, OP_0, OP_0, OP_0, OP_0, OP_0, OP_2ROT, OP_DROP]);
        pass(&[OP_0, OP_1, OP_0, OP_0, OP_2SWAP]);
        pass(&[OP_1, OP_0, OP_0, OP_0, OP_2SWAP, OP_DROP]);
        pass(&[OP_0, OP_1, OP_CAT]);
        pass(&[OP_1, OP_0, OP_0, OP_2, OP_0, OP_CAT, OP_PICK]);
        pass(&[OP_0, OP_0, OP_CAT, OP_IF, OP_ELSE, OP_1, OP_ENDIF]);
        pass(&[OP_PUSH + 2, OP_0, OP_1, OP_1, OP_SPLIT]);
        pass(&[OP_PUSH + 2, OP_0, OP_1, OP_2, OP_SPLIT, OP_DROP]);
        pass(&[OP_PUSH + 2, OP_0, OP_1, OP_0, OP_SPLIT]);
        pass(&[OP_0, OP_0, OP_SPLIT, OP_1]);
        pass(&[OP_1, OP_1, OP_SPLIT, OP_DROP]);
        pass(&[OP_1, OP_SIZE]);
        pass(&[OP_1, OP_SIZE, OP_DROP]);
        pass(&[OP_1, OP_1, OP_AND]);
        pass(&[OP_1, OP_1, OP_OR]);
        pass(&[OP_1, OP_1, OP_XOR, OP_IF, OP_ELSE, OP_1, OP_ENDIF]);
        pass(&[
            OP_PUSH + 3,
            0xFF,
            0x01,
            0x00,
            OP_INVERT,
            OP_PUSH + 3,
            0x00,
            0xFE,
            0xFF,
            OP_EQUAL,
        ]);
        pass(&[OP_0, OP_0, OP_LSHIFT, OP_0, OP_EQUAL]);
        pass(&[OP_4, OP_2, OP_LSHIFT, OP_16, OP_EQUAL]);
        pass(&[
            OP_PUSH + 2,
            0x12,
            0x34,
            OP_4,
            OP_LSHIFT,
            OP_PUSH + 2,
            0x23,
            0x40,
            OP_EQUAL,
        ]);
        pass(&[OP_0, OP_0, OP_RSHIFT, OP_0, OP_EQUAL]);
        pass(&[OP_4, OP_2, OP_RSHIFT, OP_1, OP_EQUAL]);
        pass(&[
            OP_PUSH + 2,
            0x12,
            0x34,
            OP_4,
            OP_RSHIFT,
            OP_PUSH + 2,
            0x01,
            0x23,
            OP_EQUAL,
        ]);
        pass(&[OP_0, OP_0, OP_EQUAL]);
        pass(&[OP_1, OP_1, OP_EQUAL]);
        pass(&[OP_1, OP_0, OP_0, OP_EQUALVERIFY]);
        pass(&[OP_0, OP_1ADD]);
        pass(&[OP_1, OP_1ADD, OP_2, OP_EQUAL]);
        pass(&[OP_2, OP_1SUB]);
        pass(&[OP_0, OP_1SUB, OP_1NEGATE, OP_EQUAL]);
        let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F];
        v.extend_from_slice(&[OP_1ADD, OP_SIZE, OP_5, OP_EQUAL]);
        pass(&v);
        let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF];
        v.extend_from_slice(&[OP_1SUB, OP_SIZE, OP_5, OP_EQUAL]);
        pass(&v);
        pass(&[OP_1, OP_NEGATE, OP_1NEGATE, OP_EQUAL]);
        pass(&[OP_1NEGATE, OP_NEGATE, OP_1, OP_EQUAL]);
        pass(&[OP_1, OP_ABS, OP_1, OP_EQUAL]);
        pass(&[OP_1NEGATE, OP_ABS, OP_1, OP_EQUAL]);
        pass(&[OP_0, OP_NOT]);
        pass(&[OP_1, OP_NOT, OP_0, OP_EQUAL]);
        pass(&[OP_2, OP_NOT, OP_0, OP_EQUAL]);
        pass(&[OP_1, OP_NOT, OP_NOT]);
        pass(&[OP_1, OP_0NOTEQUAL]);
        pass(&[OP_0, OP_0NOTEQUAL, OP_0, OP_EQUAL]);
        pass(&[OP_2, OP_0NOTEQUAL]);
        pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_1ADD]);
        pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_1SUB]);
        pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NEGATE, OP_1]);
        pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_ABS, OP_1]);
        pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NOT]);
        pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_0NOTEQUAL, OP_1]);
        pass(&[OP_0, OP_1, OP_ADD]);
        pass(&[OP_1, OP_0, OP_ADD]);
        pass(&[OP_1, OP_2, OP_ADD, OP_3, OP_EQUAL]);
        let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF];
        v.extend_from_slice(&[OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF]);
        v.extend_from_slice(&[OP_ADD, OP_SIZE, OP_5, OP_EQUAL]);
        pass(&v);
        let mut v = vec![OP_PUSH + 5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        v.extend_from_slice(&[OP_PUSH + 5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        v.extend_from_slice(&[OP_ADD, OP_SIZE, OP_6, OP_EQUAL]);
        pass(&v);
        let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F];
        v.extend_from_slice(&[OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF]);
        v.extend_from_slice(&[OP_ADD, OP_0, OP_EQUAL]);
        pass(&v);
        pass(&[OP_2, OP_1, OP_SUB]);
        pass(&[OP_1, OP_1, OP_SUB, OP_0, OP_EQUAL]);
        let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF];
        v.extend_from_slice(&[OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F]);
        v.extend_from_slice(&[OP_SUB, OP_SIZE, OP_5, OP_EQUAL]);
        pass(&v);
        let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F];
        v.extend_from_slice(&[OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F]);
        v.extend_from_slice(&[OP_SUB, OP_0, OP_EQUAL]);
        pass(&v);
        pass(&[OP_1, OP_1, OP_MUL, OP_1, OP_EQUAL]);
        pass(&[OP_2, OP_3, OP_MUL, OP_6, OP_EQUAL]);
        pass(&[
            OP_PUSH + 4,
            0xFF,
            0xFF,
            0xFF,
            0x7F,
            OP_PUSH + 4,
            0xFF,
            0xFF,
            0xFF,
            0x7F,
            OP_MUL,
        ]);
        pass(&[OP_1, OP_1NEGATE, OP_MUL, OP_1NEGATE, OP_EQUAL]);
        pass(&[OP_5, OP_2MUL, OP_10, OP_EQUAL]);
        pass(&[OP_1, OP_1, OP_DIV, OP_1, OP_EQUAL]);
        pass(&[OP_5, OP_2, OP_DIV, OP_2, OP_EQUAL]);
        pass(&[OP_2, OP_1NEGATE, OP_DIV, OP_PUSH + 1, 130, OP_EQUAL]);
        pass(&[OP_5, OP_2DIV, OP_2, OP_EQUAL]);
        pass(&[OP_1, OP_1, OP_MOD, OP_0, OP_EQUAL]);
        pass(&[OP_5, OP_2, OP_MOD, OP_1, OP_EQUAL]);
        pass(&[OP_5, OP_PUSH + 1, 130, OP_MOD, OP_1, OP_EQUAL]);
        pass(&[OP_PUSH + 1, 133, OP_2, OP_MOD, OP_1NEGATE, OP_EQUAL]);
        pass(&[OP_1, OP_1, OP_BOOLAND]);
        pass(&[OP_0, OP_1, OP_BOOLAND, OP_0, OP_EQUAL]);
        pass(&[OP_1, OP_0, OP_BOOLOR]);
        pass(&[OP_0, OP_0, OP_BOOLOR, OP_0, OP_EQUAL]);
        pass(&[OP_1, OP_1, OP_NUMEQUAL]);
        pass(&[OP_0, OP_1, OP_NUMEQUAL, OP_NOT]);
        pass(&[OP_1, OP_1, OP_NUMEQUALVERIFY, OP_1]);
        pass(&[OP_1, OP_0, OP_NUMNOTEQUAL]);
        pass(&[OP_1, OP_1, OP_NUMNOTEQUAL, OP_NOT]);
        pass(&[OP_0, OP_1, OP_LESSTHAN]);
        pass(&[OP_1NEGATE, OP_0, OP_LESSTHAN]);
        pass(&[OP_0, OP_0, OP_LESSTHAN, OP_NOT]);
        pass(&[OP_1, OP_0, OP_GREATERTHAN]);
        pass(&[OP_0, OP_1NEGATE, OP_GREATERTHAN]);
        pass(&[OP_0, OP_0, OP_GREATERTHAN, OP_NOT]);
        pass(&[OP_0, OP_1, OP_LESSTHANOREQUAL]);
        pass(&[OP_1NEGATE, OP_0, OP_LESSTHANOREQUAL]);
        pass(&[OP_0, OP_0, OP_LESSTHANOREQUAL]);
        pass(&[OP_1, OP_0, OP_GREATERTHANOREQUAL]);
        pass(&[OP_0, OP_1NEGATE, OP_GREATERTHANOREQUAL]);
        pass(&[OP_0, OP_0, OP_GREATERTHANOREQUAL]);
        pass(&[OP_0, OP_1, OP_MIN, OP_0, OP_EQUAL]);
        pass(&[OP_0, OP_0, OP_MIN, OP_0, OP_EQUAL]);
        pass(&[OP_1NEGATE, OP_0, OP_MIN, OP_1NEGATE, OP_EQUAL]);
        pass(&[OP_0, OP_1, OP_MAX, OP_1, OP_EQUAL]);
        pass(&[OP_0, OP_0, OP_MAX, OP_0, OP_EQUAL]);
        pass(&[OP_1NEGATE, OP_0, OP_MAX, OP_0, OP_EQUAL]);
        pass(&[OP_0, OP_0, OP_1, OP_WITHIN]);
        pass(&[OP_0, OP_1NEGATE, OP_1, OP_WITHIN]);

        pass(&[OP_PUSH + 9, 0, 0, 0, 0, 0, 0, 0, 0, 1, OP_BIN2NUM]);

        //pass(&[OP_PUSH + 4, 128, 0, 0, 1, OP_BIN2NUM, OP_1NEGATE, OP_EQUAL]);

        pass(&[OP_PUSH + 7, 0, 0, 0, 0, 0, 0, 0, OP_BIN2NUM, OP_0, OP_EQUAL]);
        pass(&[OP_PUSH + 5, 129, 0, 0, 0, 0, OP_BIN2NUM]);
        pass(&[OP_1, OP_16, OP_NUM2BIN]);
        pass(&[OP_0, OP_4, OP_NUM2BIN, OP_0, OP_NUMEQUAL]);

        // pass(&[OP_1, OP_DUP, OP_16, OP_NUM2BIN, OP_BIN2NUM, OP_EQUAL]);
        // pass(&[OP_1NEGATE, OP_DUP, OP_16, OP_NUM2BIN, OP_BIN2NUM, OP_EQUAL]);

        pass(&[OP_1, OP_PUSH + 5, 129, 0, 0, 0, 0, OP_NUM2BIN]);

        let mut v = Vec::new();
        v.push(OP_1);
        v.push(OP_PUSH + 2);
        v.extend_from_slice(&encode_num(520).unwrap());
        v.push(OP_NUM2BIN);
        pass(&v);
        pass(&[OP_1, OP_RIPEMD160]);
        pass(&[OP_0, OP_RIPEMD160]);
        let mut s = Script::new();
        let h = "cea1b21f1a739fba68d1d4290437d2c5609be1d3";
        s.append_data(&hex::decode(h).unwrap());
        s.append_data(&hex::decode("0123456789abcdef").unwrap());
        s.append_slice(&[OP_RIPEMD160, OP_EQUAL]);
        pass(&s.0);
        pass(&[OP_1, OP_SHA1]);
        pass(&[OP_0, OP_SHA1]);
        let mut s = Script::new();
        let h = "0ca2eadb529ac2e63abf9b4ae3df8ee121f10547";
        s.append_data(&hex::decode(h).unwrap());
        s.append_data(&hex::decode("0123456789abcdef").unwrap());
        s.append_slice(&[OP_SHA1, OP_EQUAL]);
        pass(&s.0);
        pass(&[OP_1, OP_SHA256]);
        pass(&[OP_0, OP_SHA256]);
        let mut s = Script::new();
        let h = "55c53f5d490297900cefa825d0c8e8e9532ee8a118abe7d8570762cd38be9818";
        s.append_data(&hex::decode(h).unwrap());
        s.append_data(&hex::decode("0123456789abcdef").unwrap());
        s.append_slice(&[OP_SHA256, OP_EQUAL]);
        pass(&s.0);
        pass(&[OP_1, OP_HASH160]);
        pass(&[OP_0, OP_HASH160]);
        let mut s = Script::new();
        let h = "a956ed79819901b1b2c7b3ec045081f749c588ed";
        s.append_data(&hex::decode(h).unwrap());
        s.append_data(&hex::decode("0123456789abcdef").unwrap());
        s.append_slice(&[OP_HASH160, OP_EQUAL]);
        pass(&s.0);
        pass(&[OP_1, OP_HASH256]);
        pass(&[OP_0, OP_HASH256]);
        let mut s = Script::new();
        let h = "137ad663f79da06e282ed0abbec4d70523ced5ff8e39d5c2e5641d978c5925aa";
        s.append_data(&hex::decode(h).unwrap());
        s.append_data(&hex::decode("0123456789abcdef").unwrap());
        s.append_slice(&[OP_HASH256, OP_EQUAL]);
        pass(&s.0);
        pass(&[OP_1, OP_1, OP_CHECKSIG]);
        pass(&[OP_1, OP_1, OP_CHECKSIGVERIFY, OP_1]);
        pass(&[OP_0, OP_0, OP_0, OP_CHECKMULTISIG]);
        pass(&[OP_0, OP_0, OP_9, OP_9, OP_9, OP_3, OP_CHECKMULTISIG]);
        pass(&[OP_0, OP_9, OP_1, OP_9, OP_1, OP_CHECKMULTISIG]);
        pass(&[OP_0, OP_9, OP_1, OP_9, OP_9, OP_9, OP_3, OP_CHECKMULTISIG]);
        let mut c = MockChecker::sig_checks(vec![true]);
        assert!(eval(
            &[OP_0, OP_9, OP_1, OP_9, OP_1, OP_CHECKMULTISIG],
            &mut c,
            NO_FLAGS
        )
        .is_ok());
        let mut c = MockChecker::sig_checks(vec![false, true, true]);
        let mut s = vec![OP_0, OP_9, OP_9, OP_2, OP_9, OP_9, OP_9, OP_3];
        s.push(OP_CHECKMULTISIG);
        assert!(eval(&s, &mut c, NO_FLAGS).is_ok());
        pass_pregenesis(&[OP_0, OP_CHECKLOCKTIMEVERIFY, OP_1]);
        pass(&[OP_CHECKLOCKTIMEVERIFY, OP_1]);
        pass_pregenesis(&[OP_0, OP_CHECKSEQUENCEVERIFY, OP_1]);
        pass(&[OP_CHECKSEQUENCEVERIFY, OP_1]);
        pass(&[OP_NOP1, OP_1]);
        pass(&[OP_NOP9, OP_1]);
        pass(&[OP_NOP10, OP_1]);
        let mut v = vec![OP_DEPTH; 501];
        v.push(OP_1);
        pass(&v);
        pass(&vec![OP_1; 10001]);
    }

    #[test]
    fn invalid() {
        fail(&[]);
        fail(&[OP_FALSE]);
        fail(&[OP_PUSH + 1]);
        fail(&[OP_PUSH + 3, 0, 1]);
        fail(&[OP_PUSHDATA1, 0]);
        fail(&[OP_PUSHDATA1, 1]);
        fail(&[OP_PUSHDATA1, 10, 0]);
        fail(&[OP_PUSHDATA2, 20, 0]);
        fail(&[OP_PUSHDATA4, 30, 0]);
        fail(&[OP_IF, OP_ENDIF]);
        fail(&[OP_1, OP_1, OP_IF]);
        fail(&[OP_1, OP_1, OP_NOTIF]);
        fail(&[OP_1, OP_ELSE]);
        fail(&[OP_1, OP_ENDIF]);
        fail(&[OP_1, OP_1, OP_IF, OP_ELSE]);
        fail(&[OP_1, OP_1, OP_IF, OP_IF, OP_ENDIF]);
        fail(&[OP_0, OP_IF, OP_1, OP_ELSE, OP_0, OP_ENDIF]);
        fail(&[OP_0, OP_IF, OP_PUSHDATA1, 1, 1, OP_1, OP_ENDIF]);
        fail(&[OP_VERIFY]);
        fail(&[OP_0, OP_VERIFY]);
        fail(&[OP_RETURN]);
        fail(&[OP_FALSE, OP_RETURN]);
        fail_pregenesis(&[OP_RETURN]);
        fail_pregenesis(&[OP_1, OP_RETURN, OP_1]);
        fail(&[OP_TOALTSTACK]);
        fail(&[OP_FROMALTSTACK]);
        fail(&[OP_0, OP_TOALTSTACK, OP_1, OP_FROMALTSTACK]);
        fail(&[OP_IFDUP]);
        fail(&[OP_DROP]);
        fail(&[OP_1, OP_DROP, OP_DROP]);
        fail(&[OP_DUP]);
        fail(&[OP_NIP]);
        fail(&[OP_1, OP_NIP]);
        fail(&[OP_OVER]);
        fail(&[OP_1, OP_OVER]);
        fail(&[OP_PICK]);
        fail(&[OP_0, OP_PICK]);
        fail(&[OP_0, OP_1, OP_PICK]);
        fail(&[OP_ROLL]);
        fail(&[OP_0, OP_ROLL]);
        fail(&[OP_0, OP_1, OP_ROLL]);
        fail(&[OP_ROT]);
        fail(&[OP_1, OP_ROT]);
        fail(&[OP_1, OP_1, OP_ROT]);
        fail(&[OP_0, OP_1, OP_1, OP_ROT]);
        fail(&[OP_SWAP]);
        fail(&[OP_1, OP_SWAP]);
        fail(&[OP_0, OP_1, OP_SWAP]);
        fail(&[OP_TUCK]);
        fail(&[OP_1, OP_TUCK]);
        fail(&[OP_1, OP_0, OP_TUCK]);
        fail(&[OP_2DROP]);
        fail(&[OP_1, OP_2DROP]);
        fail(&[OP_1, OP_1, OP_2DROP]);
        fail(&[OP_2DUP]);
        fail(&[OP_1, OP_2DUP]);
        fail(&[OP_1, OP_0, OP_2DUP]);
        fail(&[OP_3DUP]);
        fail(&[OP_1, OP_3DUP]);
        fail(&[OP_1, OP_1, OP_3DUP]);
        fail(&[OP_1, OP_1, OP_0, OP_3DUP]);
        fail(&[OP_2OVER]);
        fail(&[OP_1, OP_2OVER]);
        fail(&[OP_1, OP_1, OP_2OVER]);
        fail(&[OP_1, OP_1, OP_1, OP_2OVER]);
        fail(&[OP_1, OP_0, OP_1, OP_1, OP_2OVER]);
        fail(&[OP_2ROT]);
        fail(&[OP_1, OP_2ROT]);
        fail(&[OP_1, OP_1, OP_2ROT]);
        fail(&[OP_1, OP_1, OP_1, OP_2ROT]);
        fail(&[OP_1, OP_1, OP_1, OP_1, OP_2ROT]);
        fail(&[OP_1, OP_1, OP_1, OP_1, OP_1, OP_2ROT]);
        fail(&[OP_1, OP_0, OP_1, OP_1, OP_1, OP_1, OP_2ROT]);
        fail(&[OP_2SWAP]);
        fail(&[OP_1, OP_2SWAP]);
        fail(&[OP_1, OP_1, OP_2SWAP]);
        fail(&[OP_1, OP_1, OP_1, OP_2SWAP]);
        fail(&[OP_1, OP_0, OP_1, OP_1, OP_2SWAP]);
        fail(&[OP_CAT]);
        fail(&[OP_1, OP_CAT]);
        fail(&[OP_1, OP_0, OP_0, OP_CAT]);
        fail(&[OP_SPLIT]);
        fail(&[OP_1, OP_SPLIT]);
        fail(&[OP_0, OP_1, OP_SPLIT]);
        fail(&[OP_1, OP_2, OP_SPLIT]);
        fail(&[OP_1, OP_1NEGATE, OP_SPLIT]);
        fail(&[OP_0, OP_SIZE]);
        fail(&[OP_AND]);
        fail(&[OP_0, OP_AND]);
        fail(&[OP_0, OP_1, OP_AND]);
        fail(&[OP_OR]);
        fail(&[OP_0, OP_OR]);
        fail(&[OP_0, OP_1, OP_OR]);
        fail(&[OP_XOR]);
        fail(&[OP_0, OP_XOR]);
        fail(&[OP_0, OP_1, OP_XOR]);
        fail(&[OP_LSHIFT]);
        fail(&[OP_1, OP_LSHIFT]);
        fail(&[OP_1, OP_1NEGATE, OP_LSHIFT]);
        fail(&[OP_RSHIFT]);
        fail(&[OP_1, OP_RSHIFT]);
        fail(&[OP_1, OP_1NEGATE, OP_RSHIFT]);
        fail(&[OP_INVERT]);
        fail(&[OP_EQUAL]);
        fail(&[OP_0, OP_EQUAL]);
        fail(&[OP_1, OP_0, OP_EQUAL]);
        fail(&[OP_1, OP_0, OP_EQUALVERIFY, OP_1]);
        fail(&[OP_1ADD]);
        fail(&[OP_1SUB]);
        fail(&[OP_NEGATE]);
        fail(&[OP_ABS]);
        fail(&[OP_NOT]);
        fail(&[OP_0NOTEQUAL]);
        fail(&[OP_ADD]);
        fail(&[OP_1, OP_ADD]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_ADD]);
        fail(&[OP_SUB]);
        fail(&[OP_1, OP_SUB]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_SUB]);
        fail(&[OP_MUL]);
        fail(&[OP_1, OP_MUL]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_MUL]);
        fail(&[OP_PUSH + 2, 0, 0, OP_PUSH + 2, 0, 0, OP_MUL]);
        fail(&[OP_DIV]);
        fail(&[OP_2MUL]);
        fail(&[OP_1, OP_DIV]);
        fail(&[OP_2DIV]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_DIV]);
        fail(&[OP_1, OP_0, OP_DIV]);
        fail(&[OP_MOD]);
        fail(&[OP_1, OP_MOD]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_MOD]);
        fail(&[OP_1, OP_0, OP_MOD]);
        fail(&[OP_BOOLAND]);
        fail(&[OP_1, OP_BOOLAND]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_BOOLAND]);
        fail(&[OP_BOOLOR]);
        fail(&[OP_1, OP_BOOLOR]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_BOOLOR]);
        fail(&[OP_NUMEQUAL]);
        fail(&[OP_1, OP_NUMEQUAL]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NUMEQUAL]);
        fail(&[OP_0, OP_1, OP_NUMEQUAL]);
        fail(&[OP_NUMEQUALVERIFY]);
        fail(&[OP_1, OP_NUMEQUALVERIFY]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NUMEQUALVERIFY]);
        fail(&[OP_1, OP_2, OP_NUMEQUALVERIFY]);
        fail(&[OP_NUMNOTEQUAL]);
        fail(&[OP_1, OP_NUMNOTEQUAL]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NUMNOTEQUAL]);
        fail(&[OP_1, OP_1, OP_NUMNOTEQUAL]);
        fail(&[OP_LESSTHAN]);
        fail(&[OP_1, OP_LESSTHAN]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_LESSTHAN]);
        fail(&[OP_1, OP_0, OP_LESSTHAN]);
        fail(&[OP_GREATERTHAN]);
        fail(&[OP_1, OP_GREATERTHAN]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_GREATERTHAN]);
        fail(&[OP_0, OP_1, OP_GREATERTHAN]);
        fail(&[OP_LESSTHANOREQUAL]);
        fail(&[OP_1, OP_LESSTHANOREQUAL]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_LESSTHANOREQUAL]);
        fail(&[OP_1, OP_0, OP_LESSTHANOREQUAL]);
        fail(&[OP_GREATERTHANOREQUAL]);
        fail(&[OP_1, OP_GREATERTHANOREQUAL]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_GREATERTHANOREQUAL]);
        fail(&[OP_0, OP_1, OP_GREATERTHANOREQUAL]);
        fail(&[OP_MIN]);
        fail(&[OP_1, OP_MIN]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_MIN]);
        fail(&[OP_MAX]);
        fail(&[OP_1, OP_MAX]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_MAX]);
        fail(&[OP_WITHIN]);
        fail(&[OP_1, OP_WITHIN]);
        fail(&[OP_1, OP_1, OP_WITHIN]);
        fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_WITHIN]);
        fail(&[OP_0, OP_1, OP_2, OP_WITHIN]);
        fail(&[OP_0, OP_1NEGATE, OP_0, OP_WITHIN]);
        fail(&[OP_BIN2NUM]);
        fail(&[OP_NUM2BIN]);
        fail(&[OP_1, OP_NUM2BIN]);
        fail(&[OP_1, OP_0, OP_NUM2BIN]);
        fail(&[OP_1, OP_1NEGATE, OP_NUM2BIN]);
        fail(&[OP_PUSH + 5, 129, 0, 0, 0, 0, OP_1, OP_NUM2BIN]);
        fail(&[OP_RIPEMD160]);
        fail(&[OP_SHA1]);
        fail(&[OP_SHA256]);
        fail(&[OP_HASH160]);
        fail(&[OP_HASH256]);
        fail(&[OP_CHECKSIG]);
        fail(&[OP_1, OP_CHECKSIG]);
        let mut c = MockChecker::sig_checks(vec![false; 1]);
        assert!(eval(&[OP_1, OP_1, OP_CHECKSIG], &mut c, NO_FLAGS).is_err());
        fail(&[OP_CHECKSIGVERIFY]);
        fail(&[OP_1, OP_CHECKSIGVERIFY]);
        let mut c = MockChecker::sig_checks(vec![false; 1]);
        assert!(eval(&[OP_1, OP_1, OP_CHECKSIGVERIFY, OP_1], &mut c, NO_FLAGS).is_err());
        fail(&[OP_CHECKMULTISIG]);
        fail(&[OP_1, OP_CHECKMULTISIG]);
        fail(&[OP_0, OP_0, OP_CHECKMULTISIG]);
        fail(&[OP_0, OP_0, OP_1NEGATE, OP_CHECKMULTISIG]);
        fail(&[OP_0, OP_1NEGATE, OP_0, OP_CHECKMULTISIG]);
        fail(&[OP_0, OP_0, OP_1, OP_CHECKMULTISIG]);
        fail(&[OP_0, OP_0, OP_PUSH + 1, 21, OP_CHECKMULTISIG]);
        fail(&[OP_0, OP_9, OP_9, OP_2, OP_9, OP_1, OP_CHECKMULTISIG]);
        let mut c = MockChecker::sig_checks(vec![false; 1]);
        assert!(eval(
            &[OP_0, OP_9, OP_1, OP_9, OP_1, OP_CHECKMULTISIG],
            &mut c,
            NO_FLAGS
        )
        .is_err());
        let mut c = MockChecker::sig_checks(vec![true, false]);
        let s = [OP_0, OP_9, OP_9, OP_2, OP_9, OP_9, OP_2, OP_CHECKMULTISIG];
        assert!(eval(&s, &mut c, NO_FLAGS).is_err());
        let mut c = MockChecker::sig_checks(vec![false, true, false]);
        let mut s = vec![OP_0, OP_9, OP_9, OP_2, OP_9, OP_9, OP_9, OP_3];
        s.push(OP_CHECKMULTISIG);
        assert!(eval(&s, &mut c, NO_FLAGS).is_err());
        fail_pregenesis(&[OP_CHECKLOCKTIMEVERIFY, OP_1]);
        fail_pregenesis(&[OP_PUSH + 5, 129, 0, 0, 0, 0, OP_CHECKLOCKTIMEVERIFY, OP_1]);
        let mut c = MockChecker::locktime_checks(vec![false]);
        assert!(eval(
            &[OP_0, OP_CHECKLOCKTIMEVERIFY, OP_1],
            &mut c,
            PREGENESIS_RULES
        )
        .is_err());
        fail_pregenesis(&[OP_CHECKSEQUENCEVERIFY, OP_1]);
        fail_pregenesis(&[OP_PUSH + 5, 129, 0, 0, 0, 0, OP_CHECKSEQUENCEVERIFY, OP_1]);
        let mut c = MockChecker::sequence_checks(vec![false]);
        assert!(eval(
            &[OP_0, OP_CHECKSEQUENCEVERIFY, OP_1],
            &mut c,
            PREGENESIS_RULES
        )
        .is_err());
        fail(&[OP_RESERVED, OP_1]);
        fail(&[OP_VER, OP_1]);
        fail(&[OP_VERIF, OP_1]);
        fail(&[OP_VERNOTIF, OP_1]);
        fail(&[OP_RESERVED1, OP_1]);
        fail(&[OP_RESERVED2, OP_1]);
        fail(&[OP_INVERT, OP_1]);
        fail(&[OP_MUL, OP_1]);
        fail(&[OP_LSHIFT, OP_1]);
        fail(&[OP_RSHIFT, OP_1]);
        fail(&[OP_INVALID_ABOVE, OP_1]);
        fail(&[OP_PUBKEYHASH, OP_1]);
        fail(&[OP_PUBKEY, OP_1]);
        fail(&[OP_INVALIDOPCODE, OP_1]);
    }

    #[test]
    fn next_op_tests() {
        let script = [];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_0, OP_CHECKSIG, OP_ADD];
        assert!(next_op(0, &script) == 1);
        assert!(next_op(1, &script) == 2);
        assert!(next_op(2, &script) == script.len());

        let script = [OP_1, OP_PUSH + 4, 1, 2, 3, 4, OP_1];
        assert!(next_op(0, &script) == 1);
        assert!(next_op(1, &script) == 6);
        assert!(next_op(6, &script) == script.len());

        let script = [OP_1, OP_PUSHDATA1, 2, 3, 4, OP_1];
        assert!(next_op(0, &script) == 1);
        assert!(next_op(1, &script) == 5);
        assert!(next_op(5, &script) == script.len());

        let script = [OP_1, OP_PUSHDATA2, 2, 0, 3, 4, OP_1];
        assert!(next_op(0, &script) == 1);
        assert!(next_op(1, &script) == 6);
        assert!(next_op(6, &script) == script.len());

        let script = [OP_1, OP_PUSHDATA4, 2, 0, 0, 0, 3, 4, OP_1];
        assert!(next_op(0, &script) == 1);
        assert!(next_op(1, &script) == 8);
        assert!(next_op(8, &script) == script.len());

        // Parse failures

        let script = [OP_PUSH + 1];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_PUSH + 3, 1, 2];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_PUSHDATA1];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_PUSHDATA1, 2, 1];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_PUSHDATA2];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_PUSHDATA2, 0];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_PUSHDATA2, 2, 0, 1];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_PUSHDATA4];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_PUSHDATA4, 1, 2, 3];
        assert!(next_op(0, &script) == script.len());

        let script = [OP_PUSHDATA4, 2, 0, 0, 0, 1];
        assert!(next_op(0, &script) == script.len());
    }

    #[test]
    fn remove_sig_tests() {
        let expected: Vec<u8> = vec![];
        assert!(remove_sig(&[], &[]) == expected);

        let expected: Vec<u8> = vec![OP_0];
        assert!(remove_sig(&[], &[OP_0]) == expected);

        let expected: Vec<u8> = vec![];
        assert!(remove_sig(&[OP_0], &[OP_0]) == expected);
        let v = [OP_0, OP_1, OP_2, OP_3, OP_4, OP_0, OP_1, OP_2, OP_3, OP_4];
        assert!(remove_sig(&[OP_2, OP_3], &v) == vec![OP_0, OP_1, OP_4, OP_0, OP_1, OP_4]);
    }

    /// A test run that doesn't do signature checks and expects failure
    fn pass(script: &[u8]) {
        let mut c = MockChecker::new();
        assert!(eval(script, &mut c, NO_FLAGS).is_ok());
    }

    /// A test run that doesn't do signature checks and expects failure
    fn fail(script: &[u8]) {
        let mut c = MockChecker::new();
        assert!(eval(script, &mut c, NO_FLAGS).is_err());
    }

    /// Pre-genesis versions of the above checks
    fn pass_pregenesis(script: &[u8]) {
        let mut c = MockChecker::new();
        assert!(eval(script, &mut c, PREGENESIS_RULES).is_ok());
    }

    /// A test run that doesn't do signature checks and expects failure
    fn fail_pregenesis(script: &[u8]) {
        let mut c = MockChecker::new();
        assert!(eval(script, &mut c, PREGENESIS_RULES).is_err());
    }

    /// Mocks a transaction checker to always return a set of values
    struct MockChecker {
        sig_checks: RefCell<Vec<bool>>,
        locktime_checks: RefCell<Vec<bool>>,
        sequence_checks: RefCell<Vec<bool>>,
        tx_version: Option<i32>,
    }

    impl MockChecker {
        fn new() -> MockChecker {
            MockChecker {
                sig_checks: RefCell::new(vec![true; 32]),
                locktime_checks: RefCell::new(vec![true; 32]),
                sequence_checks: RefCell::new(vec![true; 32]),
                tx_version: None,
            }
        }

        fn with_tx_version(version: i32) -> MockChecker {
            MockChecker {
                sig_checks: RefCell::new(vec![true; 32]),
                locktime_checks: RefCell::new(vec![true; 32]),
                sequence_checks: RefCell::new(vec![true; 32]),
                tx_version: Some(version),
            }
        }

        fn sig_checks(sig_checks: Vec<bool>) -> MockChecker {
            MockChecker {
                sig_checks: RefCell::new(sig_checks),
                locktime_checks: RefCell::new(vec![true; 32]),
                sequence_checks: RefCell::new(vec![true; 32]),
                tx_version: None,
            }
        }

        fn locktime_checks(locktime_checks: Vec<bool>) -> MockChecker {
            MockChecker {
                sig_checks: RefCell::new(vec![true; 32]),
                locktime_checks: RefCell::new(locktime_checks),
                sequence_checks: RefCell::new(vec![true; 32]),
                tx_version: None,
            }
        }

        fn sequence_checks(sequence_checks: Vec<bool>) -> MockChecker {
            MockChecker {
                sig_checks: RefCell::new(vec![true; 32]),
                locktime_checks: RefCell::new(vec![true; 32]),
                sequence_checks: RefCell::new(sequence_checks),
                tx_version: None,
            }
        }
    }

    impl Checker for MockChecker {
        fn check_sig(
            &mut self,
            _sig: &[u8],
            _pubkey: &[u8],
            _script: &[u8],
        ) -> Result<bool, ChainGangError> {
            Ok(self.sig_checks.borrow_mut().pop().unwrap())
        }

        fn check_locktime(&self, _locktime: i32) -> Result<bool, ChainGangError> {
            Ok(self.locktime_checks.borrow_mut().pop().unwrap())
        }

        fn check_sequence(&self, _sequence: i32) -> Result<bool, ChainGangError> {
            Ok(self.sequence_checks.borrow_mut().pop().unwrap())
        }

        fn tx_version(&self) -> Result<i32, ChainGangError> {
            self.tx_version.ok_or_else(|| {
                ChainGangError::IllegalState("Illegal transaction version check".to_string())
            })
        }
    }

    fn pass_with_version(script: &[u8], version: i32) {
        let mut c = MockChecker::with_tx_version(version);
        assert!(eval(script, &mut c, NO_FLAGS).is_ok());
    }

    #[test]
    fn chronicle_op_ver_pushes_tx_version() {
        pass_with_version(&[OP_VER, OP_2, OP_NUMEQUAL], 2);
    }

    #[test]
    fn chronicle_op_verif_executes_when_version_is_high_enough() {
        pass_with_version(&[OP_2, OP_VERIF, OP_1, OP_ELSE, OP_0, OP_ENDIF], 2);
        let mut c = MockChecker::with_tx_version(1);
        assert!(eval(
            &[OP_2, OP_VERIF, OP_1, OP_ELSE, OP_0, OP_ENDIF],
            &mut c,
            NO_FLAGS
        )
        .is_err());
    }

    #[test]
    fn chronicle_op_vernotif_executes_when_version_is_too_low() {
        pass_with_version(&[OP_2, OP_VERNOTIF, OP_1, OP_ELSE, OP_0, OP_ENDIF], 1);
        let mut c = MockChecker::with_tx_version(2);
        assert!(eval(
            &[OP_2, OP_VERNOTIF, OP_1, OP_ELSE, OP_0, OP_ENDIF],
            &mut c,
            NO_FLAGS
        )
        .is_err());
    }

    #[test]
    fn chronicle_op_substr_left_right() {
        let mut s = Script::new();
        s.append_data(b"BSV Blockchain");
        s.append(OP_4);
        s.append(OP_5);
        s.append(OP_SUBSTR);
        s.append_data(b"Block");
        s.append(OP_EQUALVERIFY);
        s.append_data(b"BSV Blockchain");
        s.append(OP_3);
        s.append(OP_LEFT);
        s.append_data(b"BSV");
        s.append(OP_EQUALVERIFY);
        s.append_data(b"BSV Blockchain");
        s.append(OP_5);
        s.append(OP_RIGHT);
        s.append_data(b"chain");
        s.append(OP_EQUAL);
        pass(&s.0);
    }

    #[test]
    fn chronicle_op_lshiftnum_rshiftnum() {
        pass(&[OP_4, OP_2, OP_LSHIFTNUM, OP_16, OP_EQUAL]);
        pass(&[OP_4, OP_2, OP_RSHIFTNUM, OP_1, OP_EQUAL]);
    }

    #[test]
    fn uses_two_phase_eval_gated_on_version() {
        assert!(!uses_two_phase_eval(1));
        assert!(uses_two_phase_eval(2));
    }

    #[test]
    fn chronicle_two_phase_carries_stack() {
        let unlock = [OP_2, OP_3, OP_ADD];
        let lock = [OP_5, OP_EQUAL];
        let mut c = MockChecker::new();
        assert!(eval_two_phase(&unlock, &lock, &mut c, NO_FLAGS).is_ok());
        let mut c2 = MockChecker::new();
        let (stack, _) = eval_two_phase_with_stack(&unlock, &lock, &mut c2, NO_FLAGS).unwrap();
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn chronicle_two_phase_clears_alt_stack() {
        let unlock = [OP_1, OP_TOALTSTACK, OP_2, OP_3, OP_ADD];
        let lock = [OP_5, OP_EQUAL];
        let mut c = MockChecker::new();
        assert!(eval_two_phase(&unlock, &lock, &mut c, NO_FLAGS).is_ok());
    }

    struct ScriptRecordingChecker {
        sig_checks: RefCell<Vec<bool>>,
        scripts: RefCell<Vec<Vec<u8>>>,
    }

    impl ScriptRecordingChecker {
        fn new(sig_checks: Vec<bool>) -> Self {
            ScriptRecordingChecker {
                sig_checks: RefCell::new(sig_checks),
                scripts: RefCell::new(Vec::new()),
            }
        }
    }

    impl Checker for ScriptRecordingChecker {
        fn check_sig(
            &mut self,
            _sig: &[u8],
            _pubkey: &[u8],
            script: &[u8],
        ) -> Result<bool, ChainGangError> {
            self.scripts.borrow_mut().push(script.to_vec());
            Ok(self.sig_checks.borrow_mut().pop().unwrap())
        }

        fn check_locktime(&self, _locktime: i32) -> Result<bool, ChainGangError> {
            Ok(true)
        }

        fn check_sequence(&self, _sequence: i32) -> Result<bool, ChainGangError> {
            Ok(true)
        }
    }

    #[test]
    fn chronicle_two_phase_unlock_checksig_script_code() {
        let mut unlock = Script::new();
        unlock.append_data(b"s0");
        unlock.append_data(b"s1");
        unlock.append(OP_CODESEPARATOR);
        unlock.append_data(b"p1");
        unlock.append(OP_CHECKSIG);

        let mut lock = Script::new();
        lock.append_data(b"p0");
        lock.append(OP_CHECKSIG);

        let mut expected_unlock_code = Script::new();
        expected_unlock_code.append_data(b"p1");
        expected_unlock_code.append(OP_CHECKSIG);
        expected_unlock_code.append_data(b"p0");
        expected_unlock_code.append(OP_CHECKSIG);

        let mut expected_lock_code = Script::new();
        expected_lock_code.append_data(b"p0");
        expected_lock_code.append(OP_CHECKSIG);

        let mut checker = ScriptRecordingChecker::new(vec![true, true]);
        assert!(eval_two_phase(&unlock.0, &lock.0, &mut checker, NO_FLAGS).is_ok());

        let scripts = checker.scripts.borrow();
        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts[0], expected_unlock_code.0);
        assert_eq!(scripts[1], expected_lock_code.0);
    }

    #[test]
    fn strip_code_separators_removes_codesep_bytes() {
        let mut script = Script::new();
        script.append(OP_1);
        script.append(OP_CODESEPARATOR);
        script.append(OP_2);
        script.append(OP_CODESEPARATOR);
        script.append(OP_3);

        let stripped = strip_code_separators(&script.0);
        assert_eq!(stripped, [OP_1, OP_2, OP_3]);
    }

    #[test]
    fn uses_relaxed_malleability_gated_on_version() {
        assert!(!uses_relaxed_malleability(1));
        assert!(uses_relaxed_malleability(2));
    }

    #[test]
    fn chronicle_relaxed_clean_stack_allows_extra_items() {
        let mut c = MockChecker::with_tx_version(2);
        assert!(eval(&[OP_1, OP_1], &mut c, NO_FLAGS).is_ok());
    }

    #[test]
    fn strict_clean_stack_rejects_extra_items() {
        let mut c = MockChecker::with_tx_version(1);
        assert!(eval(&[OP_1, OP_1], &mut c, NO_FLAGS).is_err());
    }

    #[test]
    fn chronicle_minimalif_allows_non_minimal_true_operand() {
        let mut s = Script::new();
        s.append_data(&[0, 0, 0, 127]);
        s.append(OP_IF);
        s.append(OP_1);
        s.append(OP_ENDIF);
        s.append(OP_1);
        let mut c = MockChecker::with_tx_version(2);
        assert!(eval(&s.0, &mut c, NO_FLAGS).is_ok());
    }

    #[test]
    fn strict_minimalif_rejects_non_minimal_true_operand() {
        let mut s = Script::new();
        s.append_data(&[0, 0, 0, 127]);
        s.append(OP_IF);
        s.append(OP_1);
        s.append(OP_ENDIF);
        s.append(OP_1);
        let mut c = MockChecker::with_tx_version(1);
        assert!(eval(&s.0, &mut c, NO_FLAGS).is_err());
    }

    #[test]
    fn chronicle_nullfail_allows_failed_checksig_with_nonempty_sig() {
        let mut c = MockChecker {
            sig_checks: RefCell::new(vec![false]),
            locktime_checks: RefCell::new(vec![true; 32]),
            sequence_checks: RefCell::new(vec![true; 32]),
            tx_version: Some(2),
        };
        let mut script = Script::new();
        script.append_data(&[0x01]);
        script.append_data(&[0x02]);
        script.append(OP_CHECKSIG);
        script.append(OP_DROP);
        script.append(OP_1);
        assert!(eval(&script.0, &mut c, NO_FLAGS).is_ok());
    }

    #[test]
    fn strict_nullfail_rejects_failed_checksig_with_nonempty_sig() {
        let mut c = MockChecker {
            sig_checks: RefCell::new(vec![false]),
            locktime_checks: RefCell::new(vec![true; 32]),
            sequence_checks: RefCell::new(vec![true; 32]),
            tx_version: Some(1),
        };
        let mut script = Script::new();
        script.append_data(&[0x01]);
        script.append_data(&[0x02]);
        script.append(OP_CHECKSIG);
        assert!(eval(&script.0, &mut c, NO_FLAGS).is_err());
    }

    #[test]
    fn is_push_only_accepts_data_pushes() {
        let mut script = Script::new();
        script.append_data(b"sig");
        script.append_data(b"pubkey");
        assert!(is_push_only(&script.0));
    }

    #[test]
    fn is_push_only_rejects_functional_opcodes() {
        assert!(!is_push_only(&[OP_2, OP_3, OP_ADD]));
    }

    #[test]
    fn max_script_num_length_by_context() {
        let c1 = MockChecker::with_tx_version(1);
        assert_eq!(
            max_script_num_length(&c1, NO_FLAGS),
            MAX_SCRIPT_NUM_LENGTH_GENESIS
        );
        let c2 = MockChecker::with_tx_version(2);
        assert_eq!(
            max_script_num_length(&c2, NO_FLAGS),
            MAX_SCRIPT_NUM_LENGTH_CHRONICLE
        );
        let c3 = MockChecker::new();
        assert_eq!(
            max_script_num_length(&c3, NO_FLAGS),
            MAX_SCRIPT_NUM_LENGTH_GENESIS
        );
        assert_eq!(
            max_script_num_length(&c3, PREGENESIS_RULES),
            MAX_SCRIPT_NUM_LENGTH_PREGENESIS
        );
    }

    #[test]
    fn genesis_script_num_limit_rejects_oversized_bin2num() {
        let mut script = Script::new();
        script.append(OP_PUSHDATA4);
        script.append_slice(&(750_001_u32).to_le_bytes());
        script.0.extend(std::iter::repeat_n(0u8, 750_001));
        script.append(OP_BIN2NUM);
        script.append(OP_1);
        let mut c = MockChecker::with_tx_version(1);
        assert!(eval(&script.0, &mut c, NO_FLAGS).is_err());
    }

    #[test]
    fn chronicle_script_num_limit_accepts_genesis_max_bin2num() {
        let mut script = Script::new();
        script.append(OP_PUSHDATA4);
        script.append_slice(&(750_000_u32).to_le_bytes());
        script.0.extend(std::iter::repeat_n(0u8, 750_000));
        script.append(OP_BIN2NUM);
        script.append(OP_1);
        let mut c = MockChecker::with_tx_version(2);
        assert!(eval(&script.0, &mut c, NO_FLAGS).is_ok());
    }
}
