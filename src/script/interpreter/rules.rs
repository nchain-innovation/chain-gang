use crate::script::stack::{
    check_script_num_length, decode_bigint, decode_bool, is_minimally_encoded,
    pop_bigint_checked_minimal, pop_bool_minimal, pop_num_minimal, Stack,
    MAX_SCRIPT_NUM_LENGTH_CHRONICLE, MAX_SCRIPT_NUM_LENGTH_GENESIS,
    MAX_SCRIPT_NUM_LENGTH_PREGENESIS,
};
use crate::script::Checker;
use crate::util::ChainGangError;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use super::{CONSENSUS_ONLY, PREGENESIS_RULES};

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

/// Longest encoded result of a numeric opcode.
///
/// Before Genesis the node limits numeric operands to 4 bytes but not results:
/// it does the arithmetic in 64 bits and pushes whatever comes out, so
/// `2147483647 DUP ADD` leaves a 5-byte number and two 4-byte operands
/// multiply to 8 bytes. Such a result can be compared as bytes or cast to a
/// boolean; only using it as a number again fails. After Genesis results keep
/// the operand limit.
pub(crate) fn max_script_num_result_length<T: Checker>(checker: &T, flags: u32) -> usize {
    if flags & PREGENESIS_RULES != 0 {
        return usize::MAX;
    }
    max_script_num_length(checker, flags)
}

/// Longest operand `OP_CHECKLOCKTIMEVERIFY` and `OP_CHECKSEQUENCEVERIFY` read.
///
/// One byte past the 4-byte limit on other operands, as in the node, so that
/// a lock time or sequence number, both unsigned 32-bit fields, fits: 5 bytes
/// hold up to 2^39 - 1.
const LOCKTIME_OPERAND_MAX_LENGTH: usize = 5;

/// Reads the operand of `OP_CHECKLOCKTIMEVERIFY` or `OP_CHECKSEQUENCEVERIFY` as
/// the node does: up to 5 bytes, minimally encoded where the policy rules
/// apply, not negative, and left on the stack.
///
/// Both opcodes took over a NOP, so they leave the stack as they found it; a
/// script drops the operand itself. chain-gang used to pop it, and to read it
/// as an ordinary 4-byte number, so it rejected `2147483648
/// OP_CHECKSEQUENCEVERIFY`, which the node accepts.
pub(crate) fn peek_locktime_operand(stack: &Stack, policy: bool) -> Result<i64, ChainGangError> {
    let top = stack
        .last()
        .ok_or_else(|| ChainGangError::ScriptError("Stack too small: 1".to_string()))?;
    check_script_num_length(top.len(), LOCKTIME_OPERAND_MAX_LENGTH)?;
    if policy && !is_minimally_encoded(top) {
        return Err(ChainGangError::ScriptError(
            "Number is not minimally encoded".to_string(),
        ));
    }
    let n = decode_bigint(&mut top.clone())
        .to_i64()
        .expect("5 bytes fit in an i64");
    if n < 0 {
        return Err(ChainGangError::ScriptError(
            "Negative lock time".to_string(),
        ));
    }
    Ok(n)
}

pub(crate) fn tx_enforces_malleability_rules<T: Checker>(checker: &T) -> bool {
    match checker.tx_version() {
        Ok(version) => !uses_relaxed_malleability(version as u32),
        Err(_) => false,
    }
}

/// Whether the node's policy rules apply: `MINIMALDATA`, `NULLDUMMY` and
/// `CLEANSTACK`.
///
/// The node applies each only when its flag is set and non-malleability is
/// enforced (`VerifyX(flags) && EnforceNonMalleability(...)`). Its mempool
/// sets all three and its block validation sets none, so here they apply
/// unless [`CONSENSUS_ONLY`] is given, and only to non-malleable transactions.
/// NULLFAIL is not among them: it is mandatory, and stays on
/// [`tx_enforces_malleability_rules`] alone.
pub(crate) fn enforces_policy_rules<T: Checker>(checker: &T, flags: u32) -> bool {
    flags & CONSENSUS_ONLY == 0 && tx_enforces_malleability_rules(checker)
}

/// Pops a number operand, minimally encoded where the policy rules apply.
pub(crate) fn pop_num_for_eval(stack: &mut Stack, policy: bool) -> Result<i32, ChainGangError> {
    pop_num_minimal(stack, policy)
}

/// Pops a bigint operand, minimally encoded where the policy rules apply.
pub(crate) fn pop_bigint_for_eval(
    stack: &mut Stack,
    max_len: usize,
    policy: bool,
) -> Result<BigInt, ChainGangError> {
    pop_bigint_checked_minimal(stack, max_len, policy)
}

/// Pops an `OP_IF`/`OP_NOTIF` operand as a boolean, with no minimality check.
///
/// The node's minimal-`IF` rule sits behind `SCRIPT_VERIFY_MINIMALIF`, which is
/// in neither its mandatory nor its standard flags: only a caller that asks
/// for it explicitly gets it. So any non-zero operand is true here, as in the
/// node's blocks and mempool (#205).
pub(crate) fn pop_bool_for_if(stack: &mut Stack) -> Result<bool, ChainGangError> {
    pop_bool_minimal(stack, false)
}

/// The checks a finished script must pass: a true value on top, and, where the
/// policy rules apply, nothing else under it (`CLEANSTACK`).
pub(crate) fn validate_final_stack(stack: &Stack, policy: bool) -> Result<(), ChainGangError> {
    if stack.is_empty() {
        return Err(ChainGangError::ScriptError("Stack empty".to_string()));
    }
    if !decode_bool(&stack[stack.len() - 1]) {
        return Err(ChainGangError::ScriptError(
            "Top of stack is false".to_string(),
        ));
    }
    if policy && stack.len() != 1 {
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

#[cfg(test)]
mod tests {
    use super::super::eval;
    use super::*;
    use crate::script::op_codes::*;
    use crate::script::{TxVersionChecker, ZVersionChecker, CONSENSUS_ONLY, NO_FLAGS};
    use crate::transaction::generate_signature;
    use crate::util::Hash256;
    use k256::ecdsa::SigningKey;

    /// Evaluates `script` for a non-malleable (version 1) transaction, with
    /// the node's policy rules (default) or consensus only.
    fn run(script: &[u8], flags: u32) -> Result<(), ChainGangError> {
        eval(script, &mut TxVersionChecker { tx_version: 1 }, flags)
    }

    fn assert_policy_only(script: &[u8], reason: &str) {
        match run(script, NO_FLAGS) {
            Err(e) if e.to_string().contains(reason) => {}
            other => panic!("with policy rules, expected {reason:?}, got {other:?}"),
        }
        run(script, CONSENSUS_ONLY)
            .unwrap_or_else(|e| panic!("consensus only should accept it, got {e}"));
    }

    /// Each of the node's policy rules applies by default, for a transaction
    /// the node holds to them, and not under CONSENSUS_ONLY (#205). The node
    /// has all three in STANDARD_SCRIPT_VERIFY_FLAGS and none in either
    /// mandatory set: its mempool rejects these, its blocks accept them.
    #[test]
    fn policy_rules_apply_by_default_and_not_under_consensus_only() {
        // MINIMALDATA, push: one byte that OP_5 pushes on its own.
        assert_policy_only(&[1, 0x05, OP_DROP, OP_1], "Non-minimal push");
        // MINIMALDATA, number: 01 00 is 1 with a padding byte.
        assert_policy_only(
            &[2, 0x01, 0x00, OP_1ADD, OP_2, OP_EQUAL],
            "Number is not minimally encoded",
        );
        // NULLDUMMY: a 0-of-0 CHECKMULTISIG with a non-empty dummy.
        assert_policy_only(&[OP_1, OP_0, OP_0, OP_CHECKMULTISIG], "NULLDUMMY");
        // CLEANSTACK: true on top, something under it.
        assert_policy_only(&[OP_1, OP_1], "Clean stack violation");
    }

    /// NULLFAIL is consensus, in both of the node's mandatory sets, so
    /// CONSENSUS_ONLY does not lift it.
    #[test]
    fn nullfail_applies_under_consensus_only_too() {
        let sig = generate_signature(&[5; 32], &Hash256([9; 32]), 0x41).unwrap();
        let other_key: Vec<u8> = SigningKey::from_slice(&[6; 32])
            .unwrap()
            .verifying_key()
            .to_sec1_bytes()
            .to_vec();
        let mut script = vec![sig.len() as u8];
        script.extend_from_slice(&sig);
        script.push(other_key.len() as u8);
        script.extend_from_slice(&other_key);
        script.extend_from_slice(&[OP_CHECKSIG, OP_NOT]);
        for flags in [NO_FLAGS, CONSENSUS_ONLY] {
            let mut checker = ZVersionChecker {
                z: Hash256([9; 32]),
                tx_version: 1,
            };
            let err = eval(&script, &mut checker, flags).unwrap_err();
            assert!(
                err.to_string().contains("NULLFAIL"),
                "flags {flags:#x}: {err}"
            );
        }
    }

    /// The node never enables MINIMALIF in its blocks or its mempool, so an
    /// OP_IF operand of 2 is simply true, under either mode. chain-gang used to
    /// reject it for version 1 transactions.
    #[test]
    fn minimal_if_is_not_enforced() {
        let script = [OP_2, OP_IF, OP_1, OP_ENDIF];
        run(&script, NO_FLAGS).unwrap();
        run(&script, CONSENSUS_ONLY).unwrap();
    }

    /// Malleable transactions (version 2 under Chronicle) are exempt from the
    /// policy rules either way, as in the node.
    #[test]
    fn malleable_transactions_are_exempt_either_way() {
        let mut checker = TxVersionChecker { tx_version: 2 };
        eval(&[OP_1, OP_1], &mut checker, NO_FLAGS).unwrap();
        assert!(!enforces_policy_rules(&checker, NO_FLAGS));
        assert!(enforces_policy_rules(
            &TxVersionChecker { tx_version: 1 },
            NO_FLAGS
        ));
        assert!(!enforces_policy_rules(
            &TxVersionChecker { tx_version: 1 },
            CONSENSUS_ONLY
        ));
    }
}
