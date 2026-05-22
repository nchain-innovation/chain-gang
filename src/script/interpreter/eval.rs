use crate::script::op_codes::*;
use crate::script::stack::{
    check_script_num_length, decode_bigint, decode_bool, encode_bigint, encode_num,
    is_minimally_encoded, pop_bigint_checked, pop_bool, push_bigint_checked, Stack,
};
use crate::script::Checker;
use crate::util::{hash160, lshift, rshift, sha1::sha1, sha256::sha256, sha256d, ChainGangError};

use num_bigint::BigInt;
use num_traits::{One, ToPrimitive, Zero};
use ripemd::{Digest, Ripemd160};

use super::multisig::check_multisig;
use super::push::{check_canonical_push, check_stack_size, next_op, remains, skip_branch};
use super::rules::{
    max_script_num_length, pop_bool_for_if, pop_num_for_eval, substr_error,
    tx_enforces_malleability_rules, verif_branch_exec,
};
use super::script_code::{checksig_script_code, multisig_script_code, TwoPhaseEvalContext};
use super::{ALT_STACK_CAPACITY, PREGENESIS_RULES, STACK_CAPACITY};

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
