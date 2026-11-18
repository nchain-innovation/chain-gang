//! Current Chronicle: every supplied signature participates in scriptCode
//! cleanup, including entries skipped by CHECKMULTISIG's early termination.
use chain_gang::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    script::{op_codes::*, Script},
    transaction::{
        generate_signature,
        sighash::{sighash_from_script_code, SigHashCache},
    },
    util::Hash256,
};
use k256::ecdsa::SigningKey;
use linked_hash_map::LinkedHashMap;
use std::collections::HashSet;

#[test]
fn unvisited_entries_still_clean_current_multisig_script_code() {
    let key = SigningKey::from_slice(&[7; 32]).unwrap();
    let pubkey = key.verifying_key().to_sec1_bytes();
    for empty in [false, true] {
        for flag in [0x41, 0x61] {
            for sign_cleaned in [false, true] {
                let unused = if empty { vec![] } else { vec![0x30, 1] };
                let mut lock = Script::new();
                lock.append_data(&unused);
                let pushed_len = lock.0.len();
                lock.append(OP_DROP);
                lock.append(OP_2);
                // The second visited pair must abort if reached: malformed
                // signature, or an empty signature with malformed key shape.
                lock.append_data(if empty { &[2] } else { &pubkey });
                lock.append_data(&pubkey);
                lock.append(OP_2);
                lock.append(OP_CHECKMULTISIG);
                lock.append(OP_NOT);
                let point = OutPoint {
                    hash: Hash256([23; 32]),
                    index: 0,
                };
                let mut tx = Tx {
                    version: 2,
                    inputs: vec![TxIn {
                        prev_output: point.clone(),
                        unlock_script: Script::new(),
                        sequence: u32::MAX,
                    }],
                    outputs: vec![TxOut {
                        satoshis: 900,
                        lock_script: Script(vec![OP_1]),
                    }],
                    lock_time: 0,
                };
                let code = if sign_cleaned {
                    &lock.0[pushed_len..]
                } else {
                    &lock.0
                };
                let digest =
                    sighash_from_script_code(&tx, 0, code, 1000, flag, &mut SigHashCache::new())
                        .unwrap();
                let sig = generate_signature(&[7; 32], &digest, flag).unwrap();
                let mut unlock = Script::new();
                unlock.append(OP_0);
                unlock.append_data(&unused);
                unlock.append_data(&sig);
                tx.inputs[0].unlock_script = unlock;
                let utxos = LinkedHashMap::from_iter([(
                    point,
                    TxOut {
                        satoshis: 1000,
                        lock_script: lock,
                    },
                )]);
                let result = tx.validate_consensus(true, true, &utxos, &HashSet::new());
                assert_eq!(
                    result.is_ok(),
                    !sign_cleaned,
                    "empty={empty}, flag={flag:x}, sign_cleaned={sign_cleaned}: {result:?}"
                );
            }
        }
    }
}
