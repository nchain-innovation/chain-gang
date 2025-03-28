use crate::util::{ChainGangError, Serializable};
use hex;
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    fmt, io,
    io::{Read, Write},
};

use db_key::Key;

/// 256-bit hash for blocks and transactions
///
/// It is interpreted as a single little-endian number for display.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    /// Converts the hash into a hex string
    pub fn encode(&self) -> String {
        let mut r = self.0;
        r.reverse();
        hex::encode(r)
    }

    /// Converts a string of 64 hex characters into a hash
    pub fn decode(s: &str) -> Result<Hash256, ChainGangError> {
        let decoded_bytes = hex::decode(s)?;
        let mut hash_bytes = [0; 32];
        if decoded_bytes.len() != 32 {
            let msg = format!("Length {} of {:?}", decoded_bytes.len(), decoded_bytes);
            return Err(ChainGangError::BadArgument(msg));
        }
        hash_bytes.clone_from_slice(&decoded_bytes);
        hash_bytes.reverse();
        Ok(Hash256(hash_bytes))
    }
}

impl Serializable<Hash256> for Hash256 {
    fn read(reader: &mut dyn Read) -> Result<Hash256, ChainGangError> {
        let mut bytes = [0; 32];
        reader.read_exact(&mut bytes)?;
        Ok(Hash256(bytes))
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        match writer.write(&self.0) {
            Ok(_size) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// Hashes a data array twice using SHA256
pub fn sha256d(data: &[u8]) -> Hash256 {
    // Double hash of payload
    let sha256 = Sha256::digest(data);
    let sha256d = Sha256::digest(sha256);

    let mut hash256 = [0; 32];
    hash256.clone_from_slice(sha256d.as_ref());
    Hash256(hash256)
}

impl Ord for Hash256 {
    fn cmp(&self, other: &Hash256) -> Ordering {
        for i in (0..32).rev() {
            match self.0[i].cmp(&other.0[i]) {
                Ordering::Greater => return Ordering::Greater,
                Ordering::Less => return Ordering::Less,
                Ordering::Equal => {}
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for Hash256 {
    fn partial_cmp(&self, other: &Hash256) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

// For use in LevelDB database
impl Key for Hash256 {
    fn from_u8(key: &[u8]) -> Hash256 {
        assert!(key.len() == 32);
        let mut bytes = [0; 32];
        bytes.clone_from_slice(key);
        Hash256(bytes)
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        f(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;
    use std::io::Cursor;

    #[test]
    fn sha256d_test() {
        let x = hex::decode("0123456789abcdef").unwrap();
        let e = hex::encode(sha256d(&x).0);
        assert!(e == "137ad663f79da06e282ed0abbec4d70523ced5ff8e39d5c2e5641d978c5925aa");
    }

    #[test]
    fn hash_decode() {
        // Valid
        let s1 = "0000000000000000000000000000000000000000000000000000000000000000";
        let s2 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let s3 = "abcdef0000112233445566778899abcdef000011223344556677889912345678";
        assert!(Hash256::decode(s1).is_ok());
        assert!(Hash256::decode(s2).is_ok());
        assert!(Hash256::decode(s3).is_ok());

        // Invalid
        let s1 = "000000000000000000000000000000000000000000000000000000000000000";
        let s2 = "00000000000000000000000000000000000000000000000000000000000000000";
        let s3 = "000000000000000000000000000000000000000000000000000000000000000g";
        assert!(Hash256::decode(s1).is_err());
        assert!(Hash256::decode(s2).is_err());
        assert!(Hash256::decode(s3).is_err());
    }

    #[test]
    fn hash_decode_write_read_encode() {
        let s1 = "abcdef0000112233445566778899abcdef000011223344556677889912345678";
        let h1 = Hash256::decode(s1).unwrap();
        let mut v = Vec::new();
        h1.write(&mut v).unwrap();
        let h2 = Hash256::read(&mut Cursor::new(v)).unwrap();
        let s2 = h2.encode();
        assert!(s1 == s2);
    }

    #[test]
    fn hash_compare() {
        let s1 = "5555555555555555555555555555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash256::decode(s1).unwrap() == Hash256::decode(s2).unwrap());

        let s1 = "0555555555555555555555555555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash256::decode(s1).unwrap() < Hash256::decode(s2).unwrap());

        let s1 = "5555555555555555555555555555555555555555555555555555555555555550";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash256::decode(s1).unwrap() < Hash256::decode(s2).unwrap());

        let s1 = "6555555555555555555555555555555555555555555555555555555555555555";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash256::decode(s1).unwrap() > Hash256::decode(s2).unwrap());

        let s1 = "5555555555555555555555555555555555555555555555555555555555555556";
        let s2 = "5555555555555555555555555555555555555555555555555555555555555555";
        assert!(Hash256::decode(s1).unwrap() > Hash256::decode(s2).unwrap());
    }
}
