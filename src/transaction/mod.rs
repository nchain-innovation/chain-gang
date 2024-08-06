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

use crate::util::{Hash256, Result};
use k256::ecdsa::{
    signature::{hazmat::PrehashSigner, SignatureEncoding},
    Signature, SigningKey,
};

pub mod p2pkh;
pub mod sighash;

/// Generates a signature for a transaction sighash
pub fn generate_signature(
    private_key: &[u8; 32],
    sighash: &Hash256,
    sighash_type: u8,
) -> Result<Vec<u8>> {
    let message = sighash.0;

    let signing_key = SigningKey::from_slice(private_key)?;
    let signature: Signature = signing_key.sign_prehash(&message)?;
    let signature = signature.normalize_s().unwrap_or(signature);
    let sig_der = signature.to_der();
    let mut sig = sig_der.to_vec();

    sig.push(sighash_type);
    Ok(sig)
}
