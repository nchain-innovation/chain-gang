use super::*;
use super::multisig::remove_sig;
use super::script_code::strip_code_separators;
use crate::script::op_codes::*;
use crate::script::stack::{
    encode_num, MAX_SCRIPT_NUM_LENGTH_CHRONICLE, MAX_SCRIPT_NUM_LENGTH_GENESIS,
    MAX_SCRIPT_NUM_LENGTH_PREGENESIS,
};
use crate::script::Script;
use hex;
use std::cell::RefCell;

#[test]
fn valid() {
    pass(&[OP_TRUE]);
    pass(&[OP_16]);
    pass(&[OP_PUSH + 1, 1]);
    pass(&[OP_PUSHDATA1, 2, 0, 1]);
    pass(&[OP_PUSHDATA2, 2, 0, 0, 1]);
    pass(&[OP_PUSHDATA4, 2, 0, 0, 0, 0, 1]);
    pass(&[OP_NOP, OP_NOP, OP_NOP, OP_1]);
    pass(&[OP_1, OP_1, OP_IF, OP_ELSE, OP_ENDIF]);
    pass(&[OP_1, OP_1, OP_1, OP_IF, OP_IF, OP_ENDIF, OP_ENDIF]);
    pass(&[OP_1, OP_IF, OP_1, OP_ELSE, OP_0, OP_ENDIF]);
    pass(&[OP_0, OP_IF, OP_0, OP_ELSE, OP_1, OP_ENDIF]);
    pass(&[OP_1, OP_IF, OP_0, OP_1, OP_ENDIF]);
    pass(&[OP_1, OP_IF, OP_0, OP_IF, OP_ELSE, OP_1, OP_ENDIF, OP_ENDIF]);
    pass(&[OP_1, OP_IF, OP_PUSHDATA1, 1, 0, OP_1, OP_ENDIF]);
    pass(&[OP_1, OP_IF, OP_ELSE, OP_ELSE, OP_1, OP_ENDIF]);
    pass(&[
        OP_1, OP_IF, OP_ELSE, OP_ELSE, OP_ELSE, OP_ELSE, OP_1, OP_ENDIF,
    ]);
    pass(&[OP_1, OP_VERIFY, OP_1]);
    pass(&[OP_1, OP_RETURN]);
    pass(&[OP_FALSE, OP_TRUE, OP_RETURN]);
    pass(&[OP_1, OP_0, OP_TOALTSTACK]);
    pass(&[OP_1, OP_TOALTSTACK, OP_FROMALTSTACK]);
    pass(&[OP_1, OP_IFDUP, OP_DROP, OP_DROP, OP_1]);
    pass(&[OP_DEPTH, OP_1]);
    pass(&[OP_0, OP_DEPTH]);
    pass(&[OP_1, OP_0, OP_DROP]);
    pass(&[OP_0, OP_DUP, OP_DROP, OP_DROP, OP_1]);
    pass(&[OP_1, OP_0, OP_0, OP_NIP, OP_DROP]);
    pass(&[OP_1, OP_0, OP_OVER]);
    pass(&[OP_1, OP_0, OP_PICK]);
    pass(&[OP_1, OP_0, OP_0, OP_0, OP_0, OP_4, OP_PICK]);
    pass(&[OP_1, OP_0, OP_ROLL]);
    pass(&[OP_1, OP_0, OP_0, OP_ROLL, OP_DROP]);
    pass(&[OP_1, OP_0, OP_0, OP_0, OP_0, OP_4, OP_ROLL]);
    pass(&[OP_1, OP_0, OP_0, OP_ROT]);
    pass(&[OP_0, OP_1, OP_0, OP_ROT, OP_ROT]);
    pass(&[OP_0, OP_0, OP_1, OP_ROT, OP_ROT, OP_ROT]);
    pass(&[OP_1, OP_0, OP_SWAP]);
    pass(&[OP_0, OP_1, OP_TUCK, OP_DROP, OP_DROP]);
    pass(&[OP_1, OP_0, OP_0, OP_2DROP]);
    pass(&[OP_0, OP_1, OP_2DUP]);
    pass(&[OP_0, OP_1, OP_2DUP, OP_DROP, OP_DROP]);
    pass(&[OP_0, OP_0, OP_1, OP_3DUP]);
    pass(&[OP_0, OP_0, OP_1, OP_3DUP, OP_DROP, OP_DROP, OP_DROP]);
    pass(&[OP_0, OP_1, OP_0, OP_0, OP_2OVER]);
    pass(&[OP_0, OP_0, OP_0, OP_1, OP_2OVER, OP_DROP, OP_DROP]);
    pass(&[OP_0, OP_1, OP_0, OP_0, OP_0, OP_0, OP_2ROT]);
    pass(&[OP_0, OP_0, OP_0, OP_1, OP_0, OP_0, OP_2ROT, OP_2ROT]);
    pass(&[
        OP_0, OP_0, OP_0, OP_0, OP_0, OP_1, OP_2ROT, OP_2ROT, OP_2ROT,
    ]);
    pass(&[OP_1, OP_0, OP_0, OP_0, OP_0, OP_0, OP_2ROT, OP_DROP]);
    pass(&[OP_0, OP_1, OP_0, OP_0, OP_2SWAP]);
    pass(&[OP_1, OP_0, OP_0, OP_0, OP_2SWAP, OP_DROP]);
    pass(&[OP_0, OP_1, OP_CAT]);
    pass(&[OP_1, OP_0, OP_0, OP_2, OP_0, OP_CAT, OP_PICK]);
    pass(&[OP_0, OP_0, OP_CAT, OP_IF, OP_ELSE, OP_1, OP_ENDIF]);
    pass(&[OP_PUSH + 2, OP_0, OP_1, OP_1, OP_SPLIT]);
    pass(&[OP_PUSH + 2, OP_0, OP_1, OP_2, OP_SPLIT, OP_DROP]);
    pass(&[OP_PUSH + 2, OP_0, OP_1, OP_0, OP_SPLIT]);
    pass(&[OP_0, OP_0, OP_SPLIT, OP_1]);
    pass(&[OP_1, OP_1, OP_SPLIT, OP_DROP]);
    pass(&[OP_1, OP_SIZE]);
    pass(&[OP_1, OP_SIZE, OP_DROP]);
    pass(&[OP_1, OP_1, OP_AND]);
    pass(&[OP_1, OP_1, OP_OR]);
    pass(&[OP_1, OP_1, OP_XOR, OP_IF, OP_ELSE, OP_1, OP_ENDIF]);
    pass(&[
        OP_PUSH + 3,
        0xFF,
        0x01,
        0x00,
        OP_INVERT,
        OP_PUSH + 3,
        0x00,
        0xFE,
        0xFF,
        OP_EQUAL,
    ]);
    pass(&[OP_0, OP_0, OP_LSHIFT, OP_0, OP_EQUAL]);
    pass(&[OP_4, OP_2, OP_LSHIFT, OP_16, OP_EQUAL]);
    pass(&[
        OP_PUSH + 2,
        0x12,
        0x34,
        OP_4,
        OP_LSHIFT,
        OP_PUSH + 2,
        0x23,
        0x40,
        OP_EQUAL,
    ]);
    pass(&[OP_0, OP_0, OP_RSHIFT, OP_0, OP_EQUAL]);
    pass(&[OP_4, OP_2, OP_RSHIFT, OP_1, OP_EQUAL]);
    pass(&[
        OP_PUSH + 2,
        0x12,
        0x34,
        OP_4,
        OP_RSHIFT,
        OP_PUSH + 2,
        0x01,
        0x23,
        OP_EQUAL,
    ]);
    pass(&[OP_0, OP_0, OP_EQUAL]);
    pass(&[OP_1, OP_1, OP_EQUAL]);
    pass(&[OP_1, OP_0, OP_0, OP_EQUALVERIFY]);
    pass(&[OP_0, OP_1ADD]);
    pass(&[OP_1, OP_1ADD, OP_2, OP_EQUAL]);
    pass(&[OP_2, OP_1SUB]);
    pass(&[OP_0, OP_1SUB, OP_1NEGATE, OP_EQUAL]);
    let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F];
    v.extend_from_slice(&[OP_1ADD, OP_SIZE, OP_5, OP_EQUAL]);
    pass(&v);
    let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF];
    v.extend_from_slice(&[OP_1SUB, OP_SIZE, OP_5, OP_EQUAL]);
    pass(&v);
    pass(&[OP_1, OP_NEGATE, OP_1NEGATE, OP_EQUAL]);
    pass(&[OP_1NEGATE, OP_NEGATE, OP_1, OP_EQUAL]);
    pass(&[OP_1, OP_ABS, OP_1, OP_EQUAL]);
    pass(&[OP_1NEGATE, OP_ABS, OP_1, OP_EQUAL]);
    pass(&[OP_0, OP_NOT]);
    pass(&[OP_1, OP_NOT, OP_0, OP_EQUAL]);
    pass(&[OP_2, OP_NOT, OP_0, OP_EQUAL]);
    pass(&[OP_1, OP_NOT, OP_NOT]);
    pass(&[OP_1, OP_0NOTEQUAL]);
    pass(&[OP_0, OP_0NOTEQUAL, OP_0, OP_EQUAL]);
    pass(&[OP_2, OP_0NOTEQUAL]);
    pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_1ADD]);
    pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_1SUB]);
    pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NEGATE, OP_1]);
    pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_ABS, OP_1]);
    pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NOT]);
    pass(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_0NOTEQUAL, OP_1]);
    pass(&[OP_0, OP_1, OP_ADD]);
    pass(&[OP_1, OP_0, OP_ADD]);
    pass(&[OP_1, OP_2, OP_ADD, OP_3, OP_EQUAL]);
    let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF];
    v.extend_from_slice(&[OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF]);
    v.extend_from_slice(&[OP_ADD, OP_SIZE, OP_5, OP_EQUAL]);
    pass(&v);
    let mut v = vec![OP_PUSH + 5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    v.extend_from_slice(&[OP_PUSH + 5, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    v.extend_from_slice(&[OP_ADD, OP_SIZE, OP_6, OP_EQUAL]);
    pass(&v);
    let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F];
    v.extend_from_slice(&[OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF]);
    v.extend_from_slice(&[OP_ADD, OP_0, OP_EQUAL]);
    pass(&v);
    pass(&[OP_2, OP_1, OP_SUB]);
    pass(&[OP_1, OP_1, OP_SUB, OP_0, OP_EQUAL]);
    let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0xFF];
    v.extend_from_slice(&[OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F]);
    v.extend_from_slice(&[OP_SUB, OP_SIZE, OP_5, OP_EQUAL]);
    pass(&v);
    let mut v = vec![OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F];
    v.extend_from_slice(&[OP_PUSH + 4, 0xFF, 0xFF, 0xFF, 0x7F]);
    v.extend_from_slice(&[OP_SUB, OP_0, OP_EQUAL]);
    pass(&v);
    pass(&[OP_1, OP_1, OP_MUL, OP_1, OP_EQUAL]);
    pass(&[OP_2, OP_3, OP_MUL, OP_6, OP_EQUAL]);
    pass(&[
        OP_PUSH + 4,
        0xFF,
        0xFF,
        0xFF,
        0x7F,
        OP_PUSH + 4,
        0xFF,
        0xFF,
        0xFF,
        0x7F,
        OP_MUL,
    ]);
    pass(&[OP_1, OP_1NEGATE, OP_MUL, OP_1NEGATE, OP_EQUAL]);
    pass(&[OP_5, OP_2MUL, OP_10, OP_EQUAL]);
    pass(&[OP_1, OP_1, OP_DIV, OP_1, OP_EQUAL]);
    pass(&[OP_5, OP_2, OP_DIV, OP_2, OP_EQUAL]);
    pass(&[OP_2, OP_1NEGATE, OP_DIV, OP_PUSH + 1, 130, OP_EQUAL]);
    pass(&[OP_5, OP_2DIV, OP_2, OP_EQUAL]);
    pass(&[OP_1, OP_1, OP_MOD, OP_0, OP_EQUAL]);
    pass(&[OP_5, OP_2, OP_MOD, OP_1, OP_EQUAL]);
    pass(&[OP_5, OP_PUSH + 1, 130, OP_MOD, OP_1, OP_EQUAL]);
    pass(&[OP_PUSH + 1, 133, OP_2, OP_MOD, OP_1NEGATE, OP_EQUAL]);
    pass(&[OP_1, OP_1, OP_BOOLAND]);
    pass(&[OP_0, OP_1, OP_BOOLAND, OP_0, OP_EQUAL]);
    pass(&[OP_1, OP_0, OP_BOOLOR]);
    pass(&[OP_0, OP_0, OP_BOOLOR, OP_0, OP_EQUAL]);
    pass(&[OP_1, OP_1, OP_NUMEQUAL]);
    pass(&[OP_0, OP_1, OP_NUMEQUAL, OP_NOT]);
    pass(&[OP_1, OP_1, OP_NUMEQUALVERIFY, OP_1]);
    pass(&[OP_1, OP_0, OP_NUMNOTEQUAL]);
    pass(&[OP_1, OP_1, OP_NUMNOTEQUAL, OP_NOT]);
    pass(&[OP_0, OP_1, OP_LESSTHAN]);
    pass(&[OP_1NEGATE, OP_0, OP_LESSTHAN]);
    pass(&[OP_0, OP_0, OP_LESSTHAN, OP_NOT]);
    pass(&[OP_1, OP_0, OP_GREATERTHAN]);
    pass(&[OP_0, OP_1NEGATE, OP_GREATERTHAN]);
    pass(&[OP_0, OP_0, OP_GREATERTHAN, OP_NOT]);
    pass(&[OP_0, OP_1, OP_LESSTHANOREQUAL]);
    pass(&[OP_1NEGATE, OP_0, OP_LESSTHANOREQUAL]);
    pass(&[OP_0, OP_0, OP_LESSTHANOREQUAL]);
    pass(&[OP_1, OP_0, OP_GREATERTHANOREQUAL]);
    pass(&[OP_0, OP_1NEGATE, OP_GREATERTHANOREQUAL]);
    pass(&[OP_0, OP_0, OP_GREATERTHANOREQUAL]);
    pass(&[OP_0, OP_1, OP_MIN, OP_0, OP_EQUAL]);
    pass(&[OP_0, OP_0, OP_MIN, OP_0, OP_EQUAL]);
    pass(&[OP_1NEGATE, OP_0, OP_MIN, OP_1NEGATE, OP_EQUAL]);
    pass(&[OP_0, OP_1, OP_MAX, OP_1, OP_EQUAL]);
    pass(&[OP_0, OP_0, OP_MAX, OP_0, OP_EQUAL]);
    pass(&[OP_1NEGATE, OP_0, OP_MAX, OP_0, OP_EQUAL]);
    pass(&[OP_0, OP_0, OP_1, OP_WITHIN]);
    pass(&[OP_0, OP_1NEGATE, OP_1, OP_WITHIN]);

    pass(&[OP_PUSH + 9, 0, 0, 0, 0, 0, 0, 0, 0, 1, OP_BIN2NUM]);

    //pass(&[OP_PUSH + 4, 128, 0, 0, 1, OP_BIN2NUM, OP_1NEGATE, OP_EQUAL]);

    pass(&[OP_PUSH + 7, 0, 0, 0, 0, 0, 0, 0, OP_BIN2NUM, OP_0, OP_EQUAL]);
    pass(&[OP_PUSH + 5, 129, 0, 0, 0, 0, OP_BIN2NUM]);
    pass(&[OP_1, OP_16, OP_NUM2BIN]);
    pass(&[OP_0, OP_4, OP_NUM2BIN, OP_0, OP_NUMEQUAL]);

    // pass(&[OP_1, OP_DUP, OP_16, OP_NUM2BIN, OP_BIN2NUM, OP_EQUAL]);
    // pass(&[OP_1NEGATE, OP_DUP, OP_16, OP_NUM2BIN, OP_BIN2NUM, OP_EQUAL]);

    pass(&[OP_1, OP_PUSH + 5, 129, 0, 0, 0, 0, OP_NUM2BIN]);

    let mut v = Vec::new();
    v.push(OP_1);
    v.push(OP_PUSH + 2);
    v.extend_from_slice(&encode_num(520).unwrap());
    v.push(OP_NUM2BIN);
    pass(&v);
    pass(&[OP_1, OP_RIPEMD160]);
    pass(&[OP_0, OP_RIPEMD160]);
    let mut s = Script::new();
    let h = "cea1b21f1a739fba68d1d4290437d2c5609be1d3";
    s.append_data(&hex::decode(h).unwrap());
    s.append_data(&hex::decode("0123456789abcdef").unwrap());
    s.append_slice(&[OP_RIPEMD160, OP_EQUAL]);
    pass(&s.0);
    pass(&[OP_1, OP_SHA1]);
    pass(&[OP_0, OP_SHA1]);
    let mut s = Script::new();
    let h = "0ca2eadb529ac2e63abf9b4ae3df8ee121f10547";
    s.append_data(&hex::decode(h).unwrap());
    s.append_data(&hex::decode("0123456789abcdef").unwrap());
    s.append_slice(&[OP_SHA1, OP_EQUAL]);
    pass(&s.0);
    pass(&[OP_1, OP_SHA256]);
    pass(&[OP_0, OP_SHA256]);
    let mut s = Script::new();
    let h = "55c53f5d490297900cefa825d0c8e8e9532ee8a118abe7d8570762cd38be9818";
    s.append_data(&hex::decode(h).unwrap());
    s.append_data(&hex::decode("0123456789abcdef").unwrap());
    s.append_slice(&[OP_SHA256, OP_EQUAL]);
    pass(&s.0);
    pass(&[OP_1, OP_HASH160]);
    pass(&[OP_0, OP_HASH160]);
    let mut s = Script::new();
    let h = "a956ed79819901b1b2c7b3ec045081f749c588ed";
    s.append_data(&hex::decode(h).unwrap());
    s.append_data(&hex::decode("0123456789abcdef").unwrap());
    s.append_slice(&[OP_HASH160, OP_EQUAL]);
    pass(&s.0);
    pass(&[OP_1, OP_HASH256]);
    pass(&[OP_0, OP_HASH256]);
    let mut s = Script::new();
    let h = "137ad663f79da06e282ed0abbec4d70523ced5ff8e39d5c2e5641d978c5925aa";
    s.append_data(&hex::decode(h).unwrap());
    s.append_data(&hex::decode("0123456789abcdef").unwrap());
    s.append_slice(&[OP_HASH256, OP_EQUAL]);
    pass(&s.0);
    pass(&[OP_1, OP_1, OP_CHECKSIG]);
    pass(&[OP_1, OP_1, OP_CHECKSIGVERIFY, OP_1]);
    pass(&[OP_0, OP_0, OP_0, OP_CHECKMULTISIG]);
    pass(&[OP_0, OP_0, OP_9, OP_9, OP_9, OP_3, OP_CHECKMULTISIG]);
    pass(&[OP_0, OP_9, OP_1, OP_9, OP_1, OP_CHECKMULTISIG]);
    pass(&[OP_0, OP_9, OP_1, OP_9, OP_9, OP_9, OP_3, OP_CHECKMULTISIG]);
    let mut c = MockChecker::sig_checks(vec![true]);
    assert!(eval(
        &[OP_0, OP_9, OP_1, OP_9, OP_1, OP_CHECKMULTISIG],
        &mut c,
        NO_FLAGS
    )
    .is_ok());
    let mut c = MockChecker::sig_checks(vec![false, true, true]);
    let mut s = vec![OP_0, OP_9, OP_9, OP_2, OP_9, OP_9, OP_9, OP_3];
    s.push(OP_CHECKMULTISIG);
    assert!(eval(&s, &mut c, NO_FLAGS).is_ok());
    pass_pregenesis(&[OP_0, OP_CHECKLOCKTIMEVERIFY, OP_1]);
    pass(&[OP_CHECKLOCKTIMEVERIFY, OP_1]);
    pass_pregenesis(&[OP_0, OP_CHECKSEQUENCEVERIFY, OP_1]);
    pass(&[OP_CHECKSEQUENCEVERIFY, OP_1]);
    pass(&[OP_NOP1, OP_1]);
    pass(&[OP_NOP9, OP_1]);
    pass(&[OP_NOP10, OP_1]);
    let mut v = vec![OP_DEPTH; 501];
    v.push(OP_1);
    pass(&v);
    pass(&vec![OP_1; 10001]);
}

#[test]
fn invalid() {
    fail(&[]);
    fail(&[OP_FALSE]);
    fail(&[OP_PUSH + 1]);
    fail(&[OP_PUSH + 3, 0, 1]);
    fail(&[OP_PUSHDATA1, 0]);
    fail(&[OP_PUSHDATA1, 1]);
    fail(&[OP_PUSHDATA1, 10, 0]);
    fail(&[OP_PUSHDATA2, 20, 0]);
    fail(&[OP_PUSHDATA4, 30, 0]);
    fail(&[OP_IF, OP_ENDIF]);
    fail(&[OP_1, OP_1, OP_IF]);
    fail(&[OP_1, OP_1, OP_NOTIF]);
    fail(&[OP_1, OP_ELSE]);
    fail(&[OP_1, OP_ENDIF]);
    fail(&[OP_1, OP_1, OP_IF, OP_ELSE]);
    fail(&[OP_1, OP_1, OP_IF, OP_IF, OP_ENDIF]);
    fail(&[OP_0, OP_IF, OP_1, OP_ELSE, OP_0, OP_ENDIF]);
    fail(&[OP_0, OP_IF, OP_PUSHDATA1, 1, 1, OP_1, OP_ENDIF]);
    fail(&[OP_VERIFY]);
    fail(&[OP_0, OP_VERIFY]);
    fail(&[OP_RETURN]);
    fail(&[OP_FALSE, OP_RETURN]);
    fail_pregenesis(&[OP_RETURN]);
    fail_pregenesis(&[OP_1, OP_RETURN, OP_1]);
    fail(&[OP_TOALTSTACK]);
    fail(&[OP_FROMALTSTACK]);
    fail(&[OP_0, OP_TOALTSTACK, OP_1, OP_FROMALTSTACK]);
    fail(&[OP_IFDUP]);
    fail(&[OP_DROP]);
    fail(&[OP_1, OP_DROP, OP_DROP]);
    fail(&[OP_DUP]);
    fail(&[OP_NIP]);
    fail(&[OP_1, OP_NIP]);
    fail(&[OP_OVER]);
    fail(&[OP_1, OP_OVER]);
    fail(&[OP_PICK]);
    fail(&[OP_0, OP_PICK]);
    fail(&[OP_0, OP_1, OP_PICK]);
    fail(&[OP_ROLL]);
    fail(&[OP_0, OP_ROLL]);
    fail(&[OP_0, OP_1, OP_ROLL]);
    fail(&[OP_ROT]);
    fail(&[OP_1, OP_ROT]);
    fail(&[OP_1, OP_1, OP_ROT]);
    fail(&[OP_0, OP_1, OP_1, OP_ROT]);
    fail(&[OP_SWAP]);
    fail(&[OP_1, OP_SWAP]);
    fail(&[OP_0, OP_1, OP_SWAP]);
    fail(&[OP_TUCK]);
    fail(&[OP_1, OP_TUCK]);
    fail(&[OP_1, OP_0, OP_TUCK]);
    fail(&[OP_2DROP]);
    fail(&[OP_1, OP_2DROP]);
    fail(&[OP_1, OP_1, OP_2DROP]);
    fail(&[OP_2DUP]);
    fail(&[OP_1, OP_2DUP]);
    fail(&[OP_1, OP_0, OP_2DUP]);
    fail(&[OP_3DUP]);
    fail(&[OP_1, OP_3DUP]);
    fail(&[OP_1, OP_1, OP_3DUP]);
    fail(&[OP_1, OP_1, OP_0, OP_3DUP]);
    fail(&[OP_2OVER]);
    fail(&[OP_1, OP_2OVER]);
    fail(&[OP_1, OP_1, OP_2OVER]);
    fail(&[OP_1, OP_1, OP_1, OP_2OVER]);
    fail(&[OP_1, OP_0, OP_1, OP_1, OP_2OVER]);
    fail(&[OP_2ROT]);
    fail(&[OP_1, OP_2ROT]);
    fail(&[OP_1, OP_1, OP_2ROT]);
    fail(&[OP_1, OP_1, OP_1, OP_2ROT]);
    fail(&[OP_1, OP_1, OP_1, OP_1, OP_2ROT]);
    fail(&[OP_1, OP_1, OP_1, OP_1, OP_1, OP_2ROT]);
    fail(&[OP_1, OP_0, OP_1, OP_1, OP_1, OP_1, OP_2ROT]);
    fail(&[OP_2SWAP]);
    fail(&[OP_1, OP_2SWAP]);
    fail(&[OP_1, OP_1, OP_2SWAP]);
    fail(&[OP_1, OP_1, OP_1, OP_2SWAP]);
    fail(&[OP_1, OP_0, OP_1, OP_1, OP_2SWAP]);
    fail(&[OP_CAT]);
    fail(&[OP_1, OP_CAT]);
    fail(&[OP_1, OP_0, OP_0, OP_CAT]);
    fail(&[OP_SPLIT]);
    fail(&[OP_1, OP_SPLIT]);
    fail(&[OP_0, OP_1, OP_SPLIT]);
    fail(&[OP_1, OP_2, OP_SPLIT]);
    fail(&[OP_1, OP_1NEGATE, OP_SPLIT]);
    fail(&[OP_0, OP_SIZE]);
    fail(&[OP_AND]);
    fail(&[OP_0, OP_AND]);
    fail(&[OP_0, OP_1, OP_AND]);
    fail(&[OP_OR]);
    fail(&[OP_0, OP_OR]);
    fail(&[OP_0, OP_1, OP_OR]);
    fail(&[OP_XOR]);
    fail(&[OP_0, OP_XOR]);
    fail(&[OP_0, OP_1, OP_XOR]);
    fail(&[OP_LSHIFT]);
    fail(&[OP_1, OP_LSHIFT]);
    fail(&[OP_1, OP_1NEGATE, OP_LSHIFT]);
    fail(&[OP_RSHIFT]);
    fail(&[OP_1, OP_RSHIFT]);
    fail(&[OP_1, OP_1NEGATE, OP_RSHIFT]);
    fail(&[OP_INVERT]);
    fail(&[OP_EQUAL]);
    fail(&[OP_0, OP_EQUAL]);
    fail(&[OP_1, OP_0, OP_EQUAL]);
    fail(&[OP_1, OP_0, OP_EQUALVERIFY, OP_1]);
    fail(&[OP_1ADD]);
    fail(&[OP_1SUB]);
    fail(&[OP_NEGATE]);
    fail(&[OP_ABS]);
    fail(&[OP_NOT]);
    fail(&[OP_0NOTEQUAL]);
    fail(&[OP_ADD]);
    fail(&[OP_1, OP_ADD]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_ADD]);
    fail(&[OP_SUB]);
    fail(&[OP_1, OP_SUB]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_SUB]);
    fail(&[OP_MUL]);
    fail(&[OP_1, OP_MUL]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_MUL]);
    fail(&[OP_PUSH + 2, 0, 0, OP_PUSH + 2, 0, 0, OP_MUL]);
    fail(&[OP_DIV]);
    fail(&[OP_2MUL]);
    fail(&[OP_1, OP_DIV]);
    fail(&[OP_2DIV]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_DIV]);
    fail(&[OP_1, OP_0, OP_DIV]);
    fail(&[OP_MOD]);
    fail(&[OP_1, OP_MOD]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_MOD]);
    fail(&[OP_1, OP_0, OP_MOD]);
    fail(&[OP_BOOLAND]);
    fail(&[OP_1, OP_BOOLAND]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_BOOLAND]);
    fail(&[OP_BOOLOR]);
    fail(&[OP_1, OP_BOOLOR]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_BOOLOR]);
    fail(&[OP_NUMEQUAL]);
    fail(&[OP_1, OP_NUMEQUAL]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NUMEQUAL]);
    fail(&[OP_0, OP_1, OP_NUMEQUAL]);
    fail(&[OP_NUMEQUALVERIFY]);
    fail(&[OP_1, OP_NUMEQUALVERIFY]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NUMEQUALVERIFY]);
    fail(&[OP_1, OP_2, OP_NUMEQUALVERIFY]);
    fail(&[OP_NUMNOTEQUAL]);
    fail(&[OP_1, OP_NUMNOTEQUAL]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_NUMNOTEQUAL]);
    fail(&[OP_1, OP_1, OP_NUMNOTEQUAL]);
    fail(&[OP_LESSTHAN]);
    fail(&[OP_1, OP_LESSTHAN]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_LESSTHAN]);
    fail(&[OP_1, OP_0, OP_LESSTHAN]);
    fail(&[OP_GREATERTHAN]);
    fail(&[OP_1, OP_GREATERTHAN]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_GREATERTHAN]);
    fail(&[OP_0, OP_1, OP_GREATERTHAN]);
    fail(&[OP_LESSTHANOREQUAL]);
    fail(&[OP_1, OP_LESSTHANOREQUAL]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_LESSTHANOREQUAL]);
    fail(&[OP_1, OP_0, OP_LESSTHANOREQUAL]);
    fail(&[OP_GREATERTHANOREQUAL]);
    fail(&[OP_1, OP_GREATERTHANOREQUAL]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_GREATERTHANOREQUAL]);
    fail(&[OP_0, OP_1, OP_GREATERTHANOREQUAL]);
    fail(&[OP_MIN]);
    fail(&[OP_1, OP_MIN]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_MIN]);
    fail(&[OP_MAX]);
    fail(&[OP_1, OP_MAX]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_MAX]);
    fail(&[OP_WITHIN]);
    fail(&[OP_1, OP_WITHIN]);
    fail(&[OP_1, OP_1, OP_WITHIN]);
    fail(&[OP_PUSH + 5, 0, 0, 0, 0, 0, OP_WITHIN]);
    fail(&[OP_0, OP_1, OP_2, OP_WITHIN]);
    fail(&[OP_0, OP_1NEGATE, OP_0, OP_WITHIN]);
    fail(&[OP_BIN2NUM]);
    fail(&[OP_NUM2BIN]);
    fail(&[OP_1, OP_NUM2BIN]);
    fail(&[OP_1, OP_0, OP_NUM2BIN]);
    fail(&[OP_1, OP_1NEGATE, OP_NUM2BIN]);
    fail(&[OP_PUSH + 5, 129, 0, 0, 0, 0, OP_1, OP_NUM2BIN]);
    fail(&[OP_RIPEMD160]);
    fail(&[OP_SHA1]);
    fail(&[OP_SHA256]);
    fail(&[OP_HASH160]);
    fail(&[OP_HASH256]);
    fail(&[OP_CHECKSIG]);
    fail(&[OP_1, OP_CHECKSIG]);
    let mut c = MockChecker::sig_checks(vec![false; 1]);
    assert!(eval(&[OP_1, OP_1, OP_CHECKSIG], &mut c, NO_FLAGS).is_err());
    fail(&[OP_CHECKSIGVERIFY]);
    fail(&[OP_1, OP_CHECKSIGVERIFY]);
    let mut c = MockChecker::sig_checks(vec![false; 1]);
    assert!(eval(&[OP_1, OP_1, OP_CHECKSIGVERIFY, OP_1], &mut c, NO_FLAGS).is_err());
    fail(&[OP_CHECKMULTISIG]);
    fail(&[OP_1, OP_CHECKMULTISIG]);
    fail(&[OP_0, OP_0, OP_CHECKMULTISIG]);
    fail(&[OP_0, OP_0, OP_1NEGATE, OP_CHECKMULTISIG]);
    fail(&[OP_0, OP_1NEGATE, OP_0, OP_CHECKMULTISIG]);
    fail(&[OP_0, OP_0, OP_1, OP_CHECKMULTISIG]);
    fail(&[OP_0, OP_0, OP_PUSH + 1, 21, OP_CHECKMULTISIG]);
    fail(&[OP_0, OP_9, OP_9, OP_2, OP_9, OP_1, OP_CHECKMULTISIG]);
    let mut c = MockChecker::sig_checks(vec![false; 1]);
    assert!(eval(
        &[OP_0, OP_9, OP_1, OP_9, OP_1, OP_CHECKMULTISIG],
        &mut c,
        NO_FLAGS
    )
    .is_err());
    let mut c = MockChecker::sig_checks(vec![true, false]);
    let s = [OP_0, OP_9, OP_9, OP_2, OP_9, OP_9, OP_2, OP_CHECKMULTISIG];
    assert!(eval(&s, &mut c, NO_FLAGS).is_err());
    let mut c = MockChecker::sig_checks(vec![false, true, false]);
    let mut s = vec![OP_0, OP_9, OP_9, OP_2, OP_9, OP_9, OP_9, OP_3];
    s.push(OP_CHECKMULTISIG);
    assert!(eval(&s, &mut c, NO_FLAGS).is_err());
    fail_pregenesis(&[OP_CHECKLOCKTIMEVERIFY, OP_1]);
    fail_pregenesis(&[OP_PUSH + 5, 129, 0, 0, 0, 0, OP_CHECKLOCKTIMEVERIFY, OP_1]);
    let mut c = MockChecker::locktime_checks(vec![false]);
    assert!(eval(
        &[OP_0, OP_CHECKLOCKTIMEVERIFY, OP_1],
        &mut c,
        PREGENESIS_RULES
    )
    .is_err());
    fail_pregenesis(&[OP_CHECKSEQUENCEVERIFY, OP_1]);
    fail_pregenesis(&[OP_PUSH + 5, 129, 0, 0, 0, 0, OP_CHECKSEQUENCEVERIFY, OP_1]);
    let mut c = MockChecker::sequence_checks(vec![false]);
    assert!(eval(
        &[OP_0, OP_CHECKSEQUENCEVERIFY, OP_1],
        &mut c,
        PREGENESIS_RULES
    )
    .is_err());
    fail(&[OP_RESERVED, OP_1]);
    fail(&[OP_VER, OP_1]);
    fail(&[OP_VERIF, OP_1]);
    fail(&[OP_VERNOTIF, OP_1]);
    fail(&[OP_RESERVED1, OP_1]);
    fail(&[OP_RESERVED2, OP_1]);
    fail(&[OP_INVERT, OP_1]);
    fail(&[OP_MUL, OP_1]);
    fail(&[OP_LSHIFT, OP_1]);
    fail(&[OP_RSHIFT, OP_1]);
    fail(&[OP_INVALID_ABOVE, OP_1]);
    fail(&[OP_PUBKEYHASH, OP_1]);
    fail(&[OP_PUBKEY, OP_1]);
    fail(&[OP_INVALIDOPCODE, OP_1]);
}

#[test]
fn next_op_tests() {
    let script = [];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_0, OP_CHECKSIG, OP_ADD];
    assert!(next_op(0, &script) == 1);
    assert!(next_op(1, &script) == 2);
    assert!(next_op(2, &script) == script.len());

    let script = [OP_1, OP_PUSH + 4, 1, 2, 3, 4, OP_1];
    assert!(next_op(0, &script) == 1);
    assert!(next_op(1, &script) == 6);
    assert!(next_op(6, &script) == script.len());

    let script = [OP_1, OP_PUSHDATA1, 2, 3, 4, OP_1];
    assert!(next_op(0, &script) == 1);
    assert!(next_op(1, &script) == 5);
    assert!(next_op(5, &script) == script.len());

    let script = [OP_1, OP_PUSHDATA2, 2, 0, 3, 4, OP_1];
    assert!(next_op(0, &script) == 1);
    assert!(next_op(1, &script) == 6);
    assert!(next_op(6, &script) == script.len());

    let script = [OP_1, OP_PUSHDATA4, 2, 0, 0, 0, 3, 4, OP_1];
    assert!(next_op(0, &script) == 1);
    assert!(next_op(1, &script) == 8);
    assert!(next_op(8, &script) == script.len());

    // Parse failures

    let script = [OP_PUSH + 1];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_PUSH + 3, 1, 2];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_PUSHDATA1];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_PUSHDATA1, 2, 1];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_PUSHDATA2];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_PUSHDATA2, 0];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_PUSHDATA2, 2, 0, 1];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_PUSHDATA4];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_PUSHDATA4, 1, 2, 3];
    assert!(next_op(0, &script) == script.len());

    let script = [OP_PUSHDATA4, 2, 0, 0, 0, 1];
    assert!(next_op(0, &script) == script.len());
}

#[test]
fn remove_sig_tests() {
    let expected: Vec<u8> = vec![];
    assert!(remove_sig(&[], &[]) == expected);

    let expected: Vec<u8> = vec![OP_0];
    assert!(remove_sig(&[], &[OP_0]) == expected);

    let expected: Vec<u8> = vec![];
    assert!(remove_sig(&[OP_0], &[OP_0]) == expected);
    let v = [OP_0, OP_1, OP_2, OP_3, OP_4, OP_0, OP_1, OP_2, OP_3, OP_4];
    assert!(remove_sig(&[OP_2, OP_3], &v) == vec![OP_0, OP_1, OP_4, OP_0, OP_1, OP_4]);
}

/// A test run that doesn't do signature checks and expects failure
fn pass(script: &[u8]) {
    let mut c = MockChecker::new();
    assert!(eval(script, &mut c, NO_FLAGS).is_ok());
}

/// A test run that doesn't do signature checks and expects failure
fn fail(script: &[u8]) {
    let mut c = MockChecker::new();
    assert!(eval(script, &mut c, NO_FLAGS).is_err());
}

/// Pre-genesis versions of the above checks
fn pass_pregenesis(script: &[u8]) {
    let mut c = MockChecker::new();
    assert!(eval(script, &mut c, PREGENESIS_RULES).is_ok());
}

/// A test run that doesn't do signature checks and expects failure
fn fail_pregenesis(script: &[u8]) {
    let mut c = MockChecker::new();
    assert!(eval(script, &mut c, PREGENESIS_RULES).is_err());
}

/// Mocks a transaction checker to always return a set of values
struct MockChecker {
    sig_checks: RefCell<Vec<bool>>,
    locktime_checks: RefCell<Vec<bool>>,
    sequence_checks: RefCell<Vec<bool>>,
    tx_version: Option<i32>,
}

impl MockChecker {
    fn new() -> MockChecker {
        MockChecker {
            sig_checks: RefCell::new(vec![true; 32]),
            locktime_checks: RefCell::new(vec![true; 32]),
            sequence_checks: RefCell::new(vec![true; 32]),
            tx_version: None,
        }
    }

    fn with_tx_version(version: i32) -> MockChecker {
        MockChecker {
            sig_checks: RefCell::new(vec![true; 32]),
            locktime_checks: RefCell::new(vec![true; 32]),
            sequence_checks: RefCell::new(vec![true; 32]),
            tx_version: Some(version),
        }
    }

    fn sig_checks(sig_checks: Vec<bool>) -> MockChecker {
        MockChecker {
            sig_checks: RefCell::new(sig_checks),
            locktime_checks: RefCell::new(vec![true; 32]),
            sequence_checks: RefCell::new(vec![true; 32]),
            tx_version: None,
        }
    }

    fn locktime_checks(locktime_checks: Vec<bool>) -> MockChecker {
        MockChecker {
            sig_checks: RefCell::new(vec![true; 32]),
            locktime_checks: RefCell::new(locktime_checks),
            sequence_checks: RefCell::new(vec![true; 32]),
            tx_version: None,
        }
    }

    fn sequence_checks(sequence_checks: Vec<bool>) -> MockChecker {
        MockChecker {
            sig_checks: RefCell::new(vec![true; 32]),
            locktime_checks: RefCell::new(vec![true; 32]),
            sequence_checks: RefCell::new(sequence_checks),
            tx_version: None,
        }
    }
}

impl Checker for MockChecker {
    fn check_sig(
        &mut self,
        _sig: &[u8],
        _pubkey: &[u8],
        _script: &[u8],
    ) -> Result<bool, ChainGangError> {
        Ok(self.sig_checks.borrow_mut().pop().unwrap())
    }

    fn check_locktime(&self, _locktime: i32) -> Result<bool, ChainGangError> {
        Ok(self.locktime_checks.borrow_mut().pop().unwrap())
    }

    fn check_sequence(&self, _sequence: i32) -> Result<bool, ChainGangError> {
        Ok(self.sequence_checks.borrow_mut().pop().unwrap())
    }

    fn tx_version(&self) -> Result<i32, ChainGangError> {
        self.tx_version.ok_or_else(|| {
            ChainGangError::IllegalState("Illegal transaction version check".to_string())
        })
    }
}

fn pass_with_version(script: &[u8], version: i32) {
    let mut c = MockChecker::with_tx_version(version);
    assert!(eval(script, &mut c, NO_FLAGS).is_ok());
}

#[test]
fn chronicle_op_ver_pushes_tx_version() {
    pass_with_version(&[OP_VER, OP_2, OP_NUMEQUAL], 2);
}

#[test]
fn chronicle_op_verif_executes_when_version_is_high_enough() {
    pass_with_version(&[OP_2, OP_VERIF, OP_1, OP_ELSE, OP_0, OP_ENDIF], 2);
    let mut c = MockChecker::with_tx_version(1);
    assert!(eval(
        &[OP_2, OP_VERIF, OP_1, OP_ELSE, OP_0, OP_ENDIF],
        &mut c,
        NO_FLAGS
    )
    .is_err());
}

#[test]
fn chronicle_op_vernotif_executes_when_version_is_too_low() {
    pass_with_version(&[OP_2, OP_VERNOTIF, OP_1, OP_ELSE, OP_0, OP_ENDIF], 1);
    let mut c = MockChecker::with_tx_version(2);
    assert!(eval(
        &[OP_2, OP_VERNOTIF, OP_1, OP_ELSE, OP_0, OP_ENDIF],
        &mut c,
        NO_FLAGS
    )
    .is_err());
}

#[test]
fn chronicle_op_substr_left_right() {
    let mut s = Script::new();
    s.append_data(b"BSV Blockchain");
    s.append(OP_4);
    s.append(OP_5);
    s.append(OP_SUBSTR);
    s.append_data(b"Block");
    s.append(OP_EQUALVERIFY);
    s.append_data(b"BSV Blockchain");
    s.append(OP_3);
    s.append(OP_LEFT);
    s.append_data(b"BSV");
    s.append(OP_EQUALVERIFY);
    s.append_data(b"BSV Blockchain");
    s.append(OP_5);
    s.append(OP_RIGHT);
    s.append_data(b"chain");
    s.append(OP_EQUAL);
    pass(&s.0);
}

#[test]
fn chronicle_op_lshiftnum_rshiftnum() {
    pass(&[OP_4, OP_2, OP_LSHIFTNUM, OP_16, OP_EQUAL]);
    pass(&[OP_4, OP_2, OP_RSHIFTNUM, OP_1, OP_EQUAL]);
}

#[test]
fn uses_two_phase_eval_gated_on_version() {
    assert!(!uses_two_phase_eval(1));
    assert!(uses_two_phase_eval(2));
}

#[test]
fn chronicle_two_phase_carries_stack() {
    let unlock = [OP_2, OP_3, OP_ADD];
    let lock = [OP_5, OP_EQUAL];
    let mut c = MockChecker::new();
    assert!(eval_two_phase(&unlock, &lock, &mut c, NO_FLAGS).is_ok());
    let mut c2 = MockChecker::new();
    let (stack, _) = eval_two_phase_with_stack(&unlock, &lock, &mut c2, NO_FLAGS).unwrap();
    assert_eq!(stack.len(), 1);
}

#[test]
fn chronicle_two_phase_clears_alt_stack() {
    let unlock = [OP_1, OP_TOALTSTACK, OP_2, OP_3, OP_ADD];
    let lock = [OP_5, OP_EQUAL];
    let mut c = MockChecker::new();
    assert!(eval_two_phase(&unlock, &lock, &mut c, NO_FLAGS).is_ok());
}

struct ScriptRecordingChecker {
    sig_checks: RefCell<Vec<bool>>,
    scripts: RefCell<Vec<Vec<u8>>>,
}

impl ScriptRecordingChecker {
    fn new(sig_checks: Vec<bool>) -> Self {
        ScriptRecordingChecker {
            sig_checks: RefCell::new(sig_checks),
            scripts: RefCell::new(Vec::new()),
        }
    }
}

impl Checker for ScriptRecordingChecker {
    fn check_sig(
        &mut self,
        _sig: &[u8],
        _pubkey: &[u8],
        script: &[u8],
    ) -> Result<bool, ChainGangError> {
        self.scripts.borrow_mut().push(script.to_vec());
        Ok(self.sig_checks.borrow_mut().pop().unwrap())
    }

    fn check_locktime(&self, _locktime: i32) -> Result<bool, ChainGangError> {
        Ok(true)
    }

    fn check_sequence(&self, _sequence: i32) -> Result<bool, ChainGangError> {
        Ok(true)
    }
}

#[test]
fn chronicle_two_phase_unlock_checksig_script_code() {
    let mut unlock = Script::new();
    unlock.append_data(b"s0");
    unlock.append_data(b"s1");
    unlock.append(OP_CODESEPARATOR);
    unlock.append_data(b"p1");
    unlock.append(OP_CHECKSIG);

    let mut lock = Script::new();
    lock.append_data(b"p0");
    lock.append(OP_CHECKSIG);

    let mut expected_unlock_code = Script::new();
    expected_unlock_code.append_data(b"p1");
    expected_unlock_code.append(OP_CHECKSIG);
    expected_unlock_code.append_data(b"p0");
    expected_unlock_code.append(OP_CHECKSIG);

    let mut expected_lock_code = Script::new();
    expected_lock_code.append_data(b"p0");
    expected_lock_code.append(OP_CHECKSIG);

    let mut checker = ScriptRecordingChecker::new(vec![true, true]);
    assert!(eval_two_phase(&unlock.0, &lock.0, &mut checker, NO_FLAGS).is_ok());

    let scripts = checker.scripts.borrow();
    assert_eq!(scripts.len(), 2);
    assert_eq!(scripts[0], expected_unlock_code.0);
    assert_eq!(scripts[1], expected_lock_code.0);
}

#[test]
fn strip_code_separators_removes_codesep_bytes() {
    let mut script = Script::new();
    script.append(OP_1);
    script.append(OP_CODESEPARATOR);
    script.append(OP_2);
    script.append(OP_CODESEPARATOR);
    script.append(OP_3);

    let stripped = strip_code_separators(&script.0);
    assert_eq!(stripped, [OP_1, OP_2, OP_3]);
}

#[test]
fn uses_relaxed_malleability_gated_on_version() {
    assert!(!uses_relaxed_malleability(1));
    assert!(uses_relaxed_malleability(2));
}

#[test]
fn chronicle_relaxed_clean_stack_allows_extra_items() {
    let mut c = MockChecker::with_tx_version(2);
    assert!(eval(&[OP_1, OP_1], &mut c, NO_FLAGS).is_ok());
}

#[test]
fn strict_clean_stack_rejects_extra_items() {
    let mut c = MockChecker::with_tx_version(1);
    assert!(eval(&[OP_1, OP_1], &mut c, NO_FLAGS).is_err());
}

#[test]
fn chronicle_minimalif_allows_non_minimal_true_operand() {
    let mut s = Script::new();
    s.append_data(&[0, 0, 0, 127]);
    s.append(OP_IF);
    s.append(OP_1);
    s.append(OP_ENDIF);
    s.append(OP_1);
    let mut c = MockChecker::with_tx_version(2);
    assert!(eval(&s.0, &mut c, NO_FLAGS).is_ok());
}

#[test]
fn strict_minimalif_rejects_non_minimal_true_operand() {
    let mut s = Script::new();
    s.append_data(&[0, 0, 0, 127]);
    s.append(OP_IF);
    s.append(OP_1);
    s.append(OP_ENDIF);
    s.append(OP_1);
    let mut c = MockChecker::with_tx_version(1);
    assert!(eval(&s.0, &mut c, NO_FLAGS).is_err());
}

#[test]
fn chronicle_nullfail_allows_failed_checksig_with_nonempty_sig() {
    let mut c = MockChecker {
        sig_checks: RefCell::new(vec![false]),
        locktime_checks: RefCell::new(vec![true; 32]),
        sequence_checks: RefCell::new(vec![true; 32]),
        tx_version: Some(2),
    };
    let mut script = Script::new();
    script.append_data(&[0x01]);
    script.append_data(&[0x02]);
    script.append(OP_CHECKSIG);
    script.append(OP_DROP);
    script.append(OP_1);
    assert!(eval(&script.0, &mut c, NO_FLAGS).is_ok());
}

#[test]
fn strict_nullfail_rejects_failed_checksig_with_nonempty_sig() {
    let mut c = MockChecker {
        sig_checks: RefCell::new(vec![false]),
        locktime_checks: RefCell::new(vec![true; 32]),
        sequence_checks: RefCell::new(vec![true; 32]),
        tx_version: Some(1),
    };
    let mut script = Script::new();
    script.append_data(&[0x01]);
    script.append_data(&[0x02]);
    script.append(OP_CHECKSIG);
    assert!(eval(&script.0, &mut c, NO_FLAGS).is_err());
}

#[test]
fn is_push_only_accepts_data_pushes() {
    let mut script = Script::new();
    script.append_data(b"sig");
    script.append_data(b"pubkey");
    assert!(is_push_only(&script.0));
}

#[test]
fn is_push_only_rejects_functional_opcodes() {
    assert!(!is_push_only(&[OP_2, OP_3, OP_ADD]));
}

#[test]
fn max_script_num_length_by_context() {
    let c1 = MockChecker::with_tx_version(1);
    assert_eq!(
        max_script_num_length(&c1, NO_FLAGS),
        MAX_SCRIPT_NUM_LENGTH_GENESIS
    );
    let c2 = MockChecker::with_tx_version(2);
    assert_eq!(
        max_script_num_length(&c2, NO_FLAGS),
        MAX_SCRIPT_NUM_LENGTH_CHRONICLE
    );
    let c3 = MockChecker::new();
    assert_eq!(
        max_script_num_length(&c3, NO_FLAGS),
        MAX_SCRIPT_NUM_LENGTH_GENESIS
    );
    assert_eq!(
        max_script_num_length(&c3, PREGENESIS_RULES),
        MAX_SCRIPT_NUM_LENGTH_PREGENESIS
    );
}

#[test]
fn genesis_script_num_limit_rejects_oversized_bin2num() {
    let mut script = Script::new();
    script.append(OP_PUSHDATA4);
    script.append_slice(&(750_001_u32).to_le_bytes());
    script.0.extend(std::iter::repeat_n(0u8, 750_001));
    script.append(OP_BIN2NUM);
    script.append(OP_1);
    let mut c = MockChecker::with_tx_version(1);
    assert!(eval(&script.0, &mut c, NO_FLAGS).is_err());
}

#[test]
fn chronicle_script_num_limit_accepts_genesis_max_bin2num() {
    let mut script = Script::new();
    script.append(OP_PUSHDATA4);
    script.append_slice(&(750_000_u32).to_le_bytes());
    script.0.extend(std::iter::repeat_n(0u8, 750_000));
    script.append(OP_BIN2NUM);
    script.append(OP_1);
    let mut c = MockChecker::with_tx_version(2);
    assert!(eval(&script.0, &mut c, NO_FLAGS).is_ok());
}
