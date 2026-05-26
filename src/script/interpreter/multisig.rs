use crate::script::stack::Stack;
use crate::script::Checker;
use crate::transaction::sighash::SIGHASH_FORKID;
use crate::util::ChainGangError;

use super::push::check_stack_size;
use super::push::next_op;
use super::rules::{pop_num_for_eval, tx_enforces_malleability_rules};

#[inline]
pub(crate) fn check_multisig<T: Checker>(
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

pub(crate) fn prefork(sig: &[u8]) -> bool {
    !sig.is_empty() && sig[sig.len() - 1] & SIGHASH_FORKID == 0
}

/// Removes any instances of the signature from the lock_script in pre-fork transactions
pub(crate) fn remove_sig(sig: &[u8], script: &[u8]) -> Vec<u8> {
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
