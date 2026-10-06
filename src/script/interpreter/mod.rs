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

use rules::validate_final_stack;

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
