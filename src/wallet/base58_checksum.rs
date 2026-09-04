use crate::util::{sha256d, ChainGangError};

/// Number of trailing bytes that carry the Base58Check checksum
pub(crate) const CHECKSUM_LEN: usize = 4;

/// Returns the first 4 bytes of the double SHA-256 of `data` (Base58Check checksum).
pub fn short_double_sha256_checksum(data: &[u8]) -> Vec<u8> {
    sha256d(data).0[..CHECKSUM_LEN].to_vec()
}

/// Given the string return the checked base58 value
pub fn decode_base58_checksum(input: &str) -> Result<Vec<u8>, ChainGangError> {
    let decoded: Vec<u8> = bs58::decode(input)
        .into_vec()
        .map_err(|e| ChainGangError::Base58Error(format!("{e:?}")))?;
    // A value shorter than the checksum cannot be split into payload and
    // checksum at all, so reject it rather than subtracting past zero.
    let split = decoded.len().checked_sub(CHECKSUM_LEN).ok_or_else(|| {
        ChainGangError::BadData(format!(
            "Base58 value '{input}' decodes to {} bytes, too short to carry a {CHECKSUM_LEN}-byte checksum.",
            decoded.len()
        ))
    })?;
    let (shortened, decoded_checksum) = decoded.split_at(split);
    let hash_checksum: Vec<u8> = short_double_sha256_checksum(shortened);
    if hash_checksum != decoded_checksum {
        let err_msg = format!(
            "Decoded checksum {decoded_checksum:x?} derived from '{input}' is not equal to hash checksum {hash_checksum:x?}."
        );
        Err(ChainGangError::BadData(err_msg))
    } else {
        Ok(shortened.to_vec())
    }
}

/// Return base58 with checksum
/// Used to turn public key into an address
pub fn encode_base58_checksum(input: &[u8]) -> String {
    let hash = short_double_sha256_checksum(input);
    let mut data: Vec<u8> = input.to_vec();
    data.extend(hash);
    bs58::encode(data).into_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn short_sha256d_test() {
        let x = hex::decode("0123456789abcdef").unwrap();
        let e = hex::encode(short_double_sha256_checksum(&x));
        assert!(e == "137ad663");
    }

    #[test]
    fn round_trip() {
        let payload = hex::decode("00ea2407829a5055466b27784cde8cf463167946bf").unwrap();
        let encoded = encode_base58_checksum(&payload);
        assert_eq!(decode_base58_checksum(&encoded).unwrap(), payload);
    }

    #[test]
    fn decode_rejects_input_shorter_than_the_checksum() {
        // Valid base58 that decodes to fewer than four bytes used to underflow
        // when splitting off the checksum. "1" is a single zero byte.
        for input in ["", "1", "11", "111", "1111z"] {
            let decoded_len = bs58::decode(input).into_vec().map(|v| v.len());
            if let Ok(len) = decoded_len {
                if len >= CHECKSUM_LEN {
                    continue;
                }
            }
            assert!(
                decode_base58_checksum(input).is_err(),
                "expected an error for {input:?}"
            );
        }
    }

    #[test]
    fn decode_rejects_a_bad_checksum() {
        // Four bytes of payload plus four deliberately wrong checksum bytes
        let encoded = bs58::encode([1u8, 2, 3, 4, 0, 0, 0, 0]).into_string();
        assert!(decode_base58_checksum(&encoded).is_err());
    }
}
