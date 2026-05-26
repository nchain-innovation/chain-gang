use crate::script::stack::{
    decode_bool, pop_bool_minimal, pop_num_minimal, Stack, MAX_SCRIPT_NUM_LENGTH_CHRONICLE,
    MAX_SCRIPT_NUM_LENGTH_GENESIS, MAX_SCRIPT_NUM_LENGTH_PREGENESIS,
};
use crate::script::Checker;
use crate::util::ChainGangError;

use num_bigint::BigInt;

use super::PREGENESIS_RULES;

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

pub(crate) fn tx_enforces_malleability_rules<T: Checker>(checker: &T) -> bool {
    match checker.tx_version() {
        Ok(version) => !uses_relaxed_malleability(version as u32),
        Err(_) => false,
    }
}

pub(crate) fn pop_num_for_eval<T: Checker>(
    stack: &mut Stack,
    checker: &T,
) -> Result<i32, ChainGangError> {
    pop_num_minimal(stack, tx_enforces_malleability_rules(checker))
}

pub(crate) fn pop_bool_for_if<T: Checker>(
    stack: &mut Stack,
    checker: &T,
) -> Result<bool, ChainGangError> {
    pop_bool_minimal(stack, tx_enforces_malleability_rules(checker))
}

pub(crate) fn validate_final_stack<T: Checker>(
    stack: &Stack,
    checker: &T,
) -> Result<(), ChainGangError> {
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

pub(crate) fn verif_branch_exec<T: Checker>(
    checker: &T,
    comparison: BigInt,
    invert: bool,
) -> Result<bool, ChainGangError> {
    let version = BigInt::from(checker.tx_version()?);
    let execute = version >= comparison;
    Ok(if invert { !execute } else { execute })
}

pub(crate) fn substr_error(msg: &str) -> ChainGangError {
    ChainGangError::ScriptError(msg.to_string())
}
