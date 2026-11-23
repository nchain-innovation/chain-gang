//! Stateless transaction validation accepts the full unsigned nLockTime range.
//! Block and mempool finality are evaluated separately by the node.
use chain_gang::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    network::Network,
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
            assert!(
                tx.validate_at_height(
                    true,
                    true,
                    &utxos,
                    &HashSet::new(),
                    967_896,
                    Network::BSV_Mainnet,
                )
                .is_ok(),
                "{lock_time:#x}/{sequence:#x}"
            );
            assert!(
                tx.validate_at_height_consensus(
                    true,
                    true,
                    &utxos,
                    &HashSet::new(),
                    967_896,
                    Network::BSV_Mainnet,
                )
                .is_ok(),
                "{lock_time:#x}/{sequence:#x}"
            );
        }
    }

    let tx = Tx {
        version: 2,
        inputs: vec![TxIn {
            prev_output: point,
            unlock_script: Script(vec![]),
            sequence: 0,
        }],
        outputs: vec![TxOut {
            satoshis: 1100,
            lock_script: Script(vec![OP_1]),
        }],
        lock_time: u32::MAX,
    };
    assert!(tx.validate(true, true, &utxos, &HashSet::new()).is_err());
    assert!(tx
        .validate_consensus(true, true, &utxos, &HashSet::new())
        .is_err());
}
