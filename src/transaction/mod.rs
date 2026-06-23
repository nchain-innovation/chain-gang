//! Build and sign transactions
//!
//! # Examples
//!
//! Sign a transaction:
//!
//! ```rust
//! use chain_gang::messages::{Tx, TxIn};
//! use chain_gang::transaction::generate_signature;
//! use chain_gang::transaction::p2pkh::{create_lock_script, create_unlock_script};
//! use chain_gang::transaction::sighash::{sighash, SigHashCache, SIGHASH_FORKID, SIGHASH_NONE};
//! use chain_gang::util::{hash160};
//!
//! // Use real values here
//! let mut tx = Tx {
//!     inputs: vec![TxIn {
//!         ..Default::default()
//!     }],
//!     ..Default::default()
//! };
//! let private_key = [1; 32];
//! let public_key = [1; 33];
//!
//! let lock_script = create_lock_script(&hash160(&public_key));
//! let mut cache = SigHashCache::new();
//! let sighash_type = SIGHASH_NONE | SIGHASH_FORKID;
//! let sighash = sighash(&tx, 0, &lock_script.0, 0, sighash_type, &mut cache).unwrap();
//! let signature = generate_signature(&private_key, &sighash, sighash_type).unwrap();
//! tx.inputs[0].unlock_script = create_unlock_script(&signature, &public_key);
//! ```

use crate::util::{ChainGangError, Hash256};
use k256::ecdsa::{
    signature::{hazmat::PrehashSigner, SignatureEncoding},
    Signature, SigningKey,
};

pub mod p2pkh;
pub mod sighash;

/// Returns true when low-S normalization should be applied before encoding a signature.
///
/// Chronicle signatures (`SIGHASH_CHRONICLE`) may use high-S values per the Chronicle spec.
pub fn uses_low_s_signing(sighash_type: u8) -> bool {
    sighash_type & sighash::SIGHASH_CHRONICLE == 0
}

/// Applies low-S normalization policy and appends the sighash type byte.
fn encode_signature(signature: Signature, sighash_type: u8) -> Vec<u8> {
    let signature = if uses_low_s_signing(sighash_type) {
        signature.normalize_s()
    } else {
        signature
    };
    let mut sig = signature.to_der().to_vec();
    sig.push(sighash_type);
    sig
}

/// Generates a signature for a transaction sighash
pub fn generate_signature(
    private_key: &[u8; 32],
    sighash: &Hash256,
    sighash_type: u8,
) -> Result<Vec<u8>, ChainGangError> {
    let signing_key = SigningKey::from_slice(private_key)?;
    let signature: Signature = signing_key.sign_prehash(&sighash.0)?;
    Ok(encode_signature(signature, sighash_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::sighash::{SIGHASH_ALL, SIGHASH_CHRONICLE, SIGHASH_FORKID};
    use k256::ecdsa::SigningKey;
    use num_bigint::BigUint;

    const SECP256K1_N: &str =
        "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141";

    fn signature_has_high_s(der: &[u8]) -> bool {
        let sig = Signature::from_der(der).unwrap();
        sig.normalize_s() != sig
    }

    fn flip_der_to_high_s(der: &[u8]) -> Vec<u8> {
        let sig = Signature::from_der(der).unwrap();
        let low_s = sig.normalize_s();
        assert!(
            !signature_has_high_s(low_s.to_der().as_bytes()),
            "expected low-S input for flip helper"
        );

        let compact = low_s.to_bytes();
        let n = BigUint::parse_bytes(SECP256K1_N.as_bytes(), 16).unwrap();
        let s = BigUint::from_bytes_be(&compact[32..]);
        let high_s = &n - &s;
        assert!(high_s > n >> 1);

        let mut high_compact = compact;
        let high_s_bytes = high_s.to_bytes_be();
        high_compact[64 - high_s_bytes.len()..].copy_from_slice(&high_s_bytes);
        let high_sig = Signature::try_from(high_compact.as_ref()).unwrap();
        assert!(signature_has_high_s(high_sig.to_der().as_bytes()));
        high_sig.to_der().to_vec()
    }

    #[test]
    fn uses_low_s_signing_without_chronicle() {
        assert!(uses_low_s_signing(SIGHASH_ALL | SIGHASH_FORKID));
    }

    #[test]
    fn uses_low_s_signing_disabled_with_chronicle() {
        assert!(!uses_low_s_signing(
            SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE
        ));
    }

    #[test]
    fn generate_signature_without_chronicle_is_low_s() {
        let key = [2u8; 32];
        let sighash = Hash256([3u8; 32]);
        let sig = generate_signature(&key, &sighash, SIGHASH_ALL | SIGHASH_FORKID).unwrap();
        assert!(!signature_has_high_s(&sig[..sig.len() - 1]));
    }

    #[test]
    fn encode_signature_normalizes_high_s_without_chronicle() {
        let key = [2u8; 32];
        let sighash = Hash256([3u8; 32]);
        let signing_key = SigningKey::from_slice(&key).unwrap();
        let raw: Signature = signing_key.sign_prehash(&sighash.0).unwrap();
        let high_s = Signature::from_der(flip_der_to_high_s(raw.to_der().as_bytes()).as_slice()).unwrap();

        let encoded = encode_signature(high_s, SIGHASH_ALL | SIGHASH_FORKID);
        assert!(!signature_has_high_s(&encoded[..encoded.len() - 1]));
    }

    #[test]
    fn encode_signature_preserves_high_s_with_chronicle() {
        let key = [2u8; 32];
        let sighash = Hash256([3u8; 32]);
        let signing_key = SigningKey::from_slice(&key).unwrap();
        let raw: Signature = signing_key.sign_prehash(&sighash.0).unwrap();
        let high_s = Signature::from_der(flip_der_to_high_s(raw.to_der().as_bytes()).as_slice()).unwrap();

        let encoded = encode_signature(high_s, SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE);
        assert!(signature_has_high_s(&encoded[..encoded.len() - 1]));
    }

    #[test]
    fn high_s_ecdsa_verifies_with_k256() {
        use k256::ecdsa::signature::hazmat::PrehashVerifier;
        let key = [2u8; 32];
        let sighash = Hash256([3u8; 32]);
        let signing_key = SigningKey::from_slice(&key).unwrap();
        let raw: Signature = signing_key.sign_prehash(&sighash.0).unwrap();
        let high_s =
            Signature::from_der(flip_der_to_high_s(raw.to_der().as_bytes()).as_slice()).unwrap();
        let normalized = high_s.normalize_s();
        assert_eq!(normalized, raw);
        assert!(
            signing_key
                .verifying_key()
                .verify_prehash(&sighash.0, &normalized)
                .is_ok()
        );
        // k256 rejects non-normalized S at verify time; Chronicle nodes accept high-S
        // by verifying the equivalent normalized signature.
        assert!(
            signing_key
                .verifying_key()
                .verify_prehash(&sighash.0, &high_s)
                .is_err()
        );
    }

    #[test]
    fn generate_signature_chronicle_uses_raw_signer_output() {
        let key = [2u8; 32];
        let sighash = Hash256([3u8; 32]);
        let signing_key = SigningKey::from_slice(&key).unwrap();
        let raw: Signature = signing_key.sign_prehash(&sighash.0).unwrap();
        let chronicle = generate_signature(
            &key,
            &sighash,
            SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE,
        )
        .unwrap();
        let mut expected = raw.to_der().to_vec();
        expected.push(SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE);
        assert_eq!(chronicle, expected);
    }
}

