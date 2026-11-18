use chain_gang::{
    chronicle::*,
    messages::{OutPoint, Tx, TxIn, TxOut},
    network::Network,
    script::{op_codes::*, Script},
    transaction::{generate_signature, sighash::*},
    util::Hash256,
};
use linked_hash_map::LinkedHashMap;
use serde_json::json;
use std::collections::{HashMap, HashSet};
#[test]
fn explicit_chronicle_context_matches_node() {
    assert_eq!(activation_height(Network::BSV_Mainnet), Some(943816));
    assert_eq!(activation_height(Network::BSV_Testnet), Some(1713168));
    assert_eq!(activation_height(Network::BSV_STN), Some(250));
    assert_eq!(activation_height(Network::BSV_Regtest), Some(15000));
    let mut rows = vec![];
    for active in [false, true] {
        for new_utxo in [false, true] {
            if new_utxo && !active {
                continue;
            }
            for version in [1u32, 2, u32::MAX] {
                for (label, code, wants_new) in [
                    ("2mul", vec![OP_1, OP_2MUL, OP_2, OP_EQUAL], true),
                    ("nop-left", vec![OP_LEFT, OP_1], false),
                    ("ver", vec![OP_VER, OP_DROP, OP_1], true),
                ] {
                    let point = OutPoint {
                        hash: Hash256([3; 32]),
                        index: 0,
                    };
                    let mut heights = HashMap::new();
                    heights.insert(point.clone(), if new_utxo { 200 } else { 100 });
                    let tx = Tx {
                        version,
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
                    let mut utxos = LinkedHashMap::new();
                    utxos.insert(
                        point,
                        TxOut {
                            satoshis: 1,
                            lock_script: Script(code.clone()),
                        },
                    );
                    let ctx = ChronicleValidationContext {
                        block_height: if active { 200 } else { 199 },
                        activation_height: 200,
                        utxo_heights: &heights,
                        script_num_policy_limit: None,
                    };
                    let library = tx
                        .validate_with_chronicle_context(
                            true,
                            true,
                            &utxos,
                            &HashSet::new(),
                            &ctx,
                            true,
                        )
                        .is_ok();
                    let expected = new_utxo == wants_new;
                    let flags = (1u32 << 18)
                        | (1 << 19)
                        | if active { 1 << 20 } else { 0 }
                        | if new_utxo { 1 << 21 } else { 0 };
                    let name = format!("{label}-active{active}-new{new_utxo}-v{version}");
                    rows.push(json!({"name":name,"library":library,"expected":expected,"request":{"tx":tx.as_hexstr(),"n":0,"flags":flags,"txo":{"lock":hex::encode(code),"value":1},"reportflags":true}}));
                    assert_eq!(library, expected, "{name}");
                }
            }
        }
    }
    // Real signatures exercise activation independently of the version gate.
    let key = k256::ecdsa::SigningKey::from_slice(&[9; 32]).unwrap();
    let mut lock = Script::new();
    lock.append_data(&key.verifying_key().to_sec1_bytes());
    lock.append(OP_CHECKSIG);
    for version in [1, 2, u32::MAX] {
        for active in [false, true] {
            for high_s in [false, true] {
                let point = OutPoint {
                    hash: Hash256([3; 32]),
                    index: 0,
                };
                let mut heights = HashMap::new();
                heights.insert(point.clone(), 100);
                let mut tx = Tx {
                    version,
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
                let digest =
                    sighash_from_script_code(&tx, 0, &lock.0, 1, 0x61, &mut SigHashCache::new())
                        .unwrap();
                let mut sig = generate_signature(&[9; 32], &digest, 0x61).unwrap();
                if high_s {
                    use k256::ecdsa::{signature::SignatureEncoding, Signature};
                    use num_bigint::BigUint;
                    let signature = Signature::from_der(&sig[..sig.len() - 1]).unwrap();
                    let mut raw = signature.to_bytes();
                    let order = BigUint::parse_bytes(
                        b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
                        16,
                    )
                    .unwrap();
                    let high = (order - BigUint::from_bytes_be(&raw[32..])).to_bytes_be();
                    raw[32..].fill(0);
                    raw[64 - high.len()..].copy_from_slice(&high);
                    sig = Signature::try_from(raw.as_ref()).unwrap().to_der().to_vec();
                    sig.push(0x61);
                }
                tx.inputs[0].unlock_script.append_data(&sig);
                let mut utxos = LinkedHashMap::new();
                utxos.insert(
                    point,
                    TxOut {
                        satoshis: 1,
                        lock_script: lock.clone(),
                    },
                );
                let ctx = ChronicleValidationContext {
                    block_height: if active { 200 } else { 199 },
                    activation_height: 200,
                    utxo_heights: &heights,
                    script_num_policy_limit: None,
                };
                let library = tx
                    .validate_with_chronicle_context(
                        true,
                        true,
                        &utxos,
                        &HashSet::new(),
                        &ctx,
                        true,
                    )
                    .is_ok();
                let expected = active && (!high_s || version == 2);
                assert_eq!(library, expected);
                rows.push(json!({"name":format!("signature-active{active}-v{version}-high{high_s}"),"expected":expected,"library":library,"request":{"tx":tx.as_hexstr(),"n":0,"flags":(1u32<<18)|(1<<19)|(1<<16)|2|4|8|if active{1<<20}else{0},"txo":{"lock":hex::encode(&lock.0),"value":1},"reportflags":true}}));
            }
        }
    }
    if let Ok(path) = std::env::var("CHRONICLE_DIFF_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    }
}

#[test]
fn explicit_metadata_and_policy_errors() {
    let point = OutPoint {
        hash: Hash256([1; 32]),
        index: 0,
    };
    let tx = Tx {
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
    let mut lock = Script::new();
    lock.append_data(&[0, 0, 0, 0, 1]);
    lock.append_slice(&[OP_1ADD, OP_DROP, OP_1]);
    let mut utxos = LinkedHashMap::new();
    utxos.insert(
        point.clone(),
        TxOut {
            satoshis: 1,
            lock_script: lock,
        },
    );
    let validate = |heights: &HashMap<OutPoint, u64>, policy, consensus| {
        tx.validate_with_chronicle_context(
            true,
            true,
            &utxos,
            &HashSet::new(),
            &ChronicleValidationContext {
                block_height: 200,
                activation_height: 200,
                utxo_heights: heights,
                script_num_policy_limit: policy,
            },
            consensus,
        )
    };
    let mut heights = HashMap::new();
    assert!(validate(&heights, None, true)
        .unwrap_err()
        .to_string()
        .contains("missing UTXO"));
    heights.insert(point.clone(), 201);
    assert!(validate(&heights, None, true)
        .unwrap_err()
        .to_string()
        .contains("exceeds spending"));
    heights.insert(point, 200);
    assert!(validate(&heights, Some(4), false).is_err());
    assert!(validate(&heights, Some(4), true).is_ok());
    assert!(validate(&heights, Some(0), false).is_ok());
    assert!(validate(&heights, Some(3), false).is_err());
    assert_eq!(
        effective_chronicle_tx_version(u32::MAX, Some(0), Some(Network::BSV_Regtest)),
        u32::MAX
    );
}
