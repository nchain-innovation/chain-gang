use chain_gang::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    script::{op_codes::*, Script},
    transaction::{generate_signature, sighash::*},
    util::Hash256,
};
use k256::ecdsa::signature::SignatureEncoding;
use k256::ecdsa::{Signature, SigningKey};
use linked_hash_map::LinkedHashMap;
use num_bigint::BigUint;
use serde_json::json;
use std::collections::HashSet;
fn high(sig: &[u8]) -> Vec<u8> {
    let s = Signature::from_der(&sig[..sig.len() - 1]).unwrap();
    let mut raw = s.to_bytes();
    let n = BigUint::parse_bytes(
        b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
        16,
    )
    .unwrap();
    let h = (n - BigUint::from_bytes_be(&raw[32..])).to_bytes_be();
    raw[32..].fill(0);
    raw[64 - h.len()..].copy_from_slice(&h);
    let mut out = Signature::try_from(raw.as_ref()).unwrap().to_der().to_vec();
    out.push(*sig.last().unwrap());
    out
}
#[test]
fn node_signature_cases() {
    let key = SigningKey::from_slice(&[9; 32]).unwrap();
    let pk = key.verifying_key().to_sec1_bytes();
    let mut rows = vec![];
    for (name, unlock_phase, multisig, high_s) in [
        ("bip143-lock-separator", false, false, false),
        ("bip143-unlock-separator", true, false, false),
        ("mixed-multisig", false, true, false),
        ("high-s-chronicle", false, false, true),
    ] {
        let mut checked = Script::new();
        if multisig {
            checked.append(OP_2);
        }
        checked.append_data(&pk);
        if multisig {
            checked.append_data(&pk);
            checked.append(OP_2);
            checked.append(OP_CHECKMULTISIGVERIFY);
        } else {
            checked.append(OP_CHECKSIGVERIFY);
        }
        checked.append(OP_CODESEPARATOR);
        checked.append(OP_1);
        let lock = if unlock_phase {
            Script(vec![OP_DROP, OP_1])
        } else {
            checked.clone()
        };
        let point = OutPoint {
            hash: Hash256([3; 32]),
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
                satoshis: 1,
                lock_script: Script(vec![OP_1]),
            }],
            lock_time: 0,
        };
        let mut code = checked.0.clone();
        if unlock_phase {
            code.extend_from_slice(&lock.0);
        }
        if multisig {
            tx.inputs[0].unlock_script.append(OP_0);
        }
        for flags in if multisig {
            vec![0x41, 0x61]
        } else if high_s {
            vec![0x61]
        } else {
            vec![0x41]
        } {
            let digest =
                sighash_from_script_code(&tx, 0, &code, 1, flags, &mut SigHashCache::new())
                    .unwrap();
            let mut sig = generate_signature(&[9; 32], &digest, flags).unwrap();
            if high_s {
                sig = high(&sig);
            }
            tx.inputs[0].unlock_script.append_data(&sig);
        }
        if unlock_phase {
            tx.inputs[0].unlock_script.append(OP_CODESEPARATOR);
            tx.inputs[0].unlock_script.append_slice(&checked.0);
        }
        let mut utxos = LinkedHashMap::new();
        utxos.insert(
            point,
            TxOut {
                satoshis: 1,
                lock_script: lock.clone(),
            },
        );
        let verdict = tx.validate_consensus(true, true, &utxos, &HashSet::new());
        let library = verdict.is_ok();
        eprintln!("{name}: {verdict:?}");
        rows.push(json!({"name":name,"expected":true,"library":library,"request":{"tx":tx.as_hexstr(),"n":0,"flags":(1u32<<18)|(1<<19)|(1<<20)|(1<<21)|(1<<16)|2|4|8|(1<<14),"reportflags":true,"txo":{"lock":hex::encode(&lock.0),"value":1}}}));
        assert!(library, "{name}");
    }
    if let Ok(path) = std::env::var("CHRONICLE_DIFF_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    }
}
