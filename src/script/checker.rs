use crate::messages::Tx;
use crate::transaction::sighash::{
    sighash_from_script_code, SigHashCache, SIGHASH_ANYONECANPAY, SIGHASH_CHRONICLE, SIGHASH_FORKID,
};
use crate::util::{ChainGangError, Hash256};

use k256::ecdsa::{signature::hazmat::PrehashVerifier, Signature, VerifyingKey};

/// Locktimes greater than or equal to this are interpreted as timestamps. Less then, block heights.
const LOCKTIME_THRESHOLD: i64 = 500000000;

/// Disables the relative lock time for the sequence field
pub(crate) const SEQUENCE_LOCKTIME_DISABLE_FLAG: u32 = 1 << 31;
/// When set, sequence uses time. When unset, it uses block height.
const SEQUENCE_LOCKTIME_TYPE_FLAG: u32 = 1 << 22;
/// The bits of a sequence that hold the relative lock time's value.
const SEQUENCE_LOCKTIME_MASK: u32 = 0x0000ffff;

/// Whether a signature's sighash type is one the node understands, its
/// `SigHashType::isDefined`: with the CHRONICLE, FORKID and ANYONECANPAY bits
/// cleared, what is left must be ALL, NONE or SINGLE (1, 2 or 3).
///
/// The node rejects anything else under `STRICTENC`, which has been mandatory,
/// alongside `SIGHASH_FORKID`, since the 2017 fork, so it is checked wherever
/// FORKID is required (#208). An undefined type still hashes, as if it were
/// ALL, so without this a signature over it verified and chain-gang accepted
/// a transaction the network rejects.
fn is_defined_sighash_type(sighash_type: u8) -> bool {
    matches!(
        sighash_type & !(SIGHASH_CHRONICLE | SIGHASH_FORKID | SIGHASH_ANYONECANPAY),
        1..=3
    )
}

/// The node's message for an undefined sighash type (`SCRIPT_ERR_SIG_HASHTYPE`).
const UNDEFINED_SIGHASH_TYPE: &str = "Signature hash type missing or not understood";

/// Checks that external values are correct in the script
pub trait Checker {
    /// Checks that a signature and public key validate within a script
    ///
    /// `script` is the script code the interpreter built for this check: from
    /// just past the last executed `OP_CODESEPARATOR` to the end, with the
    /// signature already removed where the rules require it. It is ready to
    /// hash, so an implementation should pass it to
    /// [`sighash_from_script_code`] as it is. [`sighash`](crate::transaction::sighash::sighash)
    /// expects a whole locking script and would cut it again.
    fn check_sig(
        &mut self,
        sig: &[u8],
        pubkey: &[u8],
        script: &[u8],
    ) -> Result<bool, ChainGangError>;

    /// Checks that the lock time is valid according to BIP 65
    ///
    /// `locktime` is `OP_CHECKLOCKTIMEVERIFY`'s operand, which the interpreter
    /// reads as up to 5 bytes, as the node does, so it can exceed `i32`: a
    /// timestamp from 2038 on is past `i32::MAX`, and a transaction's
    /// `lock_time` is a `u32`.
    fn check_locktime(&self, locktime: i64) -> Result<bool, ChainGangError>;

    /// Checks that the relative lock time enforced by the sequence is valid according to BIP 112
    ///
    /// `sequence` is `OP_CHECKSEQUENCEVERIFY`'s operand, read as up to 5 bytes
    /// like `OP_CHECKLOCKTIMEVERIFY`'s, so it can exceed `i32` too.
    fn check_sequence(&self, sequence: i64) -> Result<bool, ChainGangError>;

    /// Returns the executing transaction version for Chronicle OP_VER opcodes
    fn tx_version(&self) -> Result<i32, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction version check".to_string(),
        ))
    }
}

/// Script checker that fails all transaction checks
pub struct TransactionlessChecker {}

impl Checker for TransactionlessChecker {
    fn check_sig(
        &mut self,
        _sig: &[u8],
        _pubkey: &[u8],
        _script: &[u8],
    ) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }

    fn check_locktime(&self, _locktime: i64) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }

    fn check_sequence(&self, _sequence: i64) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }
}

/// Script checker that uses a provided transaction hash (z) for the check_sig and
/// fails all other transaction checks
pub struct ZChecker {
    /// z is sig_hash of transaction
    pub z: Hash256,
}

impl Checker for ZChecker {
    // Given a signature and public key, check signature matches signed script hash (z)
    fn check_sig(
        &mut self,
        sig: &[u8],
        pubkey: &[u8],
        _script: &[u8],
    ) -> Result<bool, ChainGangError> {
        // An empty signature fails the check rather than the script, as in the
        // node: "a compact way to provide an invalid signature for use with
        // CHECK(MULTI)SIG", passed by its encoding checks and simply false. It
        // is also the one failing signature NULLFAIL allows, which the
        // interpreter already exempts (#210).
        if sig.is_empty() {
            return Ok(false);
        }
        let sighash_type = sig[sig.len() - 1];
        if !is_defined_sighash_type(sighash_type) {
            return Err(ChainGangError::ScriptError(
                UNDEFINED_SIGHASH_TYPE.to_string(),
            ));
        }
        if sighash_type & SIGHASH_FORKID == 0 {
            return Err(ChainGangError::ScriptError(
                "SIGHASH_FORKID not present".to_string(),
            ));
        }

        let sig_hash = self.z;
        let der_sig = &sig[0..sig.len() - 1];
        let signature = Signature::from_der(der_sig)?;

        let message = sig_hash.0;

        let verifying_key = VerifyingKey::from_sec1_bytes(pubkey)?;

        Ok(verifying_key.verify_prehash(&message, &signature).is_ok())
    }

    fn check_locktime(&self, _locktime: i64) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }

    fn check_sequence(&self, _sequence: i64) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }
}

/// Script checker that supplies `tx.version` for Chronicle opcodes without transaction validation.
pub struct TxVersionChecker {
    /// Transaction version supplied to Chronicle opcodes
    pub tx_version: i32,
}

impl Checker for TxVersionChecker {
    fn check_sig(
        &mut self,
        _sig: &[u8],
        _pubkey: &[u8],
        _script: &[u8],
    ) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }

    fn check_locktime(&self, _locktime: i64) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }

    fn check_sequence(&self, _sequence: i64) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }

    fn tx_version(&self) -> Result<i32, ChainGangError> {
        Ok(self.tx_version)
    }
}

/// Script checker that uses a provided sighash and transaction version (Chronicle debugging).
pub struct ZVersionChecker {
    /// Precomputed sighash digest used to verify signatures
    pub z: Hash256,
    /// Transaction version supplied to Chronicle opcodes
    pub tx_version: i32,
}

impl Checker for ZVersionChecker {
    fn check_sig(
        &mut self,
        sig: &[u8],
        pubkey: &[u8],
        _script: &[u8],
    ) -> Result<bool, ChainGangError> {
        // An empty signature fails the check rather than the script, as in the
        // node: "a compact way to provide an invalid signature for use with
        // CHECK(MULTI)SIG", passed by its encoding checks and simply false. It
        // is also the one failing signature NULLFAIL allows, which the
        // interpreter already exempts (#210).
        if sig.is_empty() {
            return Ok(false);
        }
        let sighash_type = sig[sig.len() - 1];
        if !is_defined_sighash_type(sighash_type) {
            return Err(ChainGangError::ScriptError(
                UNDEFINED_SIGHASH_TYPE.to_string(),
            ));
        }
        if sighash_type & SIGHASH_FORKID == 0 {
            return Err(ChainGangError::ScriptError(
                "SIGHASH_FORKID not present".to_string(),
            ));
        }

        let sig_hash = self.z;
        let der_sig = &sig[0..sig.len() - 1];
        let signature = Signature::from_der(der_sig)?;

        let message = sig_hash.0;

        let verifying_key = VerifyingKey::from_sec1_bytes(pubkey)?;

        Ok(verifying_key.verify_prehash(&message, &signature).is_ok())
    }

    fn check_locktime(&self, _locktime: i64) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }

    fn check_sequence(&self, _sequence: i64) -> Result<bool, ChainGangError> {
        Err(ChainGangError::IllegalState(
            "Illegal transaction check".to_string(),
        ))
    }

    fn tx_version(&self) -> Result<i32, ChainGangError> {
        Ok(self.tx_version)
    }
}

/// Checks that external values in a script are correct for a specific transaction spend
pub struct TransactionChecker<'a> {
    /// Spending transaction
    pub tx: &'a Tx,
    /// Cache for intermediate sighash values
    pub sig_hash_cache: &'a mut SigHashCache,
    /// Spending input for the script
    pub input: usize,
    /// Amount of satoshis being spent
    pub satoshis: i64,
    /// True if the signature must have SIGHASH_FORKID present, false if not
    pub require_sighash_forkid: bool,
    /// Override transaction version for Chronicle script rules (activation height gating).
    pub script_tx_version: Option<u32>,
}

impl<'a> TransactionChecker<'a> {
    fn chronicle_script_version(&self) -> u32 {
        self.script_tx_version.unwrap_or(self.tx.version)
    }
}

impl Checker for TransactionChecker<'_> {
    // Given a signature and public key, check signature matches signed script hash
    fn check_sig(
        &mut self,
        sig: &[u8],
        pubkey: &[u8],
        script: &[u8],
    ) -> Result<bool, ChainGangError> {
        // An empty signature fails the check rather than the script, as in the
        // node: "a compact way to provide an invalid signature for use with
        // CHECK(MULTI)SIG", passed by its encoding checks and simply false. It
        // is also the one failing signature NULLFAIL allows, which the
        // interpreter already exempts (#210).
        if sig.is_empty() {
            return Ok(false);
        }
        let sighash_type = sig[sig.len() - 1];
        // STRICTENC came with FORKID; before it, transactions with undefined
        // types were valid and old blocks still hold some.
        if self.require_sighash_forkid && !is_defined_sighash_type(sighash_type) {
            return Err(ChainGangError::ScriptError(
                UNDEFINED_SIGHASH_TYPE.to_string(),
            ));
        }
        if self.require_sighash_forkid && sighash_type & SIGHASH_FORKID == 0 {
            return Err(ChainGangError::ScriptError(
                "SIGHASH_FORKID not present".to_string(),
            ));
        }
        // `script` is the script code the interpreter built for this check —
        // cut at the last executed OP_CODESEPARATOR, signature removed — so it
        // is hashed as given. `sighash` would cut it a second time.
        let sig_hash = sighash_from_script_code(
            self.tx,
            self.input,
            script,
            self.satoshis,
            sighash_type,
            self.sig_hash_cache,
        )?;
        let der_sig = &sig[0..sig.len() - 1];

        let mut signature = Signature::from_der(der_sig)?;
        if self.chronicle_script_version() > 1 {
            // Chronicle lifts the low-S rule; normalize so k256 accepts high-S encodings.
            signature = signature.normalize_s();
        }
        let message = sig_hash.0;
        let verifying_key: VerifyingKey = VerifyingKey::from_sec1_bytes(pubkey)?;
        Ok(verifying_key.verify_prehash(&message, &signature).is_ok())
    }

    fn tx_version(&self) -> Result<i32, ChainGangError> {
        Ok(self.chronicle_script_version() as i32)
    }

    fn check_locktime(&self, locktime: i64) -> Result<bool, ChainGangError> {
        if locktime < 0 {
            return Err(ChainGangError::ScriptError("locktime negative".to_string()));
        }
        // Widened, not cast: `lock_time as i32` made every lock time from
        // 2038 on negative, so a transaction locked past then could never
        // satisfy a CHECKLOCKTIMEVERIFY.
        let tx_lock_time = i64::from(self.tx.lock_time);
        if (locktime >= LOCKTIME_THRESHOLD && tx_lock_time < LOCKTIME_THRESHOLD)
            || (locktime < LOCKTIME_THRESHOLD && tx_lock_time >= LOCKTIME_THRESHOLD)
        {
            return Err(ChainGangError::ScriptError(
                "locktime types different".to_string(),
            ));
        }
        if locktime > tx_lock_time {
            return Err(ChainGangError::ScriptError(
                "locktime greater than tx".to_string(),
            ));
        }
        if self.tx.inputs[self.input].sequence == 0xffffffff {
            return Err(ChainGangError::ScriptError(
                "sequence is 0xffffffff".to_string(),
            ));
        }
        Ok(true)
    }

    fn check_sequence(&self, sequence: i64) -> Result<bool, ChainGangError> {
        if sequence < 0 {
            return Err(ChainGangError::ScriptError("sequence negative".to_string()));
        }
        // Only the low 32 bits mean anything: like the transaction's sequence
        // it is a u32, and the node masks the bits it compares out of it.
        let sequence = (sequence & 0xffff_ffff) as u32;
        if sequence & SEQUENCE_LOCKTIME_DISABLE_FLAG != 0 {
            return Ok(true);
        }
        if self.tx.version < 2 {
            return Err(ChainGangError::ScriptError(
                "tx version less than 2".to_string(),
            ));
        }
        if self.tx.inputs[self.input].sequence & SEQUENCE_LOCKTIME_DISABLE_FLAG != 0 {
            let msg = "tx sequence disable flag set".to_string();
            return Err(ChainGangError::ScriptError(msg));
        }
        // The type flag stays in, as in the node's `nLockTimeMask`. Masking it
        // off along with the other unused bits left the type check below
        // comparing two values under 2^16 against 2^22, so it never fired: a
        // time-based CSV was met by a height-based input, and the reverse.
        let mask = SEQUENCE_LOCKTIME_TYPE_FLAG | SEQUENCE_LOCKTIME_MASK;
        let sequence_masked = sequence & mask;
        let tx_sequence_masked = self.tx.inputs[self.input].sequence & mask;
        if (sequence_masked < SEQUENCE_LOCKTIME_TYPE_FLAG
            && tx_sequence_masked >= SEQUENCE_LOCKTIME_TYPE_FLAG)
            || (sequence_masked >= SEQUENCE_LOCKTIME_TYPE_FLAG
                && tx_sequence_masked < SEQUENCE_LOCKTIME_TYPE_FLAG)
        {
            let msg = "sequence types different".to_string();
            return Err(ChainGangError::ScriptError(msg));
        }
        if sequence_masked > tx_sequence_masked {
            let msg = "sequence greater than tx".to_string();
            return Err(ChainGangError::ScriptError(msg));
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{OutPoint, TxIn, TxOut};
    use crate::script::op_codes::*;
    use crate::script::{Script, NO_FLAGS};
    use crate::transaction::generate_signature;
    use crate::transaction::sighash::sighash;
    use crate::transaction::sighash::{SIGHASH_ALL, SIGHASH_FORKID};
    use crate::util::hash160;
    use k256::ecdsa::signature::hazmat::PrehashSigner;
    use k256::ecdsa::signature::SignatureEncoding;
    use k256::ecdsa::{SigningKey, VerifyingKey};

    #[test]
    fn standard_p2pkh() {
        standard_p2pkh_test(SIGHASH_ALL);
        standard_p2pkh_test(SIGHASH_ALL | SIGHASH_FORKID);
    }

    #[test]
    fn chronicle_high_s_signature_verifies() {
        use crate::transaction::sighash::SIGHASH_CHRONICLE;
        use k256::ecdsa::Signature;

        let private_key = [2; 32];
        let secret_key = SigningKey::from_slice(&private_key).unwrap();
        let verifying_key = secret_key.verifying_key();
        let pk = verifying_key_as_bytes(verifying_key);
        let pkh = hash160(&pk);
        let lock_script = crate::transaction::p2pkh::create_lock_script(&pkh);

        let tx_1 = Tx {
            version: 2,
            inputs: vec![],
            outputs: vec![TxOut {
                satoshis: 10,
                lock_script,
            }],
            lock_time: 0,
        };

        let mut tx_2 = Tx {
            version: 2,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: tx_1.hash(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0xffffffff,
            }],
            outputs: vec![],
            lock_time: 0,
        };

        let sighash_type = SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE;
        let mut cache = SigHashCache::new();
        let lock_script = &tx_1.outputs[0].lock_script.0;
        let sig_hash = sighash(&tx_2, 0, lock_script, 10, sighash_type, &mut cache).unwrap();

        let low_sig: Signature = secret_key.sign_prehash(&sig_hash.0).unwrap();
        let high_sig = crate::test_util::flip_to_high_s(&low_sig);

        let mut sig = high_sig.to_der().to_vec();
        sig.push(sighash_type);

        let mut unlock_script = Script::new();
        unlock_script.append_data(&sig);
        unlock_script.append_data(&pk);
        tx_2.inputs[0].unlock_script = unlock_script;

        let mut cache = SigHashCache::new();
        let mut c = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 10,
            require_sighash_forkid: true,
            script_tx_version: None,
        };

        let mut script = Script::new();
        script.append_slice(&tx_2.inputs[0].unlock_script.0);
        script.append(OP_CODESEPARATOR);
        script.append_slice(&tx_1.outputs[0].lock_script.0);
        assert!(script.eval(&mut c, NO_FLAGS).is_ok());
    }

    /// Signing and verifying still agree on a script with OP_CODESEPARATORs.
    ///
    /// The lock script has three OP_CHECKSIGs with a separator between each
    /// pair. Each is signed through `sighash_checksig_index` with the whole lock
    /// script and its index, the way a wallet signs; the interpreter then
    /// verifies them, handing `check_sig` the script it has already cut at the
    /// executed separator. Those are two different routes to the BIP-143 script
    /// code, and #193 changed what both produce, so this checks they still meet.
    ///
    /// This is a consistency check, not a correctness one: before #193 both
    /// routes deleted the separators, so they agreed with each other while
    /// disagreeing with the node. Correctness against the node is
    /// `bip143_keeps_separators_after_the_executed_one` in `sighash.rs`.
    #[test]
    fn codeseparator_script_signs_and_verifies_under_bip143() {
        use crate::transaction::sighash::sighash_checksig_index;

        let keys: Vec<[u8; 32]> = vec![[11; 32], [12; 32], [13; 32]];
        let pubkeys: Vec<[u8; 33]> = keys
            .iter()
            .map(|k| verifying_key_as_bytes(SigningKey::from_slice(k).unwrap().verifying_key()))
            .collect();

        let mut lock_script = Script::new();
        for (i, pk) in pubkeys.iter().enumerate() {
            lock_script.append_data(pk);
            lock_script.append(OP_CHECKSIG);
            if i < pubkeys.len() - 1 {
                lock_script.append(OP_VERIFY);
                lock_script.append(OP_CODESEPARATOR);
            }
        }

        let tx_1 = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOut {
                satoshis: 10,
                lock_script: lock_script.clone(),
            }],
            lock_time: 0,
        };
        let mut tx_2 = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: tx_1.hash(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 9,
                lock_script: Script(vec![OP_TRUE]),
            }],
            lock_time: 0,
        };

        let sighash_type = SIGHASH_ALL | SIGHASH_FORKID;
        let sigs: Vec<Vec<u8>> = keys
            .iter()
            .enumerate()
            .map(|(checksig_index, key)| {
                let mut cache = SigHashCache::new();
                let hash = sighash_checksig_index(
                    &tx_2,
                    0,
                    &lock_script.0,
                    checksig_index,
                    10,
                    sighash_type,
                    &mut cache,
                )
                .unwrap();
                generate_signature(key, &hash, sighash_type).unwrap()
            })
            .collect();

        // The first OP_CHECKSIG consumes the top of the stack, so its
        // signature is pushed last.
        let mut unlock_script = Script::new();
        for sig in sigs.iter().rev() {
            unlock_script.append_data(sig);
        }
        tx_2.inputs[0].unlock_script = unlock_script.clone();

        let mut cache = SigHashCache::new();
        let mut checker = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 10,
            require_sighash_forkid: true,
            script_tx_version: None,
        };
        let mut script = Script::new();
        script.append_slice(&unlock_script.0);
        script.append(OP_CODESEPARATOR);
        script.append_slice(&lock_script.0);
        script.eval(&mut checker, NO_FLAGS).unwrap();
    }

    /// Funds `lock_script`, spends it with `unlock(&spending_tx)` and runs the
    /// spend through the interpreter the way `Tx::validate` does before
    /// Chronicle: unlocking script, a separator, locking script.
    fn eval_spend(
        lock_script: &Script,
        unlock: impl Fn(&Tx) -> Script,
    ) -> Result<(), ChainGangError> {
        let tx_1 = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOut {
                satoshis: 10,
                lock_script: lock_script.clone(),
            }],
            lock_time: 0,
        };
        let mut tx_2 = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: tx_1.hash(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 9,
                lock_script: Script(vec![OP_TRUE]),
            }],
            lock_time: 0,
        };
        tx_2.inputs[0].unlock_script = unlock(&tx_2);

        let mut cache = SigHashCache::new();
        let mut checker = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 10,
            require_sighash_forkid: true,
            script_tx_version: None,
        };
        let mut script = Script::new();
        script.append_slice(&tx_2.inputs[0].unlock_script.0);
        script.append(OP_CODESEPARATOR);
        script.append_slice(&lock_script.0);
        script.eval(&mut checker, NO_FLAGS)
    }

    /// Signs `script_code` — the script code the node would hand its digest
    /// function for this check — for input 0 of `tx`.
    fn sign_script_code(tx: &Tx, key: &[u8; 32], script_code: &[u8]) -> Vec<u8> {
        let sighash_type = SIGHASH_ALL | SIGHASH_FORKID;
        let mut cache = SigHashCache::new();
        let hash =
            sighash_from_script_code(tx, 0, script_code, 10, sighash_type, &mut cache).unwrap();
        generate_signature(key, &hash, sighash_type).unwrap()
    }

    /// A separator after an OP_CHECKSIGVERIFY, then an OP_CHECKMULTISIG:
    ///
    /// ```text
    /// <pkA> CHECKSIGVERIFY CODESEPARATOR 1 <pkB> 1 CHECKMULTISIG
    /// ```
    ///
    /// When CHECKSIGVERIFY runs the separator has not executed, so the node
    /// signs the whole script, separator included. chain-gang's verifier used
    /// to send that through `sighash`, which cuts at a separator by counting
    /// OP_CHECKSIGs — and there are none, only CHECKSIGVERIFY and
    /// CHECKMULTISIG — so verification failed with "checksig_index 0 exceeds
    /// the number of OP_CHECKSIGs (0)" on a spend the node accepts.
    #[test]
    fn separator_with_checksigverify_and_multisig_verifies() {
        let (key_a, key_b) = ([21; 32], [22; 32]);
        let pk_a = verifying_key_as_bytes(SigningKey::from_slice(&key_a).unwrap().verifying_key());
        let pk_b = verifying_key_as_bytes(SigningKey::from_slice(&key_b).unwrap().verifying_key());

        let mut multisig = Script::new();
        multisig.append(OP_1);
        multisig.append_data(&pk_b);
        multisig.append(OP_1);
        multisig.append(OP_CHECKMULTISIG);

        let mut lock_script = Script::new();
        lock_script.append_data(&pk_a);
        lock_script.append(OP_CHECKSIGVERIFY);
        lock_script.append(OP_CODESEPARATOR);
        lock_script.append_slice(&multisig.0);

        eval_spend(&lock_script, |tx| {
            // CHECKSIGVERIFY signs everything; CHECKMULTISIG signs what follows
            // the separator, which by then has executed.
            let sig_a = sign_script_code(tx, &key_a, &lock_script.0);
            let sig_b = sign_script_code(tx, &key_b, &multisig.0);
            let mut unlock = Script::new();
            unlock.append(OP_0);
            unlock.append_data(&sig_b);
            unlock.append_data(&sig_a);
            unlock
        })
        .unwrap();
    }

    /// Separators in a branch that does not execute do not move where the
    /// script code starts:
    ///
    /// ```text
    /// 0 IF CODESEPARATOR CODESEPARATOR ENDIF <pk> CHECKSIG
    /// ```
    ///
    /// Neither separator runs, so the node signs the whole script. Cutting it
    /// again by position, as the verifier used to, started after the second
    /// separator — it cannot know which ones executed — and the check failed.
    #[test]
    fn unexecuted_separators_do_not_move_the_script_code() {
        let key = [23; 32];
        let pk = verifying_key_as_bytes(SigningKey::from_slice(&key).unwrap().verifying_key());

        let mut lock_script = Script::new();
        lock_script.append(OP_0);
        lock_script.append(OP_IF);
        lock_script.append(OP_CODESEPARATOR);
        lock_script.append(OP_CODESEPARATOR);
        lock_script.append(OP_ENDIF);
        lock_script.append_data(&pk);
        lock_script.append(OP_CHECKSIG);

        eval_spend(&lock_script, |tx| {
            let mut unlock = Script::new();
            unlock.append_data(&sign_script_code(tx, &key, &lock_script.0));
            unlock
        })
        .unwrap();
    }

    fn verifying_key_as_bytes(verifying_key: &VerifyingKey) -> [u8; 33] {
        let vk_bytes = verifying_key.to_sec1_bytes();
        let vk_vec = vk_bytes.to_vec();
        assert!(vk_vec.len() == 33);
        vk_vec[..].try_into().unwrap()
    }
    /// Spends `lock_script` with `unlock_script` through a TransactionChecker,
    /// with FORKID required and malleability rules enforced (version 1).
    fn eval_spend_with(lock_script: &Script, unlock_script: &[u8]) -> Result<(), ChainGangError> {
        let tx_1 = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOut {
                satoshis: 10,
                lock_script: lock_script.clone(),
            }],
            lock_time: 0,
        };
        let tx_2 = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: tx_1.hash(),
                    index: 0,
                },
                unlock_script: Script(unlock_script.to_vec()),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 9,
                lock_script: Script(vec![OP_TRUE]),
            }],
            lock_time: 0,
        };
        let mut cache = SigHashCache::new();
        let mut checker = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 10,
            require_sighash_forkid: true,
            script_tx_version: None,
        };
        let mut script = Script::new();
        script.append_slice(unlock_script);
        script.append(OP_CODESEPARATOR);
        script.append_slice(&lock_script.0);
        script.eval(&mut checker, NO_FLAGS)
    }

    fn empty_sig_test_key() -> [u8; 33] {
        verifying_key_as_bytes(SigningKey::from_slice(&[41; 32]).unwrap().verifying_key())
    }

    fn assert_fails_with(result: Result<(), ChainGangError>, reason: &str) {
        match result {
            Err(e) if e.to_string().contains(reason) => {}
            other => panic!("expected {reason:?}, got {other:?}"),
        }
    }

    /// An empty signature makes CHECKSIG push false; it does not abort the
    /// script (#210). A bitcoin-sv node accepted and mined exactly this spend,
    /// and its script_tests.json expects `0 | <pk> CHECKSIG NOT` to pass,
    /// NULLFAIL included.
    #[test]
    fn empty_signature_fails_checksig_without_aborting() {
        let mut lock = Script::new();
        lock.append_data(&empty_sig_test_key());
        lock.append(OP_CHECKSIG);
        let mut negated = lock.clone();
        negated.append(OP_NOT);

        eval_spend_with(&negated, &[OP_0]).unwrap();
        // Un-negated, the spend fails because the check was false, not
        // because the signature was empty.
        assert_fails_with(eval_spend_with(&lock, &[OP_0]), "Top of stack is false");
    }

    /// The same for CHECKMULTISIG: an empty signature matches no key.
    #[test]
    fn empty_signature_fails_checkmultisig_without_aborting() {
        let mut lock = Script::new();
        lock.append(OP_1);
        lock.append_data(&empty_sig_test_key());
        lock.append(OP_1);
        lock.append(OP_CHECKMULTISIG);
        lock.append(OP_NOT);
        eval_spend_with(&lock, &[OP_0, OP_0]).unwrap();
    }

    /// CHECKSIGVERIFY with an empty signature fails as a failed check.
    #[test]
    fn empty_signature_fails_checksigverify() {
        let mut lock = Script::new();
        lock.append_data(&empty_sig_test_key());
        lock.append(OP_CHECKSIGVERIFY);
        lock.append(OP_1);
        assert_fails_with(eval_spend_with(&lock, &[OP_0]), "OP_CHECKSIGVERIFY failed");
    }

    /// An empty signature can steer a script down its failed-check branch.
    #[test]
    fn empty_signature_takes_the_failed_check_branch() {
        let mut lock = Script::new();
        lock.append_data(&empty_sig_test_key());
        lock.append(OP_CHECKSIG);
        lock.append(OP_IF);
        lock.append(OP_0);
        lock.append(OP_ELSE);
        lock.append(OP_1);
        lock.append(OP_ENDIF);
        eval_spend_with(&lock, &[OP_0]).unwrap();
    }

    /// Only the empty signature is exempt: a non-empty one that fails still
    /// breaks NULLFAIL, as in the node.
    #[test]
    fn nullfail_still_rejects_a_non_empty_invalid_signature() {
        let mut lock = Script::new();
        lock.append_data(&empty_sig_test_key());
        lock.append(OP_CHECKSIG);
        lock.append(OP_NOT);
        let sighash_type = SIGHASH_ALL | SIGHASH_FORKID;
        let wrong = generate_signature(&[42; 32], &Hash256([9; 32]), sighash_type).unwrap();
        let mut unlock = Script::new();
        unlock.append_data(&wrong);
        assert_fails_with(eval_spend_with(&lock, &unlock.0), "NULLFAIL");
    }

    /// The Z checkers treat an empty signature the same way.
    #[test]
    fn z_checkers_return_false_for_an_empty_signature() {
        let pk = empty_sig_test_key();
        let mut z = ZChecker {
            z: Hash256([0; 32]),
        };
        assert!(!z.check_sig(&[], &pk, &[]).unwrap());
        let mut zv = ZVersionChecker {
            z: Hash256([0; 32]),
            tx_version: 1,
        };
        assert!(!zv.check_sig(&[], &pk, &[]).unwrap());
    }

    /// Spends a P2PKH output with a signature made under `sighash_type`, and
    /// returns the interpreter's verdict through a TransactionChecker.
    fn p2pkh_spend_with(
        sighash_type: u8,
        require_sighash_forkid: bool,
    ) -> Result<(), ChainGangError> {
        let private_key = [3; 32];
        let pk = verifying_key_as_bytes(
            SigningKey::from_slice(&private_key)
                .unwrap()
                .verifying_key(),
        );
        let lock_script = crate::transaction::p2pkh::create_lock_script(&hash160(&pk));
        let tx_1 = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOut {
                satoshis: 10,
                lock_script: lock_script.clone(),
            }],
            lock_time: 0,
        };
        let mut tx_2 = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: tx_1.hash(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 9,
                lock_script: lock_script.clone(),
            }],
            lock_time: 0,
        };
        let hash = sighash(
            &tx_2,
            0,
            &lock_script.0,
            10,
            sighash_type,
            &mut SigHashCache::new(),
        )
        .unwrap();
        let sig = generate_signature(&private_key, &hash, sighash_type).unwrap();
        let mut unlock = Script::new();
        unlock.append_data(&sig);
        unlock.append_data(&pk);
        tx_2.inputs[0].unlock_script = unlock.clone();

        let mut cache = SigHashCache::new();
        let mut checker = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 10,
            require_sighash_forkid,
            script_tx_version: None,
        };
        let mut script = Script::new();
        script.append_slice(&unlock.0);
        script.append(OP_CODESEPARATOR);
        script.append_slice(&lock_script.0);
        script.eval(&mut checker, NO_FLAGS)
    }

    /// A signature must carry a defined sighash type wherever FORKID is
    /// required: with the CHRONICLE, FORKID and ANYONECANPAY bits cleared, what
    /// remains must be ALL, NONE or SINGLE (#208).
    ///
    /// A live bitcoin-sv node rejected 0x51 and 0x40 as mandatory failures
    /// ("Signature hash type missing or not understood"), and chain-gang
    /// accepted both. Every signature here is genuine: only the type is wrong.
    #[test]
    fn undefined_sighash_types_are_rejected() {
        for sighash_type in [0x40, 0x44, 0x48, 0x50, 0x51, 0x5f, 0xc0, 0xc4] {
            match p2pkh_spend_with(sighash_type, true) {
                Err(e) if e.to_string().contains(UNDEFINED_SIGHASH_TYPE) => {}
                other => panic!("{sighash_type:#04x} should be undefined, got {other:?}"),
            }
        }
    }

    /// Every combination the node defines still verifies: each base type, with
    /// and without ANYONECANPAY, with FORKID, and with CHRONICLE on top.
    #[test]
    fn defined_sighash_types_still_verify() {
        for base in [SIGHASH_ALL, 0x02, 0x03] {
            for flags in [
                SIGHASH_FORKID,
                SIGHASH_FORKID | SIGHASH_ANYONECANPAY,
                SIGHASH_FORKID | SIGHASH_CHRONICLE,
                SIGHASH_FORKID | SIGHASH_CHRONICLE | SIGHASH_ANYONECANPAY,
            ] {
                let sighash_type = base | flags;
                p2pkh_spend_with(sighash_type, true)
                    .unwrap_or_else(|e| panic!("{sighash_type:#04x} is defined: {e}"));
            }
        }
    }

    /// Before the 2017 fork neither FORKID nor STRICTENC was a rule, and old
    /// blocks hold signatures with undefined types; bitcoin-sv's own
    /// script_tests.json expects "P2PKH with invalid sighashtype" (0x11) to
    /// pass without STRICTENC. Where FORKID is not required, they still verify.
    #[test]
    fn undefined_sighash_types_verify_before_the_fork() {
        p2pkh_spend_with(0x11, false).unwrap();
    }

    /// The Z checkers always require FORKID, so they always apply the rule.
    #[test]
    fn z_checker_rejects_undefined_sighash_types() {
        let mut checker = ZChecker {
            z: Hash256([0; 32]),
        };
        let mut sig = vec![0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x01];
        sig.push(0x51);
        let err = checker.check_sig(&sig, &[2; 33], &[]).unwrap_err();
        assert!(err.to_string().contains(UNDEFINED_SIGHASH_TYPE), "{err}");
    }

    fn standard_p2pkh_test(sighash_type: u8) {
        //let secp = Secp256k1::new();
        let private_key = [1; 32];
        let secret_key = SigningKey::from_slice(&private_key).unwrap();
        let verifying_key = secret_key.verifying_key();
        let pk = verifying_key_as_bytes(verifying_key);

        let pkh = hash160(&pk);
        let lock_script = crate::transaction::p2pkh::create_lock_script(&pkh);

        let tx_1 = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOut {
                satoshis: 10,
                lock_script,
            }],
            lock_time: 0,
        };

        let mut tx_2 = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: tx_1.hash(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0xffffffff,
            }],
            outputs: vec![],
            lock_time: 0,
        };

        let mut cache = SigHashCache::new();
        let lock_script = &tx_1.outputs[0].lock_script.0;
        let sig_hash = sighash(&tx_2, 0, lock_script, 10, sighash_type, &mut cache).unwrap();
        let sig = generate_signature(&private_key, &sig_hash, sighash_type).unwrap();

        let mut unlock_script = Script::new();
        unlock_script.append_data(&sig);
        unlock_script.append_data(&pk);
        tx_2.inputs[0].unlock_script = unlock_script;

        let mut cache = SigHashCache::new();
        let mut c = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 10,
            require_sighash_forkid: false,
            script_tx_version: None,
        };

        let mut script = Script::new();
        script.append_slice(&tx_2.inputs[0].unlock_script.0);
        script.append(OP_CODESEPARATOR);
        script.append_slice(&tx_1.outputs[0].lock_script.0);
        assert!(script.eval(&mut c, NO_FLAGS).is_ok());
    }

    #[test]
    fn multisig() {
        multisig_test(SIGHASH_ALL);
        multisig_test(SIGHASH_ALL | SIGHASH_FORKID);
    }

    fn multisig_test(sighash_type: u8) {
        //let secp = Secp256k1::new();
        let private_key1 = [1; 32];
        let private_key2 = [2; 32];
        let private_key3 = [3; 32];

        let secret_key1 = SigningKey::from_slice(&private_key1).unwrap();
        let secret_key2 = SigningKey::from_slice(&private_key2).unwrap();
        let secret_key3 = SigningKey::from_slice(&private_key3).unwrap();

        //let secret_key1 = SecretKey::from_slice(&private_key1).unwrap();
        //let secret_key2 = SecretKey::from_slice(&private_key2).unwrap();
        // let secret_key3 = SecretKey::from_slice(&private_key3).unwrap();

        //let verifying_key = secret_key.verifying_key();
        //let pk = verifying_key_as_bytes(verifying_key);

        //let pk1 = PublicKey::from_secret_key(&secp, &secret_key1).serialize();

        let verifying_key1 = secret_key1.verifying_key();
        let pk1 = verifying_key_as_bytes(verifying_key1);

        //let pk2 = PublicKey::from_secret_key(&secp, &secret_key2).serialize();

        let verifying_key2 = secret_key2.verifying_key();
        let pk2 = verifying_key_as_bytes(verifying_key2);

        //let pk3 = PublicKey::from_secret_key(&secp, &secret_key3).serialize();
        let verifying_key3 = secret_key3.verifying_key();
        let pk3 = verifying_key_as_bytes(verifying_key3);

        let mut lock_script = Script::new();
        lock_script.append(OP_2);
        lock_script.append_data(&pk1);
        lock_script.append_data(&pk2);
        lock_script.append_data(&pk3);
        lock_script.append(OP_3);
        lock_script.append(OP_CHECKMULTISIG);

        let tx_1 = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOut {
                satoshis: 10,
                lock_script,
            }],
            lock_time: 0,
        };

        let mut tx_2 = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: tx_1.hash(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0xffffffff,
            }],
            outputs: vec![],
            lock_time: 0,
        };

        let mut cache = SigHashCache::new();
        let lock_script = &tx_1.outputs[0].lock_script.0;
        let sig_hash = sighash(&tx_2, 0, lock_script, 10, sighash_type, &mut cache).unwrap();
        let sig1 = generate_signature(&private_key1, &sig_hash, sighash_type).unwrap();
        let sig3 = generate_signature(&private_key3, &sig_hash, sighash_type).unwrap();

        let mut unlock_script = Script::new();
        unlock_script.append(OP_0);
        unlock_script.append_data(&sig1);
        unlock_script.append_data(&sig3);
        tx_2.inputs[0].unlock_script = unlock_script;

        let mut cache = SigHashCache::new();
        let mut c = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 10,
            require_sighash_forkid: false,
            script_tx_version: None,
        };

        let mut script = Script::new();
        script.append_slice(&tx_2.inputs[0].unlock_script.0);
        script.append(OP_CODESEPARATOR);
        script.append_slice(&tx_1.outputs[0].lock_script.0);
        assert!(script.eval(&mut c, NO_FLAGS).is_ok());
    }

    /*
    #[test]
    fn blank_check() {
        blank_check_test(SIGHASH_NONE | SIGHASH_ANYONECANPAY);
        blank_check_test(SIGHASH_NONE | SIGHASH_ANYONECANPAY | SIGHASH_FORKID);
    }


    fn blank_check_test(sighash_type: u8) {
        //let secp = Secp256k1::new();

        let private_key1 = [1; 32];
        let secret_key1 = SecretKey::from_slice(&private_key1).unwrap();
        let pk1 = PublicKey::from_secret_key(&secp, &secret_key1).serialize();
        let pkh1 = hash160(&pk1);

        let private_key2 = [2; 32];
        let secret_key2 = SecretKey::from_slice(&private_key2).unwrap();
        let pk2 = PublicKey::from_secret_key(&secp, &secret_key2).serialize();
        let pkh2 = hash160(&pk2);

        let mut lock_script1 = Script::new();
        lock_script1.append(OP_DUP);
        lock_script1.append(OP_HASH160);
        lock_script1.append_data(&pkh1.0);
        lock_script1.append(OP_EQUALVERIFY);
        lock_script1.append(OP_CHECKSIG);

        let mut lock_script2 = Script::new();
        lock_script2.append(OP_DUP);
        lock_script2.append(OP_HASH160);
        lock_script2.append_data(&pkh2.0);
        lock_script2.append(OP_EQUALVERIFY);
        lock_script2.append(OP_CHECKSIG);

        let tx_1 = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![
                TxOut {
                    satoshis: 10,
                    lock_script: lock_script1,
                },
                TxOut {
                    satoshis: 20,
                    lock_script: lock_script2,
                },
            ],
            lock_time: 0,
        };

        let mut tx_2 = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: tx_1.hash(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0xffffffff,
            }],
            outputs: vec![],
            lock_time: 0,
        };

        // Sign the first input

        let mut cache = SigHashCache::new();
        let lock_script = &tx_1.outputs[0].lock_script.0;
        let sig_hash1 = sighash(&tx_2, 0, lock_script, 10, sighash_type, &mut cache).unwrap();
        let sig1 = generate_signature(&private_key1, &sig_hash1, sighash_type).unwrap();

        let mut unlock_script1 = Script::new();
        unlock_script1.append_data(&sig1);
        unlock_script1.append_data(&pk1);
        tx_2.inputs[0].unlock_script = unlock_script1;

        // Add another input and sign that separately

        tx_2.inputs.push(TxIn {
            prev_output: OutPoint {
                hash: tx_1.hash(),
                index: 1,
            },
            unlock_script: Script(vec![]),
            sequence: 0xffffffff,
        });

        let mut cache = SigHashCache::new();
        let lock_script = &tx_1.outputs[1].lock_script.0;

        let sig_hash2 = sighash(&tx_2, 1, lock_script, 20, sighash_type, &mut cache).unwrap();
        let sig2 = generate_signature(&private_key2, &sig_hash2, sighash_type).unwrap();

        let mut unlock_script2 = Script::new();
        unlock_script2.append_data(&sig2);
        unlock_script2.append_data(&pk2);
        tx_2.inputs[1].unlock_script = unlock_script2;

        let mut cache = SigHashCache::new();
        let mut c1 = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 10,
            require_sighash_forkid: false,
            script_tx_version: None,
        };

        let mut script1 = Script::new();
        script1.append_slice(&tx_2.inputs[0].unlock_script.0);
        script1.append(OP_CODESEPARATOR);
        script1.append_slice(&tx_1.outputs[0].lock_script.0);
        assert!(script1.eval(&mut c1, NO_FLAGS).is_ok());

        let mut cache = SigHashCache::new();
        let mut c2 = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 1,
            satoshis: 20,
            require_sighash_forkid: false,
            script_tx_version: None,
        };

        let mut script2 = Script::new();
        script2.append_slice(&tx_2.inputs[1].unlock_script.0);
        script2.append(OP_CODESEPARATOR);
        script2.append_slice(&tx_1.outputs[1].lock_script.0);
        assert!(script2.eval(&mut c2, NO_FLAGS).is_ok());
    }

    #[test]
    fn batch() {
        batch_test(SIGHASH_SINGLE | SIGHASH_ANYONECANPAY);
        batch_test(SIGHASH_SINGLE | SIGHASH_ANYONECANPAY | SIGHASH_FORKID);
    }

    fn batch_test(sighash_type: u8) {
        let secp = Secp256k1::new();

        let private_key1 = [1; 32];
        let secret_key1 = SecretKey::from_slice(&private_key1).unwrap();
        let pk1 = PublicKey::from_secret_key(&secp, &secret_key1).serialize();
        let pkh1 = hash160(&pk1);

        let private_key2 = [2; 32];
        let secret_key2 = SecretKey::from_slice(&private_key2).unwrap();
        let pk2 = PublicKey::from_secret_key(&secp, &secret_key2).serialize();
        let pkh2 = hash160(&pk2);

        let mut lock_script1 = Script::new();
        lock_script1.append(OP_DUP);
        lock_script1.append(OP_HASH160);
        lock_script1.append_data(&pkh1.0);
        lock_script1.append(OP_EQUALVERIFY);
        lock_script1.append(OP_CHECKSIG);

        let mut lock_script2 = Script::new();
        lock_script2.append(OP_DUP);
        lock_script2.append(OP_HASH160);
        lock_script2.append_data(&pkh2.0);
        lock_script2.append(OP_EQUALVERIFY);
        lock_script2.append(OP_CHECKSIG);

        let tx_1 = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![
                TxOut {
                    satoshis: 10,
                    lock_script: lock_script1.clone(),
                },
                TxOut {
                    satoshis: 20,
                    lock_script: lock_script2.clone(),
                },
            ],
            lock_time: 0,
        };

        let mut tx_2 = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: tx_1.hash(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 10,
                lock_script: lock_script1.clone(),
            }],
            lock_time: 0,
        };

        // Sign the first input and output

        let mut cache = SigHashCache::new();
        let lock_script = &tx_1.outputs[0].lock_script.0;
        let sig_hash1 = sighash(&tx_2, 0, lock_script, 10, sighash_type, &mut cache).unwrap();
        let sig1 = generate_signature(&private_key1, &sig_hash1, sighash_type).unwrap();

        let mut unlock_script1 = Script::new();
        unlock_script1.append_data(&sig1);
        unlock_script1.append_data(&pk1);
        tx_2.inputs[0].unlock_script = unlock_script1;

        // Add another input and output and sign that separately

        tx_2.inputs.push(TxIn {
            prev_output: OutPoint {
                hash: tx_1.hash(),
                index: 1,
            },
            unlock_script: Script(vec![]),
            sequence: 0xffffffff,
        });
        tx_2.outputs.push(TxOut {
            satoshis: 20,
            lock_script: lock_script2.clone(),
        });

        let mut cache = SigHashCache::new();
        let sig_hash2 = sighash(
            &tx_2,
            1,
            &tx_1.outputs[1].lock_script.0,
            20,
            sighash_type,
            &mut cache,
        )
        .unwrap();
        let sig2 = generate_signature(&private_key2, &sig_hash2, sighash_type).unwrap();

        let mut unlock_script2 = Script::new();
        unlock_script2.append_data(&sig2);
        unlock_script2.append_data(&pk2);
        tx_2.inputs[1].unlock_script = unlock_script2;

        let mut cache = SigHashCache::new();
        let mut c1 = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 10,
            require_sighash_forkid: false,
            script_tx_version: None,
        };

        let mut script1 = Script::new();
        script1.append_slice(&tx_2.inputs[0].unlock_script.0);
        script1.append(OP_CODESEPARATOR);
        script1.append_slice(&tx_1.outputs[0].lock_script.0);
        assert!(script1.eval(&mut c1, NO_FLAGS).is_ok());

        let mut cache = SigHashCache::new();
        let mut c2 = TransactionChecker {
            tx: &tx_2,
            sig_hash_cache: &mut cache,
            input: 1,
            satoshis: 20,
            require_sighash_forkid: false,
            script_tx_version: None,
        };

        let mut script2 = Script::new();
        script2.append_slice(&tx_2.inputs[1].unlock_script.0);
        script2.append(OP_CODESEPARATOR);
        script2.append_slice(&tx_1.outputs[1].lock_script.0);
        assert!(script2.eval(&mut c2, NO_FLAGS).is_ok());
    }

    #[test]
    fn check_locktime() {
        let mut lock_script = Script::new();
        lock_script.append_num(500).unwrap();
        lock_script.append(OP_CHECKLOCKTIMEVERIFY);
        lock_script.append(OP_1);
        let mut tx = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: Hash256([0; 32]),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0,
            }],
            outputs: vec![],
            lock_time: 499,
        };
        {
            let mut cache = SigHashCache::new();
            let mut c = TransactionChecker {
                tx: &tx,
                sig_hash_cache: &mut cache,
                input: 0,
                satoshis: 0,
                require_sighash_forkid: false,
                script_tx_version: None,
            };
            assert!(lock_script.eval(&mut c, PREGENESIS_RULES).is_err());
        }
        {
            tx.lock_time = 500;
            let mut cache = SigHashCache::new();
            let mut c = TransactionChecker {
                tx: &tx,
                sig_hash_cache: &mut cache,
                input: 0,
                satoshis: 0,
                require_sighash_forkid: false,
                script_tx_version: None,
            };
            assert!(lock_script.eval(&mut c, PREGENESIS_RULES).is_ok());
        }
    }

    #[test]
    fn check_sequence() {
        let mut lock_script = Script::new();
        lock_script
            .append_num(500 | SEQUENCE_LOCKTIME_TYPE_FLAG as i32)
            .unwrap();
        lock_script.append(OP_CHECKSEQUENCEVERIFY);
        lock_script.append(OP_1);
        let mut tx = Tx {
            version: 2,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: Hash256([0; 32]),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 499 | SEQUENCE_LOCKTIME_TYPE_FLAG,
            }],
            outputs: vec![],
            lock_time: 0,
        };
        {
            let mut cache = SigHashCache::new();
            let mut c = TransactionChecker {
                tx: &tx,
                sig_hash_cache: &mut cache,
                input: 0,
                satoshis: 0,
                require_sighash_forkid: false,
                script_tx_version: None,
            };
            assert!(lock_script.eval(&mut c, PREGENESIS_RULES).is_err());
        }
        {
            tx.inputs[0].sequence = 500 | SEQUENCE_LOCKTIME_TYPE_FLAG;
            let mut cache = SigHashCache::new();
            let mut c = TransactionChecker {
                tx: &tx,
                sig_hash_cache: &mut cache,
                input: 0,
                satoshis: 0,
                require_sighash_forkid: false,
                script_tx_version: None,
            };
            assert!(lock_script.eval(&mut c, PREGENESIS_RULES).is_ok());
        }
    }
    */

    /// A transaction with one input, `sequence` 0 so it is not final.
    fn locked_tx(lock_time: u32) -> Tx {
        Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: Hash256([0; 32]),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0,
            }],
            outputs: vec![],
            lock_time,
        }
    }

    fn check_locktime_against(lock_time: u32, locktime: i64) -> Result<bool, ChainGangError> {
        let tx = locked_tx(lock_time);
        let mut cache = SigHashCache::new();
        let checker = TransactionChecker {
            tx: &tx,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 0,
            require_sighash_forkid: false,
            script_tx_version: None,
        };
        checker.check_locktime(locktime)
    }

    /// A lock time from 2038 on is past `i32::MAX`. The checker used to cast
    /// the transaction's `lock_time` to an `i32`, which made it negative, so
    /// no CHECKLOCKTIMEVERIFY could be met by a transaction locked past then.
    /// The node compares them as 64-bit numbers.
    #[test]
    fn check_locktime_past_2038() {
        let late = 0xffff_fff0_u32;
        for locktime in [500_000_000, 0x8000_0000, i64::from(late)] {
            assert!(
                check_locktime_against(late, locktime).unwrap(),
                "{locktime}"
            );
        }
        // Later than the transaction, including past u32, which 5 bytes allow.
        for locktime in [i64::from(late) + 1, 1 << 32, (1 << 39) - 1] {
            let err = check_locktime_against(late, locktime).unwrap_err();
            assert!(
                err.to_string().contains("greater than tx"),
                "{locktime}: {err}"
            );
        }
        // A block height against a timestamp.
        let err = check_locktime_against(late, 499_999_999).unwrap_err();
        assert!(err.to_string().contains("types different"), "{err}");
        let err = check_locktime_against(1_000, 0x8000_0000).unwrap_err();
        assert!(err.to_string().contains("types different"), "{err}");
        // Block heights still compare as before.
        assert!(check_locktime_against(1_000, 1_000).unwrap());
        assert!(check_locktime_against(1_000, 1_001).is_err());
    }

    fn check_sequence_against(tx_sequence: u32, sequence: i64) -> Result<bool, ChainGangError> {
        let mut tx = locked_tx(0);
        tx.version = 2;
        tx.inputs[0].sequence = tx_sequence;
        let mut cache = SigHashCache::new();
        let checker = TransactionChecker {
            tx: &tx,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: 0,
            require_sighash_forkid: false,
            script_tx_version: None,
        };
        checker.check_sequence(sequence)
    }

    /// CHECKSEQUENCEVERIFY's operand is up to 5 bytes, so it reaches the
    /// checker as an i64. Bits above the low 32 never count: the node masks
    /// the operand to the bits it compares.
    #[test]
    fn check_sequence_takes_five_byte_operands() {
        assert!(check_sequence_against(10, 10).unwrap());
        assert!(check_sequence_against(10, (1 << 32) + 10).unwrap());
        assert!(check_sequence_against(10, (0x7f << 32) + 10).unwrap());
        let err = check_sequence_against(10, (1 << 32) + 11).unwrap_err();
        assert!(err.to_string().contains("greater than tx"), "{err}");
        let err = check_sequence_against(10, -1).unwrap_err();
        assert!(err.to_string().contains("negative"), "{err}");
    }

    /// A CSV operand and the input's sequence must lock by the same measure:
    /// both by time (type flag set) or both by block height, as in the node.
    /// The flag was masked off before the type check, so it never fired.
    #[test]
    fn check_sequence_requires_matching_lock_types() {
        let time = |value: u32| SEQUENCE_LOCKTIME_TYPE_FLAG | value;
        let check = |tx_sequence: u32, sequence: u32| {
            check_sequence_against(tx_sequence, i64::from(sequence))
        };
        // Same type: the values compare.
        assert!(check(10, 10).unwrap());
        assert!(check(time(10), time(10)).unwrap());
        assert!(check(time(10), time(9)).unwrap());
        let err = check(time(10), time(11)).unwrap_err();
        assert!(err.to_string().contains("greater than tx"), "{err}");
        // Different types fail, whatever the values.
        for (tx_sequence, sequence) in [(time(10), 10), (10, time(10)), (10, time(0))] {
            let err = check(tx_sequence, sequence).unwrap_err();
            assert!(
                err.to_string().contains("types different"),
                "{tx_sequence:#x} against {sequence:#x}: {err}"
            );
        }
        // Bits outside the type flag and the value still do not count.
        assert!(check(time(10) | 0x0001_0000, time(10)).unwrap());
        assert!(check(10, 0x0001_0000 | 10).unwrap());
    }

    /// The same through the interpreter: 2^32 + 10, five bytes, before Genesis.
    #[test]
    fn csv_five_byte_operand_through_the_interpreter() {
        use crate::script::PREGENESIS_RULES;
        let mut script = Script::new();
        script.append_data(&[10, 0x00, 0x00, 0x00, 0x01]);
        script.append(OP_CHECKSEQUENCEVERIFY);
        for (tx_sequence, ok) in [(10, true), (11, true), (9, false)] {
            let mut tx = locked_tx(0);
            tx.version = 2;
            tx.inputs[0].sequence = tx_sequence;
            let mut cache = SigHashCache::new();
            let mut checker = TransactionChecker {
                tx: &tx,
                sig_hash_cache: &mut cache,
                input: 0,
                satoshis: 0,
                require_sighash_forkid: false,
                script_tx_version: None,
            };
            let result = script.eval(&mut checker, PREGENESIS_RULES);
            assert_eq!(result.is_ok(), ok, "sequence {tx_sequence}: {result:?}");
        }
    }

    /// The same through the interpreter: a 5-byte CLTV operand past `i32`,
    /// against a transaction locked as late, before Genesis.
    #[test]
    fn cltv_past_2038_through_the_interpreter() {
        use crate::script::PREGENESIS_RULES;
        let mut script = Script::new();
        script.append_data(&[0x00, 0x00, 0x00, 0x80, 0x00]); // 2^31
        script.append(OP_CHECKLOCKTIMEVERIFY);
        for (lock_time, ok) in [
            (0x8000_0000, true),
            (0xffff_ffff, true),
            (0x7fff_ffff, false),
        ] {
            let tx = locked_tx(lock_time);
            let mut cache = SigHashCache::new();
            let mut checker = TransactionChecker {
                tx: &tx,
                sig_hash_cache: &mut cache,
                input: 0,
                satoshis: 0,
                require_sighash_forkid: false,
                script_tx_version: None,
            };
            let result = script.eval(&mut checker, PREGENESIS_RULES);
            assert_eq!(result.is_ok(), ok, "lock_time {lock_time:#x}: {result:?}");
        }
    }
}
