//! Boundary expectations from bitcoin-sv 6504a3aff65ba97c0f6c80962b033e35ecbfed4b,
//! src/script/interpreter.cpp:2086-2123 (SignatureHashOriginal / dispatch).
//! Source-derived cases complement the vendored random node vectors; these
//! tests do not claim to execute a live node.
use chain_gang::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    network::Network,
    script::{op_codes::*, Script},
    transaction::{generate_signature, sighash::*},
    util::{sha256d, Hash256},
    wallet::Wallet,
};
use k256::ecdsa::SigningKey;
use linked_hash_map::LinkedHashMap;
use std::collections::HashSet;

fn one() -> Hash256 {
    Hash256::decode("0000000000000000000000000000000000000000000000000000000000000001").unwrap()
}

fn tx() -> Tx {
    Tx {
        version: 2,
        inputs: (0..2)
            .map(|index| TxIn {
                prev_output: OutPoint {
                    hash: Hash256([2; 32]),
                    index,
                },
                unlock_script: Script::new(),
                sequence: u32::MAX,
            })
            .collect(),
        outputs: vec![TxOut {
            satoshis: 1,
            lock_script: Script(vec![OP_1]),
        }],
        lock_time: 0,
    }
}

#[test]
fn original_digest_and_preimage_boundaries() {
    let tx = tx();
    for era in [0, SIGHASH_CHRONICLE, SIGHASH_CHRONICLE | SIGHASH_FORKID] {
        for acp in [0, SIGHASH_ANYONECANPAY] {
            for base in [SIGHASH_SINGLE, SIGHASH_ALL, SIGHASH_NONE] {
                let flags = era | acp | base;
                for input in [1, 0, 2, usize::MAX] {
                    let exceptional = input >= 2 || (input == 1 && base == SIGHASH_SINGLE);
                    let mut cache = SigHashCache::new();
                    let digest =
                        sighash(&tx, input, &[OP_CHECKSIG], 10, flags, &mut cache).unwrap();
                    assert_eq!(digest == one(), exceptional);
                    assert_eq!(
                        sighash_from_script_code(&tx, input, &[OP_CHECKSIG], 10, flags, &mut cache)
                            .unwrap(),
                        digest
                    );
                    assert_eq!(
                        sighash_checksig_index(
                            &tx,
                            input,
                            &[OP_CHECKSIG],
                            0,
                            10,
                            flags,
                            &mut cache
                        )
                        .unwrap(),
                        digest
                    );
                    let preimage =
                        sig_hash_preimage(&tx, input, &[OP_CHECKSIG], 10, flags, &mut cache);
                    let indexed = sig_hash_preimage_checksig_index(
                        &tx,
                        input,
                        &[OP_CHECKSIG],
                        0,
                        10,
                        flags,
                        &mut cache,
                    );
                    if exceptional {
                        assert!(preimage
                            .unwrap_err()
                            .to_string()
                            .contains("no serialized preimage"));
                        assert!(indexed
                            .unwrap_err()
                            .to_string()
                            .contains("no serialized preimage"));
                        // Node returns before inspecting script; no CHECKSIG to select.
                        assert_eq!(
                            sighash_checksig_index(
                                &tx,
                                input,
                                &[OP_CODESEPARATOR],
                                99,
                                10,
                                flags,
                                &mut cache
                            )
                            .unwrap(),
                            one()
                        );
                    } else {
                        assert_eq!(sha256d(&preimage.unwrap()), digest);
                        assert_eq!(sha256d(&indexed.unwrap()), digest);
                    }
                }
            }
        }
    }
    assert_eq!(one().0[0], 1);
    assert_eq!(&one().0[1..], &[0; 31]);
}

#[test]
fn bip143_missing_output_has_a_real_preimage() {
    let mut tx = tx();
    for acp in [0, SIGHASH_ANYONECANPAY] {
        let flags = SIGHASH_SINGLE | SIGHASH_FORKID | acp;
        let mut cache = SigHashCache::new();
        let preimage = sig_hash_preimage(&tx, 1, &[OP_CHECKSIG], 10, flags, &mut cache).unwrap();
        // hashOutputs is immediately before locktime and sighash type.
        assert_eq!(&preimage[preimage.len() - 40..preimage.len() - 8], &[0; 32]);
        let digest = sighash(&tx, 1, &[OP_CHECKSIG], 10, flags, &mut cache).unwrap();
        assert_eq!(sha256d(&preimage), digest);
        assert_ne!(digest, one());
        tx.lock_time += 1;
        assert_ne!(
            sighash(&tx, 1, &[OP_CHECKSIG], 10, flags, &mut SigHashCache::new()).unwrap(),
            digest
        );
        for input in [2, usize::MAX] {
            assert!(sighash(&tx, input, &[], 10, flags, &mut cache).is_err());
            assert!(sig_hash_preimage(&tx, input, &[], 10, flags, &mut cache).is_err());
        }
    }
}

#[test]
fn consensus_accepts_constant_signature_but_wallet_refuses_to_create_it() {
    let key = SigningKey::from_slice(&[7; 32]).unwrap();
    let wallet = Wallet::new(key.clone(), *key.verifying_key(), Network::BSV_Testnet);
    let mut lock = Script::new();
    lock.append_data(&key.verifying_key().to_sec1_bytes());
    lock.append(OP_CHECKSIG);
    let funding = Tx {
        version: 1,
        inputs: vec![],
        outputs: vec![
            TxOut {
                satoshis: 10,
                lock_script: Script(vec![OP_1]),
            },
            TxOut {
                satoshis: 10,
                lock_script: lock,
            },
        ],
        lock_time: 0,
    };
    let mut utxos = LinkedHashMap::new();
    for (index, output) in funding.outputs.iter().enumerate() {
        utxos.insert(
            OutPoint {
                hash: funding.hash(),
                index: index as u32,
            },
            output.clone(),
        );
    }
    for era in [0, SIGHASH_CHRONICLE, SIGHASH_CHRONICLE | SIGHASH_FORKID] {
        for acp in [0, SIGHASH_ANYONECANPAY] {
            let flags = SIGHASH_SINGLE | era | acp;
            let mut spend = tx();
            for input in &mut spend.inputs {
                input.prev_output.hash = funding.hash();
            }
            let before = spend.clone();
            assert!(wallet
                .sign_tx_input(&funding, &mut spend, 1, flags)
                .is_err());
            assert!(wallet
                .sign_tx_input_checksig_index(&funding, &mut spend, 1, flags, 0)
                .is_err());
            assert_eq!(spend, before);
            // Explicit low-level signing remains available. Sign the independently
            // specified sentinel, not a digest computed by the implementation.
            let sig = generate_signature(&[7; 32], &one(), flags).unwrap();
            assert!(wallet.sign_sighash(one(), flags).is_ok());
            spend.inputs[1].unlock_script.append_data(&sig);
            assert!(spend
                .validate_consensus(false, true, &utxos, &HashSet::new())
                .is_ok());
            spend.lock_time = 123;
            assert!(spend
                .validate_consensus(false, true, &utxos, &HashSet::new())
                .is_ok());
            // Adding the paired output restores a real digest and invalidates it.
            spend.outputs.push(TxOut {
                satoshis: 1,
                lock_script: Script(vec![OP_1]),
            });
            assert!(spend
                .validate_consensus(false, true, &utxos, &HashSet::new())
                .is_err());
            assert!(wallet.sign_tx_input(&funding, &mut spend, 1, flags).is_ok());
        }
    }
}

#[test]
fn empty_outputs_and_wallet_bip143_policy() {
    let mut spend = tx();
    spend.outputs.clear();
    assert_eq!(
        sighash(&spend, 0, &[], 0, SIGHASH_SINGLE, &mut SigHashCache::new()).unwrap(),
        one()
    );
    let key = SigningKey::from_slice(&[7; 32]).unwrap();
    let wallet = Wallet::new(key.clone(), *key.verifying_key(), Network::BSV_Testnet);
    let funding = Tx {
        version: 1,
        inputs: vec![],
        outputs: vec![TxOut {
            satoshis: 10,
            lock_script: Script(vec![OP_CHECKSIG]),
        }],
        lock_time: 0,
    };
    spend.inputs[0].prev_output.hash = funding.hash();
    let flags = SIGHASH_SINGLE | SIGHASH_FORKID;
    assert!(wallet.sign_tx_input(&funding, &mut spend, 0, flags).is_ok());
    assert!(wallet
        .sign_tx_input_checksig_index(&funding, &mut spend, 0, flags, 0)
        .is_ok());
    for flags in [SIGHASH_SINGLE, flags] {
        assert!(wallet
            .sign_tx_input(&funding, &mut spend, usize::MAX, flags)
            .is_err());
        assert!(wallet
            .sign_tx_input_checksig_index(&funding, &mut spend, usize::MAX, flags, 0)
            .is_err());
    }
}
