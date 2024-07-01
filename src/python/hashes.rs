use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

/*
// hash256 - two rounds of sha256
pub fn hash256(data: &[u8]) -> Vec<u8> {
    // Double hash of data
    let sha256 = Sha256::digest(data);
    let sha256d = Sha256::digest(sha256);
    sha256d.to_vec()
}
*/

/// Return first 4 digits of double sha256
pub fn short_double_sha256_checksum(data: &[u8]) -> Vec<u8> {
    // Double hash of data
    let sha256 = Sha256::digest(data);
    let sha256d = Sha256::digest(sha256);
    // Return first 4 digits
    sha256d.as_slice()[..4].to_vec()
}

pub fn hash160(data: &[u8]) -> Vec<u8> {
    let sha256 = Sha256::digest(data);
    Ripemd160::digest(sha256).to_vec()
}
