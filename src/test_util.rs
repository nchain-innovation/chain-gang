//! Shared helpers used only by unit/integration tests.

use k256::ecdsa::Signature;
use num_bigint::BigUint;

/// The secp256k1 group order *N*, in hex.
pub(crate) const SECP256K1_N: &str =
    "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141";

/// Returns the high-S counterpart (`N - s`) of a low-S signature.
///
/// Used by tests that need to construct high-S signatures (rejected by the
/// default low-S policy, allowed for Chronicle `SIGHASH_CHRONICLE`).
pub(crate) fn flip_to_high_s(low_sig: &Signature) -> Signature {
    let compact = low_sig.to_bytes();
    let n = BigUint::parse_bytes(SECP256K1_N.as_bytes(), 16).unwrap();
    let s = BigUint::from_bytes_be(&compact[32..]);
    let high_s = &n - &s;
    let mut high_compact = compact;
    let high_s_bytes = high_s.to_bytes_be();
    high_compact[64 - high_s_bytes.len()..].copy_from_slice(&high_s_bytes);
    Signature::try_from(high_compact.as_ref()).unwrap()
}
