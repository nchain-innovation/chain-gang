use crate::messages::message::Payload;

use crate::util::{var_int, Error, Result, Serializable};
use byteorder::{ReadBytesExt, WriteBytesExt};
use std::io;
use std::io::{Read, Write};

/// [createstrm message format] <https://github.com/bitcoin-sv-specs/protocol/blob/master/p2p/multistreams.md>

pub const MIN_SUPPORTED_STREAM_TYPE: u8 = 1;
pub const MAX_SUPPORTED_STREAM_TYPE: u8 = 4;

/// Createstrm payload
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct Createstrm {
    /// ID to use to identify this new association
    pub association_id: Vec<u8>,
    /// Enumeration to identify the type of this new stream (1 - 4)
    pub stream_type: u8,
    /// (Optional) Name of the stream policy to use on this association
    pub stream_policy: String,
}

impl Createstrm {
    /// Checks if the message is valid
    pub fn validate(&self) -> Result<()> {
        // check stream_type is in range
        if self.stream_type < MIN_SUPPORTED_STREAM_TYPE
            || self.stream_type > MAX_SUPPORTED_STREAM_TYPE
        {
            let msg = format!("Unsupported stream type: {}", self.stream_type);
            return Err(Error::BadData(msg));
        }
        // check there is an association_id
        if self.association_id.is_empty() {
            let msg = "Association ID is empty".to_string();
            return Err(Error::BadData(msg));
        }
        Ok(())
    }
}

impl Serializable<Createstrm> for Createstrm {
    fn read(reader: &mut dyn Read) -> Result<Createstrm> {
        let mut ret = Createstrm {
            ..Default::default()
        };
        // Read association_id
        if let Ok(ass_len) = reader.read_u8() {
            if ass_len > 0 {
                ret.association_id = vec![0; ass_len.into()];
                reader.read_exact(&mut ret.association_id)?;
            }
        }
        // Read stream_type
        ret.stream_type = reader.read_u8()?;
        // Read stream_policy
        if let Ok(stream_policy_size) = var_int::read(reader) {
            let mut stream_policy_bytes = vec![0; stream_policy_size.try_into().unwrap()];
            reader.read_exact(&mut stream_policy_bytes)?;
            ret.stream_policy = String::from_utf8(stream_policy_bytes)?;
        }
        Ok(ret)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        // Write association_id
        writer.write_u8(self.association_id.len().try_into().unwrap())?;
        writer.write_all(&self.association_id)?;

        // Write stream_type
        writer.write_u8(self.stream_type)?;

        // Write stream_policy
        if !self.stream_policy.is_empty() {
            var_int::write(self.stream_policy.as_bytes().len() as u64, writer)?;
            writer.write_all(self.stream_policy.as_bytes())?;
        }
        Ok(())
    }
}

impl Payload<Createstrm> for Createstrm {
    fn size(&self) -> usize {
        // association_id
        1 + self.association_id.len() +
        // stream type
        1 +
        // stream_policy
        if self.stream_policy.is_empty() {
            0
        } else {
            var_int::size(self.stream_policy.as_bytes().len() as u64) +
            self.stream_policy.as_bytes().len()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Serializable;
    use std::io::Cursor;

    #[test]
    fn write_read() {
        let mut v = Vec::new();
        let m = Createstrm {
            association_id: vec![1, 2, 3, 4],
            stream_type: MIN_SUPPORTED_STREAM_TYPE,
            stream_policy: "policy".to_string(),
        };

        m.write(&mut v).unwrap();
        assert!(v.len() == m.size());
        assert!(Createstrm::read(&mut Cursor::new(&v)).unwrap() == m);
    }

    #[test]
    fn validate() {
        let m = Createstrm {
            association_id: vec![1, 2, 3, 4],
            stream_type: MIN_SUPPORTED_STREAM_TYPE,
            stream_policy: "policy".to_string(),
        };

        // Valid
        assert!(m.validate().is_ok());
        // Unsupported stream_type
        let m2 = Createstrm {
            stream_type: 0,
            ..m.clone()
        };
        assert!(m2.validate().is_err());
        // Bad association_id - should be ignored
        let m3 = Createstrm {
            association_id: vec![],
            ..m.clone()
        };
        assert!(m3.validate().is_err());
    }
}
