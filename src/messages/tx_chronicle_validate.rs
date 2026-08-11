//! Integration tests for Chronicle rules through `Tx::validate`.

use super::{OutPoint, Tx, TxIn, TxOut};
use crate::chronicle::CHRONICLE_ACTIVATION_MAINNET;
use crate::network::Network;
use crate::script::op_codes::*;
use crate::script::Script;
use crate::test_util::flip_to_high_s;
use crate::transaction::p2pkh::create_lock_script;
use crate::transaction::sighash::{sighash, SigHashCache, SIGHASH_ALL, SIGHASH_CHRONICLE, SIGHASH_FORKID};
use crate::util::hash160;
use k256::ecdsa::signature::hazmat::PrehashSigner;
use k256::ecdsa::signature::SignatureEncoding;
use k256::ecdsa::{SigningKey, VerifyingKey};
use linked_hash_map::LinkedHashMap;
use std::collections::HashSet;

fn utxos_for_funding(funding: &Tx) -> LinkedHashMap<OutPoint, TxOut> {
    let mut utxos = LinkedHashMap::new();
    utxos.insert(
        OutPoint {
            hash: funding.hash(),
            index: 0,
        },
        funding.outputs[0].clone(),
    );
    utxos
}

fn verifying_key_as_bytes(verifying_key: &VerifyingKey) -> [u8; 33] {
    verifying_key.to_sec1_bytes().to_vec()[..].try_into().unwrap()
}

#[test]
fn chronicle_validate_two_phase_functional_unlock() {
    let funding = Tx {
        version: 1,
        inputs: vec![],
        outputs: vec![TxOut {
            satoshis: 1_000,
            lock_script: Script(vec![OP_5, OP_EQUAL]),
        }],
        lock_time: 0,
    };
    let utxos = utxos_for_funding(&funding);

    let spend = Tx {
        version: 2,
        inputs: vec![TxIn {
            prev_output: OutPoint {
                hash: funding.hash(),
                index: 0,
            },
            unlock_script: Script(vec![OP_2, OP_3, OP_ADD]),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis: 900,
            lock_script: Script(vec![]),
        }],
        lock_time: 0,
    };

    assert!(spend.validate(true, true, &utxos, &HashSet::new()).is_ok());
}

#[test]
fn chronicle_validate_version_one_rejects_functional_unlock() {
    let funding = Tx {
        version: 1,
        inputs: vec![],
        outputs: vec![TxOut {
            satoshis: 1_000,
            lock_script: Script(vec![OP_5, OP_EQUAL]),
        }],
        lock_time: 0,
    };
    let utxos = utxos_for_funding(&funding);

    let spend = Tx {
        version: 1,
        inputs: vec![TxIn {
            prev_output: OutPoint {
                hash: funding.hash(),
                index: 0,
            },
            unlock_script: Script(vec![OP_2, OP_3, OP_ADD]),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis: 900,
            lock_script: Script(vec![]),
        }],
        lock_time: 0,
    };

    assert!(spend.validate(true, true, &utxos, &HashSet::new()).is_err());
}

#[test]
fn chronicle_validate_relaxed_clean_stack() {
    let funding = Tx {
        version: 1,
        inputs: vec![],
        outputs: vec![TxOut {
            satoshis: 1_000,
            lock_script: Script(vec![OP_1, OP_1]),
        }],
        lock_time: 0,
    };
    let utxos = utxos_for_funding(&funding);
    let prev_output = OutPoint {
        hash: funding.hash(),
        index: 0,
    };

    let chronicle_spend = Tx {
        version: 2,
        inputs: vec![TxIn {
            prev_output: prev_output.clone(),
            unlock_script: Script(vec![OP_1]),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis: 900,
            lock_script: Script(vec![]),
        }],
        lock_time: 0,
    };
    assert!(
        chronicle_spend
            .validate(true, true, &utxos, &HashSet::new())
            .is_ok()
    );

    let legacy_spend = Tx {
        version: 1,
        inputs: vec![TxIn {
            prev_output,
            unlock_script: Script(vec![OP_1]),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis: 900,
            lock_script: Script(vec![]),
        }],
        lock_time: 0,
    };
    assert!(
        legacy_spend
            .validate(true, true, &utxos, &HashSet::new())
            .is_err()
    );
}

#[test]
fn chronicle_validate_high_s_p2pkh() {
    let private_key = [2; 32];
    let secret_key = SigningKey::from_slice(&private_key).unwrap();
    let public_key = verifying_key_as_bytes(secret_key.verifying_key());

    let funding = Tx {
        version: 1,
        inputs: vec![],
        outputs: vec![TxOut {
            satoshis: 10,
            lock_script: create_lock_script(&hash160(&public_key)),
        }],
        lock_time: 0,
    };
    let utxos = utxos_for_funding(&funding);

    let mut spend = Tx {
        version: 2,
        inputs: vec![TxIn {
            prev_output: OutPoint {
                hash: funding.hash(),
                index: 0,
            },
            unlock_script: Script(vec![]),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis: 5,
            lock_script: Script(vec![]),
        }],
        lock_time: 0,
    };

    let sighash_type = SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE;
    let mut cache = SigHashCache::new();
    let lock_script = &funding.outputs[0].lock_script.0;
    let sig_hash = sighash(&spend, 0, lock_script, 10, sighash_type, &mut cache).unwrap();
    let low_sig = secret_key.sign_prehash(&sig_hash.0).unwrap();
    let high_sig = flip_to_high_s(&low_sig);

    let mut unlock_script = Script::new();
    unlock_script.append_data(
        &[high_sig.to_der().to_vec(), vec![sighash_type]].concat(),
    );
    unlock_script.append_data(&public_key);
    spend.inputs[0].unlock_script = unlock_script;

    assert!(spend.validate(true, true, &utxos, &HashSet::new()).is_ok());
}

#[test]
fn chronicle_validate_at_height_rejects_pre_activation_spend() {
    let funding = Tx {
        version: 1,
        inputs: vec![],
        outputs: vec![TxOut {
            satoshis: 1_000,
            lock_script: Script(vec![OP_5, OP_EQUAL]),
        }],
        lock_time: 0,
    };
    let utxos = utxos_for_funding(&funding);

    let spend = Tx {
        version: 2,
        inputs: vec![TxIn {
            prev_output: OutPoint {
                hash: funding.hash(),
                index: 0,
            },
            unlock_script: Script(vec![OP_2, OP_3, OP_ADD]),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis: 900,
            lock_script: Script(vec![]),
        }],
        lock_time: 0,
    };

    assert!(spend
        .validate_at_height(
            true,
            true,
            &utxos,
            &HashSet::new(),
            CHRONICLE_ACTIVATION_MAINNET,
            Network::BSV_Mainnet,
        )
        .is_ok());
    assert!(spend
        .validate_at_height(
            true,
            true,
            &utxos,
            &HashSet::new(),
            CHRONICLE_ACTIVATION_MAINNET - 1,
            Network::BSV_Mainnet,
        )
        .is_err());
}
