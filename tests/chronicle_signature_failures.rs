//! Current BSV distinguishes encoding errors from an ECDSA false result.
use chain_gang::{
    messages::{OutPoint, Tx, TxIn, TxOut},
    script::{op_codes::*, Checker, Script, ZVersionChecker},
    util::{ChainGangError, Hash256},
};
use linked_hash_map::LinkedHashMap;
use std::collections::HashSet;

fn check(sig: &[u8], key: &[u8], version: u32, multisig: bool, verify: bool) -> bool {
    validate(sig, key, version, multisig, verify).is_ok()
}

fn validate(
    sig: &[u8],
    key: &[u8],
    version: u32,
    multisig: bool,
    verify: bool,
) -> Result<(), ChainGangError> {
    let point = OutPoint {
        hash: Hash256([19; 32]),
        index: 0,
    };
    let mut lock = Script::new();
    let mut unlock = Script::new();
    if multisig {
        unlock.append(OP_0);
        lock.append(OP_1);
    }
    unlock.append_data(sig);
    lock.append_data(key);
    if multisig {
        lock.append(OP_1);
    }
    lock.append(match (multisig, verify) {
        (true, true) => OP_CHECKMULTISIGVERIFY,
        (true, false) => OP_CHECKMULTISIG,
        (false, true) => OP_CHECKSIGVERIFY,
        (false, false) => OP_CHECKSIG,
    });
    lock.append(if verify { OP_1 } else { OP_NOT });
    let utxos = LinkedHashMap::from_iter([(
        point.clone(),
        TxOut {
            satoshis: 1000,
            lock_script: lock,
        },
    )]);
    let tx = Tx {
        version,
        inputs: vec![TxIn {
            prev_output: point,
            unlock_script: unlock,
            sequence: u32::MAX,
        }],
        outputs: vec![TxOut {
            satoshis: 900,
            lock_script: Script(vec![OP_1]),
        }],
        lock_time: 0,
    };
    tx.validate_consensus(true, true, &utxos, &HashSet::new())
}

#[test]
fn encoded_invalid_ecdsa_is_false_under_chronicle() {
    let pubkey =
        hex::decode("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798").unwrap();
    let zero_r = hex::decode("300602010002010161").unwrap();
    for multisig in [false, true] {
        assert!(check(&zero_r, &pubkey, 2, multisig, false));
        assert!(!check(&zero_r, &pubkey, 1, multisig, false));
        assert!(!check(&zero_r, &pubkey, 2, multisig, true));
    }
    let mut checker = ZVersionChecker {
        z: Hash256([1; 32]),
        tx_version: 2,
    };
    assert!(!checker.check_sig(&zero_r, &pubkey, &[]).unwrap());
}

#[test]
fn empty_signature_does_not_bypass_pubkey_encoding() {
    assert!(!check(&[], &[0x02], 2, false, false));
}

#[test]
fn canonical_scalars_and_key_shapes_keep_the_node_boundary() {
    let key =
        hex::decode("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798").unwrap();
    let order =
        hex::decode("00fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141").unwrap();
    let der = |r: &[u8], s: &[u8], flag| {
        let mut result = vec![0x30, (r.len() + s.len() + 4) as u8, 2, r.len() as u8];
        result.extend_from_slice(r);
        result.extend_from_slice(&[2, s.len() as u8]);
        result.extend_from_slice(s);
        result.push(flag);
        result
    };
    for (r, s) in [
        (&[0][..], &[1][..]),
        (&[1][..], &[0][..]),
        (order.as_slice(), &[1][..]),
        (&[1][..], order.as_slice()),
        (&[1][..], &[1][..]),
    ] {
        for flag in [0x41, 0x61, 0x63, 0xe3] {
            let sig = der(r, s, flag);
            for multi in [false, true] {
                assert!(check(&sig, &key, 2, multi, false));
                assert!(!check(&sig, &key, 1, multi, false));
                assert!(!check(&sig, &key, 2, multi, true));
            }
        }
    }
    let off_curve = [vec![2], vec![255; 32]].concat();
    for sig in [vec![], der(&[1], &[1], 0x61)] {
        assert!(check(&sig, &off_curve, 2, false, false));
        for invalid_key in [vec![], vec![2], vec![6; 65], vec![4; 33]] {
            assert!(!check(&sig, &invalid_key, 2, false, false));
        }
    }
    for malformed in [
        "300602018002010161",
        "30070202000102010161",
        "30060201010201010061",
        "300602010102010160",
        "300602010102010121",
    ] {
        assert!(!check(
            &hex::decode(malformed).unwrap(),
            &key,
            2,
            false,
            false
        ));
    }
    assert!(check(&[], &key, 1, false, false));
}

#[test]
fn debug_checker_accepts_high_s_only_for_chronicle_versions() {
    use chain_gang::transaction::generate_signature;
    use k256::ecdsa::{signature::SignatureEncoding, Signature, SigningKey};
    use num_bigint::BigUint;

    fn high(sig: &[u8]) -> Vec<u8> {
        let s = Signature::from_der(&sig[..sig.len() - 1]).unwrap();
        let mut raw = s.to_bytes();
        let n = BigUint::parse_bytes(
            b"FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141",
            16,
        )
        .unwrap();
        let h = (n - BigUint::from_bytes_be(&raw[32..])).to_bytes_be();
        raw[32..].fill(0);
        raw[64 - h.len()..].copy_from_slice(&h);
        let mut out = Signature::try_from(raw.as_ref()).unwrap().to_der().to_vec();
        out.push(*sig.last().unwrap());
        out
    }

    let private_key = [9; 32];
    let key = SigningKey::from_slice(&private_key).unwrap();
    let pubkey = key.verifying_key().to_sec1_bytes();
    let digest = Hash256([3; 32]);
    let signature = high(&generate_signature(&private_key, &digest, 0x61).unwrap());
    for version in [-1, 0, 1, 2] {
        let mut checker = ZVersionChecker {
            z: digest,
            tx_version: version,
        };
        assert_eq!(
            checker.check_sig(&signature, &pubkey, &[]).unwrap(),
            version > 1
        );
    }
}

#[test]
fn signature_errors_precede_pubkey_errors() {
    for (signature, expected) in [
        (
            "300602010102010160",
            "Signature hash type missing or not understood",
        ),
        ("300602010102010101", "SIGHASH_FORKID not present"),
        ("3006020101020101ffff", "Non-canonical DER signature"),
        ("300602010102010141", "Non-canonical public key"),
        ("", "Non-canonical public key"),
    ] {
        let sig = hex::decode(signature).unwrap();
        for multisig in [false, true] {
            for verify in [false, true] {
                let err = validate(&sig, &[2, 0], 2, multisig, verify).unwrap_err();
                assert!(
                    matches!(err, ChainGangError::ScriptError(ref msg) if msg == expected),
                    "signature={signature} multisig={multisig} verify={verify}: {err}"
                );
            }
        }
    }
}
