use chain_gang::{
    chronicle::*,
    messages::{OutPoint, Tx, TxIn, TxOut},
    script::{op_codes::*, Script},
    util::Hash256,
};
use linked_hash_map::LinkedHashMap;
use serde_json::json;
use std::collections::{HashMap, HashSet};
#[test]
fn script_number_era_boundaries() {
    let mut rows = vec![];
    // Small serialized scripts construct large operands inside the interpreter.
    // Enable the 32 MB edge cases explicitly for live-node/large-memory checks.
    let large = std::env::var("CHRONICLE_LARGE_BOUNDARIES").as_deref() == Ok("1");
    for post in [false, true] {
        for version in [1, 2] {
            let limit = if post { 32_000_000usize } else { 750_000 };
            let lengths = if post && !large {
                vec![750_001]
            } else {
                vec![limit - 1, limit, limit + 1]
            };
            for length in lengths {
                let mut lock = Script::new();
                lock.append(OP_0);
                lock.append_num((length - 1) as i32).unwrap();
                lock.append(OP_NUM2BIN);
                lock.append(OP_1);
                lock.append(OP_CAT);
                lock.append(OP_BIN2NUM);
                lock.append(OP_DROP);
                lock.append(OP_1);
                let point = OutPoint {
                    hash: Hash256([3; 32]),
                    index: 0,
                };
                let mut heights = HashMap::new();
                heights.insert(point.clone(), if post { 200 } else { 100 });
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
                        lock_script: lock.clone(),
                    },
                );
                let ctx = ChronicleValidationContext {
                    block_height: 200,
                    activation_height: 200,
                    utxo_heights: &heights,
                    script_num_policy_limit: None,
                };
                let got = tx.validate_with_chronicle_context(
                    true,
                    true,
                    &utxos,
                    &HashSet::new(),
                    &ctx,
                    true,
                );
                let library = got.is_ok();
                let expected = length <= limit;
                let name = format!("number-new{post}-v{version}-length{length}");
                assert_eq!(library, expected, "{name}: {got:?}");
                rows.push(json!({"name":name,"library":library,"expected":expected,"request":{"tx":tx.as_hexstr(),"n":0,"flags":(1u32<<18)|(1<<19)|(1<<20)|if post{1<<21}else{0},"reportflags":true,"txo":{"lock":hex::encode(&lock.0),"value":1}}}));
            }
        }
    }
    if let Ok(path) = std::env::var("CHRONICLE_DIFF_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    }
}
