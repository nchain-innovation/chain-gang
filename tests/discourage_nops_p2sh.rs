use chain_gang::messages::{OutPoint, Tx, TxIn, TxOut};
use chain_gang::script::{op_codes::*, Script};
use chain_gang::util::{hash160, ChainGangError, Hash256};
use linked_hash_map::LinkedHashMap;
use std::collections::HashSet;

#[test]
fn policy_reaches_p2sh_redeem_script_without_enabling_other_policy() {
    for opcode in [OP_SUBSTR, OP_LEFT, OP_RIGHT, OP_LSHIFTNUM, OP_RSHIFTNUM] {
        for version in [1, 2] {
            for consensus_only in [false, true] {
                let redeem = [opcode, OP_1];
                let mut locking = vec![OP_HASH160, 20];
                locking.extend_from_slice(&hash160(&redeem).0);
                locking.push(OP_EQUAL);
                let mut unlocking = Script::new();
                unlocking.append_data(&redeem);
                let point = OutPoint {
                    hash: Hash256([0x11; 32]),
                    index: 0,
                };
                let utxos = LinkedHashMap::from_iter([(
                    point.clone(),
                    TxOut {
                        satoshis: 1000,
                        lock_script: Script(locking),
                    },
                )]);
                let mut pregenesis = HashSet::new();
                pregenesis.insert(point.clone());
                let tx = Tx {
                    version,
                    inputs: vec![TxIn {
                        prev_output: point,
                        unlock_script: unlocking,
                        sequence: u32::MAX,
                    }],
                    outputs: vec![TxOut {
                        satoshis: 900,
                        lock_script: Script(vec![OP_1]),
                    }],
                    lock_time: 0,
                };
                let off = tx.validate_with_nop_policy(
                    true,
                    true,
                    &utxos,
                    &pregenesis,
                    consensus_only,
                    false,
                );
                assert!(
                    off.is_ok(),
                    "opcode={opcode:#x} version={version} consensus={consensus_only}: {off:?}"
                );
                let on = tx.validate_with_nop_policy(
                    true,
                    true,
                    &utxos,
                    &pregenesis,
                    consensus_only,
                    true,
                );
                assert!(
                    matches!(on, Err(ChainGangError::ScriptError(ref m)) if m == "discouraged upgradeable NOP"),
                    "opcode={opcode:#x} version={version} consensus={consensus_only}: {on:?}"
                );
            }
        }
    }
}
