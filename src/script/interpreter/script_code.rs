use crate::script::op_codes::*;

use super::multisig::{prefork, remove_sig};
use super::push::next_op;

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

pub(crate) fn strip_code_separators(script: &[u8]) -> Vec<u8> {
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

pub(crate) fn checksig_script_code(
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

pub(crate) fn multisig_script_code(
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
