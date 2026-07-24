use crate::util::ChainGangError;
use std::io;
use std::io::{Read, Write};

/// An object that may be serialized and deserialized
pub trait Serializable<T> {
    /// Reads the object from serialized form
    fn read(reader: &mut dyn Read) -> Result<T, ChainGangError>
    where
        Self: Sized;

    /// Writes the object to serialized form
    fn write(&self, writer: &mut dyn Write) -> io::Result<()>;
}

/// Serde glue that (de)serializes a [`Serializable`] type using its raw wire-format bytes.
///
/// The produced representation is exactly the byte stream written by
/// [`Serializable::write`], so it round-trips through [`Serializable::read`] and matches
/// the on-the-wire encoding used elsewhere in the crate.
pub mod serde_bytes {
    use super::Serializable;
    use serde::de::{Error as DeError, SeqAccess, Visitor};
    use serde::ser::Error as SerError;
    use serde::{Deserializer, Serializer};
    use std::fmt;
    use std::io::Cursor;

    /// Serializes `value` as the raw bytes produced by [`Serializable::write`].
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serializable<T>,
        S: Serializer,
    {
        let mut bytes = Vec::new();
        value.write(&mut bytes).map_err(S::Error::custom)?;
        serializer.serialize_bytes(&bytes)
    }

    /// Deserializes a [`Serializable`] type from the raw bytes written by [`serialize`].
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: Serializable<T>,
        D: Deserializer<'de>,
    {
        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a byte array")
            }

            fn visit_bytes<E: DeError>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(v.to_vec())
            }

            fn visit_byte_buf<E: DeError>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(v)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(byte) = seq.next_element::<u8>()? {
                    bytes.push(byte);
                }
                Ok(bytes)
            }
        }

        let bytes = deserializer.deserialize_byte_buf(BytesVisitor)?;
        T::read(&mut Cursor::new(bytes)).map_err(D::Error::custom)
    }
}

impl Serializable<[u8; 16]> for [u8; 16] {
    fn read(reader: &mut dyn Read) -> Result<[u8; 16], ChainGangError> {
        let mut d = [0; 16];
        reader.read_exact(&mut d)?;
        Ok(d)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        writer.write_all(self)?;
        Ok(())
    }
}

impl Serializable<[u8; 32]> for [u8; 32] {
    fn read(reader: &mut dyn Read) -> Result<[u8; 32], ChainGangError> {
        let mut d = [0; 32];
        reader.read_exact(&mut d)?;
        Ok(d)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        writer.write_all(self)?;
        Ok(())
    }
}
