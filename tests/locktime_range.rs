//! BSV v1.2.3 accepted and mined these full-width locktimes. Finality is
//! contextual; stateless transaction/script validation must not narrow u32.
use chain_gang::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    script::{op_codes::OP_1, Script},
    util::Hash256,
};
use linked_hash_map::LinkedHashMap;
use std::collections::HashSet;
#[test]
fn full_uint32_locktime_is_valid_transaction_data() {
    let point = OutPoint {
        hash: Hash256([1; 32]),
        index: 0,
    };
    let mut utxos = LinkedHashMap::new();
    utxos.insert(
        point.clone(),
        TxOut {
            satoshis: 1000,
            lock_script: Script(vec![OP_1]),
        },
    );
    for lock_time in [
        0,
        499_999_999,
        500_000_000,
        0x7fff_ffff,
        0x8000_0000,
        u32::MAX,
    ] {
        for sequence in [0, 1, 0xffff_fffe, u32::MAX] {
            let tx = Tx {
                version: 2,
                inputs: vec![TxIn {
                    prev_output: point.clone(),
                    unlock_script: Script::new(),
                    sequence,
                }],
                outputs: vec![TxOut {
                    satoshis: 900,
                    lock_script: Script(vec![OP_1]),
                }],
                lock_time,
            };
            assert!(
                tx.validate_consensus(true, true, &utxos, &HashSet::new())
                    .is_ok(),
                "{lock_time:#x}/{sequence:#x}"
            );
            assert!(
                tx.validate(true, true, &utxos, &HashSet::new()).is_ok(),
                "{lock_time:#x}/{sequence:#x}"
            );
        }
    }
}
