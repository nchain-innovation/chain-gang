use std::io;
use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::messages::message::Payload;
use crate::util::{var_int, Error, Result, Serializable};

/// Supported number of fields (2).
pub const SUPPORTED_NUMBER_OF_FIELDS: u64 = 2;

/// The minimum value for this parameter is 1.048.576. Advertising a lower value is treated as a protocol violation.
pub const MIN_MAX_RECV_PAYLOAD_LENGTH: u32 = 1_048_576;

/// The message payload consists of the following fields:
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]

pub struct Protoconf {
    /// Contains number of fields following this field (2).
    pub number_of_fields: u64, // varint
    /// The value of this parameter specifies the maximum size of P2P message payload (without header)
    /// that the peer sending the protoconf message is willing to accept (receive) from another peer.
    pub max_recv_payload_length: u32,
    /// This parameter is a variable length string containing a comma separated list of stream policy names the sending peer knows about and is happy to use.
    pub stream_policies: String,
}

impl Protoconf {
    // Checks the protoconf message is valid
    pub fn validate(&self) -> Result<()> {
        if self.number_of_fields != SUPPORTED_NUMBER_OF_FIELDS {
            let msg = format!("Unsupported number_of_fields: {}", self.number_of_fields);
            return Err(Error::BadData(msg));
        }
        if self.max_recv_payload_length < MIN_MAX_RECV_PAYLOAD_LENGTH {
            let msg = format!(
                "Unsupported max_recv_payload_length: {}",
                self.max_recv_payload_length
            );
            return Err(Error::BadData(msg));
        }
        Ok(())
    }
}

impl Serializable<Protoconf> for Protoconf {
    fn read(reader: &mut dyn Read) -> Result<Protoconf> {
        let mut ret = Protoconf {
            ..Default::default()
        };
        ret.number_of_fields = var_int::read(reader)?;
        ret.max_recv_payload_length = reader.read_u32::<LittleEndian>()?;

        let stream_policy_size = var_int::read(reader)? as usize;
        let mut stream_policy_bytes = vec![0; stream_policy_size];
        reader.read_exact(&mut stream_policy_bytes)?;
        ret.stream_policies = String::from_utf8(stream_policy_bytes)?;
        Ok(ret)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        var_int::write(self.number_of_fields, writer)?;
        writer.write_u32::<LittleEndian>(self.max_recv_payload_length)?;
        var_int::write(self.stream_policies.as_bytes().len() as u64, writer)?;
        writer.write_all(self.stream_policies.as_bytes())?;
        Ok(())
    }
}

impl Payload<Protoconf> for Protoconf {
    fn size(&self) -> usize {
        var_int::size(self.number_of_fields)
            + 4 //+ self.max_recv_payload_length.size()
            + var_int::size(self.stream_policies.as_bytes().len() as u64)
            + self.stream_policies.as_bytes().len()
    }
}
