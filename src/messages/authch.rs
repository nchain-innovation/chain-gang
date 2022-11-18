use std::io;
use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::messages::message::Payload;
use crate::util::{Error, Result, Serializable};

/// The message version, should be 0x01
pub const SUPPORTED_VERSION: i32 = 0x01;

/// The authch message starts the authentication handshake
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct Authch {
    /// The message version, should be 0x01
    pub version: i32,
    /// The length of the payload
    pub message_length: u32,
    /// The payload (random text)
    pub message: Vec<u8>,
}

impl Authch {
    // Checks the authch message is valid
    pub fn validate(&self) -> Result<()> {
        if self.version != SUPPORTED_VERSION {
            let msg = format!("Unsupported version: {}", self.version);
            return Err(Error::BadData(msg));
        }
        Ok(())
    }
}

impl Serializable<Authch> for Authch {
    fn read(reader: &mut dyn Read) -> Result<Authch> {
        let version = reader.read_i32::<LittleEndian>()?;
        let message_length = reader.read_u32::<LittleEndian>()?;
        let message_size: usize = message_length.try_into().unwrap();

        let mut message_buf: Vec<u8> = vec![0; message_size];
        reader.read_exact(&mut message_buf)?;

        let ret = Authch {
            version,
            message_length,
            message: message_buf,
        };

        Ok(ret)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        writer.write_i32::<LittleEndian>(self.version)?;
        writer.write_u32::<LittleEndian>(self.message_length)?;
        writer.write_all(&self.message)?;
        Ok(())
    }
}

impl Payload<Authch> for Authch {
    fn size(&self) -> usize {
        4   // version: i32,
        + 4 // message_length: u32,
        + self.message.len()
    }
}
