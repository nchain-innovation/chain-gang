//! A declared length must never size an allocation on its own.
//!
//! Every one of these inputs previously aborted the process with
//! `memory allocation of N bytes failed` and SIGABRT. Allocation failure in
//! Rust does not unwind, so these could not be caught by a consumer — the test
//! harness itself would die. If any of them regress, this file takes the whole
//! test binary down rather than failing, which is the intended alarm.

use std::io::Cursor;

use chain_gang::messages::{
    Authch, Block, FilterAdd, FilterLoad, MerkleBlock, MessageHeader, Reject, Tx, TxIn, TxOut,
    Version,
};
use chain_gang::util::Serializable;

/// A var_int declaring `n`, in its widest encoding.
fn var_int(n: u64) -> Vec<u8> {
    let mut v = vec![0xff];
    v.extend_from_slice(&n.to_le_bytes());
    v
}

const HUGE: u64 = 1 << 48;

#[test]
fn tx_out_does_not_allocate_a_declared_script_length() {
    // The original reproduction: 27 bytes declaring a 2^48-byte locking script.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u32.to_le_bytes()); // version
    bytes.push(0x00); // no inputs
    bytes.push(0x01); // one output
    bytes.extend_from_slice(&0i64.to_le_bytes()); // satoshis
    bytes.extend_from_slice(&var_int(HUGE)); // locking script length
    bytes.extend_from_slice(&0u32.to_le_bytes()); // lock_time
    assert_eq!(bytes.len(), 27);

    assert!(Tx::read(&mut Cursor::new(&bytes)).is_err());
}

#[test]
fn tx_out_alone_does_not_allocate() {
    let mut bytes = 0i64.to_le_bytes().to_vec();
    bytes.extend_from_slice(&var_int(u64::MAX));
    assert!(TxOut::read(&mut Cursor::new(&bytes)).is_err());
}

#[test]
fn tx_in_does_not_allocate_a_declared_script_length() {
    let mut bytes = vec![0u8; 32]; // prev hash
    bytes.extend_from_slice(&0u32.to_le_bytes()); // prev index
    bytes.extend_from_slice(&var_int(HUGE)); // unlocking script length
    assert!(TxIn::read(&mut Cursor::new(&bytes)).is_err());
}

#[test]
fn tx_does_not_allocate_declared_input_and_output_counts() {
    let mut inputs = 1u32.to_le_bytes().to_vec();
    inputs.extend_from_slice(&var_int(u64::MAX));
    assert!(Tx::read(&mut Cursor::new(&inputs)).is_err());

    let mut outputs = 1u32.to_le_bytes().to_vec();
    outputs.push(0x00);
    outputs.extend_from_slice(&var_int(u64::MAX));
    assert!(Tx::read(&mut Cursor::new(&outputs)).is_err());
}

#[test]
fn block_does_not_allocate_a_declared_transaction_count() {
    let mut bytes = vec![0u8; 80]; // header
    bytes.extend_from_slice(&var_int(u64::MAX));
    assert!(Block::read(&mut Cursor::new(&bytes)).is_err());
}

// Reachable before the handshake completes, which makes it worse than the
// transaction path: no peer relationship is needed at all.
#[test]
fn version_does_not_allocate_a_declared_user_agent_length() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&70015u32.to_le_bytes()); // version
    bytes.extend_from_slice(&0u64.to_le_bytes()); // services
    bytes.extend_from_slice(&0i64.to_le_bytes()); // timestamp
    bytes.extend_from_slice(&[0u8; 26]); // recv addr
    bytes.extend_from_slice(&[0u8; 26]); // tx addr
    bytes.extend_from_slice(&0u64.to_le_bytes()); // nonce
    bytes.extend_from_slice(&var_int(HUGE)); // user agent length
    assert!(Version::read(&mut Cursor::new(&bytes)).is_err());
}

#[test]
fn reject_does_not_allocate_declared_string_lengths() {
    let mut bytes = var_int(HUGE);
    assert!(Reject::read(&mut Cursor::new(&bytes)).is_err());

    // And the second length, past an honest first one.
    bytes = vec![0x01, b'x', 0x01];
    bytes.extend_from_slice(&var_int(HUGE));
    assert!(Reject::read(&mut Cursor::new(&bytes)).is_err());
}

#[test]
fn filter_add_does_not_allocate_a_declared_data_length() {
    assert!(FilterAdd::read(&mut Cursor::new(&var_int(HUGE))).is_err());
}

#[test]
fn filter_load_does_not_allocate_a_declared_filter_length() {
    assert!(FilterLoad::read(&mut Cursor::new(&var_int(HUGE))).is_err());
}

#[test]
fn merkle_block_does_not_allocate_declared_counts_or_flag_lengths() {
    let mut bytes = vec![0u8; 80]; // header
    bytes.extend_from_slice(&0u32.to_le_bytes()); // total transactions
    bytes.extend_from_slice(&var_int(u64::MAX)); // hash count
    assert!(MerkleBlock::read(&mut Cursor::new(&bytes)).is_err());
}

#[test]
fn authch_does_not_allocate_a_declared_message_length() {
    let mut bytes = 1i32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&u32::MAX.to_le_bytes()); // 4 GB
    assert!(Authch::read(&mut Cursor::new(&bytes)).is_err());
}

// The outermost case, and the worst: MessageHeader::validate exempts BLOCK
// from its size cap, so payload_size is unbounded for that command. A 24-byte
// header could demand 4 GB.
#[test]
fn message_header_payload_does_not_allocate_a_declared_payload_size() {
    let mut header_bytes = Vec::new();
    header_bytes.extend_from_slice(&[0xe3, 0xe1, 0xf3, 0xe8]); // magic
    header_bytes.extend_from_slice(b"block\0\0\0\0\0\0\0"); // command
    header_bytes.extend_from_slice(&u32::MAX.to_le_bytes()); // payload_size
    header_bytes.extend_from_slice(&[0u8; 4]); // checksum

    let header = MessageHeader::read(&mut Cursor::new(&header_bytes)).expect("header reads");
    assert_eq!(header.payload_size, u32::MAX);

    // Nothing follows the header.
    assert!(header.payload(&mut Cursor::new(Vec::new())).is_err());
}

// The legitimate path must be untouched.
#[test]
fn honest_messages_still_round_trip() {
    let tx = Tx {
        version: 1,
        inputs: vec![TxIn::default()],
        outputs: vec![TxOut {
            satoshis: 1000,
            lock_script: chain_gang::script::Script(vec![0x76, 0xa9, 0x14]),
        }],
        lock_time: 0,
    };
    let mut buf = Vec::new();
    tx.write(&mut buf).expect("writes");
    let parsed = Tx::read(&mut Cursor::new(&buf)).expect("reads back");
    assert_eq!(parsed, tx);
}
