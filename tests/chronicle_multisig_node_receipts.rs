//! Replay current-transaction cases actually admitted/rejected and mined or
//! submitted as blocks to BSV Node v1.2.3 in Apple Container.
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
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("data/chronicle_multisig_node.json")).unwrap();
    assert_eq!(cases.len(), 81);
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
        assert_eq!(
            tx.validate(true, true, &utxos, &HashSet::new()).is_ok(),
            case["policy"].as_bool().unwrap(),
            "policy: {}",
            case["name"]
        );
        assert_eq!(
            tx.validate_consensus(true, true, &utxos, &HashSet::new())
                .is_ok(),
            case["consensus"].as_bool().unwrap(),
            "consensus: {}",
            case["name"]
        );
    }
}
