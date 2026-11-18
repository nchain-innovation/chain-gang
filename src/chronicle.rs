//! Chronicle (Bitcoin SV) helpers for transaction and script validation.
//!
//! Chronicle activation, spending-transaction version, and UTXO creation era are
//! independent inputs. Use [`ChronicleValidationContext`] with complete output
//! heights for historical/mixed-era validation. Context-free helpers retain
//! legacy assumptions and are not a replacement for chain context.
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
pub use crate::transaction::sighash::SIGHASH_CHRONICLE;
pub use crate::transaction::uses_low_s_signing;

/// Chronicle activation height on BSV mainnet.
pub const CHRONICLE_ACTIVATION_MAINNET: u64 = 943_816;
/// Chronicle activation height on BSV testnet.
pub const CHRONICLE_ACTIVATION_TESTNET: u64 = 1_713_168;
/// Default Chronicle activation height on BSV regtest (node-configurable).
pub const CHRONICLE_ACTIVATION_REGTEST: u64 = 15_000;
/// Default Chronicle activation height on STN.
pub const CHRONICLE_ACTIVATION_STN: u64 = 250;

/// Returns the Chronicle activation height for a BSV network, if defined.
pub fn activation_height(network: Network) -> Option<u64> {
    match network {
        Network::BSV_Mainnet => Some(CHRONICLE_ACTIVATION_MAINNET),
        Network::BSV_Testnet => Some(CHRONICLE_ACTIVATION_TESTNET),
        Network::BSV_STN => Some(CHRONICLE_ACTIVATION_STN),
        // Private-chain overrides belong in ChronicleValidationContext.
        Network::BSV_Regtest => Some(CHRONICLE_ACTIVATION_REGTEST),
        // Chronicle is a BSV upgrade, so it has no height on the other chains.
        // Listed rather than caught, so a new network has to choose.
        Network::BTC_Mainnet
        | Network::BTC_Testnet
        | Network::BCH_Mainnet
        | Network::BCH_Testnet => None,
    }
}

/// Whether Chronicle version-dependent malleability relaxation applies.
///
/// When `block_height` and `network` are both omitted, Chronicle is enabled for `tx.version > 1`
/// (library default). When both are provided on a BSV network, activation height is enforced.
pub fn chronicle_rules_active(
    tx_version: u32,
    block_height: Option<u64>,
    network: Option<Network>,
) -> bool {
    if (tx_version as i32) <= 1 {
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
    } else if (tx_version as i32) > 1 {
        1
    } else {
        tx_version
    }
}

/// Explicit Chronicle chain and UTXO context. Heights are mandatory for every
/// spent output; missing metadata is an error, never guessed from tx.version.
/// Genesis classification still comes from the existing pregenesis output set.
pub struct ChronicleValidationContext<'a> {
    /// Height of the block containing the spending transaction.
    pub block_height: u64,
    /// Network activation height, or an explicit private-chain override.
    pub activation_height: u64,
    /// Creation heights of spent outputs (including same-block outputs).
    pub utxo_heights: &'a std::collections::HashMap<crate::messages::OutPoint, u64>,
    /// Optional numeric policy bound in bytes; ignored by consensus validation.
    pub script_num_policy_limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::sighash::{SIGHASH_ALL, SIGHASH_FORKID};

    #[test]
    fn chronicle_reexports_match_underlying_modules() {
        assert_eq!(
            SIGHASH_CHRONICLE,
            crate::transaction::sighash::SIGHASH_CHRONICLE
        );
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
        assert_eq!(
            activation_height(Network::BSV_Regtest),
            Some(CHRONICLE_ACTIVATION_REGTEST)
        );
        assert!(activation_height(Network::BTC_Mainnet).is_none());
    }

    #[test]
    fn regtest_uses_node_default_height() {
        assert!(!chronicle_rules_active(
            2,
            Some(14999),
            Some(Network::BSV_Regtest)
        ));
        assert!(chronicle_rules_active(
            2,
            Some(15000),
            Some(Network::BSV_Regtest)
        ));
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
