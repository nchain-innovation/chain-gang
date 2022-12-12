use crate::messages::{Payload, Tx, MAX_SATOSHIS};
use crate::util::{var_int, Error, Hash256, Result, Serializable};
use std::io;
use std::io::{Read, Write};

/// The blocktxn message is defined as a message containing a serialized BlockTransactions message and pchCommand == "blocktxn".
///
///
/// blocktxn payload - BlockTransactions
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub struct Blocktxn {
    /// Hash of the block
    pub blockhash: Hash256,
    // List of Transactions
    pub transactions: Vec<Tx>,
}

impl Blocktxn {
    /// Checks if the tx is valid - without recourse to UTXO etc.
    pub fn validate(&self) -> Result<()> {
        for tx in &self.transactions {
            // Make sure neither in or out lists are empty
            if tx.inputs.is_empty() {
                return Err(Error::BadData("inputs empty".to_string()));
            }
            if tx.outputs.is_empty() {
                return Err(Error::BadData("outputs empty".to_string()));
            }

            // Each output value, as well as the total, must be in legal money range
            let mut total_out = 0;
            for tx_out in tx.outputs.iter() {
                if tx_out.satoshis < 0 {
                    return Err(Error::BadData("tx_out satoshis negative".to_string()));
                }
                total_out += tx_out.satoshis;
            }
            if total_out > MAX_SATOSHIS {
                return Err(Error::BadData("Total out exceeds max satoshis".to_string()));
            }
        }
        Ok(())
    }
}

impl Serializable<Blocktxn> for Blocktxn {
    fn read(reader: &mut dyn Read) -> Result<Blocktxn> {
        let mut ret = Blocktxn {
            ..Default::default()
        };
        // Read blockhash
        ret.blockhash = Hash256::read(reader)?;

        // read transactions_length: var_int,
        if let Ok(transactions_length) = var_int::read(reader) {
            for _i in 0..transactions_length {
                let tx = Tx::read(reader)?;
                ret.transactions.push(tx);
            }
        }
        Ok(ret)
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        // Write blockhash
        Hash256::write(&self.blockhash, writer)?;

        // Write transactions_length: var_int
        var_int::write(self.transactions.len() as u64, writer)?;
        for tx in &self.transactions {
            Tx::write(tx, writer)?;
        }
        Ok(())
    }
}

impl Payload<Blocktxn> for Blocktxn {
    fn size(&self) -> usize {
        32 + // blockhash
        var_int::size(self.transactions.len() as u64) +
        self.transactions.iter().map(|x| x.size()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn write_read() {
        let mut v = Vec::new();
        let m = Blocktxn {
            ..Default::default()
        };

        m.write(&mut v).unwrap();
        assert!(v.len() == m.size());
        assert!(Blocktxn::read(&mut Cursor::new(&v)).unwrap() == m);
    }
}
