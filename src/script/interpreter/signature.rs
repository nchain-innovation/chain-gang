use super::rules::tx_enforces_malleability_rules;
use crate::script::Checker;
use crate::util::ChainGangError;
use k256::elliptic_curve::Curve;
use k256::pkcs8::der::{asn1::UintRef, Decode, Reader, SliceReader};
use k256::{Secp256k1, U256};

/// Check LOW_S before delegating hash type, public key and ECDSA checks.
pub(super) fn check_sig<T: Checker>(
    checker: &mut T,
    sig: &[u8],
    key: &[u8],
    script: &[u8],
) -> Result<bool, ChainGangError> {
    if !sig.is_empty() && tx_enforces_malleability_rules(checker) {
        check_low_s(sig)?;
    }
    checker.check_sig(sig, key, script)
}

fn check_low_s(sig: &[u8]) -> Result<(), ChainGangError> {
    let der_error = || ChainGangError::ScriptError("Non-canonical DER signature".into());
    if !(9..=73).contains(&sig.len()) {
        return Err(der_error());
    }
    let mut reader = SliceReader::new(&sig[..sig.len() - 1]).map_err(|_| der_error())?;
    let (r, s) = reader
        .sequence(|r| Ok::<_, k256::pkcs8::der::Error>((UintRef::decode(r)?, UintRef::decode(r)?)))
        .map_err(|_| der_error())?;
    reader.finish().map_err(|_| der_error())?;
    // Node's lax scalar decoder zeroes both components on overflow. Zero is
    // representable, so R=0 with an otherwise valid high S still fails LOW_S.
    let scalar = |x: UintRef<'_>| {
        (x.as_bytes().len() <= 32).then(|| U256::from_be_slice_truncated(x.as_bytes(), 256))
    };
    if let (Some(r), Some(s)) = (scalar(r), scalar(s)) {
        let order = Secp256k1::ORDER.as_ref();
        if r < *order && s < *order && s > order.shr_vartime(1) {
            return Err(ChainGangError::ScriptError(
                "Non-canonical signature: S value is unnecessarily high".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_boundaries_match_node_check_low_s() {
        let order =
            hex::decode("00fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141")
                .unwrap();
        let half = hex::decode("7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a0")
            .unwrap();
        let mut above_half = half.clone();
        *above_half.last_mut().unwrap() += 1;
        for (r, s, high) in [
            (&[1][..], half.as_slice(), false),
            (&[0], &above_half, true),
            (order.as_slice(), &above_half, false),
            (&[1], &order, false),
            (&[1], &[0], false),
        ] {
            let mut sig = vec![0x30, (r.len() + s.len() + 4) as u8, 2, r.len() as u8];
            sig.extend_from_slice(r);
            sig.extend_from_slice(&[2, s.len() as u8]);
            sig.extend_from_slice(s);
            sig.push(0x41);
            match check_low_s(&sig) {
                Ok(()) => assert!(!high),
                Err(ChainGangError::ScriptError(msg)) if high => assert_eq!(
                    msg,
                    "Non-canonical signature: S value is unnecessarily high"
                ),
                other => panic!("r={r:x?} s={s:x?}: {other:?}"),
            }
        }
    }
}
