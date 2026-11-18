//! Positive/negative commitment matrix: expectations follow Node's serializer,
//! independently of the signing digest under test. Exported cases run on real Node.
use chain_gang::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    script::{op_codes::*, Script},
    transaction::{generate_signature, sighash::*},
    util::Hash256,
};
use k256::ecdsa::SigningKey;
use linked_hash_map::LinkedHashMap;
use serde_json::json;
use std::collections::HashSet;
#[test]
fn commitment_matrix() {
    let pk = SigningKey::from_slice(&[9; 32])
        .unwrap()
        .verifying_key()
        .to_sec1_bytes();
    let mut rows = vec![];
    for flag in [
        0x41u8, 0x42, 0x43, 0xc1, 0xc2, 0xc3, 0x61, 0x62, 0x63, 0xe1, 0xe2, 0xe3,
    ] {
        for index in 0..2 {
            for style in ["lock", "unlock", "mixed-multisig"] {
                for missing in [false, true] {
                    if missing && (index != 1 || flag & 31 != 3) {
                        continue;
                    }
                    let multi = style == "mixed-multisig";
                    let flags = if multi {
                        vec![flag, flag ^ 0x20]
                    } else {
                        vec![flag]
                    };
                    let mut code = Script::new();
                    if multi {
                        code.append(OP_2);
                    }
                    code.append_data(&pk);
                    if multi {
                        code.append_data(&pk);
                        code.append(OP_2);
                        code.append(OP_CHECKMULTISIGVERIFY);
                    } else {
                        code.append(OP_CHECKSIGVERIFY);
                    }
                    // This separator is skipped; the first separator below is executed. A pushed
                    // 0xab must never be treated as an opcode by either digest serializer.
                    code.append_slice(&[OP_0, OP_IF, OP_CODESEPARATOR, OP_ENDIF]);
                    code.append_data(&[OP_CODESEPARATOR]);
                    code.append(OP_DROP);
                    code.append(OP_1);
                    let mut lock = Script::new();
                    lock.append(OP_CODESEPARATOR);
                    lock.append_slice(&code.0);
                    if style == "unlock" {
                        lock = Script(vec![OP_DROP, OP_1]);
                    }
                    let mut script_code = code.0.clone();
                    if style == "unlock" {
                        script_code.extend_from_slice(&lock.0);
                    }
                    let mut tx = Tx {
                        version: 2,
                        inputs: (0..2)
                            .map(|i| TxIn {
                                prev_output: OutPoint {
                                    hash: Hash256([i + 1; 32]),
                                    index: 0,
                                },
                                unlock_script: Script::new(),
                                sequence: 100 + i as u32,
                            })
                            .collect(),
                        outputs: (0..if missing { 1 } else { 3 })
                            .map(|i| TxOut {
                                satoshis: 10 + i,
                                lock_script: Script(vec![OP_1]),
                            })
                            .collect(),
                        lock_time: 0,
                    };
                    let mut unlock = Script::new();
                    if multi {
                        unlock.append(OP_0);
                    }
                    for f in &flags {
                        let digest = sighash_from_script_code(
                            &tx,
                            index,
                            &script_code,
                            100,
                            *f,
                            &mut SigHashCache::new(),
                        )
                        .unwrap();
                        unlock.append_data(&generate_signature(&[9; 32], &digest, *f).unwrap());
                    }
                    if style == "unlock" {
                        unlock.append(OP_CODESEPARATOR);
                        unlock.append_slice(&code.0);
                    }
                    tx.inputs[index].unlock_script = unlock;
                    for mutation in [
                        "none",
                        "paired-output",
                        "other-output",
                        "current-sequence",
                        "other-sequence",
                        "current-prevout",
                        "other-prevout",
                        "amount",
                        "locktime",
                        "version",
                        "signature",
                        "append-output",
                        "other-unlock",
                        "paired-script",
                        "other-script",
                    ] {
                        if missing && matches!(mutation, "paired-output" | "paired-script") {
                            continue;
                        }
                        let mut changed = tx.clone();
                        let mut amount = 100;
                        match mutation {
                            "append-output" => changed.outputs.push(TxOut {
                                satoshis: 1,
                                lock_script: Script(vec![OP_1]),
                            }),
                            "other-unlock" => {
                                changed.inputs[1 - index].unlock_script =
                                    Script(vec![OP_0, OP_DROP])
                            }
                            "paired-script" => {
                                changed.outputs[index].lock_script = Script(vec![OP_2])
                            }
                            "other-script" => {
                                changed.outputs[1 - index].lock_script = Script(vec![OP_2])
                            }
                            "paired-output" => changed.outputs[index].satoshis += 1,
                            "other-output" => {
                                changed.outputs[if index == 0 { 1 } else { 0 }].satoshis += 1
                            }
                            "current-sequence" => changed.inputs[index].sequence += 1,
                            "other-sequence" => changed.inputs[1 - index].sequence += 1,
                            "current-prevout" => changed.inputs[index].prev_output.hash.0[0] ^= 8,
                            "other-prevout" => changed.inputs[1 - index].prev_output.hash.0[0] ^= 8,
                            "amount" => amount += 1,
                            "locktime" => changed.lock_time += 1,
                            "version" => changed.version += 1,
                            "signature" => {
                                let offset = if multi { 6 } else { 5 };
                                changed.inputs[index].unlock_script.0[offset] ^= 1;
                            }
                            _ => (),
                        }
                        let expected = flags.iter().all(|f| {
                            if mutation == "signature" {
                                return false;
                            }
                            let base = f & 31;
                            let acp = f & 0x80 != 0;
                            let otda = f & 0x20 != 0;
                            if mutation == "append-output" {
                                return base == 2 || (base == 3 && !missing);
                            }
                            if otda && missing {
                                return true;
                            }
                            match mutation {
                                "none" | "other-unlock" => true,
                                "paired-output" | "paired-script" => base == 2,
                                "other-output" | "other-script" => base != 1,
                                "other-sequence" => acp || base != 1,
                                "other-prevout" => acp,
                                "amount" => otda,
                                _ => false,
                            }
                        });
                        let mut utxos = LinkedHashMap::new();
                        for (i, input) in changed.inputs.iter().enumerate() {
                            utxos.insert(
                                input.prev_output.clone(),
                                TxOut {
                                    satoshis: if i == index { amount } else { 100 },
                                    lock_script: if i == index {
                                        lock.clone()
                                    } else {
                                        Script(vec![OP_1])
                                    },
                                },
                            );
                        }
                        let result =
                            changed.validate_consensus(true, true, &utxos, &HashSet::new());
                        let library = result.is_ok();
                        let name =
                            format!("{flag:02x}-{index}-{style}-missing={missing}-{mutation}");
                        assert_eq!(library, expected, "{name}: {result:?}");
                        rows.push(json!({"name":name,"expected":expected,"library":library,"request":{"tx":changed.as_hexstr(),"n":index,"flags":3997710,"reportflags":true,"txo":{"lock":hex::encode(&lock.0),"value":amount}}}));
                    }
                }
            }
        }
    }
    eprintln!("{} commitment cases", rows.len());
    if let Ok(path) = std::env::var("CHRONICLE_DIFF_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    }
}
