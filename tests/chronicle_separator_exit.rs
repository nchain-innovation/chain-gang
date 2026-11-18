//! Replay current Chronicle transactions and their committed parents from
//! Apple Container's BSV Node v1.2.3 admission and block validation.
use chain_gang::{
    messages::{OutPoint, Tx},
    util::Serializable,
};
use linked_hash_map::LinkedHashMap;
use serde_json::Value;
use std::collections::HashSet;

fn parse(hex: &str) -> Tx {
    Tx::read(&mut hex::decode(hex).unwrap().as_slice()).unwrap()
}

#[test]
fn current_chronicle_separator_and_early_exit_node_cases() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("data/chronicle_separator_exit.json")).unwrap();
    assert_eq!(cases.len(), 47);
    let mut mismatches = Vec::new();
    for case in cases {
        let txn = parse(case["tx"].as_str().unwrap());
        let parent = parse(case["parents"][0].as_str().unwrap());
        let utxos = LinkedHashMap::from_iter([(
            OutPoint {
                hash: parent.hash(),
                index: 0,
            },
            parent.outputs[0].clone(),
        )]);
        let expected = case["accepted"].as_bool().unwrap();
        let pregenesis = HashSet::new();
        for (mode, result) in [
            (
                "consensus",
                txn.validate_consensus(true, true, &utxos, &pregenesis),
            ),
            ("policy", txn.validate(true, true, &utxos, &pregenesis)),
        ] {
            if result.is_ok() != expected {
                mismatches.push(format!(
                    "{} ({mode}): expected {expected}, got {result:?}",
                    case["name"]
                ));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
