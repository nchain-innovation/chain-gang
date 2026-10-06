//! chain-gang's digests checked against bitcoin-sv's own `sighash.json` (CS-491).
//!
//! The hand-written sighash tests compare against values the author derived from
//! reading the consensus rules. When that reading is wrong, code and test are
//! wrong together and the test still passes — which is how CS-483 reached
//! testnet, and how the two divergences recorded below survived. These vectors
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
//! # What this test asserts
//!
//! Every row is classified before it is run. A row the classifier calls clean
//! must reproduce the node's digest exactly. A row it predicts will diverge is
//! counted, and the counts are pinned, so neither fixing a divergence nor
//! widening one can pass unnoticed. A row that diverges without a prediction,
//! or matches against one, fails the test and names itself.
//!
//! The classifier calls the same `find_all_occurances_of` the implementation
//! uses, so it cannot drift from it.

use super::*;
use crate::script::op_codes::OP_CODESEPARATOR;
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

/// Which digest algorithm a column exercises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Algorithm {
    /// `SignatureHashBIP143`.
    Bip143,
    /// `SignatureHashOriginal`, the Original Transaction Digest Algorithm.
    Otda,
}

/// A known reason chain-gang disagrees with the node.
///
/// Each is a real defect, not a property of the vectors. They are recorded here
/// rather than fixed because each changes consensus behaviour and belongs in its
/// own change, with its own release note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Divergence {
    /// The script code holds an `OP_CODESEPARATOR` but no `OP_CHECKSIG`, so
    /// `extract_subscript` rejects it. The node has no such requirement: it
    /// deletes the separators and hashes what is left. A script ending in
    /// `OP_CHECKMULTISIG` reaches this in ordinary use. CS-492.
    SeparatorWithoutChecksig,
    /// The script code holds an `OP_CODESEPARATOR` before the `OP_CHECKSIG`
    /// that `extract_subscript` selects, so chain-gang truncates there. The
    /// node does not truncate at all: its interpreter tracks `pbegincodehash`
    /// and passes the subscript in already cut. CS-492.
    TruncatesAtSeparator,
    /// `SignatureHashBIP143` serializes the script code as it is given, with no
    /// `FindAndDelete`. chain-gang routes the BIP-143 path through
    /// `extract_subscript`, which strips every `OP_CODESEPARATOR` first.
    Bip143StripsSeparators,
    /// Under `SIGHASH_SINGLE` the node blanks every output *except* the one at
    /// `nIn` (`SerializeOutput` in the node's `interpreter.cpp`). chain-gang
    /// blanks *only* the one at `nIn` — the condition is inverted — so every
    /// `SIGHASH_SINGLE` digest on this path is wrong.
    SingleBlanksWrongOutputs,
}

/// The script code the node hashes, given the one in the vector.
///
/// `SignatureHashBIP143` serializes it untouched. `SignatureHashOriginal` runs
/// it through `CTransactionSignatureSerializer`, which deletes every
/// `OP_CODESEPARATOR` and truncates nothing — the interpreter has already
/// handed it the subscript it wants signed.
fn node_subscript(script_code: &[u8], algorithm: Algorithm) -> Vec<u8> {
    if algorithm == Algorithm::Bip143 {
        return script_code.to_vec();
    }
    let mut out = Vec::with_capacity(script_code.len());
    let mut i = 0;
    while i < script_code.len() {
        let next = next_op(i, script_code);
        if script_code[i] != OP_CODESEPARATOR {
            out.extend_from_slice(&script_code[i..next]);
        }
        i = next;
    }
    out
}

/// Predicts how chain-gang will treat a row, or `None` if it should agree.
///
/// Rather than restating `extract_subscript`'s rules — which is how the
/// hand-written tests went wrong in the first place — this asks it directly and
/// compares what it produces against what the node would hash. Precedence
/// follows the order the implementation reaches each defect: the subscript is
/// built first and can fail, so its cases come before the output-serialization
/// one.
fn divergence(script_code: &[u8], hash_type: u32, algorithm: Algorithm) -> Option<Divergence> {
    match extract_subscript(script_code, 0) {
        // The only error reachable here: a script holding a separator but no
        // OP_CHECKSIG for `checksig_index` 0 to select.
        Err(_) => return Some(Divergence::SeparatorWithoutChecksig),
        Ok(ours) if ours != node_subscript(script_code, algorithm) => {
            return Some(match algorithm {
                Algorithm::Bip143 => Divergence::Bip143StripsSeparators,
                Algorithm::Otda => Divergence::TruncatesAtSeparator,
            })
        }
        Ok(_) => {}
    }

    // BIP-143 hashes the single output correctly; only the original algorithm
    // has the inverted condition.
    if algorithm == Algorithm::Otda && (hash_type & 31) == u32::from(SIGHASH_SINGLE) {
        return Some(Divergence::SingleBlanksWrongOutputs);
    }

    None
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

/// Results for one column.
#[derive(Default)]
struct Tally {
    matched: usize,
    diverged: Vec<Divergence>,
    unexplained: Vec<String>,
}

impl Tally {
    fn record(
        &mut self,
        v: &Vector,
        algorithm: Algorithm,
        computed: Result<Hash256, ChainGangError>,
        expected: &str,
        column: &str,
    ) {
        let predicted = divergence(&v.script_code, v.hash_type, algorithm);
        let agrees = matches!(&computed, Ok(hash) if hash.encode() == expected);

        match (agrees, predicted) {
            (true, None) => self.matched += 1,
            (false, Some(d)) => self.diverged.push(d),
            (true, Some(d)) => self.unexplained.push(format!(
                "line {}: {column} matched the node although {d:?} was predicted",
                v.line
            )),
            (false, None) => {
                let got = match &computed {
                    Ok(hash) => hash.encode(),
                    Err(e) => format!("error: {e}"),
                };
                self.unexplained.push(format!(
                    "line {}: {column} is {got} but the node says {expected} \
                     (hash_type {:#010x}, script {})",
                    v.line,
                    v.hash_type,
                    hex::encode(&v.script_code)
                ));
            }
        }
    }

    fn count(&self, d: Divergence) -> usize {
        self.diverged.iter().filter(|got| **got == d).count()
    }
}

/// Every row of bitcoin-sv's `sighash.json`, through both digest algorithms.
#[test]
fn bitcoin_sv_sighash_vectors() {
    let Some(vectors) = load() else { return };
    assert_eq!(vectors.len(), 1000, "unexpected vector count");

    let mut regular = Tally::default();
    let mut original = Tally::default();

    for v in &vectors {
        // Column 5: the node's SignatureHash, amount zero. Runs through the same
        // dispatch the public `sighash` uses.
        let mut cache = SigHashCache::new();
        let computed = sighash_checksig_index_u32(
            &v.tx,
            v.n_input,
            &v.script_code,
            0,
            0,
            v.hash_type,
            &mut cache,
        );
        let algorithm = if uses_bip143(v.hash_type) {
            Algorithm::Bip143
        } else {
            Algorithm::Otda
        };
        regular.record(v, algorithm, computed, &v.expected_regular, "regular");

        // Column 6: the node's SignatureHashOriginal, whatever the hash type says.
        let computed = otda_sighash(&v.tx, v.n_input, &v.script_code, 0, v.hash_type);
        original.record(
            v,
            Algorithm::Otda,
            computed,
            &v.expected_original,
            "original",
        );
    }

    for tally in [&regular, &original] {
        assert!(
            tally.unexplained.is_empty(),
            "{} rows are not explained by a known divergence:\n{}",
            tally.unexplained.len(),
            tally.unexplained.join("\n")
        );
    }

    // Pinned. These are not targets; they are the measured size of three open
    // defects. Fixing one fails this test, which is the point — the numbers are
    // how a fix proves itself.
    assert_eq!(regular.matched, 708, "regular digests reproducing the node");
    assert_eq!(
        original.matched, 731,
        "original digests reproducing the node"
    );

    assert_eq!(
        regular.count(Divergence::SeparatorWithoutChecksig),
        230,
        "CS-492: separated script with no OP_CHECKSIG, regular column"
    );
    assert_eq!(
        regular.count(Divergence::Bip143StripsSeparators),
        38,
        "BIP-143 strips separators the node keeps"
    );
    assert_eq!(
        regular.count(Divergence::TruncatesAtSeparator),
        5,
        "CS-492: truncation at a separator, regular column"
    );
    assert_eq!(
        regular.count(Divergence::SingleBlanksWrongOutputs),
        19,
        "SIGHASH_SINGLE blanks the wrong outputs, regular column"
    );

    assert_eq!(
        original.count(Divergence::SeparatorWithoutChecksig),
        230,
        "CS-492: separated script with no OP_CHECKSIG, original column"
    );
    assert_eq!(
        original.count(Divergence::Bip143StripsSeparators),
        0,
        "the original column never takes the BIP-143 path"
    );
    assert_eq!(
        original.count(Divergence::TruncatesAtSeparator),
        8,
        "CS-492: truncation at a separator, original column"
    );
    assert_eq!(
        original.count(Divergence::SingleBlanksWrongOutputs),
        31,
        "SIGHASH_SINGLE blanks the wrong outputs, original column"
    );

    // Nothing diverges for a reason outside the classification.
    assert_eq!(
        regular.matched + regular.diverged.len(),
        1000,
        "every regular row is either reproduced or classified"
    );
    assert_eq!(
        original.matched + original.diverged.len(),
        1000,
        "every original row is either reproduced or classified"
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
        let internal = sighash_checksig_index_u32(
            &v.tx,
            v.n_input,
            &v.script_code,
            0,
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
