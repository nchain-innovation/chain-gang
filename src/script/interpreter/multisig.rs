use crate::script::stack::Stack;
use crate::script::{Checker, Script};
use crate::transaction::sighash::SIGHASH_FORKID;
use crate::util::ChainGangError;

use super::push::check_stack_size;
use super::push::next_op;
use super::rules::{check_pregenesis_op_count, pop_num_for_eval, tx_enforces_malleability_rules};
use super::MAX_PUBKEYS_PER_MULTISIG_PREGENESIS;

/// `pregenesis_op_count` is the script's opcode count so far, before Genesis,
/// and `None` after it. The keys count towards it, as in the node.
#[inline]
pub(crate) fn check_multisig<T: Checker>(
    stack: &mut Stack,
    checker: &mut T,
    script: &[u8],
    policy: bool,
    pregenesis_op_count: Option<&mut usize>,
) -> Result<bool, ChainGangError> {
    // Pop the keys
    let total = pop_num_for_eval(stack, policy)?;
    if total < 0 {
        return Err(ChainGangError::ScriptError(
            "total out of range".to_string(),
        ));
    }
    if let Some(op_count) = pregenesis_op_count {
        if total > MAX_PUBKEYS_PER_MULTISIG_PREGENESIS {
            return Err(ChainGangError::ScriptError(format!(
                "OP_CHECKMULTISIG has more than the pre-Genesis limit of \
                 {MAX_PUBKEYS_PER_MULTISIG_PREGENESIS} keys"
            )));
        }
        *op_count += total as usize;
        check_pregenesis_op_count(*op_count)?;
    }
    check_stack_size(total as usize, stack)?;
    let mut keys = Vec::with_capacity(total as usize);
    for _i in 0..total {
        keys.push(stack.pop().unwrap());
    }

    // Pop the sigs
    let required = pop_num_for_eval(stack, policy)?;
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
    // NULLDUMMY is policy; NULLFAIL, below, is consensus.
    if policy && !dummy.is_empty() {
        return Err(ChainGangError::ScriptError(
            "OP_CHECKMULTISIG NULLDUMMY".to_string(),
        ));
    }

    // Cleanup precedes key matching, including signatures never visited by
    // early termination. Current BSV still applies this to entries without FORKID.
    let mut cleaned_script = script.to_vec();
    for sig in sigs.iter() {
        if prefork(sig) {
            cleaned_script = remove_sig(sig, &cleaned_script);
        }
    }

    // As the node does, stop as soon as the remaining signatures outnumber the
    // remaining keys, so pairs it never checks are not checked here either.
    // A failure falls through to NULLFAIL below.
    let mut key = 0;
    let mut sig = 0;
    let mut success = true;
    while success && sig < sigs.len() {
        if checker.check_sig(&sigs[sig], &keys[key], &cleaned_script)? {
            sig += 1;
        }
        key += 1;
        if sigs.len() - sig > keys.len() - key {
            success = false;
        }
    }
    if !success && tx_enforces_malleability_rules(checker) && sigs.iter().any(|sig| !sig.is_empty())
    {
        return Err(ChainGangError::ScriptError(
            "OP_CHECKMULTISIG NULLFAIL".to_string(),
        ));
    }
    Ok(success)
}

/// Whether this entry is eligible for legacy FindAndDelete cleanup.
pub(crate) fn prefork(sig: &[u8]) -> bool {
    match sig.last() {
        None => true,
        Some(flag) => flag & SIGHASH_FORKID == 0,
    }
}

/// Node FindAndDelete(CScript(sig)): remove the serialized data push at opcode
/// boundaries, not raw signature bytes or matching bytes inside another push.
/// The empty signature is encoded as OP_0 and is not a no-op here.
pub(crate) fn remove_sig(sig: &[u8], script: &[u8]) -> Vec<u8> {
    let mut pushed = Script::new();
    pushed.append_data(sig);
    let needle = &pushed.0;
    let mut result = Vec::with_capacity(script.len());
    let mut i = 0;
    let mut start = 0;
    while i + needle.len() <= script.len() {
        if script[i..i + needle.len()] == **needle {
            result.extend_from_slice(&script[start..i]);
            start = i + needle.len();
            i = start;
        } else {
            i = next_op(i, script);
        }
    }
    result.extend_from_slice(&script[start..]);
    result
}
