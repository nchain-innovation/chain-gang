use chain_gang::chronicle::ChronicleValidationContext;
use chain_gang::messages::{OutPoint, Tx, TxIn, TxOut};
use chain_gang::script::{op_codes::*, Script};
use chain_gang::util::{ChainGangError, Hash256};
use linked_hash_map::LinkedHashMap;
use std::collections::{HashMap, HashSet};

fn lock(opcode: u8) -> Script {
    let mut bytes = if opcode == OP_SUBSTR {
        vec![1, b'a', OP_0, OP_1]
    } else if opcode == OP_LEFT || opcode == OP_RIGHT {
        vec![1, b'a', OP_1]
    } else {
        vec![OP_1, OP_1]
    };
    bytes.extend([opcode, OP_DROP, OP_1]);
    Script(bytes)
}
fn input(point: &OutPoint, script: Script) -> TxIn {
    TxIn {
        prev_output: point.clone(),
        unlock_script: script,
        sequence: u32::MAX,
    }
}
fn verify(
    tx: &Tx,
    utxos: &LinkedHashMap<OutPoint, TxOut>,
    heights: &HashMap<OutPoint, u64>,
    flag: bool,
) -> Result<(), ChainGangError> {
    let context = ChronicleValidationContext::new(200, 200, heights, None);
    tx.validate_with_chronicle_context_and_nop_policy(
        true,
        true,
        utxos,
        &HashSet::new(),
        &context,
        true,
        flag,
    )
}
#[test]
fn mixed_output_eras_follow_each_inputs_height() {
    for opcode in [OP_SUBSTR, OP_LEFT, OP_RIGHT, OP_LSHIFTNUM, OP_RSHIFTNUM] {
        for version in [1, 2] {
            let points = [
                OutPoint {
                    hash: Hash256([1; 32]),
                    index: 0,
                },
                OutPoint {
                    hash: Hash256([2; 32]),
                    index: 0,
                },
            ];
            let utxos = LinkedHashMap::from_iter(points.iter().cloned().map(|p| {
                (
                    p,
                    TxOut {
                        satoshis: 1000,
                        lock_script: lock(opcode),
                    },
                )
            }));
            for old_index in [0, 1] {
                let mut heights = HashMap::from_iter(points.iter().cloned().map(|p| (p, 200)));
                heights.insert(points[old_index].clone(), 199);
                let tx = Tx {
                    version,
                    inputs: points.iter().map(|p| input(p, Script::new())).collect(),
                    outputs: vec![TxOut {
                        satoshis: 1000,
                        lock_script: Script(vec![OP_1]),
                    }],
                    lock_time: 0,
                };
                assert!(
                    verify(&tx, &utxos, &heights, false).is_ok(),
                    "opcode {opcode:02x} version {version} old {old_index} flag false"
                );
                let actual = verify(&tx, &utxos, &heights, true);
                assert!(
                    matches!(actual, Err(ChainGangError::ScriptError(ref s)) if s == "discouraged upgradeable NOP"),
                    "opcode {opcode:02x} version {version} old {old_index}: {actual:?}"
                );
                heights.insert(points[old_index].clone(), 200);
                assert!(
                    verify(&tx, &utxos, &heights, true).is_ok(),
                    "both new opcode {opcode:02x} version {version}"
                );
            }
        }
    }
}
#[test]
fn branch_return_does_not_execute_following_nops() {
    for opcode in [OP_SUBSTR, OP_LEFT, OP_RIGHT, OP_LSHIFTNUM, OP_RSHIFTNUM] {
        for version in [1, 2] {
            let point = OutPoint {
                hash: Hash256([3; 32]),
                index: 0,
            };
            let script = Script(vec![OP_1, OP_1, OP_IF, OP_RETURN, opcode, OP_ENDIF]);
            let utxos = LinkedHashMap::from_iter([(
                point.clone(),
                TxOut {
                    satoshis: 1000,
                    lock_script: script,
                },
            )]);
            let tx = Tx {
                version,
                inputs: vec![input(&point, Script::new())],
                outputs: vec![TxOut {
                    satoshis: 900,
                    lock_script: Script(vec![OP_1]),
                }],
                lock_time: 0,
            };
            for height in [199, 200] {
                for flag in [false, true] {
                    let heights = HashMap::from([(point.clone(), height)]);
                    let actual = verify(&tx, &utxos, &heights, flag);
                    assert!(actual.is_ok(), "opcode {opcode:02x} version {version} height {height} flag {flag}: {actual:?}");
                }
            }
        }
    }
}

#[test]
fn legacy_context_defaults_select_the_previous_nop_policy() {
    let point = OutPoint {
        hash: Hash256([4; 32]),
        index: 0,
    };
    let utxos = LinkedHashMap::from_iter([(
        point.clone(),
        TxOut {
            satoshis: 1000,
            lock_script: Script(vec![OP_SUBSTR, OP_1]),
        },
    )]);
    let heights = HashMap::from([(point.clone(), 199)]);
    let context = ChronicleValidationContext::new(200, 200, &heights, None);
    for version in [1, 2] {
        let tx = Tx {
            version,
            inputs: vec![input(&point, Script::new())],
            outputs: vec![TxOut {
                satoshis: 900,
                lock_script: Script(vec![OP_1]),
            }],
            lock_time: 0,
        };
        let consensus =
            tx.validate_with_chronicle_context(true, true, &utxos, &HashSet::new(), &context, true);
        assert!(consensus.is_ok(), "version {version}: {consensus:?}");
        let policy = tx.validate_with_chronicle_context(
            true,
            true,
            &utxos,
            &HashSet::new(),
            &context,
            false,
        );
        assert!(
            matches!(policy, Err(ChainGangError::ScriptError(ref s)) if s == "discouraged upgradeable NOP"),
            "version {version}: {policy:?}"
        );
    }
}
