//! Full P-256 companion spends reproduced from bsv-script-lab f8f184b.
//! Both fail with #240 alone and pass on BSV Node v1.2.3. Preserve the complete
//! application script alongside the smaller commitment-matrix regressions.
use chain_gang::{
    messages::{OutPoint, Tx},
    util::Serializable,
};
use linked_hash_map::LinkedHashMap;
use serde_json::Value;
use std::collections::HashSet;
fn read_tx(hexstr: &str) -> Tx {
    Tx::read(&mut hex::decode(hexstr).unwrap().as_slice()).unwrap()
}
#[test]
fn complete_companion_spends() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("data/otda_companion.json")).unwrap();
    for case in cases {
        let tx = read_tx(case["request"]["tx"].as_str().unwrap());
        let mut utxos = LinkedHashMap::new();
        for p in case["parents"].as_array().unwrap() {
            let parent = read_tx(p.as_str().unwrap());
            for (index, out) in parent.outputs.iter().enumerate() {
                utxos.insert(
                    OutPoint {
                        hash: parent.hash(),
                        index: index as u32,
                    },
                    out.clone(),
                );
            }
        }
        let validate = |tx: &Tx| tx.validate_consensus(true, true, &utxos, &HashSet::new());
        assert!(validate(&tx).is_ok(), "{}", case["flag"]);
        // Unpaired SINGLE does not commit to the existing unrelated output.
        let mut changed = tx.clone();
        changed.outputs[0].satoshis -= 1;
        assert!(validate(&changed).is_ok());
        // Adding the paired output switches the digest away from the constant. The
        // existing authorization cannot become a valid paired spend.
        changed.outputs.push(tx.outputs[0].clone());
        assert!(validate(&changed).is_err());
        // The companion authorization binds locktime and input identity even though
        // the OTDA constant alone does not commit to them.
        let mut changed = tx.clone();
        changed.lock_time += 1;
        assert!(validate(&changed).is_err());
    }
}
