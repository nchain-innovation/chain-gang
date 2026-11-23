//! Replay current-transaction cases actually admitted/rejected and mined or
//! submitted as blocks to BSV Node v1.2.3 in Apple Container.
//! These 32 cases exercise serialized-signature cleanup independently of the
//! separate signature-encoding and exceptional-sighash changes in PR #250.
use chain_gang::{
    messages::{OutPoint, Tx},
    util::Serializable,
};
use linked_hash_map::LinkedHashMap;
use serde_json::Value;
use std::collections::HashSet;
fn parse(s: &str) -> Tx {
    Tx::read(&mut hex::decode(s).unwrap().as_slice()).unwrap()
}
#[test]
fn current_multisig_node_receipts() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/chronicle_multisig_node.json");
    if !path.exists() {
        assert_eq!(
            std::env::var("CHAIN_GANG_VECTORS_OPTIONAL").as_deref(),
            Ok("1")
        );
        return;
    }
    let cases: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(cases.len(), 32);
    let mut failures = Vec::new();
    for case in cases {
        let tx = parse(case["tx"].as_str().unwrap());
        let mut utxos = LinkedHashMap::new();
        for parent in case["parents"].as_array().unwrap() {
            let p = parse(parent.as_str().unwrap());
            for (index, output) in p.outputs.iter().enumerate() {
                utxos.insert(
                    OutPoint {
                        hash: p.hash(),
                        index: index as u32,
                    },
                    output.clone(),
                );
            }
        }
        let policy = tx.validate(true, true, &utxos, &HashSet::new()).is_ok();
        let consensus = tx
            .validate_consensus(true, true, &utxos, &HashSet::new())
            .is_ok();
        if policy != case["policy"].as_bool().unwrap()
            || consensus != case["consensus"].as_bool().unwrap()
        {
            failures.push(case["name"].as_str().unwrap().to_string());
        }
    }
    assert!(failures.is_empty(), "mismatched cases: {failures:?}");
}
