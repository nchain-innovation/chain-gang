//! bitcoin-sv's own `OP_CODESEPARATOR` script tests, run through the interpreter.
//!
//! `tests/data/script_tests_codeseparator.json` holds, verbatim, the seven rows
//! of bitcoin-sv's `src/test/data/script_tests.json` that mention
//! `CODESEPARATOR` (see `tests/data/README.md`). Six carry real signatures over
//! the node test framework's own crediting and spending transactions, so each
//! row says what the node concludes when it executes the script: where every
//! check's script code starts, and whether the signature over it is good.
//!
//! These exercise verification — the interpreter tracking the executed
//! separator and the digest over what it builds — not signing, since the
//! signatures are given. That is still what CS-492's fix rests on: the
//! signer's cut is checked against the interpreter in `sighash.rs`, so this is
//! the node's own data confirming that oracle.
//!
//! Each row runs the way the node's `DoTest` runs it: `VerifyScript` with a
//! signature checker over the spending transaction, not full transaction
//! validation. The difference shows in the first row, whose unlocking script is
//! `NOP`. The node enforces push-only unlocking scripts only under the
//! `SIGPUSHONLY` flag, which that row does not set, so it is valid there.
//! `Tx::validate` applies push-only as a transaction rule regardless and
//! rejects it, which is a statement about flags, not separators.
//!
//! The whole of `script_tests.json` is run too, by `tests/script_vectors.rs`,
//! through `Tx::validate`. These seven stay here because they say more than
//! accept or reject: where each check's script code starts.

use chain_gang::messages::{OutPoint, Tx, TxIn, TxOut};
use chain_gang::script::op_codes::*;
use chain_gang::script::{Script, TransactionChecker, PREGENESIS_RULES};
use chain_gang::transaction::sighash::SigHashCache;
use chain_gang::util::Hash256;
use std::path::PathBuf;

/// Set to `1` to skip when the vendored data is absent, as it is in a crate
/// unpacked from crates.io. See `src/transaction/sighash_vectors.rs`.
const OPTIONAL_ENV: &str = "CHAIN_GANG_VECTORS_OPTIONAL";

/// The node's `ParseScript`, for the tokens these rows use: decimals push
/// minimally (`1` is `OP_1`), `0x..` is raw bytes inserted as-is, and names are
/// opcodes. Anything else panics rather than being read wrongly.
fn parse_script(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for word in s.split_whitespace() {
        if let Some(hex) = word.strip_prefix("0x") {
            out.extend(hex::decode(hex).expect("hex token"));
            continue;
        }
        if let Ok(n) = word.parse::<i64>() {
            out.push(match n {
                0 => OP_0,
                -1 => OP_1NEGATE,
                1..=16 => OP_1 + (n as u8 - 1),
                _ => panic!("numeric push {n} is not needed by these rows; add it before using it"),
            });
            continue;
        }
        out.push(match word.trim_start_matches("OP_") {
            "NOP" => OP_NOP,
            "VERIFY" => OP_VERIFY,
            "CODESEPARATOR" => OP_CODESEPARATOR,
            "CHECKSIG" => OP_CHECKSIG,
            "CHECKSIGVERIFY" => OP_CHECKSIGVERIFY,
            other => panic!("opcode {other} is not needed by these rows; add it before using it"),
        });
    }
    out
}

/// The node test framework's `BuildCreditingTransaction`: a coinbase-shaped
/// transaction whose input pushes two zeros and whose one output, of
/// `n_value`, is locked by `lock_script`.
fn crediting_tx(lock_script: &[u8], n_value: i64) -> Tx {
    Tx {
        version: 1,
        inputs: vec![TxIn {
            prev_output: OutPoint {
                hash: Hash256([0; 32]),
                index: 0xffffffff,
            },
            unlock_script: Script(vec![OP_0, OP_0]),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis: n_value,
            lock_script: Script(lock_script.to_vec()),
        }],
        lock_time: 0,
    }
}

/// `BuildSpendingTransaction`: spends the crediting output with `unlock_script`
/// into one output of the same value with an empty locking script.
fn spending_tx(credit: &Tx, unlock_script: &[u8], version: u32) -> Tx {
    Tx {
        version,
        inputs: vec![TxIn {
            prev_output: OutPoint {
                hash: credit.hash(),
                index: 0,
            },
            unlock_script: Script(unlock_script.to_vec()),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis: credit.outputs[0].satoshis,
            lock_script: Script(vec![]),
        }],
        lock_time: 0,
    }
}

#[test]
fn bitcoin_sv_codeseparator_script_tests() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("script_tests_codeseparator.json");
    if !path.exists() {
        assert!(
            std::env::var(OPTIONAL_ENV).as_deref() == Ok("1"),
            "{} is missing; set {OPTIONAL_ENV}=1 to skip vendored vectors",
            path.display()
        );
        return;
    }
    let rows: Vec<Vec<serde_json::Value>> =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(rows.len(), 7);

    let mut disagreements = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        // [txn version, scriptSig, scriptPubKey, flags, expected_scripterror, comment?]
        let field = |n: usize| row[n].as_str().unwrap();
        let version: u32 = field(0).parse().unwrap();
        let (unlock, lock, expected) = (parse_script(field(1)), parse_script(field(2)), field(4));
        let comment = row.get(5).and_then(|c| c.as_str()).unwrap_or("");

        let credit = crediting_tx(&lock, 0);
        let spend = spending_tx(&credit, &unlock, version);

        // VerifyScript: the unlocking script, then the locking script, checked
        // against the spending transaction. No SIGHASH_FORKID flag, and the
        // signatures do not carry it; no UTXO_AFTER_GENESIS flag, so the output
        // predates Genesis.
        let mut cache = SigHashCache::new();
        let mut checker = TransactionChecker {
            tx: &spend,
            sig_hash_cache: &mut cache,
            input: 0,
            satoshis: credit.outputs[0].satoshis,
            require_sighash_forkid: false,
            script_tx_version: None,
        };
        let mut script = Script::new();
        script.append_slice(&unlock);
        script.append(OP_CODESEPARATOR);
        script.append_slice(&lock);
        let result = script.eval(&mut checker, PREGENESIS_RULES);
        let node_accepts = expected == "OK";
        if result.is_ok() != node_accepts {
            disagreements.push(format!(
                "row {}: node says {expected}, chain-gang says {result:?} — {comment}",
                i + 1
            ));
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} of 7 rows disagree with the node:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}
