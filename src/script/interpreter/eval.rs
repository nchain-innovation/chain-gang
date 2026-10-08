use crate::script::checker::SEQUENCE_LOCKTIME_DISABLE_FLAG;
use crate::script::op_codes::*;
use crate::script::stack::{
    check_script_num_length, decode_bigint, decode_bool, encode_bigint, encode_num, pop_bool,
    push_bigint_checked, Stack,
};
use crate::script::Checker;
use crate::util::{hash160, lshift, rshift, sha1::sha1, sha256::sha256, sha256d, ChainGangError};

use num_bigint::BigInt;
use num_traits::{One, ToPrimitive, Zero};
use ripemd::{Digest, Ripemd160};

use super::multisig::check_multisig;
use super::push::{
    check_canonical_push, check_pregenesis_push_size, check_stack_size, next_op, remains,
};
use super::rules::{
    count_pregenesis_op, enforces_policy_rules, max_script_num_length,
    max_script_num_result_length, peek_locktime_operand, pop_bigint_for_eval, pop_bool_for_if,
    pop_num_for_eval, substr_error, tx_enforces_malleability_rules, verif_branch_exec,
};
use super::script_code::{checksig_script_code, multisig_script_code, TwoPhaseEvalContext};
use super::{
    ALT_STACK_CAPACITY, MAX_SCRIPT_ELEMENT_SIZE_PREGENESIS, MAX_SCRIPT_SIZE_PREGENESIS,
    MAX_STACK_ELEMENTS_PREGENESIS, PREGENESIS_RULES, STACK_CAPACITY,
};

// The interpreter entry point genuinely needs all of these: script, checker,
// consensus flags, resume/break offsets, both stacks, and the two-phase context.
// Grouping them into a struct would change a public signature for no gain here.
#[allow(clippy::too_many_arguments)]
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

    // The node's policy rules, decided once: the transaction version cannot
    // change during evaluation.
    let policy = enforces_policy_rules(checker, flags);

    let pregenesis = flags & PREGENESIS_RULES == PREGENESIS_RULES;
    let mut conditions = Conditions::default();
    // After Genesis, an OP_RETURN inside an executed branch stops execution
    // but not the script: the node still reads IF, ELSE and ENDIF for balance,
    // and an OP_RETURN reached at top level ends it.
    let mut returned_in_branch = false;
    let mut check_index = 0;
    let mut i = start_at.unwrap_or(0);
    let max_num_len = max_script_num_length(checker, flags);
    let max_result_len = max_script_num_result_length(checker, flags);

    // The node's size limits before Genesis: a script, each item pushed or
    // built, the two stacks together and the opcodes in a script. Genesis
    // lifted them.
    let mut op_count = 0;
    if pregenesis && script.len() > MAX_SCRIPT_SIZE_PREGENESIS {
        return Err(ChainGangError::ScriptError(format!(
            "Script of {} bytes exceeds the pre-Genesis limit of {MAX_SCRIPT_SIZE_PREGENESIS}",
            script.len()
        )));
    }

    'outer: while i < script.len() {
        if let Some(val) = break_at {
            // hit our breakpoint
            if i >= val {
                break;
            }
        }
        // Checked as the node reads each push, before it decides whether the
        // branch is executing, so a push in one that is not counts too.
        if pregenesis {
            check_pregenesis_push_size(i, script)?;
            count_pregenesis_op(script[i], &mut op_count)?;
        }
        let opcode = script[i];
        let exec = conditions.active() && (!returned_in_branch || opcode == OP_RETURN);
        if !exec {
            // As the node: an opcode that does not execute is only read, unless
            // it opens, switches or closes a branch.
            match opcode {
                OP_IF | OP_NOTIF | OP_VERIF | OP_VERNOTIF => conditions.push(false),
                OP_ELSE => conditions.toggle(pregenesis)?,
                OP_ENDIF => conditions.pop()?,
                _ => {}
            }
            i = next_op(i, script);
            continue;
        }
        match opcode {
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
                if policy {
                    check_canonical_push(i, script)?;
                }
                let data = &script[i + 1..i + 1 + len as usize];
                stack.push(data.to_vec());
            }
            OP_PUSHDATA1 => {
                remains(i + 1, 1, script)?;
                let len = script[i + 1] as usize;
                remains(i + 2, len, script)?;
                if policy {
                    check_canonical_push(i, script)?;
                }
                let data = &script[i + 2..i + 2 + len];
                stack.push(data.to_vec());
            }
            OP_PUSHDATA2 => {
                remains(i + 1, 2, script)?;
                let len = (script[i + 1] as usize) + ((script[i + 2] as usize) << 8);
                remains(i + 3, len, script)?;
                if policy {
                    check_canonical_push(i, script)?;
                }
                let data = &script[i + 3..i + 3 + len];
                stack.push(data.to_vec());
            }
            OP_PUSHDATA4 => {
                remains(i + 1, 4, script)?;
                let len = (script[i + 1] as usize)
                    + ((script[i + 2] as usize) << 8)
                    + ((script[i + 3] as usize) << 16)
                    + ((script[i + 4] as usize) << 24);
                remains(i + 5, len, script)?;
                if policy {
                    check_canonical_push(i, script)?;
                }
                let data = &script[i + 5..i + 5 + len];
                stack.push(data.to_vec());
            }
            OP_NOP => {}
            OP_VER => {
                stack.push(encode_num(checker.tx_version()? as i64)?);
            }
            OP_IF => conditions.push(pop_bool_for_if(&mut stack)?),
            OP_NOTIF => conditions.push(!pop_bool_for_if(&mut stack)?),
            OP_VERIF => {
                let comparison = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                conditions.push(verif_branch_exec(checker, comparison, false)?);
            }
            OP_VERNOTIF => {
                let comparison = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                conditions.push(verif_branch_exec(checker, comparison, true)?);
            }
            OP_ELSE => conditions.toggle(pregenesis)?,
            OP_ENDIF => conditions.pop()?,
            OP_VERIFY => {
                if !pop_bool(&mut stack)? {
                    return Err(ChainGangError::ScriptError("OP_VERIFY failed".to_string()));
                }
            }
            OP_RETURN => {
                if pregenesis {
                    return Err(ChainGangError::ScriptError("Hit OP_RETURN".to_string()));
                } else if conditions.is_empty() {
                    // The rest of the script is not read, balanced or not
                    break 'outer;
                } else {
                    returned_in_branch = true;
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
                let n = pop_num_for_eval(&mut stack, policy)?;
                if n < 0 {
                    let msg = "OP_PICK failed, n negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                check_stack_size(n as usize + 1, &stack)?;
                let copy = stack[stack.len() - n as usize - 1].clone();
                stack.push(copy);
            }
            OP_ROLL => {
                let n = pop_num_for_eval(&mut stack, policy)?;
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
                if pregenesis && second.len() + top.len() > MAX_SCRIPT_ELEMENT_SIZE_PREGENESIS {
                    return Err(ChainGangError::ScriptError(format!(
                        "OP_CAT result exceeds the pre-Genesis limit of {MAX_SCRIPT_ELEMENT_SIZE_PREGENESIS}"
                    )));
                }
                second.extend_from_slice(&top);
                stack.push(second);
            }
            OP_SPLIT => {
                check_stack_size(2, &stack)?;
                let n = pop_num_for_eval(&mut stack, policy)?;
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
                let length = pop_num_for_eval(&mut stack, policy)?;
                let start = pop_num_for_eval(&mut stack, policy)?;
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
                let length = pop_num_for_eval(&mut stack, policy)?;
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
                let length = pop_num_for_eval(&mut stack, policy)?;
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
                let n = pop_num_for_eval(&mut stack, policy)?;
                if n < 0 {
                    let msg = "n must be non-negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let v = stack.pop().unwrap();
                stack.push(lshift(&v, n as usize));
            }
            OP_RSHIFT => {
                check_stack_size(2, &stack)?;
                let n = pop_num_for_eval(&mut stack, policy)?;
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
                let mut x = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                x += 1;
                push_bigint_checked(&mut stack, x, max_result_len)?;
            }
            OP_1SUB => {
                let mut x = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                x -= 1;
                push_bigint_checked(&mut stack, x, max_result_len)?;
            }
            OP_NEGATE => {
                let mut x = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                x = -x;
                push_bigint_checked(&mut stack, x, max_result_len)?;
            }
            OP_ABS => {
                let mut x = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if x < BigInt::zero() {
                    x = -x;
                }
                push_bigint_checked(&mut stack, x, max_result_len)?;
            }
            OP_NOT => {
                let mut x = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if x == BigInt::zero() {
                    x = BigInt::one();
                } else {
                    x = BigInt::zero();
                }
                push_bigint_checked(&mut stack, x, max_result_len)?;
            }
            OP_0NOTEQUAL => {
                let mut x = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if x == BigInt::zero() {
                    x = BigInt::zero();
                } else {
                    x = BigInt::one();
                }
                push_bigint_checked(&mut stack, x, max_result_len)?;
            }
            OP_ADD => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let sum = a + b;
                push_bigint_checked(&mut stack, sum, max_result_len)?;
            }
            OP_SUB => {
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let difference = b - a;
                push_bigint_checked(&mut stack, difference, max_result_len)?;
            }
            OP_MUL => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let product = a * b;
                push_bigint_checked(&mut stack, product, max_result_len)?;
            }
            OP_2MUL => {
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let two = BigInt::from(2);
                let product = a * two;
                push_bigint_checked(&mut stack, product, max_result_len)?;
            }
            OP_DIV => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if b == BigInt::zero() {
                    let msg = "OP_DIV failed, divide by 0".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let quotient = a / b;
                push_bigint_checked(&mut stack, quotient, max_result_len)?;
            }
            OP_2DIV => {
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let b = BigInt::from(2);

                let quotient = a / b;
                push_bigint_checked(&mut stack, quotient, max_result_len)?;
            }
            OP_MOD => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if b == BigInt::zero() {
                    let msg = "OP_MOD failed, divide by 0".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let remainder = a % b;
                push_bigint_checked(&mut stack, remainder, max_result_len)?;
            }
            OP_BOOLAND => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a != BigInt::zero() && b != BigInt::zero() {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_BOOLOR => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a != BigInt::zero() || b != BigInt::zero() {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_NUMEQUAL => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a == b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_NUMEQUALVERIFY => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a != b {
                    let msg = "Numbers are not equal".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
            }
            OP_NUMNOTEQUAL => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a != b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_LESSTHAN => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a < b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_GREATERTHAN => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a > b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_LESSTHANOREQUAL => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a <= b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_GREATERTHANOREQUAL => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a >= b {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_MIN => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a < b {
                    push_bigint_checked(&mut stack, a, max_result_len)?;
                } else {
                    push_bigint_checked(&mut stack, b, max_result_len)?;
                }
            }
            OP_MAX => {
                let b = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let a = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if a > b {
                    push_bigint_checked(&mut stack, a, max_result_len)?;
                } else {
                    push_bigint_checked(&mut stack, b, max_result_len)?;
                }
            }
            OP_WITHIN => {
                let max = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let min = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                let x = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                if x >= min && x < max {
                    stack.push(encode_num(1)?);
                } else {
                    stack.push(encode_num(0)?);
                }
            }
            OP_NUM2BIN => {
                check_stack_size(2, &stack)?;
                let size = pop_bigint_for_eval(&mut stack, max_num_len, policy)?;
                // Before Genesis the node caps the size at its 520-byte element
                // limit. After Genesis chain-gang keeps it to the script number
                // limit, short of the node's i32::MAX.
                let max_size = if pregenesis {
                    MAX_SCRIPT_ELEMENT_SIZE_PREGENESIS
                } else {
                    max_num_len
                };
                let size = match size.to_usize() {
                    Some(size) if size <= max_size => size,
                    _ => {
                        let msg = format!("OP_NUM2BIN failed, size {size} out of range");
                        return Err(ChainGangError::ScriptError(msg));
                    }
                };
                // Minimally encoded first, as the node's `MinimallyEncode`:
                // padding is not part of the number, so a padded number can
                // shrink, and negative zero is zero, which fits in no bytes.
                let mut n = stack.pop().unwrap();
                let mut n = encode_bigint(decode_bigint(&mut n));
                if n.len() > size {
                    let msg = "OP_NUM2BIN failed, number does not fit the size".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                if n.len() < size {
                    // The sign moves from the number's last byte to the new
                    // last byte. It used to go on the first, so `-42 2
                    // NUM2BIN` gave `aa00` where the node gives `2a80`.
                    let sign = n.last().map_or(0, |last| last & 0x80);
                    if let Some(last) = n.last_mut() {
                        *last &= 0x7f;
                    }
                    n.resize(size, 0);
                    n[size - 1] |= sign;
                }
                stack.push(n);
            }
            OP_BIN2NUM => {
                check_stack_size(1, &stack)?;
                let mut v = stack.pop().unwrap();
                // Minimally encode first, then check the result, as the node
                // does (`MinimallyEncode` then `IsMinimallyEncoded`). The input
                // is not a number yet: padding it carries is exactly what this
                // opcode removes, so `0100000000` is 1 and within a 4-byte
                // limit. Checking its length first rejected those (#36).
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
                let cleaned_script = checksig_script_code(script, check_index, &sig, two_phase);

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
                let cleaned_script = checksig_script_code(script, check_index, &sig, two_phase);
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
                match check_multisig(
                    &mut stack,
                    checker,
                    &cleaned_script,
                    policy,
                    pregenesis.then_some(&mut op_count),
                )? {
                    true => stack.push(encode_num(1)?),
                    false => stack.push(encode_num(0)?),
                }
            }
            OP_CHECKMULTISIGVERIFY => {
                let cleaned_script = multisig_script_code(script, check_index, two_phase);
                if !check_multisig(
                    &mut stack,
                    checker,
                    &cleaned_script,
                    policy,
                    pregenesis.then_some(&mut op_count),
                )? {
                    let msg = "OP_CHECKMULTISIGVERIFY failed".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
            }
            OP_CHECKLOCKTIMEVERIFY => {
                if flags & PREGENESIS_RULES == PREGENESIS_RULES {
                    let locktime = peek_locktime_operand(&stack, policy)?;
                    if !checker.check_locktime(locktime)? {
                        let msg = "OP_CHECKLOCKTIMEVERIFY failed".to_string();
                        return Err(ChainGangError::ScriptError(msg));
                    }
                }
            }
            OP_CHECKSEQUENCEVERIFY => {
                if flags & PREGENESIS_RULES == PREGENESIS_RULES {
                    let sequence = peek_locktime_operand(&stack, policy)?;
                    // With the disable flag set the opcode is a NOP (BIP 112).
                    if sequence & i64::from(SEQUENCE_LOCKTIME_DISABLE_FLAG) == 0
                        && !checker.check_sequence(sequence)?
                    {
                        let msg = "OP_CHECKSEQUENCEVERIFY failed".to_string();
                        return Err(ChainGangError::ScriptError(msg));
                    }
                }
            }
            OP_NOP1 => {}
            OP_LSHIFTNUM => {
                check_stack_size(2, &stack)?;
                let n = pop_num_for_eval(&mut stack, policy)?;
                if n < 0 {
                    let msg = "n must be non-negative".to_string();
                    return Err(ChainGangError::ScriptError(msg));
                }
                let v = stack.pop().unwrap();
                stack.push(lshift(&v, n as usize));
            }
            OP_RSHIFTNUM => {
                check_stack_size(2, &stack)?;
                let n = pop_num_for_eval(&mut stack, policy)?;
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
        if pregenesis && stack.len() + alt_stack.len() > MAX_STACK_ELEMENTS_PREGENESIS {
            return Err(ChainGangError::ScriptError(format!(
                "Stacks hold more than the pre-Genesis limit of {MAX_STACK_ELEMENTS_PREGENESIS} items"
            )));
        }
        i = next_op(i, script);
    }

    if !conditions.is_empty() {
        return Err(ChainGangError::ScriptError("ENDIF missing".to_string()));
    }

    let optional_i = break_at.map(|_| i);
    Ok((stack, alt_stack, optional_i))
}

/// The open IFs, innermost last, as the node's condition stack: whether each
/// one's current branch runs and whether it has had its ELSE.
#[derive(Default)]
struct Conditions {
    branches: Vec<Branch>,
    /// How many open IFs are in a branch that does not run
    not_running: usize,
}

struct Branch {
    runs: bool,
    had_else: bool,
}

impl Conditions {
    fn is_empty(&self) -> bool {
        self.branches.is_empty()
    }

    /// Whether every open IF is in a branch that runs
    fn active(&self) -> bool {
        self.not_running == 0
    }

    fn push(&mut self, runs: bool) {
        if !runs {
            self.not_running += 1;
        }
        self.branches.push(Branch {
            runs,
            had_else: false,
        });
    }

    /// OP_ELSE. After Genesis an IF takes one ELSE; before it, each further
    /// ELSE switches branch again.
    fn toggle(&mut self, pregenesis: bool) -> Result<(), ChainGangError> {
        let Some(branch) = self.branches.last_mut() else {
            let msg = "ELSE found without matching IF".to_string();
            return Err(ChainGangError::ScriptError(msg));
        };
        if branch.had_else && !pregenesis {
            let msg = "Second ELSE for one IF".to_string();
            return Err(ChainGangError::ScriptError(msg));
        }
        if branch.runs {
            self.not_running += 1;
        } else {
            self.not_running -= 1;
        }
        branch.runs = !branch.runs;
        branch.had_else = true;
        Ok(())
    }

    /// OP_ENDIF
    fn pop(&mut self) -> Result<(), ChainGangError> {
        let Some(branch) = self.branches.pop() else {
            let msg = "ENDIF found without matching IF".to_string();
            return Err(ChainGangError::ScriptError(msg));
        };
        if !branch.runs {
            self.not_running -= 1;
        }
        Ok(())
    }
}
