//! Chronicle (Bitcoin SV) helpers for transaction and script validation.
//!
//! By default this library gates Chronicle on **`tx.version > 1`** only. Pass block height
//! and network to [`effective_chronicle_tx_version`] or [`Tx::validate_at_height`] for
//! consensus-faithful activation at documented BSV block heights.
//!
//! See [docs/Chronicle.md](https://github.com/nchain-innovation/chain-gang/blob/main/docs/Chronicle.md)
//! for sighash routing, opcodes, two-phase eval, malleability rules, and script number limits.
//!
//! # Examples
//!
//! ```
//! use chain_gang::chronicle::{
//!     activation_height, chronicle_rules_active, effective_chronicle_tx_version,
//!     uses_low_s_signing, uses_relaxed_malleability, uses_two_phase_eval, SIGHASH_CHRONICLE,
//!     CHRONICLE_ACTIVATION_MAINNET,
//! };
//! use chain_gang::network::Network;
//! use chain_gang::transaction::sighash::{SIGHASH_ALL, SIGHASH_FORKID};
//!
//! assert!(uses_two_phase_eval(2));
//! assert!(uses_relaxed_malleability(2));
//! assert!(!uses_two_phase_eval(1));
//! assert!(!uses_low_s_signing(
//!     SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE
//! ));
//! assert_eq!(
//!     activation_height(Network::BSV_Mainnet),
//!     Some(CHRONICLE_ACTIVATION_MAINNET)
//! );
//! assert!(!chronicle_rules_active(
//!     2,
//!     Some(CHRONICLE_ACTIVATION_MAINNET - 1),
//!     Some(Network::BSV_Mainnet),
//! ));
//! assert_eq!(
//!     effective_chronicle_tx_version(
//!         2,
//!         Some(CHRONICLE_ACTIVATION_MAINNET - 1),
//!         Some(Network::BSV_Mainnet),
//!     ),
//!     1
//! );
//! ```

use crate::network::Network;

pub use crate::script::{
    eval_two_phase, eval_two_phase_with_stack, is_push_only, max_script_num_length,
    uses_relaxed_malleability, uses_two_phase_eval, TxVersionChecker, ZVersionChecker,
    MAX_SCRIPT_NUM_LENGTH_CHRONICLE, MAX_SCRIPT_NUM_LENGTH_GENESIS,
    MAX_SCRIPT_NUM_LENGTH_PREGENESIS,
};
pub use crate::transaction::uses_low_s_signing;
pub use crate::transaction::sighash::SIGHASH_CHRONICLE;

/// Chronicle activation height on BSV mainnet.
pub const CHRONICLE_ACTIVATION_MAINNET: u64 = 943_835;
/// Chronicle activation height on BSV testnet (and STN).
pub const CHRONICLE_ACTIVATION_TESTNET: u64 = 1_713_022;

/// Returns the Chronicle activation height for a BSV network, if defined.
pub fn activation_height(network: Network) -> Option<u64> {
    match network {
        Network::BSV_Mainnet => Some(CHRONICLE_ACTIVATION_MAINNET),
        Network::BSV_Testnet | Network::BSV_STN => Some(CHRONICLE_ACTIVATION_TESTNET),
        _ => None,
    }
}

/// Whether Chronicle script rules apply for the given transaction version and optional context.
///
/// When `block_height` and `network` are both omitted, Chronicle is enabled for `tx.version > 1`
/// (library default). When both are provided on a BSV network, activation height is enforced.
pub fn chronicle_rules_active(
    tx_version: u32,
    block_height: Option<u64>,
    network: Option<Network>,
) -> bool {
    if tx_version <= 1 {
        return false;
    }
    match (block_height, network) {
        (None, None) => true,
        (Some(height), Some(net)) => activation_height(net)
            .map(|threshold| height >= threshold)
            .unwrap_or(false),
        _ => true,
    }
}

/// Transaction version used for Chronicle script rules after applying optional activation context.
///
/// Returns `1` when a `version > 1` transaction is evaluated before Chronicle activation.
pub fn effective_chronicle_tx_version(
    tx_version: u32,
    block_height: Option<u64>,
    network: Option<Network>,
) -> u32 {
    if chronicle_rules_active(tx_version, block_height, network) {
        tx_version
    } else if tx_version > 1 {
        1
    } else {
        tx_version
    }
}

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

    #[test]
    fn activation_height_bsv_networks() {
        assert_eq!(
            activation_height(Network::BSV_Mainnet),
            Some(CHRONICLE_ACTIVATION_MAINNET)
        );
        assert_eq!(
            activation_height(Network::BSV_Testnet),
            Some(CHRONICLE_ACTIVATION_TESTNET)
        );
        assert!(activation_height(Network::BTC_Mainnet).is_none());
    }

    #[test]
    fn chronicle_rules_version_only_by_default() {
        assert!(chronicle_rules_active(2, None, None));
        assert!(!chronicle_rules_active(1, None, None));
    }

    #[test]
    fn chronicle_rules_respect_mainnet_height() {
        assert!(!chronicle_rules_active(
            2,
            Some(CHRONICLE_ACTIVATION_MAINNET - 1),
            Some(Network::BSV_Mainnet),
        ));
        assert!(chronicle_rules_active(
            2,
            Some(CHRONICLE_ACTIVATION_MAINNET),
            Some(Network::BSV_Mainnet),
        ));
    }

    #[test]
    fn effective_version_suppresses_chronicle_pre_activation() {
        assert_eq!(
            effective_chronicle_tx_version(
                2,
                Some(CHRONICLE_ACTIVATION_MAINNET - 1),
                Some(Network::BSV_Mainnet),
            ),
            1
        );
        assert_eq!(
            effective_chronicle_tx_version(
                2,
                Some(CHRONICLE_ACTIVATION_MAINNET),
                Some(Network::BSV_Mainnet),
            ),
            2
        );
    }
}
