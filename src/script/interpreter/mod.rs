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
