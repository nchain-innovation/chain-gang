//! Transaction sighash helpers

use crate::messages::{OutPoint, Payload, Tx, TxOut};
use crate::script::op_codes::{OP_CHECKSIG, OP_CODESEPARATOR};
use crate::script::{next_op, op_codes, Script};
use crate::util::{sha256d, var_int, ChainGangError, Hash256, Serializable};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

/// Signs all of the outputs
pub const SIGHASH_ALL: u8 = 0x01;
/// Sign none of the outputs so that they may be spent anywhere
pub const SIGHASH_NONE: u8 = 0x02;
/// Sign only the output paired with the the input
pub const SIGHASH_SINGLE: u8 = 0x03;
/// Sign only the input so others may inputs to the transaction
pub const SIGHASH_ANYONECANPAY: u8 = 0x80;
/// Bitcoin Cash / SV sighash flag for use on outputs after the fork
pub const SIGHASH_FORKID: u8 = 0x40;
/// Chronicle sighash flag selecting the Original Transaction Digest Algorithm (OTDA)
pub const SIGHASH_CHRONICLE: u8 = 0x20;

/// The 24-bit fork ID for Bitcoin Cash / SV
const FORK_ID: u32 = 0;

// Other useful flags
//pub const ALL_FORKID: u8 = SIGHASH_ALL | SIGHASH_FORKID;
//const NONE_FORKID: u8 = SIGHASH_NONE | SIGHASH_FORKID;
//const SINGLE_FORKID: u8 = SIGHASH_SINGLE | SIGHASH_FORKID;
//const ALL_ANYONECANPAY_FORKID: u8 = ALL_FORKID | SIGHASH_ANYONECANPAY;
//const NONE_ANYONECANPAY_FORKID: u8 = NONE_FORKID | SIGHASH_ANYONECANPAY;
//const SINGLE_ANYONECANPAY_FORKID: u8 = SINGLE_FORKID | SIGHASH_ANYONECANPAY;

/// Generates a transaction digest for signing
///
/// When `SIGHASH_FORKID` is set without `SIGHASH_CHRONICLE`, BIP-143 is used.
/// When `SIGHASH_CHRONICLE` is set, the Original Transaction Digest Algorithm
/// (OTDA) is used. Otherwise the legacy pre-fork algorithm is used.
///
/// # Arguments
///
/// * `tx` - Spending transaction
/// * `n_input` - Spending input index
/// * `script_code` - The lock_script of the output being spent. This may be a subset of the
///   lock_script if OP_CODESEPARATOR is used.
/// * `satoshis` - The satoshi amount in the output being spent
/// * `sighash_type` - Sighash flags
/// * `cache` - Cache to store intermediate values for future sighash calls.
pub fn sighash(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    satoshis: i64,
    sighash_type: u8,
    cache: &mut SigHashCache,
) -> Result<Hash256, ChainGangError> {
    // use default value of 0
    let checksig_index: usize = 0;
    sighash_checksig_index(
        tx,
        n_input,
        script_code,
        checksig_index,
        satoshis,
        sighash_type,
        cache,
    )
}

/// Same as [`sighash`] but with an additional `checksig_index` parameter selecting the
/// OP_CHECKSIG occurrence to sign against.
// Same as above `sighash` function with an additional `checksig_index` parameter
pub fn sighash_checksig_index(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    checksig_index: usize,
    satoshis: i64,
    sighash_type: u8,
    cache: &mut SigHashCache,
) -> Result<Hash256, ChainGangError> {
    // A signature carries the sighash type in a single trailing byte, so the
    // public API takes a `u8`. The node's digest functions take a 32-bit
    // `nHashType` and serialize all of it, so widen once here and let the
    // internals work in the node's width.
    sighash_u32(
        tx,
        n_input,
        script_code,
        ScriptCode::FromLockScript { checksig_index },
        satoshis,
        u32::from(sighash_type),
        cache,
    )
}

/// Generates a transaction digest for a script code that has already been cut,
/// which is what the node's `SignatureHash` receives.
///
/// [`sighash`] takes a whole locking script and works out the script code for
/// the selected `OP_CHECKSIG` itself, which is what a signer has to hand. A
/// verifier is in a different position: by the time an `OP_CHECKSIG` runs, the
/// interpreter has taken the script from just past the last *executed*
/// `OP_CODESEPARATOR` and removed the signature, and that is already exactly
/// the script code to hash. Running it through [`sighash`] cuts it a second
/// time, by a rule that has to guess which separators executed and needs an
/// `OP_CHECKSIG` to count from. That fails `OP_CHECKMULTISIG` and
/// `OP_CHECKSIGVERIFY` scripts outright and misplaces the start whenever an
/// unexecuted separator precedes the check.
///
/// This is the function a [`Checker`](crate::script::Checker) should use. Like
/// the node, it cuts nothing and requires no `OP_CHECKSIG`: BIP-143 hashes
/// `script_code` byte for byte, and the original algorithm deletes its
/// `OP_CODESEPARATOR`s as the node's serializer does.
pub fn sighash_from_script_code(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    satoshis: i64,
    sighash_type: u8,
    cache: &mut SigHashCache,
) -> Result<Hash256, ChainGangError> {
    sighash_u32(
        tx,
        n_input,
        script_code,
        ScriptCode::AsGiven,
        satoshis,
        u32::from(sighash_type),
        cache,
    )
}

/// Where the script code a digest signs comes from.
#[derive(Debug, Clone, Copy)]
enum ScriptCode {
    /// A whole locking script, cut for the `checksig_index`-th `OP_CHECKSIG`.
    /// What a signer passes.
    FromLockScript { checksig_index: usize },
    /// Already cut by the interpreter, exactly as the node's digest functions
    /// receive it. What a verifier passes.
    AsGiven,
}

/// The node's `SignatureHash`: BIP-143 when FORKID is set and CHRONICLE is not,
/// otherwise the original algorithm.
///
/// Carries the full 32-bit `nHashType` so the consensus vectors, whose hash types
/// are random 32-bit values, can be run against the same dispatch the public API
/// uses. Not public: a signature only ever carries one byte.
fn sighash_u32(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    selection: ScriptCode,
    satoshis: i64,
    sighash_type: u32,
    cache: &mut SigHashCache,
) -> Result<Hash256, ChainGangError> {
    if uses_bip143(sighash_type) {
        bip143_sighash(
            tx,
            n_input,
            script_code,
            selection,
            satoshis,
            sighash_type,
            cache,
        )
    } else {
        otda_sighash(tx, n_input, script_code, selection, sighash_type)
    }
}

/// BIP-143 is used when FORKID is set and CHRONICLE is not, matching bitcoin-sv.
fn uses_bip143(sighash_type: u32) -> bool {
    sighash_type & u32::from(SIGHASH_FORKID) != 0
        && sighash_type & u32::from(SIGHASH_CHRONICLE) == 0
}

/// Cache for sighash intermediate values to avoid quadratic hashing
///
/// This is only valid for one transaction, but may be used for multiple signatures.
pub struct SigHashCache {
    hash_prevouts: Option<Hash256>,
    hash_sequence: Option<Hash256>,
    hash_outputs: Option<Hash256>,
}

impl SigHashCache {
    /// Creates a new cache
    pub fn new() -> SigHashCache {
        SigHashCache {
            hash_prevouts: None,
            hash_sequence: None,
            hash_outputs: None,
        }
    }
    // getter/setter/clear hash_prevouts
    /// Returns the cached hash of the previous outputs, if set
    pub fn hash_prevouts(&self) -> Option<&Hash256> {
        self.hash_prevouts.as_ref()
    }

    /// Sets the cached hash of the previous outputs
    pub fn set_hash_prevouts(&mut self, hash: Hash256) {
        self.hash_prevouts = Some(hash);
    }

    /// Clears the cached hash of the previous outputs
    pub fn clear_hash_prevouts(&mut self) {
        self.hash_prevouts = None;
    }
    //getter/setter/clear hash_sequence
    /// Returns the cached hash of the input sequence numbers, if set
    pub fn hash_sequence(&self) -> Option<&Hash256> {
        self.hash_sequence.as_ref()
    }

    /// Sets the cached hash of the input sequence numbers
    pub fn set_hash_sequence(&mut self, hash: Hash256) {
        self.hash_sequence = Some(hash);
    }

    /// Clears the cached hash of the input sequence numbers
    pub fn clear_hash_sequence(&mut self) {
        self.hash_sequence = None;
    }

    //getter/setter/clear hash_outputs
    /// Returns the cached hash of the outputs, if set
    pub fn hash_outputs(&self) -> Option<&Hash256> {
        self.hash_outputs.as_ref()
    }

    /// Sets the cached hash of the outputs
    pub fn set_hash_outputs(&mut self, hash: Hash256) {
        self.hash_outputs = Some(hash)
    }

    /// Clears the cached hash of the outputs
    pub fn clear_hash_outputs(&mut self) {
        self.hash_outputs = None;
    }
}

impl Default for SigHashCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates a transaction digest for signing using BIP-143
///
/// This is to be used for all tranasctions after the August 2017 fork.
/// It fixing quadratic hashing and includes the satoshis spent in the hash.
fn bip143_sighash(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    selection: ScriptCode,
    satoshis: i64,
    sighash_type: u32,
    cache: &mut SigHashCache,
) -> Result<Hash256, ChainGangError> {
    // The intention is to return any error(s) without any extra processing & according to the
    // docs the '?' operator is the most idiomatic & concise.
    let s = bip143_sighash_preimage(
        tx,
        n_input,
        script_code,
        selection,
        satoshis,
        sighash_type,
        cache,
    )?;
    Ok(sha256d(&s))
}

// Positions at which `operation` appears **as an opcode** (at an opcode
// boundary). Found by walking the script with `next_op` rather than scanning
// raw bytes, so a data byte that equals the opcode inside a pushdata payload
// (e.g. a 0xab/0xac byte in a P2PKH hash) is not counted. See CS-483.
fn find_all_occurances_of(script_code: &[u8], operation: u8) -> Vec<usize> {
    let mut positions: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < script_code.len() {
        if script_code[i] == operation {
            positions.push(i);
        }
        i = next_op(i, script_code);
    }
    positions
}

/// Where the script code a CHECKSIG signs begins in `script_code`.
///
/// The node's interpreter tracks `pbegincodehash`: each executed
/// `OP_CODESEPARATOR` moves it to just past itself, and an `OP_CHECKSIG` signs
/// from there to the end. Callers here pass a whole locking script and pick the
/// `OP_CHECKSIG` by `checksig_index`, so this finds the separator that would
/// last have executed before it and returns the position after it, or 0.
///
/// Both digest algorithms start from this one rule; they differ only in what
/// they do with the separators that remain (see [`extract_subscript`] and
/// [`bip143_script_code`]).
///
/// The number of separators does not matter. This used to return 0 whenever
/// there was only one, which was right only when that one was the first
/// opcode: anywhere else, the opcodes before it stayed in the script code, the
/// digest differed from the node's, and the signature failed with NULLFAIL
/// (CS-492). Even the first-opcode case was right only because the separator
/// was then deleted; once BIP-143 keeps separators, starting at 0 would sign
/// the separator itself.
fn subscript_start(script_code: &[u8], checksig_index: usize) -> Result<usize, ChainGangError> {
    // OP_CODESEPARATOR / OP_CHECKSIG positions are found opcode-aware
    // (find_all_occurances_of walks opcodes), so pushed-data bytes equal to
    // those opcodes are never mistaken for the opcodes themselves (CS-483).
    let codeseparator_positions: Vec<usize> = find_all_occurances_of(script_code, OP_CODESEPARATOR);
    if codeseparator_positions.is_empty() {
        // if there is no OP_CODESEPARATOR there is nothing to do
        return Ok(0);
    }

    // Look for all OP_CHECKSIG
    let checksig_positions: Vec<usize> = find_all_occurances_of(script_code, OP_CHECKSIG);
    // `>=` (not `> len - 1`) so an empty list — a code-separated script with no
    // OP_CHECKSIG — returns an error instead of underflowing `len - 1`.
    if checksig_index >= checksig_positions.len() {
        let err_msg = format!(
            "checksig_index {} exceeds the number of OP_CHECKSIGs ({}) found in code",
            checksig_index,
            checksig_positions.len()
        );
        return Err(ChainGangError::BadArgument(err_msg));
    }
    let checksig_pos = checksig_positions[checksig_index];

    // The last OP_CODESEPARATOR before the selected OP_CHECKSIG, however many
    // there are; the script code starts just after it, as pbegincodehash does.
    Ok(codeseparator_positions
        .iter()
        .rev()
        .find(|pos| **pos < checksig_pos)
        .map_or(0, |pos| pos + 1))
}

/// The script code the original algorithm signs: from [`subscript_start`] to
/// the end, with every `OP_CODESEPARATOR` deleted.
///
/// The deletion is the node's: `CTransactionSignatureSerializer` skips
/// separators while serializing the script code. It belongs to this algorithm
/// only — BIP-143 keeps them.
fn extract_subscript(script_code: &[u8], checksig_index: usize) -> Result<Vec<u8>, ChainGangError> {
    let start_subscript = subscript_start(script_code, checksig_index)?;
    Ok(delete_separators(&script_code[start_subscript..]))
}

/// `script_code` with every `OP_CODESEPARATOR` removed, walking opcodes so a
/// pushed byte equal to one is left alone. The node's `FindAndDelete`.
fn delete_separators(script_code: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(script_code.len());
    let mut i = 0;
    while i < script_code.len() {
        let next = next_op(i, script_code);
        if script_code[i] != op_codes::OP_CODESEPARATOR {
            out.extend_from_slice(&script_code[i..next]);
        }
        i = next;
    }
    out
}

/// The script code BIP-143 signs: from [`subscript_start`] to the end, byte for
/// byte.
///
/// `SignatureHashBIP143` serializes the script code exactly as the interpreter
/// hands it over (`ss << scriptCode`), with no `FindAndDelete`, so any
/// `OP_CODESEPARATOR` after the one that executed is part of the preimage. This
/// path used to share [`extract_subscript`] and delete them, which gave a
/// different digest for every script that had one (#193).
fn bip143_script_code(
    script_code: &[u8],
    checksig_index: usize,
) -> Result<Vec<u8>, ChainGangError> {
    let start = subscript_start(script_code, checksig_index)?;
    Ok(script_code[start..].to_vec())
}

/// Generates the transaction digest for signing using OTDA (Original Transaction Digest Algorithm).
///
/// This is the original Bitcoin sighash, selected when the Chronicle flag is set or
/// when signing pre-fork transactions without `SIGHASH_FORKID`.
fn otda_sighash(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    selection: ScriptCode,
    sighash_type: u32,
) -> Result<Hash256, ChainGangError> {
    Ok(sha256d(&otda_sighash_preimage(
        tx,
        n_input,
        script_code,
        selection,
        sighash_type,
    )?))
}

fn otda_sighash_preimage(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    selection: ScriptCode,
    sighash_type: u32,
) -> Result<Vec<u8>, ChainGangError> {
    if n_input >= tx.inputs.len() {
        return Err(ChainGangError::BadArgument(
            "input out of tx_in range".to_string(),
        ));
    }

    let mut s = Vec::with_capacity(tx.size());
    let base_type = (sighash_type & 31) as u8;
    let anyone_can_pay = sighash_type & u32::from(SIGHASH_ANYONECANPAY) != 0;

    // The node's serializer deletes every OP_CODESEPARATOR from the script code
    // it is given; a signer's whole locking script is cut first.
    let sub_script = match selection {
        ScriptCode::FromLockScript { checksig_index } => {
            extract_subscript(script_code, checksig_index)?
        }
        ScriptCode::AsGiven => delete_separators(script_code),
    };

    // Serialize the version
    s.write_u32::<LittleEndian>(tx.version)?;

    // Serialize the inputs
    let n_inputs = if anyone_can_pay { 1 } else { tx.inputs.len() };
    var_int::write(n_inputs as u64, &mut s)?;
    for i in 0..tx.inputs.len() {
        let i = if anyone_can_pay { n_input } else { i };
        let mut tx_in = tx.inputs[i].clone();
        if i == n_input {
            tx_in.unlock_script = Script(Vec::with_capacity(4 + sub_script.len()));
            tx_in.unlock_script.0.extend_from_slice(&sub_script);
        } else {
            tx_in.unlock_script = Script(vec![]);
            if base_type == SIGHASH_NONE || base_type == SIGHASH_SINGLE {
                tx_in.sequence = 0;
            }
        }
        tx_in.write(&mut s)?;
        if anyone_can_pay {
            break;
        }
    }

    // Serialize the outputs
    let tx_out_list = if base_type == SIGHASH_NONE {
        vec![]
    } else if base_type == SIGHASH_SINGLE {
        if n_input >= tx.outputs.len() {
            return Err(ChainGangError::BadArgument(
                "input out of tx_out range".to_string(),
            ));
        }
        let mut truncated_out = tx.outputs.clone();
        truncated_out.truncate(n_input + 1);
        truncated_out
    } else {
        tx.outputs.clone()
    };
    var_int::write(tx_out_list.len() as u64, &mut s)?;
    for (i, tx_out) in tx_out_list.iter().enumerate() {
        // SIGHASH_SINGLE signs the output paired with this input and leaves the
        // others free to change, so every output *except* that one is blanked.
        // The node does the same in `CTransactionSignatureSerializer`:
        //
        //     if (sigHashType.getBaseType() == BaseSigHashType::SINGLE &&
        //         nOutput != nIn) { ::Serialize(s, CTxOut()); }
        //     else { ::Serialize(s, txTo.vout[nOutput]); }
        //
        // This condition used to be inverted, which blanked the signed output
        // and signed the free ones, so no SIGHASH_SINGLE digest on this path
        // agreed with the node.
        if base_type == SIGHASH_SINGLE && i != n_input {
            let empty = TxOut {
                satoshis: -1,
                lock_script: Script(vec![]),
            };
            empty.write(&mut s)?;
        } else {
            tx_out.write(&mut s)?;
        }
    }

    // Serialize the lock time
    s.write_u32::<LittleEndian>(tx.lock_time)?;

    // Append the sighash_type and return the serialized preimage. All 32 bits
    // go out, matching the node's `ss << sigHashType`, which serializes the
    // whole `uint32_t` rather than the low byte a signature carries.
    s.write_u32::<LittleEndian>(sighash_type)?;
    Ok(s)
}

/// Returns the serialized sighash preimage (the bytes hashed to produce the digest) for signing
pub fn sig_hash_preimage(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    satoshis: i64,
    sighash_type: u8,
    cache: &mut SigHashCache,
) -> Result<Vec<u8>, ChainGangError> {
    // use default value of 0
    let checksig_index: usize = 0;
    sig_hash_preimage_checksig_index(
        tx,
        n_input,
        script_code,
        checksig_index,
        satoshis,
        sighash_type,
        cache,
    )
}

/// Same as [`sig_hash_preimage`] but with an additional `checksig_index` parameter selecting the
/// OP_CHECKSIG occurrence to sign against.
// this code was duplicated from bip143_sighash above (that function now calls this one)
// as above with checksig_index
pub fn sig_hash_preimage_checksig_index(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    checksig_index: usize,
    satoshis: i64,
    sighash_type: u8,
    cache: &mut SigHashCache,
) -> Result<Vec<u8>, ChainGangError> {
    // A signature carries the sighash type in a single trailing byte, so the
    // public API takes a `u8`. The node's digest functions take a 32-bit
    // `nHashType` and serialize all of it, so widen once here and let the
    // internals work in the node's width.
    let sighash_type = u32::from(sighash_type);
    if uses_bip143(sighash_type) {
        bip143_sighash_preimage(
            tx,
            n_input,
            script_code,
            ScriptCode::FromLockScript { checksig_index },
            satoshis,
            sighash_type,
            cache,
        )
    } else {
        otda_sighash_preimage(
            tx,
            n_input,
            script_code,
            ScriptCode::FromLockScript { checksig_index },
            sighash_type,
        )
    }
}

fn bip143_sighash_preimage(
    tx: &Tx,
    n_input: usize,
    script_code: &[u8],
    selection: ScriptCode,
    satoshis: i64,
    sighash_type: u32,
    cache: &mut SigHashCache,
) -> Result<Vec<u8>, ChainGangError> {
    if n_input >= tx.inputs.len() {
        return Err(ChainGangError::BadArgument(
            "input out of tx_in range".to_string(),
        ));
    }

    let mut s = Vec::with_capacity(tx.size());
    let base_type = (sighash_type & 31) as u8;
    let anyone_can_pay = sighash_type & u32::from(SIGHASH_ANYONECANPAY) != 0;

    // Byte for byte, separators included; a signer's whole locking script is
    // cut first.
    let sub_script = match selection {
        ScriptCode::FromLockScript { checksig_index } => {
            bip143_script_code(script_code, checksig_index)?
        }
        ScriptCode::AsGiven => script_code.to_vec(),
    };

    // Serialize the version
    s.write_u32::<LittleEndian>(tx.version)?;
    // 2. Serialize hash of prevouts
    if !anyone_can_pay {
        if cache.hash_prevouts.is_none() {
            let mut prev_outputs = Vec::with_capacity(OutPoint::SIZE * tx.inputs.len());
            for input in tx.inputs.iter() {
                input.prev_output.write(&mut prev_outputs)?;
            }
            cache.hash_prevouts = Some(sha256d(&prev_outputs));
        }
        s.write_all(&cache.hash_prevouts.unwrap().0)?;
    } else {
        s.write_all(&[0; 32])?;
    }

    // 3. Serialize hash of sequences
    if !anyone_can_pay && base_type != SIGHASH_SINGLE && base_type != SIGHASH_NONE {
        if cache.hash_sequence.is_none() {
            let mut sequences = Vec::with_capacity(4 * tx.inputs.len());
            for tx_in in tx.inputs.iter() {
                sequences.write_u32::<LittleEndian>(tx_in.sequence)?;
            }
            cache.hash_sequence = Some(sha256d(&sequences));
        }
        s.write_all(&cache.hash_sequence.unwrap().0)?;
    } else {
        s.write_all(&[0; 32])?;
    }

    // 4. Serialize prev output
    tx.inputs[n_input].prev_output.write(&mut s)?;

    // 5. Serialize input script
    var_int::write(sub_script.len() as u64, &mut s)?;
    s.write_all(&sub_script)?;

    // 6. Serialize satoshis
    s.write_i64::<LittleEndian>(satoshis)?;

    // 7. Serialize sequence
    s.write_u32::<LittleEndian>(tx.inputs[n_input].sequence)?;

    // 8. Serialize hash of outputs
    if base_type != SIGHASH_SINGLE && base_type != SIGHASH_NONE {
        if cache.hash_outputs.is_none() {
            let mut size = 0;
            for tx_out in tx.outputs.iter() {
                size += tx_out.size();
            }
            let mut outputs = Vec::with_capacity(size);
            for tx_out in tx.outputs.iter() {
                tx_out.write(&mut outputs)?;
            }
            cache.hash_outputs = Some(sha256d(&outputs));
        }
        s.write_all(&cache.hash_outputs.unwrap().0)?;
    } else if base_type == SIGHASH_SINGLE && n_input < tx.outputs.len() {
        let mut outputs = Vec::with_capacity(tx.outputs[n_input].size());
        tx.outputs[n_input].write(&mut outputs)?;
        s.write_all(&sha256d(&outputs).0)?;
    } else {
        s.write_all(&[0; 32])?;
    }

    // 9. Serialize lock_time
    s.write_u32::<LittleEndian>(tx.lock_time)?;

    // 10. Serialize hash type
    s.write_u32::<LittleEndian>((FORK_ID << 8) | sighash_type)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::addr_decode;
    use crate::messages::{OutPoint, TxIn};
    use crate::network::Network;
    use crate::script::op_codes::*;
    use crate::transaction::p2pkh;
    use hex;

    fn bip143_sighash_test_tx() -> (Tx, Vec<u8>) {
        let lock_script =
            hex::decode("76a91402b74813b047606b4b3fbdfb1a6e8e053fdb8dab88ac").unwrap();
        let addr = "mfmKD4cP6Na7T8D87XRSiR7shA1HNGSaec";
        let hash160 = addr_decode(addr, Network::BSV_Testnet).unwrap().0;
        let tx = Tx {
            version: 2,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: Hash256::decode(
                        "f671dc000ad12795e86b59b27e0c367d9b026bbd4141c227b9285867a53bb6f7",
                    )
                    .unwrap(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0,
            }],
            outputs: vec![
                TxOut {
                    satoshis: 100,
                    lock_script: p2pkh::create_lock_script(&hash160),
                },
                TxOut {
                    satoshis: 259899900,
                    lock_script: p2pkh::create_lock_script(&hash160),
                },
            ],
            lock_time: 0,
        };
        (tx, lock_script)
    }

    #[test]
    fn bip143_sighash_test() {
        let (tx, lock_script) = bip143_sighash_test_tx();
        let mut cache = SigHashCache::new();
        let sighash_type = SIGHASH_ALL | SIGHASH_FORKID;
        let sighash = bip143_sighash(
            &tx,
            0,
            &lock_script,
            ScriptCode::FromLockScript { checksig_index: 0 },
            260000000,
            u32::from(sighash_type),
            &mut cache,
        )
        .unwrap();
        let expected = "1e2121837829018daf3aeadab76f1a542c49a3600ded7bd74323ee74ce0d840c";
        assert!(sighash.0.to_vec() == hex::decode(expected).unwrap());
        assert!(cache.hash_prevouts.is_some());
        assert!(cache.hash_sequence.is_some());
        assert!(cache.hash_outputs.is_some());
    }

    #[test]
    fn sighash_without_chronicle_uses_bip143() {
        let (tx, lock_script) = bip143_sighash_test_tx();
        let mut cache = SigHashCache::new();
        let sighash_type = SIGHASH_ALL | SIGHASH_FORKID;
        let routed = sighash(&tx, 0, &lock_script, 260000000, sighash_type, &mut cache).unwrap();
        let expected = bip143_sighash(
            &tx,
            0,
            &lock_script,
            ScriptCode::FromLockScript { checksig_index: 0 },
            260000000,
            u32::from(sighash_type),
            &mut cache,
        )
        .unwrap();
        assert_eq!(routed, expected);
    }

    #[test]
    fn sighash_chronicle_routes_to_otda() {
        let (tx, lock_script) = bip143_sighash_test_tx();
        let mut cache = SigHashCache::new();
        let bip143_type = SIGHASH_ALL | SIGHASH_FORKID;
        let chronicle_type = SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE;

        let bip143_hash =
            sighash(&tx, 0, &lock_script, 260000000, bip143_type, &mut cache).unwrap();
        let chronicle_hash =
            sighash(&tx, 0, &lock_script, 260000000, chronicle_type, &mut cache).unwrap();
        let expected_otda = otda_sighash(
            &tx,
            0,
            &lock_script,
            ScriptCode::FromLockScript { checksig_index: 0 },
            u32::from(chronicle_type),
        )
        .unwrap();

        assert_ne!(bip143_hash, chronicle_hash);
        assert_eq!(chronicle_hash, expected_otda);
    }

    #[test]
    fn sighash_chronicle_preimage_matches_hash() {
        let (tx, lock_script) = bip143_sighash_test_tx();
        let mut cache = SigHashCache::new();
        let bip143_type = SIGHASH_ALL | SIGHASH_FORKID;
        let chronicle_type = SIGHASH_ALL | SIGHASH_FORKID | SIGHASH_CHRONICLE;

        let chronicle_hash =
            sighash(&tx, 0, &lock_script, 260000000, chronicle_type, &mut cache).unwrap();
        let chronicle_preimage =
            sig_hash_preimage(&tx, 0, &lock_script, 260000000, chronicle_type, &mut cache).unwrap();
        let bip143_preimage =
            sig_hash_preimage(&tx, 0, &lock_script, 260000000, bip143_type, &mut cache).unwrap();

        assert_ne!(bip143_preimage, chronicle_preimage);
        assert_eq!(chronicle_hash, sha256d(&chronicle_preimage));
    }

    #[test]
    fn otda_sighash_test() {
        let lock_script =
            hex::decode("76a914d951eb562f1ff26b6cbe89f04eda365ea6bd95ce88ac").unwrap();
        let tx = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: Hash256::decode(
                        "bf6c1139ea01ca054b8d00aa0a088daaeab4f3b8e111626c6be7d603a9dd8dff",
                    )
                    .unwrap(),
                    index: 0,
                },
                unlock_script: Script(vec![]),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 49990000,
                lock_script: Script(
                    hex::decode("76a9147865b0b301119fc3eadc7f3406ff1339908e46d488ac").unwrap(),
                ),
            }],
            lock_time: 0,
        };
        let sighash = otda_sighash(
            &tx,
            0,
            &lock_script,
            ScriptCode::FromLockScript { checksig_index: 0 },
            u32::from(SIGHASH_ALL),
        )
        .unwrap();
        let expected = "ad16084eccf26464a84c5ee2f8b96b4daff9a3154ac3c1b320346aed042abe57";
        assert!(sighash.0.to_vec() == hex::decode(expected).unwrap());
    }

    #[test]
    fn op_codeseparator_test1() {
        let mut script_code: Vec<u8> = Vec::new();
        script_code.extend_from_slice(&[OP_CODESEPARATOR, OP_DUP, OP_HASH160]);
        let decoded = hex::decode("e252b946e62e0802cfc1db8242cc842d53e2fe25").unwrap();
        script_code.push(0x14); // push the 20-byte hash (valid P2PKH encoding)
        script_code.extend_from_slice(&decoded);
        script_code.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);

        // Drop leading OP_CODESEPARATOR
        let expected_subscript = script_code[1..].to_vec();

        let actual_subscript = extract_subscript(&script_code, 0).unwrap();
        assert_eq!(actual_subscript, expected_subscript);
    }

    #[test]
    fn op_codeseparator_test2() {
        let mut script_code: Vec<u8> = Vec::new();
        script_code.extend_from_slice(&[OP_DUP, OP_HASH160]);
        let decoded = hex::decode("e252b946e62e0802cfc1db8242cc842d53e2fe25").unwrap();
        script_code.extend_from_slice(&decoded);
        script_code.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);

        // No change
        let actual_subscript = extract_subscript(&script_code, 0).unwrap();
        assert_eq!(actual_subscript, script_code);
    }

    // Regression for CS-483.
    //
    // A standard P2PKH lock script `76 a9 14 <20-byte hash> 88 ac` contains no
    // real OP_CODESEPARATOR, so extract_subscript must return it unchanged.
    // The bug: extract_subscript detects OP_CODESEPARATOR (0xab) by raw-byte
    // scanning instead of walking opcodes, so when the pushed hash contains two
    // or more 0xab bytes they are mistaken for OP_CODESEPARATOR opcodes and the
    // subscript is cut mid-hash -> wrong BIP-143 digest -> NULLFAIL on chain.
    //
    // Failing script from the ticket (hash bytes fe ef [ab] c7 [ab] 60 ... 95):
    #[test]
    fn cs483_p2pkh_hash_with_two_0xab_bytes_is_not_truncated() {
        let script_code =
            hex::decode("76a914feefabc7ab60505d68587290168a512cb3b3349588ac").unwrap();
        // Two 0xab bytes inside the pushed 20-byte hash; no real OP_CODESEPARATOR.
        assert_eq!(
            script_code
                .iter()
                .filter(|&&b| b == OP_CODESEPARATOR)
                .count(),
            2,
            "test fixture should contain two 0xab bytes in the hash"
        );

        let actual_subscript = extract_subscript(&script_code, 0).unwrap();
        assert_eq!(
            actual_subscript, script_code,
            "pushed-data 0xab bytes must not be treated as OP_CODESEPARATOR"
        );
    }

    // Control: a single 0xab in the hash is harmless today (start_subscript
    // stays 0). Kept to pin the boundary the fix must preserve.
    #[test]
    fn cs483_p2pkh_hash_with_one_0xab_byte_is_not_truncated() {
        let script_code =
            hex::decode("76a914feefabc7cc60505d68587290168a512cb3b3349588ac").unwrap();
        assert_eq!(
            script_code
                .iter()
                .filter(|&&b| b == OP_CODESEPARATOR)
                .count(),
            1
        );
        let actual_subscript = extract_subscript(&script_code, 0).unwrap();
        assert_eq!(actual_subscript, script_code);
    }

    #[test]
    fn op_codeseparator_test3() {
        let mut script_code: Vec<u8> = Vec::new();
        script_code.extend_from_slice(&[
            OP_CODESEPARATOR,
            OP_1,
            OP_DROP,
            OP_CODESEPARATOR,
            OP_DUP,
            OP_HASH160,
        ]);
        let decoded: Vec<u8> = hex::decode("e252b946e62e0802cfc1db8242cc842d53e2fe25").unwrap();
        script_code.push(0x14); // push the 20-byte hash (valid P2PKH encoding)
        script_code.extend_from_slice(&decoded);
        script_code.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);

        // Latest OP_CODESEPARATOR is the one that matters
        // Drop leading OP_CODESEPARATOR, OP_1, OP_DROP , OP_CODESEPARATOR
        let expected_subscript = script_code[4..].to_vec();

        // assert_eq!(extract_subscript(&script_code), expected_subscript);
        let actual_subscript = extract_subscript(&script_code, 0).unwrap();
        assert_eq!(actual_subscript, expected_subscript);
    }

    #[test]
    fn op_codeseparator_test4() {
        let mut script_code: Vec<u8> = Vec::new();
        script_code.extend_from_slice(&[OP_CODESEPARATOR, OP_1, OP_DROP, OP_DUP, OP_HASH160]);
        let decoded: Vec<u8> = hex::decode("e252b946e62e0802cfc1db8242cc842d53e2fe25").unwrap();
        script_code.push(0x14); // push the 20-byte hash (valid P2PKH encoding)
        script_code.extend_from_slice(&decoded);
        script_code.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG, OP_VERIFY, OP_1]);

        // Drop the OP_CODESEPARATOR
        let expected_subscript = script_code[1..].to_vec();
        let actual_subscript = extract_subscript(&script_code, 0).unwrap();

        assert_eq!(actual_subscript, expected_subscript);
    }

    // Multi-OP_CODESEPARATOR subscript extraction, checksig_index 0. Uses valid
    // P2PKH encodings (the 20-byte hash is pushed with 0x14), so the opcode walk
    // introduced for CS-483 can parse them.
    //
    // NOTE: the subscript currently removes ALL OP_CODESEPARATORs (FindAndDelete
    // style, as legacy/OTDA sighash does). Whether a separator occurring *after*
    // the executed one should be retained (BIP-143 "rule 2") is algorithm-
    // specific and pre-existing; it is a separate concern from CS-483 and is not
    // changed here. This test pins the current behaviour.
    #[test]
    fn op_codeseparator_test5_1() {
        let decoded: Vec<u8> = hex::decode("e252b946e62e0802cfc1db8242cc842d53e2fe25").unwrap();
        let mut script_code: Vec<u8> = Vec::new();
        script_code.extend_from_slice(&[
            OP_CODESEPARATOR,
            OP_2DUP,
            OP_1,
            OP_DROP,
            OP_CODESEPARATOR, // last executed before the signed OP_CHECKSIG
            OP_DUP,
            OP_HASH160,
            0x14, // push 20-byte hash
        ]);
        script_code.extend_from_slice(&decoded);
        script_code.extend_from_slice(&[
            OP_EQUALVERIFY,
            OP_CHECKSIG,
            OP_VERIFY,
            OP_CODESEPARATOR, // later separator (see NOTE above)
            OP_DUP,
            OP_HASH160,
            0x14,
        ]);
        script_code.extend_from_slice(&decoded);
        script_code.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);

        // Subscript from after the last executed OP_CODESEPARATOR, with all
        // remaining OP_CODESEPARATORs removed.
        let mut expected = Vec::new();
        expected.extend_from_slice(&[OP_DUP, OP_HASH160, 0x14]);
        expected.extend_from_slice(&decoded);
        expected.extend_from_slice(&[
            OP_EQUALVERIFY,
            OP_CHECKSIG,
            OP_VERIFY,
            OP_DUP,
            OP_HASH160,
            0x14,
        ]);
        expected.extend_from_slice(&decoded);
        expected.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);

        let actual = extract_subscript(&script_code, 0).unwrap();
        assert_eq!(actual, expected);
    }

    // Same script, checksig_index 1: the last executed OP_CODESEPARATOR before
    // the second OP_CHECKSIG is the "later" one, so the subscript is the second
    // P2PKH clause.
    #[test]
    fn op_codeseparator_test5_2() {
        let decoded: Vec<u8> = hex::decode("e252b946e62e0802cfc1db8242cc842d53e2fe25").unwrap();
        let mut script_code: Vec<u8> = Vec::new();
        script_code.extend_from_slice(&[
            OP_CODESEPARATOR,
            OP_2DUP,
            OP_1,
            OP_DROP,
            OP_CODESEPARATOR,
            OP_DUP,
            OP_HASH160,
            0x14,
        ]);
        script_code.extend_from_slice(&decoded);
        script_code.extend_from_slice(&[
            OP_EQUALVERIFY,
            OP_CHECKSIG,
            OP_VERIFY,
            OP_CODESEPARATOR, // last executed before the 2nd OP_CHECKSIG
            OP_DUP,
            OP_HASH160,
            0x14,
        ]);
        script_code.extend_from_slice(&decoded);
        script_code.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);

        let mut expected = Vec::new();
        expected.extend_from_slice(&[OP_DUP, OP_HASH160, 0x14]);
        expected.extend_from_slice(&decoded);
        expected.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);

        let actual = extract_subscript(&script_code, 1).unwrap();
        assert_eq!(actual, expected);
    }

    /// Two inputs, three outputs, so blanking the wrong ones is visible.
    ///
    /// Signing input 1 under SIGHASH_SINGLE pairs it with output 1. Output 0 is
    /// blanked and output 2 is dropped by the `n_input + 1` truncation, so only
    /// output 1 reaches the digest.
    fn sighash_single_test_tx() -> (Tx, Vec<u8>) {
        let raw = hex::decode(concat!(
            "020000000211111111111111111111111111111111111111111111111111",
            "111111111111110000000000feffffff2222222222222222222222222222",
            "2222222222222222222222222222222222220700000000fdffffff03e803",
            "0000000000001976a9143333333333333333333333333333333333333333",
            "88acc4090000000000001976a91444444444444444444444444444444444",
            "4444444488ac611e000000000000076a0548656c6c6f63000000",
        ))
        .unwrap();
        let tx = Tx::read(&mut std::io::Cursor::new(&raw)).unwrap();
        assert_eq!(tx.inputs.len(), 2);
        assert_eq!(tx.outputs.len(), 3);
        let script_code =
            hex::decode("76a914555555555555555555555555555555555555555588ac").unwrap();
        (tx, script_code)
    }

    /// SIGHASH_SINGLE commits to the output paired with the input, and to no
    /// other.
    ///
    /// Stated as a property rather than a digest, so it says what the rule is
    /// and catches the condition being inverted without anyone having to read a
    /// hex constant. With the condition the wrong way round, output 1 is the one
    /// that stops mattering and output 0 is the one that starts.
    #[test]
    fn sighash_single_signs_only_the_paired_output() {
        let (tx, script_code) = sighash_single_test_tx();
        let n_input = 1;

        let digest = |tx: &Tx| {
            let mut cache = SigHashCache::new();
            sighash(tx, n_input, &script_code, 0, SIGHASH_SINGLE, &mut cache).unwrap()
        };
        let baseline = digest(&tx);

        for (index, should_matter) in [(0, false), (1, true), (2, false)] {
            let mut altered = tx.clone();
            altered.outputs[index].satoshis += 1;
            let changed = digest(&altered) != baseline;
            let expectation = if should_matter {
                "should"
            } else {
                "should not"
            };
            assert_eq!(
                changed, should_matter,
                "changing output {index} {expectation} change the digest"
            );
        }
    }

    /// The same transaction against digests the node would produce.
    ///
    /// The property above pins the shape; these pin the bytes. They were
    /// produced by an independent implementation of `SignatureHashOriginal`
    /// written from the node's source, which reproduces all 1000 rows of
    /// bitcoin-sv's `sighash.json` exactly (see `sighash_vectors`), so they are
    /// not this code's own answer written down.
    ///
    /// All three take the original algorithm: 0x03 has no FORKID, and 0x63 has
    /// FORKID with CHRONICLE, which is the path in current use.
    #[test]
    fn sighash_single_matches_the_node() {
        let (tx, script_code) = sighash_single_test_tx();
        let cases = [
            (
                SIGHASH_SINGLE,
                "03febcdf159c853553ecd381436e6d1f78a9eaf8724fc9d060722ddc4b993f91",
            ),
            (
                SIGHASH_SINGLE | SIGHASH_FORKID | SIGHASH_CHRONICLE,
                "367473babac69ec6dcf7a314df560b44fb7529dcffee5ab265b4af478f453b5c",
            ),
            (
                SIGHASH_SINGLE | SIGHASH_ANYONECANPAY,
                "f56deb451f2ba6f76f9322ad7c880a13e974d3da518a1c18ef77338a0f61dc0a",
            ),
        ];
        for (sighash_type, expected) in cases {
            let mut cache = SigHashCache::new();
            let got = sighash(&tx, 1, &script_code, 0, sighash_type, &mut cache).unwrap();
            assert_eq!(got.encode(), expected, "sighash_type {sighash_type:#04x}");
        }
    }

    /// Three OP_CHECKSIGs, with a separator between each pair:
    ///
    /// ```text
    /// <pk1> CHECKSIG VERIFY CODESEPARATOR <pk2> CHECKSIG VERIFY CODESEPARATOR <pk3> CHECKSIG
    /// ```
    ///
    /// `checksig_index` counts OP_CHECKSIG only, so the checks are CHECKSIG
    /// VERIFY rather than CHECKSIGVERIFY.
    fn three_checksig_script() -> Vec<u8> {
        let push_key = |b: u8| {
            let mut push = vec![0x21, 0x02];
            push.extend_from_slice(&[b; 32]);
            push
        };
        let mut script = push_key(0xa1);
        script.extend_from_slice(&[OP_CHECKSIG, OP_VERIFY, OP_CODESEPARATOR]);
        script.extend(push_key(0xa2));
        script.extend_from_slice(&[OP_CHECKSIG, OP_VERIFY, OP_CODESEPARATOR]);
        script.extend(push_key(0xa3));
        script.push(OP_CHECKSIG);
        script
    }

    /// BIP-143 signs the script code from the executed separator onwards with
    /// every later separator still in it (#193).
    ///
    /// The node's interpreter hands `SignatureHashBIP143` the script from
    /// `pbegincodehash` to the end, and that function serializes it untouched.
    /// So signing the first OP_CHECKSIG covers the whole script, both
    /// separators included, and signing the second covers everything after the
    /// first separator, the second one included. chain-gang used to delete
    /// them, which changed the first two digests. The third has no separator
    /// left after its cut, so it was already right and is here as the control.
    ///
    /// The expected digests come from an independent implementation of the
    /// node's `SignatureHash` that reproduces both columns of all 1000 rows of
    /// bitcoin-sv's `sighash.json`, so they are the node's answer rather than
    /// this code's.
    #[test]
    fn bip143_keeps_separators_after_the_executed_one() {
        let (tx, _) = sighash_single_test_tx();
        let script = three_checksig_script();
        let expected = [
            "f77b35d7bcb5b0231066d33fb305cf36f2bfb9ded436c3e66ffd042c30271ebd",
            "0c6b9499857e601bad2f0192a69249a30c43e244204135163b6dba2cb197345e",
            "001710e5453f51b4c88bac9d8926391c2347616e0c81505a149fde809f0fa268",
        ];
        for (checksig_index, expected) in expected.iter().enumerate() {
            let mut cache = SigHashCache::new();
            let got = sighash_checksig_index(
                &tx,
                0,
                &script,
                checksig_index,
                50000,
                SIGHASH_ALL | SIGHASH_FORKID,
                &mut cache,
            )
            .unwrap();
            assert_eq!(got.encode(), *expected, "checksig_index {checksig_index}");
        }
    }

    /// The original algorithm still deletes every separator, as the node's
    /// `CTransactionSignatureSerializer` does. Separating where the script code
    /// starts from what happens to the separators left in it must not have
    /// moved this path: the same script, any separators removed, gives the same
    /// digest.
    #[test]
    fn original_algorithm_still_deletes_separators() {
        let (tx, _) = sighash_single_test_tx();
        let script = three_checksig_script();
        let stripped: Vec<u8> = {
            let mut out = Vec::new();
            let mut i = 0;
            while i < script.len() {
                let next = next_op(i, &script);
                if script[i] != OP_CODESEPARATOR {
                    out.extend_from_slice(&script[i..next]);
                }
                i = next;
            }
            out
        };
        let mut cache = SigHashCache::new();
        let with_separators =
            sighash_checksig_index(&tx, 0, &script, 0, 0, SIGHASH_ALL, &mut cache).unwrap();
        let mut cache = SigHashCache::new();
        let without =
            sighash_checksig_index(&tx, 0, &stripped, 0, 0, SIGHASH_ALL, &mut cache).unwrap();
        assert_eq!(with_separators, without);
    }

    /// CS-492's reproduction, as the ticket gives it: one separator, not the
    /// first opcode. The node starts the script code after it; this used to keep
    /// `OP_1 OP_DROP`.
    ///
    /// No separator is left after the cut, so the two algorithms must agree and
    /// the test pins nothing about what happens to later ones (CS-488).
    #[test]
    fn single_separator_not_first() {
        let hash = hex::decode("e252b946e62e0802cfc1db8242cc842d53e2fe25").unwrap();
        let mut script = vec![OP_1, OP_DROP, OP_CODESEPARATOR, OP_DUP, OP_HASH160, 0x14];
        script.extend_from_slice(&hash);
        script.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);

        // node: script code starts after the executed OP_CODESEPARATOR
        let mut node = vec![OP_DUP, OP_HASH160, 0x14];
        node.extend_from_slice(&hash);
        node.extend_from_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);

        assert_eq!(extract_subscript(&script, 0).unwrap(), node);
        assert_eq!(bip143_script_code(&script, 0).unwrap(), node);
    }

    /// One separator, as the first opcode. The old rule got this right only by
    /// accident: it started at 0 and the separator was then deleted. BIP-143
    /// keeps separators, so starting at 0 would sign `OP_CODESEPARATOR` itself
    /// and the node, which starts after it, would reject the signature.
    #[test]
    fn single_leading_separator_is_not_signed() {
        let mut script = vec![OP_CODESEPARATOR, 0x21, 0x02];
        script.extend_from_slice(&[0x5a; 32]);
        script.push(OP_CHECKSIG);
        let node = script[1..].to_vec();

        assert_eq!(bip143_script_code(&script, 0).unwrap(), node);
        assert_eq!(extract_subscript(&script, 0).unwrap(), node);
    }
}

#[cfg(test)]
#[path = "sighash_vectors.rs"]
mod sighash_vectors;
