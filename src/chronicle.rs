//! Chronicle (Bitcoin SV) helpers for transaction and script validation.
//!
//! Chronicle behavior is gated on **`tx.version > 1`** in this library. See
//! [docs/Chronicle.md](https://github.com/nchain-innovation/chain-gang/blob/main/docs/Chronicle.md)
//! for sighash routing, opcodes, two-phase eval, malleability rules, and script number limits.
//!
//! # Examples
//!
//! ```
//! use chain_gang::chronicle::{
//!     uses_low_s_signing, uses_relaxed_malleability, uses_two_phase_eval, SIGHASH_CHRONICLE,
//! };
//! use chain_gang::transaction::sighash::{SIGHASH_ALL, SIGHASH_FORKID};
//!
//! assert!(uses_two_phase_eval(2));
//! assert!(uses_relaxed_malleability(2));
//! assert!(!uses_two_phase_eval(1));
//! assert!(!uses_low_s_signing(
//!     SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE
//! ));
//! ```

pub use crate::script::{
    eval_two_phase, eval_two_phase_with_stack, is_push_only, max_script_num_length,
    uses_relaxed_malleability, uses_two_phase_eval, TxVersionChecker, ZVersionChecker,
    MAX_SCRIPT_NUM_LENGTH_CHRONICLE, MAX_SCRIPT_NUM_LENGTH_GENESIS,
    MAX_SCRIPT_NUM_LENGTH_PREGENESIS,
};
pub use crate::transaction::uses_low_s_signing;
pub use crate::transaction::sighash::SIGHASH_CHRONICLE;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::sighash::{SIGHASH_ALL, SIGHASH_FORKID};

    #[test]
    fn chronicle_reexports_match_underlying_modules() {
        assert_eq!(SIGHASH_CHRONICLE, crate::transaction::sighash::SIGHASH_CHRONICLE);
        assert!(uses_two_phase_eval(2));
        assert!(uses_relaxed_malleability(2));
        assert!(!uses_low_s_signing(
            SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE
        ));
        assert_eq!(
            max_script_num_length(&TxVersionChecker { tx_version: 2 }, 0),
            MAX_SCRIPT_NUM_LENGTH_CHRONICLE
        );
    }
}
