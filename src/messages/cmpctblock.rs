use std::io;
use std::io::{Read, Write};

use crate::messages::{BlockHeader, Payload, Tx, MAX_SATOSHIS};
use crate::util::{var_int, Error, Result, Serializable};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

type ShortTXID = Vec<u8>;

pub const SHORT_TX_ID_LEN: usize = 6;

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct PrefilledTransaction {
    ///  The index into the block at which this transaction is
    pub index: u64,
    ///  The transaction which is in the block at index index.
    pub tx: Tx,
}

impl PrefilledTransaction {
    /// Checks if the tx is valid - without recourse to UTXO etc.
    pub fn validate(&self) -> Result<()> {
        // Make sure neither in or out lists are empty
        if self.tx.inputs.is_empty() {
            return Err(Error::BadData("inputs empty".to_string()));
        }
        if self.tx.outputs.is_empty() {
            return Err(Error::BadData("outputs empty".to_string()));
        }

        // Each output value, as well as the total, must be in legal money range
        let mut total_out = 0;
        for tx_out in self.tx.outputs.iter() {
            if tx_out.satoshis < 0 {
                return Err(Error::BadData("tx_out satoshis negative".to_string()));
            }
            total_out += tx_out.satoshis;
        }
        if total_out > MAX_SATOSHIS {
            return Err(Error::BadData("Total out exceeds max satoshis".to_string()));
        }
        Ok(())
    }
}

impl Serializable<PrefilledTransaction> for PrefilledTransaction {
    fn read(reader: &mut dyn Read) -> Result<PrefilledTransaction> {
        let mut ret = PrefilledTransaction {
            ..Default::default()
        };
        // Read index
        ret.index = var_int::read(reader)?;
        // Read tx
        ret.tx = Tx::read(reader)?;
        Ok(ret)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        // Write index
        var_int::write(self.index, writer)?;
        // Write tx
        Tx::write(&self.tx, writer)?;
        Ok(())
    }
}

impl Payload<PrefilledTransaction> for PrefilledTransaction {
    fn size(&self) -> usize {
        var_int::size(self.index) + self.tx.size()
    }
}

/// cmpctblock defined in <https://github.com/bitcoin/bips/blob/master/bip-0152.mediawiki>
/// The cmpctblock message is defined as a message containing a serialized HeaderAndShortIDs message and pchCommand == "cmpctblock".
/// Cmpctblock payload - HeaderAndShortIDs
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct Cmpctblock {
    /// First 80 bytes of the block as defined by the encoding used by "block" messages
    pub header: BlockHeader,
    // nonce for use in short transaction ID calculations
    pub nonce: u64,

    /// The short transaction IDs calculated from the transactions which were not provided explicitly in prefilledtxn
    pub shortids: Vec<ShortTXID>,

    /// List of PrefilledTransactions
    pub prefilledtxn: Vec<PrefilledTransaction>,
}

impl Cmpctblock {
    /// Checks if the message is valid
    pub fn validate(&self) -> Result<()> {
        // Check header is valid - needs blockhash and previous headers
        // Check transactions are valid
        for pre_tx in &self.prefilledtxn {
            pre_tx.validate()?;
        }
        Ok(())
    }
}

impl Serializable<Cmpctblock> for Cmpctblock {
    fn read(reader: &mut dyn Read) -> Result<Cmpctblock> {
        let mut ret = Cmpctblock {
            ..Default::default()
        };
        // Read blockheader
        ret.header = BlockHeader::read(reader)?;
        // Read nonce
        ret.nonce = reader.read_u64::<LittleEndian>()?;

        // read shortids_length: var_int,
        if let Ok(shortids_length) = var_int::read(reader) {
            for _i in 0..shortids_length {
                let mut shortid = vec![0u8; SHORT_TX_ID_LEN];
                reader.read_exact(&mut shortid)?;
                ret.shortids.push(shortid);
            }
        }
        // Read prefilledtxn_length: var_int
        if let Ok(prefilledtxn_length) = var_int::read(reader) {
            for _i in 0..prefilledtxn_length {
                let prefilledtx: PrefilledTransaction = PrefilledTransaction::read(reader)?;
                ret.prefilledtxn.push(prefilledtx);
            }
        }
        Ok(ret)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        // Write blockheader
        BlockHeader::write(&self.header, writer)?;
        // Write nonce
        writer.write_u64::<LittleEndian>(self.nonce)?;

        // Write shortids_length: var_int
        var_int::write(self.shortids.len() as u64, writer)?;
        for shortid in &self.shortids {
            writer.write_all(shortid)?;
        }

        // Write prefilledtxn_length: var_int
        var_int::write(self.prefilledtxn.len() as u64, writer)?;
        for prefilled in &self.prefilledtxn {
            prefilled.write(writer)?;
        }
        Ok(())
    }
}

impl Payload<Cmpctblock> for Cmpctblock {
    fn size(&self) -> usize {
        self.header.size() + 8 + // Nonce
        var_int::size(self.shortids.len() as u64) + (self.shortids.len() * 6) +
        var_int::size(self.prefilledtxn.len() as u64) +
        self.prefilledtxn.iter().map(|x| x.size()).sum::<usize>()
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
        let m = Cmpctblock {
            ..Default::default()
        };

        m.write(&mut v).unwrap();
        assert!(v.len() == m.size());
        assert!(Cmpctblock::read(&mut Cursor::new(&v)).unwrap() == m);
    }
}
