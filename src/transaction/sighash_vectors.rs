//! chain-gang's digests checked against bitcoin-sv's own `sighash.json` (CS-491).
//!
//! The hand-written sighash tests compare against values the author derived from
//! reading the consensus rules. When that reading is wrong, code and test are
//! wrong together and the test still passes — which is how CS-483 reached
//! testnet, and how three more defects survived until these vectors found them
//! (#192, #193, CS-492). These vectors
//! come from the node, so they are an answer chain-gang had no hand in writing.
//!
//! # The file
//!
//! `tests/data/sighash.json`, vendored from bitcoin-sv. Each row is
//!
//! ```text
//! [raw_transaction, script, input_index, hashType, sighash (regular), sighash (no forkid)]
//! ```
//!
//! produced by that repository's `src/test/sighash_tests.cpp` under
//! `PRINT_SIGHASH_JSON`:
//!
//! - column 5 is `SignatureHash(scriptCode, tx, nIn, sigHashType, Amount(0))`,
//!   which is BIP-143 when FORKID is set and CHRONICLE is not, and the original
//!   algorithm otherwise — the same dispatch [`super::uses_bip143`] makes;
//! - column 6 is `SignatureHashOriginal(...)`, always the original algorithm.
//!
//! The amount is zero in both. Expected digests are in `uint256::GetHex()`
//! display order, which is what [`Hash256::encode`] produces.
//!
//! # What these tests assert
//!
//! A row is a script code that has already been cut, handed to the node's digest
//! function. That is exactly what chain-gang's verifier does —
//! [`sighash_from_script_code`] — so through that entry point every row must
//! reproduce the node, in both columns, with nothing excused.
//!
//! The signer's entry point, [`sighash_checksig_index`], asks a different
//! question of the same bytes: it reads them as a whole locking script and cuts
//! them itself, for the selected `OP_CHECKSIG`. Where the cut would start at 0
//! the two questions coincide, and the signer must reproduce the node too. Where
//! a separator precedes the first `OP_CHECKSIG`, the signer rightly cuts and the
//! vector rightly does not, so the row says nothing about the signer; and where
//! there is no `OP_CHECKSIG` it has nothing to select. Those rows are counted,
//! not run.

use super::*;
use crate::script::op_codes::{OP_CHECKSIG, OP_CODESEPARATOR};
use serde_json::Value;
use std::io::Cursor;
use std::path::PathBuf;

/// Set to `1` to downgrade a missing vector file from a failure to a skip.
///
/// `tests/data/` is excluded from the published package — the vectors are under
/// bitcoin-sv's Open BSV License, not this crate's MIT — so a crate unpacked
/// from crates.io has no data to run against. Inside this repository the file is
/// present, and its absence means a broken checkout, which should fail loudly
/// rather than quietly skip.
const OPTIONAL_ENV: &str = "CHAIN_GANG_VECTORS_OPTIONAL";

/// How the signer's entry point would read a row's script code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignerReading {
    /// No separator ahead of the first `OP_CHECKSIG`, so reading it as a
    /// locking script cuts nothing and the signer must give the node's answer.
    SameQuestion,
    /// A separator ahead of the first `OP_CHECKSIG`. The signer starts after it,
    /// which is right for a locking script; the vector's script code is already
    /// cut and the node does not cut again. Different questions.
    SeparatorBeforeCheck,
    /// Separators but no `OP_CHECKSIG`: nothing for `checksig_index` 0 to select.
    NothingToSelect,
}

/// Which of those a script code is. A selection, not a prediction: it decides
/// whether the row asks the signer anything, never what the answer should be.
fn signer_reading(script_code: &[u8]) -> SignerReading {
    let separators = find_all_occurances_of(script_code, OP_CODESEPARATOR);
    if separators.is_empty() {
        return SignerReading::SameQuestion;
    }
    match find_all_occurances_of(script_code, OP_CHECKSIG).first() {
        None => SignerReading::NothingToSelect,
        Some(first) if separators[0] < *first => SignerReading::SeparatorBeforeCheck,
        Some(_) => SignerReading::SameQuestion,
    }
}

/// One row of the file, decoded.
struct Vector {
    /// 1-based line in `sighash.json`, so a failure can be looked up directly.
    line: usize,
    tx: Tx,
    script_code: Vec<u8>,
    n_input: usize,
    hash_type: u32,
    expected_regular: String,
    expected_original: String,
}

/// Reads and decodes the vectors, or `None` when the file is absent and the
/// environment says that is acceptable.
fn load() -> Option<Vec<Vector>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("sighash.json");

    if !path.exists() {
        assert!(
            std::env::var(OPTIONAL_ENV).as_deref() == Ok("1"),
            "{} is missing. It is vendored from bitcoin-sv and excluded from the \
             published package; set {OPTIONAL_ENV}=1 to skip these vectors when \
             testing an unpacked crate.",
            path.display(),
        );
        eprintln!("skipping sighash vectors: {} is absent", path.display());
        return None;
    }

    let text = std::fs::read_to_string(&path).expect("read sighash.json");
    let rows: Vec<Value> = serde_json::from_str(&text).expect("parse sighash.json");

    let mut vectors = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let row = row.as_array().expect("row is an array");
        if row.len() == 1 {
            continue; // the header comment
        }
        assert_eq!(row.len(), 6, "row {i} has {} columns", row.len());

        let raw_tx = hex::decode(row[0].as_str().expect("raw_transaction")).expect("tx hex");
        let script_code = hex::decode(row[1].as_str().expect("script")).expect("script hex");

        vectors.push(Vector {
            line: i + 1,
            tx: Tx::read(&mut Cursor::new(&raw_tx)).expect("deserialize tx"),
            script_code,
            n_input: row[2].as_u64().expect("input_index") as usize,
            // The generator prints `int(nHashType)`, so a value with the top bit
            // set appears negative. Go through i64 to recover the 32 bits.
            hash_type: row[3].as_i64().expect("hashType") as u32,
            expected_regular: row[4].as_str().expect("sighash regular").to_string(),
            expected_original: row[5].as_str().expect("sighash original").to_string(),
        });
    }
    Some(vectors)
}

/// The signer's entry point reproduces the node wherever it is asked the same
/// question — through the same dispatch and widening the public API uses.
#[test]
fn signer_entry_point_agrees_where_the_question_is_the_same() {
    let Some(vectors) = load() else { return };
    assert_eq!(vectors.len(), 1000, "unexpected vector count");

    let selection = ScriptCode::FromLockScript { checksig_index: 0 };
    let mut mismatches = Vec::new();
    let (mut same, mut separator_first, mut nothing) = (0, 0, 0);
    for v in &vectors {
        match signer_reading(&v.script_code) {
            SignerReading::SeparatorBeforeCheck => {
                separator_first += 1;
                continue;
            }
            SignerReading::NothingToSelect => {
                nothing += 1;
                continue;
            }
            SignerReading::SameQuestion => same += 1,
        }
        let mut cache = SigHashCache::new();
        let regular = sighash_u32(
            &v.tx,
            v.n_input,
            &v.script_code,
            selection,
            0,
            v.hash_type,
            &mut cache,
        );
        let original = otda_sighash(&v.tx, v.n_input, &v.script_code, selection, v.hash_type);
        for (column, got, expected) in [
            ("regular", regular, &v.expected_regular),
            ("original", original, &v.expected_original),
        ] {
            match got {
                Ok(hash) if hash.encode() == *expected => {}
                other => mismatches.push(format!("line {}: {column} gave {other:?}", v.line)),
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} digests differ from the node:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );

    // These depend only on the file, not on chain-gang, so pinning them guards
    // the selection: a different file, or a selection that quietly widens, shows
    // up here rather than as a test that checks less than it says.
    assert_eq!(same, 711, "rows asking the signer the node's question");
    assert_eq!(
        separator_first, 59,
        "rows whose separator precedes the first OP_CHECKSIG"
    );
    assert_eq!(nothing, 230, "rows with separators and no OP_CHECKSIG");
}

/// The verifier's entry point reproduces every row, in both columns.
///
/// The vectors give the node's digest functions a script code that has
/// already been cut, which is exactly the contract of `sighash_from_script_code`
/// and of what the interpreter hands `TransactionChecker`. So nothing here may
/// diverge: no truncation to classify, no OP_CHECKSIG to count. The divergences
/// pinned in `bitcoin_sv_sighash_vectors` belong to the signer's entry point,
/// which reads its argument as a whole locking script and cuts it itself.
#[test]
fn verifier_entry_point_reproduces_every_vector() {
    let Some(vectors) = load() else { return };
    let mut mismatches = Vec::new();
    for v in &vectors {
        let mut cache = SigHashCache::new();
        let regular = sighash_u32(
            &v.tx,
            v.n_input,
            &v.script_code,
            ScriptCode::AsGiven,
            0,
            v.hash_type,
            &mut cache,
        );
        let original = otda_sighash(
            &v.tx,
            v.n_input,
            &v.script_code,
            ScriptCode::AsGiven,
            v.hash_type,
        );
        for (column, got, expected) in [
            ("regular", regular, &v.expected_regular),
            ("original", original, &v.expected_original),
        ] {
            match got {
                Ok(hash) if hash.encode() == *expected => {}
                other => mismatches.push(format!("line {}: {column} gave {other:?}", v.line)),
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} of 2000 digests differ from the node:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// The vectors exercise hash types the public API cannot express.
///
/// Every row's `nHashType` is a random 32-bit value, so none fits the `u8` the
/// public [`sighash`] takes. That is not a hole in coverage: a signature carries
/// the sighash type in one trailing byte, so the high bits cannot arise from a
/// real script. It is the reason the rows run through the internal 32-bit entry
/// point, and this records it so the choice is not mistaken for an oversight.
#[test]
fn no_vector_hash_type_fits_the_public_api() {
    let Some(vectors) = load() else { return };
    let expressible = vectors
        .iter()
        .filter(|v| u8::try_from(v.hash_type).is_ok())
        .count();
    assert_eq!(
        expressible, 0,
        "a vector now fits the u8 public API; run it through `sighash` as well"
    );
}

/// Widening to 32 bits left every digest a caller can actually reach unchanged.
///
/// The public API takes a `u8` and widens once. For a hash type that started as
/// one byte the high bits are zero, so the serialized tail is the same four
/// bytes it always was. This checks the two entry points agree rather than
/// assuming it.
#[test]
fn widening_preserves_single_byte_hash_types() {
    let Some(vectors) = load() else { return };
    for v in &vectors {
        let low = (v.hash_type & 0xff) as u8;
        let mut cache = SigHashCache::new();
        let public =
            sighash_checksig_index(&v.tx, v.n_input, &v.script_code, 0, 0, low, &mut cache);
        let mut cache = SigHashCache::new();
        let internal = sighash_u32(
            &v.tx,
            v.n_input,
            &v.script_code,
            ScriptCode::FromLockScript { checksig_index: 0 },
            0,
            u32::from(low),
            &mut cache,
        );
        match (public, internal) {
            (Ok(a), Ok(b)) => assert_eq!(a, b, "line {}", v.line),
            (Err(_), Err(_)) => {}
            (a, b) => panic!("line {}: public {a:?} but internal {b:?}", v.line),
        }
    }
}
