//! Bitcoin script interpreter (evaluation engine).

mod eval;
mod multisig;
mod push;
mod rules;
mod script_code;

#[cfg(test)]
mod tests;

pub use push::{is_push_only, next_op};
pub use rules::{max_script_num_length, uses_relaxed_malleability, uses_two_phase_eval};
pub use script_code::{TwoPhaseEvalContext, TwoPhasePhase};

pub use eval::core_eval;

// Stack capacity defaults, which may exceeded
pub(crate) const STACK_CAPACITY: usize = 100;
pub(crate) const ALT_STACK_CAPACITY: usize = 10;

/// Execute the script with genesis rules
pub const NO_FLAGS: u32 = 0x00;

/// Flag to execute the script with pre-genesis rules
pub const PREGENESIS_RULES: u32 = 0x01;

use crate::script::stack::Stack;
use crate::script::Checker;
use crate::util::ChainGangError;

use crate::script::stack::decode_bool;
use rules::validate_final_stack;

/// Evaluates a spend of a pre-Genesis P2SH output, as the node's `VerifyScript`
/// does it (BIP-16).
///
/// The unlocking script runs first, and the stack it leaves is kept. The
/// locking script, `OP_HASH160 <h> OP_EQUAL`, then runs on it and must leave
/// true, which only proves the last push hashes to `<h>`. That last push is the
/// redeem script: the kept stack is restored, the redeem script popped from its
/// top, and run against what remains, and it is that run which must leave true.
/// The unlocking script must be push-only, so the stack it leaves is just its
/// pushes.
///
/// Each script runs on its own, as the node runs each in its own `EvalScript`:
/// its own `OP_CODESEPARATOR` position, its own alt stack, its own branch
/// balance. A `CHECKSIG` in the redeem script therefore signs the redeem script.
/// The final-stack checks apply once, after the redeem script, as the node's
/// `CLEANSTACK` does; checked after the locking script they would see the
/// redeem script's inputs still on the stack.
///
/// Before this, chain-gang ran only the unlocking and locking scripts, so a
/// pre-Genesis P2SH spend passed as long as it pushed a script with the right
/// hash, whatever that script did (#201).
pub(crate) fn eval_p2sh<T: Checker>(
    unlock_script: &[u8],
    lock_script: &[u8],
    checker: &mut T,
    flags: u32,
) -> Result<(), ChainGangError> {
    if !is_push_only(unlock_script) {
        return Err(ChainGangError::ScriptError(
            "P2SH unlocking script must be push-only".to_string(),
        ));
    }
    let (after_unlock, _, _) =
        core_eval(unlock_script, checker, flags, None, None, None, None, None)?;

    let (stack, _, _) = core_eval(
        lock_script,
        checker,
        flags,
        None,
        None,
        Some(after_unlock.clone()),
        None,
        None,
    )?;
    if !stack.last().is_some_and(|top| decode_bool(top)) {
        return Err(ChainGangError::ScriptError(
            "P2SH script hash does not match".to_string(),
        ));
    }

    let mut stack = after_unlock;
    // Non-empty: the locking script hashed its top item and matched.
    let redeem_script = stack.pop().ok_or_else(|| {
        ChainGangError::ScriptError("P2SH unlocking script pushes nothing".to_string())
    })?;
    let (stack, _, _) = core_eval(
        &redeem_script,
        checker,
        flags,
        None,
        None,
        Some(stack),
        None,
        None,
    )?;
    validate_final_stack(&stack, checker)
}

/// Executes a script
pub fn eval<T: Checker>(script: &[u8], checker: &mut T, flags: u32) -> Result<(), ChainGangError> {
    match core_eval(script, checker, flags, None, None, None, None, None) {
        Ok((stack, _alt_stack, _script_counter)) => validate_final_stack(&stack, checker),
        Err(x) => Err(x),
    }
}

/// Evaluates a transaction input as the node's `VerifyScript` does: the
/// unlocking script runs on its own, then the locking script runs on the stack
/// it leaves.
///
/// Each script is evaluated separately, with its own alt stack and its own
/// branch balance, so neither can reach into the other: a push that runs past
/// the end of the unlocking script fails there rather than taking bytes from
/// the locking script, and an `IF` opened in one cannot close in the other.
///
/// `Tx::validate` used to evaluate `unlock OP_CODESEPARATOR lock` as one
/// script instead. Together with `is_push_only` accepting truncated pushes,
/// that let an unlocking script made of one push opcode and no data take the
/// separator and the whole locking script as its data and leave a true value,
/// so any output could be spent without a signature (for a P2PKH output the
/// unlocking script is the single byte `0x1a`). The node rejects that spend.
///
/// For `tx.version > 1` under Chronicle, use [`eval_two_phase`], whose
/// unlocking-phase `CHECKSIG` signs through the locking script.
pub fn eval_unlock_then_lock<T: Checker>(
    unlock: &[u8],
    lock: &[u8],
    checker: &mut T,
    flags: u32,
) -> Result<(), ChainGangError> {
    let (stack, _, _) = core_eval(unlock, checker, flags, None, None, None, None, None)?;
    let (stack, _, _) = core_eval(lock, checker, flags, None, None, Some(stack), None, None)?;
    validate_final_stack(&stack, checker)
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
