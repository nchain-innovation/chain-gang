use crate::{
    python::hashes::short_double_sha256_checksum,
    util::{Error, Result},
};
use base58::{FromBase58, ToBase58};

/// Given the string return the checked base58 value
pub fn decode_base58_checksum(input: &str) -> Result<Vec<u8>> {
    let decoded: Vec<u8> = input.from_base58()?;
    // Return all but the last 4
    let shortened: Vec<u8> = decoded.as_slice()[..decoded.len() - 4].to_vec();
    // Return last 4
    let decoded_checksum: Vec<u8> = decoded.as_slice()[decoded.len() - 4..].to_vec();
    let hash_checksum: Vec<u8> = short_double_sha256_checksum(&shortened);
    if hash_checksum != decoded_checksum {
        let err_msg = format!(
            "Decoded checksum {:x?} derived from '{}' is not equal to hash checksum {:x?}.",
            decoded_checksum, input, hash_checksum
        );
        Err(Error::BadData(err_msg))
    } else {
        Ok(shortened)
    }
}

/// Return base58 with checksum
/// Used to turn public key into an address
pub fn encode_base58_checksum(input: &[u8]) -> String {
    let hash = short_double_sha256_checksum(input);
    let mut data: Vec<u8> = input.to_vec();
    data.extend(hash);
    data.to_base58()
}
