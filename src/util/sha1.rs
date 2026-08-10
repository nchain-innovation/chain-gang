use sha1::{Digest, Sha1};

/// Computes the SHA-1 hash of `data`
pub fn sha1(data: &[u8]) -> Vec<u8> {
    Sha1::digest(data).to_vec()
}
