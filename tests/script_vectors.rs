//! bitcoin-sv's `script_tests.json`, every row, run through `Tx::validate`.
//!
//! `tests/data/script_tests.json` is the node's file verbatim (see
//! `tests/data/README.md`). Each row is a locking script, an unlocking script,
//! the consensus flags the node runs it under and the node's verdict. The node
//! evaluates it with `VerifyScript` over its test framework's crediting and
//! spending transactions; this builds the same two transactions and asks
//! chain-gang's transaction validation the same question.
//!
//! chain-gang does not take the node's flags one by one. It has an era
//! (pre- or post-Genesis, per output), a policy/consensus switch and the
//! sighash FORKID requirement, so each row's flags are mapped onto those (see
//! `Mode::from_flags`). Of the 1483 rows, 1335 agree with the node. The rest
//! are listed, each under its reason, in one of two places:
//!
//! - `MODELLING_GAPS`: the verdict turns on a flag chain-gang does not take
//!   one by one, such as a row from before Chronicle or one that switches
//!   off a mandatory signature rule. These say nothing against chain-gang.
//! - `KNOWN_DIFFERENCES`: under the same rules, chain-gang and the node
//!   disagree, and chain-gang appears to be wrong. These are fixes to make.
//!
//! The test fails if any other row disagrees, or if a listed row starts
//! agreeing, so a fix shows up as a row to take off the list.
//!
//! Only accept/reject is compared. The node's rows also name the error, but
//! chain-gang's errors are messages, not the node's error codes, so matching
//! them is left for later.

use chain_gang::messages::{OutPoint, Tx, TxIn, TxOut};
use chain_gang::util::Hash256;
use linked_hash_map::LinkedHashMap;
use std::collections::{BTreeMap, HashSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

/// Set to `1` to skip when the vendored data is absent, as it is in a crate
/// unpacked from crates.io. See `src/transaction/sighash_vectors.rs`.
const OPTIONAL_ENV: &str = "CHAIN_GANG_VECTORS_OPTIONAL";

/// Test rows in the vendored file, so a refreshed file is noticed.
const TEST_ROWS: usize = 1483;

const OP_0: u8 = 0x00;
const OP_PUSHDATA1: u8 = 0x4c;
const OP_PUSHDATA2: u8 = 0x4d;
const OP_PUSHDATA4: u8 = 0x4e;
const OP_1NEGATE: u8 = 0x4f;
const OP_1: u8 = 0x51;

/// The names the node's `ParseScript` accepts: `GetOpName` for every opcode
/// from `OP_PUSHDATA1` up, each with and without its `OP_` prefix. Small
/// numbers are not here; `GetOpName` spells them as numbers, which parse as
/// numbers.
const OP_NAMES: &[(&str, u8)] = &[
    ("PUSHDATA1", 0x4c),
    ("PUSHDATA2", 0x4d),
    ("PUSHDATA4", 0x4e),
    ("RESERVED", 0x50),
    ("NOP", 0x61),
    ("VER", 0x62),
    ("IF", 0x63),
    ("NOTIF", 0x64),
    ("VERIF", 0x65),
    ("VERNOTIF", 0x66),
    ("ELSE", 0x67),
    ("ENDIF", 0x68),
    ("VERIFY", 0x69),
    ("RETURN", 0x6a),
    ("TOALTSTACK", 0x6b),
    ("FROMALTSTACK", 0x6c),
    ("2DROP", 0x6d),
    ("2DUP", 0x6e),
    ("3DUP", 0x6f),
    ("2OVER", 0x70),
    ("2ROT", 0x71),
    ("2SWAP", 0x72),
    ("IFDUP", 0x73),
    ("DEPTH", 0x74),
    ("DROP", 0x75),
    ("DUP", 0x76),
    ("NIP", 0x77),
    ("OVER", 0x78),
    ("PICK", 0x79),
    ("ROLL", 0x7a),
    ("ROT", 0x7b),
    ("SWAP", 0x7c),
    ("TUCK", 0x7d),
    ("CAT", 0x7e),
    ("SPLIT", 0x7f),
    ("NUM2BIN", 0x80),
    ("BIN2NUM", 0x81),
    ("SIZE", 0x82),
    ("INVERT", 0x83),
    ("AND", 0x84),
    ("OR", 0x85),
    ("XOR", 0x86),
    ("EQUAL", 0x87),
    ("EQUALVERIFY", 0x88),
    ("RESERVED1", 0x89),
    ("RESERVED2", 0x8a),
    ("1ADD", 0x8b),
    ("1SUB", 0x8c),
    ("2MUL", 0x8d),
    ("2DIV", 0x8e),
    ("NEGATE", 0x8f),
    ("ABS", 0x90),
    ("NOT", 0x91),
    ("0NOTEQUAL", 0x92),
    ("ADD", 0x93),
    ("SUB", 0x94),
    ("MUL", 0x95),
    ("DIV", 0x96),
    ("MOD", 0x97),
    ("LSHIFT", 0x98),
    ("RSHIFT", 0x99),
    ("BOOLAND", 0x9a),
    ("BOOLOR", 0x9b),
    ("NUMEQUAL", 0x9c),
    ("NUMEQUALVERIFY", 0x9d),
    ("NUMNOTEQUAL", 0x9e),
    ("LESSTHAN", 0x9f),
    ("GREATERTHAN", 0xa0),
    ("LESSTHANOREQUAL", 0xa1),
    ("GREATERTHANOREQUAL", 0xa2),
    ("MIN", 0xa3),
    ("MAX", 0xa4),
    ("WITHIN", 0xa5),
    ("RIPEMD160", 0xa6),
    ("SHA1", 0xa7),
    ("SHA256", 0xa8),
    ("HASH160", 0xa9),
    ("HASH256", 0xaa),
    ("CODESEPARATOR", 0xab),
    ("CHECKSIG", 0xac),
    ("CHECKSIGVERIFY", 0xad),
    ("CHECKMULTISIG", 0xae),
    ("CHECKMULTISIGVERIFY", 0xaf),
    ("NOP1", 0xb0),
    ("CHECKLOCKTIMEVERIFY", 0xb1),
    ("CHECKSEQUENCEVERIFY", 0xb2),
    ("SUBSTR", 0xb3),
    ("LEFT", 0xb4),
    ("RIGHT", 0xb5),
    ("LSHIFTNUM", 0xb6),
    ("RSHIFTNUM", 0xb7),
    ("NOP9", 0xb8),
    ("NOP10", 0xb9),
];

/// `CScript << std::vector<uint8_t>`: the shortest push that holds `data`.
fn push_data(out: &mut Vec<u8>, data: &[u8]) {
    let n = data.len();
    if n < OP_PUSHDATA1 as usize {
        out.push(n as u8);
    } else if n <= 0xff {
        out.extend([OP_PUSHDATA1, n as u8]);
    } else if n <= 0xffff {
        out.push(OP_PUSHDATA2);
        out.extend((n as u16).to_le_bytes());
    } else {
        out.push(OP_PUSHDATA4);
        out.extend((n as u32).to_le_bytes());
    }
    out.extend(data);
}

/// `CScriptNum::serialize`: little-endian magnitude, sign in the top bit.
fn script_num(n: i64) -> Vec<u8> {
    let mut out = Vec::new();
    let mut abs = n.unsigned_abs();
    while abs > 0 {
        out.push((abs & 0xff) as u8);
        abs >>= 8;
    }
    if let Some(last) = out.last_mut() {
        if *last & 0x80 != 0 {
            out.push(if n < 0 { 0x80 } else { 0 });
        } else if n < 0 {
            *last |= 0x80;
        }
    }
    out
}

/// The node's `ParseScript`: decimals push minimally (`CScript << int64_t`),
/// `0x..` is raw bytes inserted as-is, `'..'` pushes the quoted bytes, and
/// anything else is an opcode name. The node's further checks that a
/// hand-written push is followed by the right number of bytes only reject
/// malformed test files, so they are left out.
fn parse_script(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for word in s.split([' ', '\t', '\n']).filter(|w| !w.is_empty()) {
        let digits = word.strip_prefix('-').unwrap_or(word);
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            let n: i64 = word.parse().expect("decimal token");
            match n {
                0 => out.push(OP_0),
                -1 => out.push(OP_1NEGATE),
                1..=16 => out.push(OP_1 + (n as u8 - 1)),
                _ => push_data(&mut out, &script_num(n)),
            }
        } else if let Some(hex) = word.strip_prefix("0x").filter(|h| !h.is_empty()) {
            out.extend(hex::decode(hex).expect("hex token"));
        } else if word.len() >= 2 && word.starts_with('\'') && word.ends_with('\'') {
            push_data(&mut out, &word.as_bytes()[1..word.len() - 1]);
        } else {
            let name = word.strip_prefix("OP_").unwrap_or(word);
            let op = OP_NAMES
                .iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("unknown token {word:?} in {s:?}"));
            out.push(op.1);
        }
    }
    out
}

/// What chain-gang is told about a row, read from the node's flags.
#[derive(Debug)]
struct Mode {
    /// The spending transaction is after Genesis (`GENESIS`), or the output
    /// it spends is (`UTXO_AFTER_GENESIS`, which implies it).
    genesis_tx: bool,
    /// The output being spent predates Genesis: no `UTXO_AFTER_GENESIS`.
    pregenesis_output: bool,
    /// Any of the node's policy flags (`MINIMALDATA`, `NULLDUMMY`,
    /// `CLEANSTACK`). chain-gang applies those three together or not at all.
    policy: bool,
    /// `SIGHASH_FORKID`.
    require_forkid: bool,
}

impl Mode {
    fn from_flags(flags: &str) -> Mode {
        let set: HashSet<&str> = flags.split(',').filter(|f| !f.is_empty()).collect();
        let utxo_after_genesis = set.contains("UTXO_AFTER_GENESIS");
        Mode {
            genesis_tx: utxo_after_genesis || set.contains("GENESIS"),
            pregenesis_output: !utxo_after_genesis,
            policy: ["MINIMALDATA", "NULLDUMMY", "CLEANSTACK"]
                .iter()
                .any(|f| set.contains(f)),
            require_forkid: set.contains("SIGHASH_FORKID"),
        }
    }
}

/// The node test framework's `BuildCreditingTransaction`.
fn crediting_tx(lock_script: &[u8], satoshis: i64) -> Tx {
    Tx {
        version: 1,
        inputs: vec![TxIn {
            prev_output: OutPoint {
                hash: Hash256([0; 32]),
                index: 0xffffffff,
            },
            unlock_script: chain_gang::script::Script(vec![OP_0, OP_0]),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis,
            lock_script: chain_gang::script::Script(lock_script.to_vec()),
        }],
        lock_time: 0,
    }
}

/// `BuildSpendingTransaction`.
fn spending_tx(credit: &Tx, unlock_script: &[u8], version: u32) -> Tx {
    Tx {
        version,
        inputs: vec![TxIn {
            prev_output: OutPoint {
                hash: credit.hash(),
                index: 0,
            },
            unlock_script: chain_gang::script::Script(unlock_script.to_vec()),
            sequence: 0xffffffff,
        }],
        outputs: vec![TxOut {
            satoshis: credit.outputs[0].satoshis,
            lock_script: chain_gang::script::Script(vec![]),
        }],
        lock_time: 0,
    }
}

/// One test row of the file.
struct Row {
    /// Index in the file's top-level array, which is how rows are named here.
    index: usize,
    satoshis: i64,
    version: u32,
    unlock: String,
    lock: String,
    flags: String,
    expected: String,
    comment: String,
}

fn load_rows(path: &PathBuf) -> Vec<Row> {
    let all: Vec<Vec<serde_json::Value>> =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut rows = Vec::new();
    for (index, row) in all.iter().enumerate() {
        let (satoshis, fields) = match row.first() {
            // [[wit..., amount], ...]: the amount is in coins, as AmountFromValue reads it
            Some(serde_json::Value::Array(a)) => {
                let coins = a.last().and_then(|v| v.as_f64()).expect("amount");
                ((coins * 100_000_000.0).round() as i64, &row[1..])
            }
            _ => (0, &row[..]),
        };
        if fields.len() < 5 {
            // Comment rows
            continue;
        }
        let text = |n: usize| fields[n].as_str().expect("string field").to_string();
        rows.push(Row {
            index,
            satoshis,
            version: text(0).parse().expect("txn version"),
            unlock: text(1),
            lock: text(2),
            flags: text(3),
            expected: text(4),
            comment: fields
                .get(5)
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    rows
}

/// Runs a row through `Tx::validate` (policy) or `Tx::validate_consensus`,
/// returning chain-gang's verdict, or the panic message if it panicked.
fn chain_gang_verdict(row: &Row) -> Result<(), String> {
    let mode = Mode::from_flags(&row.flags);
    let credit = crediting_tx(&parse_script(&row.lock), row.satoshis);
    let spend = spending_tx(&credit, &parse_script(&row.unlock), row.version);

    let outpoint = spend.inputs[0].prev_output.clone();
    let mut utxos = LinkedHashMap::new();
    utxos.insert(outpoint.clone(), credit.outputs[0].clone());
    let mut pregenesis = HashSet::new();
    if mode.pregenesis_output {
        pregenesis.insert(outpoint);
    }

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        if mode.policy {
            spend.validate(mode.require_forkid, mode.genesis_tx, &utxos, &pregenesis)
        } else {
            spend.validate_consensus(mode.require_forkid, mode.genesis_tx, &utxos, &pregenesis)
        }
    }));
    match outcome {
        Ok(result) => result.map_err(|e| e.to_string()),
        Err(panic) => Err(format!(
            "PANIC: {}",
            panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default()
        )),
    }
}

/// Rows whose verdict turns on a flag chain-gang does not take one by one,
/// by the index of the row in the file. chain-gang models the chain as it is,
/// so these are not faults in chain-gang, but they are rows it cannot check.
const MODELLING_GAPS: &[(&str, &[usize])] = &[
    (
        "Chronicle is always on in chain-gang. These rows predate it, when \
         OP_VER, OP_VERIF and OP_VERNOTIF were reserved and OP_2MUL and \
         OP_2DIV disabled.",
        &[140, 142, 144, 165, 728, 883, 884, 885, 886, 1187, 1274],
    ),
    (
        "Without the node's CHECKLOCKTIMEVERIFY and CHECKSEQUENCEVERIFY \
         flags, and before Chronicle, NOP2 to NOP8 do nothing. chain-gang \
         always runs CLTV, CSV and Chronicle's SUBSTR, LEFT, RIGHT, LSHIFTNUM \
         and RSHIFTNUM in those slots.",
        &[332, 333, 506, 507, 508, 509, 510, 511, 512],
    ),
    (
        "chain-gang always spends a pre-Genesis P2SH output as P2SH. These \
         rows leave the node's P2SH flag off.",
        &[1379, 1449],
    ),
    (
        "Tx::validate requires a push-only unlocking script wherever the \
         node's transaction validation does. The node's VerifyScript test \
         checks it only under SIGPUSHONLY, which these rows leave off.",
        &[
            83, 150, 151, 152, 153, 156, 157, 158, 167, 168, 169, 172, 173, 174, 175, 176,
        ],
    ),
    (
        "These rows set MINIMALDATA alone. chain-gang's policy rules come as \
         a set, and CLEANSTACK fails their two-item final stack.",
        &[
            624, 625, 626, 627, 628, 629, 630, 631, 632, 633, 634, 635, 636, 637, 638, 639, 640,
            641, 642,
        ],
    ),
    (
        "MINIMALIF: the node sets it in neither its mempool nor its blocks, \
         and chain-gang does not enforce it (#205).",
        &[1480, 1482, 1493, 1501, 1505, 1530, 1532],
    ),
    (
        "DISCOURAGE_UPGRADABLE_NOPS is a node policy flag chain-gang does \
         not model.",
        &[1090, 1098, 1099, 1100, 1101],
    ),
    (
        "chain-gang always enforces strict signature and key encoding, low S \
         and NULLFAIL, which are in the node's mandatory flags. These rows \
         run without some of them.",
        &[
            698, 699, 701, 702, 703, 704, 705, 706, 707, 708, 1385, 1387, 1389, 1391, 1395, 1403,
            1408, 1410, 1416, 1420, 1421, 1422, 1424, 1426, 1430, 1441, 1445, 1539, 1541, 1545,
            1547,
        ],
    ),
    (
        "chain-gang has no pre-fork mode. Without SIGHASH_FORKID it accepts \
         a signature with or without FORKID and does not check that the hash \
         type is defined; the node, under STRICTENC without the FORKID flag, \
         rejects both.",
        &[1436, 1438, 1440, 1463],
    ),
];

/// Rows where chain-gang gives a different verdict from the node under the
/// same rules: chain-gang appears to be wrong. Each is a fix to make, after
/// which its rows come off this list.
const KNOWN_DIFFERENCES: &[(&str, &[usize])] = &[
    (
        "Before Genesis the 4-byte limit applies to numeric operands, not \
         results. The node accepts a 5- or 8-byte arithmetic result and \
         compares it, casts a value of any length to a boolean, and reads \
         OP_CHECKSEQUENCEVERIFY's operand as up to 5 bytes. chain-gang \
         rejects each.",
        &[182, 313, 314, 438, 439, 709, 985, 986, 987, 988],
    ),
    (
        "Before Genesis, OP_NUM2BIN to sizes up to 520 bytes, to size 0, of \
         negative zero, and shrinking a padded number: chain-gang rejects \
         these or produces different bytes.",
        &[843, 845, 848, 849, 850, 853, 854, 855],
    ),
    (
        "Before Genesis the node limits pushes and results to 520 bytes (even \
         in an unexecuted branch), the stacks to 1,000 items, a script to \
         10,000 bytes and a multisig to 20 keys. chain-gang enforces none of \
         these for a pre-Genesis output.",
        &[827, 1179, 1180, 1183, 1184, 1185, 1268],
    ),
];

#[test]
fn bitcoin_sv_script_tests() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("script_tests.json");
    if !path.exists() {
        assert!(
            std::env::var(OPTIONAL_ENV).as_deref() == Ok("1"),
            "{} is missing; set {OPTIONAL_ENV}=1 to skip vendored vectors",
            path.display()
        );
        return;
    }
    let rows = load_rows(&path);
    assert_eq!(rows.len(), TEST_ROWS);

    let mut expected: BTreeMap<usize, &str> = BTreeMap::new();
    for (why, listed) in MODELLING_GAPS.iter().chain(KNOWN_DIFFERENCES) {
        for row in *listed {
            assert!(
                expected.insert(*row, why).is_none(),
                "row {row} is listed twice"
            );
        }
    }

    let mut unexpected = Vec::new();
    let mut now_agree = Vec::new();
    for row in &rows {
        let verdict = chain_gang_verdict(row);
        let agrees = verdict.is_ok() == (row.expected == "OK");
        let describe = || {
            format!(
                "row {}: [{}] [{}] flags [{}] node {} chain-gang {:?} — {}",
                row.index, row.unlock, row.lock, row.flags, row.expected, verdict, row.comment
            )
        };
        match (agrees, expected.get(&row.index)) {
            (false, None) => unexpected.push(describe()),
            (true, Some(why)) => now_agree.push(format!("{} (listed as: {why})", describe())),
            _ => {}
        }
    }
    assert!(
        unexpected.is_empty() && now_agree.is_empty(),
        "{} rows disagree with the node and are not listed:\n{}\n\n\
         {} listed rows now agree; take them off the list:\n{}",
        unexpected.len(),
        unexpected.join("\n"),
        now_agree.len(),
        now_agree.join("\n")
    );
}
