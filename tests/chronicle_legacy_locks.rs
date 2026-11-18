//! CLTV/CSV depend on the spent output's Genesis era, not Chronicle activation.
//! Node v1.2.3 interpreter.cpp:520-594 and 2166-2250; real-node probes are
//! scripts/node-chain/legacy_locks.py on the combined test branch.
use chain_gang::{
    chronicle::ChronicleValidationContext,
    messages::{OutPoint, Tx, TxIn, TxOut},
    script::{op_codes::*, Script},
    util::Hash256,
};
use linked_hash_map::LinkedHashMap;
use std::collections::{HashMap, HashSet};

#[test]
fn legacy_lock_opcodes_keep_their_utxo_era_across_chronicle() {
    // Operand bytes are explicit minimally encoded Script numbers, not the
    // interpreter's encoder. Five bytes are intentional (CLTV/CSV exception).
    type LockCase = (&'static str, &'static [u8], u8, u32, u32, u32, bool);
    let cases: &[LockCase] = &[
        ("cltv-below", &[1], OP_CHECKLOCKTIMEVERIFY, 2, 1, 0, false),
        ("cltv-equal", &[1], OP_CHECKLOCKTIMEVERIFY, 2, 1, 1, true),
        (
            "cltv-final-seq",
            &[1],
            OP_CHECKLOCKTIMEVERIFY,
            2,
            u32::MAX,
            1,
            false,
        ),
        (
            "cltv-negative",
            &[0x81],
            OP_CHECKLOCKTIMEVERIFY,
            2,
            1,
            1,
            false,
        ),
        (
            "cltv-type",
            &[0, 0x65, 0xcd, 0x1d],
            OP_CHECKLOCKTIMEVERIFY,
            2,
            1,
            1,
            false,
        ),
        (
            "cltv-high-below",
            &[0, 0, 0, 0x80, 0],
            OP_CHECKLOCKTIMEVERIFY,
            2,
            1,
            0x7fff_ffff,
            false,
        ),
        (
            "cltv-high-equal",
            &[0, 0, 0, 0x80, 0],
            OP_CHECKLOCKTIMEVERIFY,
            2,
            1,
            0x8000_0000,
            true,
        ),
        ("csv-below", &[1], OP_CHECKSEQUENCEVERIFY, 2, 0, 0, false),
        ("csv-equal", &[1], OP_CHECKSEQUENCEVERIFY, 2, 1, 0, true),
        (
            "csv-version-one",
            &[1],
            OP_CHECKSEQUENCEVERIFY,
            1,
            1,
            0,
            false,
        ),
        (
            "csv-unsigned-version",
            &[1],
            OP_CHECKSEQUENCEVERIFY,
            u32::MAX,
            1,
            0,
            true,
        ),
        (
            "csv-input-disabled",
            &[1],
            OP_CHECKSEQUENCEVERIFY,
            2,
            0x8000_0001,
            0,
            false,
        ),
        (
            "csv-type",
            &[1, 0, 0x40],
            OP_CHECKSEQUENCEVERIFY,
            2,
            1,
            0,
            false,
        ),
        (
            "csv-time",
            &[1, 0, 0x40],
            OP_CHECKSEQUENCEVERIFY,
            2,
            0x400001,
            0,
            true,
        ),
        (
            "csv-wide-mask",
            &[1, 0, 0, 0, 1],
            OP_CHECKSEQUENCEVERIFY,
            2,
            1,
            0,
            true,
        ),
        (
            "csv-operand-disabled",
            &[0, 0, 0, 0x80, 0],
            OP_CHECKSEQUENCEVERIFY,
            1,
            0,
            0,
            true,
        ),
        (
            "csv-negative",
            &[0x81],
            OP_CHECKSEQUENCEVERIFY,
            2,
            1,
            0,
            false,
        ),
    ];
    for &(label, operand, opcode, version, sequence, lock_time, old_ok) in cases {
        for (height, created, pregenesis) in [
            (199, 100, true),
            (200, 100, true),
            (199, 150, false),
            (200, 150, false),
            (200, 200, false),
        ] {
            let point = OutPoint {
                hash: Hash256([37; 32]),
                index: 0,
            };
            let mut script = Script::new();
            script.append_data(operand);
            script.append(opcode);
            script.append(OP_DROP);
            script.append(OP_1);
            let utxos = LinkedHashMap::from_iter([(
                point.clone(),
                TxOut {
                    satoshis: 1000,
                    lock_script: script,
                },
            )]);
            let heights = HashMap::from([(point.clone(), created)]);
            let old = if pregenesis {
                HashSet::from([point.clone()])
            } else {
                HashSet::new()
            };
            let tx = Tx {
                version,
                inputs: vec![TxIn {
                    prev_output: point,
                    unlock_script: Script::new(),
                    sequence,
                }],
                outputs: vec![TxOut {
                    satoshis: 900,
                    lock_script: Script(vec![OP_1]),
                }],
                lock_time,
            };
            let ctx = ChronicleValidationContext {
                block_height: height,
                activation_height: 200,
                utxo_heights: &heights,
                script_num_policy_limit: None,
            };
            let result = tx.validate_with_chronicle_context(true, true, &utxos, &old, &ctx, true);
            assert_eq!(
                result.is_ok(),
                !pregenesis || old_ok,
                "{label}: height={height}, created={created}, pregenesis={pregenesis}: {result:?}"
            );
        }
    }
}

#[test]
fn mixed_inputs_do_not_share_chronicle_utxo_flags() {
    let a = OutPoint {
        hash: Hash256([51; 32]),
        index: 0,
    };
    let b = OutPoint {
        hash: Hash256([52; 32]),
        index: 0,
    };
    let utxos = LinkedHashMap::from_iter([
        (
            a.clone(),
            TxOut {
                satoshis: 1000,
                lock_script: Script(vec![OP_1]),
            },
        ),
        (
            b.clone(),
            TxOut {
                satoshis: 1000,
                lock_script: Script(vec![OP_1, OP_2MUL, OP_2, OP_EQUAL]),
            },
        ),
    ]);
    for reverse in [false, true] {
        for (created, expected) in [(199, false), (200, true)] {
            let points = if reverse {
                vec![b.clone(), a.clone()]
            } else {
                vec![a.clone(), b.clone()]
            };
            let tx = Tx {
                version: 1,
                inputs: points
                    .into_iter()
                    .map(|prev_output| TxIn {
                        prev_output,
                        unlock_script: Script::new(),
                        sequence: u32::MAX,
                    })
                    .collect(),
                outputs: vec![TxOut {
                    satoshis: 1900,
                    lock_script: Script(vec![OP_1]),
                }],
                lock_time: 0,
            };
            let heights = HashMap::from([(a.clone(), 100), (b.clone(), created)]);
            let ctx = ChronicleValidationContext {
                block_height: 200,
                activation_height: 200,
                utxo_heights: &heights,
                script_num_policy_limit: None,
            };
            assert_eq!(
                tx.validate_with_chronicle_context(true, true, &utxos, &HashSet::new(), &ctx, true)
                    .is_ok(),
                expected,
                "reverse={reverse},created={created}"
            );
        }
    }
}
