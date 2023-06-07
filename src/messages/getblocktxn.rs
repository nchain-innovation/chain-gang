use crate::messages::Payload;
use std::io;
use std::io::{Read, Write};

use crate::util::{var_int, Hash256, Result, Serializable};

/// getblocktxn defined in <https://github.com/bitcoin/bips/blob/master/bip-0152.mediawiki>
/// The getblocktxn message is defined as a message containing a serialized BlockTransactionsRequest message and pchCommand == "getblocktxn".

/// getblocktxn payload - BlockTransactionsRequest
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct Getblocktxn {
    /// Hash of the block
    pub blockhash: Hash256,
    //  The indexes of the transactions being requested in the block
    pub indexes: Vec<u64>,
}

impl Serializable<Getblocktxn> for Getblocktxn {
    fn read(reader: &mut dyn Read) -> Result<Getblocktxn> {
        let mut ret = Getblocktxn {
            ..Default::default()
        };
        // Read blockhash
        ret.blockhash = Hash256::read(reader)?;

        // read indexes_length: var_int,
        if let Ok(indexes_length) = var_int::read(reader) {
            for _i in 0..indexes_length {
                let index = var_int::read(reader)?;
                ret.indexes.push(index);
            }
        }
        Ok(ret)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        // Write blockheader
        Hash256::write(&self.blockhash, writer)?;

        // Write shortids_length: var_int
        var_int::write(self.indexes.len() as u64, writer)?;
        for index in &self.indexes {
            var_int::write(*index, writer)?;
        }
        Ok(())
    }
}

impl Payload<Getblocktxn> for Getblocktxn {
    fn size(&self) -> usize {
        32 + // hash256 => 32 bytes
        var_int::size(self.indexes.len() as u64) +
        self.indexes.iter().map(|x| var_int::size(*x)).sum::<usize>()
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
        let m = Getblocktxn {
            ..Default::default()
        };

        m.write(&mut v).unwrap();
        assert!(v.len() == m.size());
        assert!(Getblocktxn::read(&mut Cursor::new(&v)).unwrap() == m);
    }
}
