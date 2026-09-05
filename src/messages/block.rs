use crate::messages::{BlockHeader, OutPoint, Payload, Tx, TxOut};
use crate::network::Network;
use crate::util::{
    sha256d, var_int, ChainGangError, Hash256, Serializable, BITCOIN_CASH_FORK_HEIGHT_MAINNET,
    BITCOIN_CASH_FORK_HEIGHT_TESTNET, GENESIS_UPGRADE_HEIGHT_MAINNET,
    GENESIS_UPGRADE_HEIGHT_TESTNET,
};
use linked_hash_map::LinkedHashMap;
use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::io;
use std::io::{Read, Write};

/// Block of transactions
#[derive(Default, PartialEq, Eq, Hash, Clone)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// Block transactions
    pub txns: Vec<Tx>,
}

impl Block {
    /// Returns a set of the inputs spent in this block
    pub fn inputs(&self) -> Result<HashSet<OutPoint>, ChainGangError> {
        let mut inputs = HashSet::new();
        for txn in self.txns.iter() {
            if !txn.coinbase() {
                for input in txn.inputs.iter() {
                    if inputs.contains(&input.prev_output) {
                        let msg = "Input double spent".to_string();
                        return Err(ChainGangError::BadData(msg));
                    }
                    inputs.insert(input.prev_output.clone());
                }
            }
        }
        Ok(inputs)
    }

    /// Returns a map of the new outputs generated from this block including those spent within the block
    pub fn outputs(&self) -> Result<LinkedHashMap<OutPoint, TxOut>, ChainGangError> {
        let mut outputs = LinkedHashMap::new();
        for txn in self.txns.iter() {
            let hash = txn.hash();
            for index in 0..txn.outputs.len() as u32 {
                outputs.insert(
                    OutPoint { hash, index },
                    txn.outputs[index as usize].clone(),
                );
            }
        }
        Ok(outputs)
    }

    /// Checks that the block is valid
    pub fn validate(
        &self,
        height: i32,
        network: Network,
        utxos: &LinkedHashMap<OutPoint, TxOut>,
        pregenesis_outputs: &HashSet<OutPoint>,
    ) -> Result<(), ChainGangError> {
        if self.txns.is_empty() {
            return Err(ChainGangError::BadData("Txn count is zero".to_string()));
        }

        if self.merkle_root() != self.header.merkle_root {
            return Err(ChainGangError::BadData("Bad merkle root".to_string()));
        }

        let mut has_coinbase = false;
        let require_sighash_forkid = match network {
            Network::BSV_Mainnet | Network::BCH_Mainnet => {
                height >= BITCOIN_CASH_FORK_HEIGHT_MAINNET
            }
            Network::BSV_Testnet | Network::BCH_Testnet => {
                height >= BITCOIN_CASH_FORK_HEIGHT_TESTNET
            }
            // A private chain runs the fork rules from its first block
            Network::BSV_STN | Network::BSV_Regtest => true,
            Network::BTC_Mainnet | Network::BTC_Testnet => false,
        };
        let use_genesis_rules = match network {
            Network::BSV_Mainnet => height >= GENESIS_UPGRADE_HEIGHT_MAINNET,
            Network::BSV_Testnet => height >= GENESIS_UPGRADE_HEIGHT_TESTNET,
            // As for STN, assume Genesis rules from the first block. Note that
            // bitcoin-sv makes the regtest activation height configurable, so a
            // node started with a non-default height will disagree below it.
            Network::BSV_STN | Network::BSV_Regtest => true,
            Network::BTC_Mainnet
            | Network::BTC_Testnet
            | Network::BCH_Mainnet
            | Network::BCH_Testnet => false,
        };
        // Chronicle is gated on the same height this function already uses for
        // the fork and Genesis rules. Without it, block validation applied
        // Chronicle rules to any version > 1 transaction whatever the height.
        // A negative height has no meaning on a chain, so treat it as 0, which
        // is pre-activation everywhere except regtest.
        let block_height = u64::try_from(height).unwrap_or(0);
        for txn in self.txns.iter() {
            if !txn.coinbase() {
                txn.validate_at_height(
                    require_sighash_forkid,
                    use_genesis_rules,
                    utxos,
                    pregenesis_outputs,
                    block_height,
                    network,
                )?;
            } else if has_coinbase {
                return Err(ChainGangError::BadData("Multiple coinbases".to_string()));
            } else {
                has_coinbase = true;
            }
        }
        if !has_coinbase {
            return Err(ChainGangError::BadData("No coinbase".to_string()));
        }

        Ok(())
    }

    /// Calculates the merkle root from the transactions
    fn merkle_root(&self) -> Hash256 {
        let mut row = VecDeque::new();
        for tx in self.txns.iter() {
            row.push_back(tx.hash());
        }
        while row.len() > 1 {
            let mut n = row.len();
            while n > 0 {
                n -= 1;
                let h1 = row.pop_front().unwrap();
                let h2 = if n == 0 {
                    h1
                } else {
                    n -= 1;
                    row.pop_front().unwrap()
                };
                let mut h = Vec::with_capacity(64);
                h1.write(&mut h).unwrap();
                h2.write(&mut h).unwrap();
                row.push_back(sha256d(&h));
            }
        }
        row.pop_front().unwrap()
    }
}

impl Serializable<Block> for Block {
    fn read(reader: &mut dyn Read) -> Result<Block, ChainGangError> {
        let header = BlockHeader::read(reader)?;
        let txn_count = var_int::read(reader)?;
        let mut txns = Vec::with_capacity(txn_count as usize);
        for _i in 0..txn_count {
            txns.push(Tx::read(reader)?);
        }
        Ok(Block { header, txns })
    }

    fn write(&self, writer: &mut dyn Write) -> io::Result<()> {
        self.header.write(writer)?;
        var_int::write(self.txns.len() as u64, writer)?;
        for txn in self.txns.iter() {
            txn.write(writer)?;
        }
        Ok(())
    }
}

impl serde::Serialize for Block {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::util::serde_bytes::serialize(self, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Block {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        crate::util::serde_bytes::deserialize(deserializer)
    }
}

impl Payload<Block> for Block {
    fn size(&self) -> usize {
        let mut size = BlockHeader::SIZE;
        size += var_int::size(self.txns.len() as u64);
        for txn in self.txns.iter() {
            size += txn.size();
        }
        size
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.txns.len() <= 3 {
            f.debug_struct("Block")
                .field("header", &self.header)
                .field("txns", &self.txns)
                .finish()
        } else {
            let txns = format!("[<{} transactions>]", self.txns.len());
            f.debug_struct("Block")
                .field("header", &self.header)
                .field("txns", &txns)
                .finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chronicle::CHRONICLE_ACTIVATION_MAINNET;
    use crate::messages::{COINBASE_OUTPOINT_HASH, COINBASE_OUTPOINT_INDEX};
    use crate::script::op_codes::{OP_2, OP_3, OP_5, OP_ADD, OP_EQUAL};

    /// A coinbase, so the block passes the has-a-coinbase check
    fn coinbase() -> Tx {
        Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: COINBASE_OUTPOINT_HASH,
                    index: COINBASE_OUTPOINT_INDEX,
                },
                unlock_script: Script(vec![OP_2, OP_3]),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 5_000_000_000,
                lock_script: Script(vec![]),
            }],
            lock_time: 0,
        }
    }

    /// A block containing `spend`, with the merkle root its contents imply
    fn block_containing(spend: Tx) -> Block {
        let mut block = Block {
            header: BlockHeader {
                version: 1,
                prev_hash: Hash256([0; 32]),
                merkle_root: Hash256([0; 32]),
                timestamp: 0,
                bits: 0x207fffff,
                nonce: 0,
            },
            txns: vec![coinbase(), spend],
        };
        block.header.merkle_root = block.merkle_root();
        block
    }

    #[test]
    fn validate_gates_chronicle_on_the_block_height() {
        // The same version 2 spend the Tx-level test uses: its unlock script is
        // not push-only, which Chronicle permits and the earlier rules do not.
        let funding = Tx {
            version: 1,
            inputs: vec![],
            outputs: vec![TxOut {
                satoshis: 1_000,
                lock_script: Script(vec![OP_5, OP_EQUAL]),
            }],
            lock_time: 0,
        };
        let mut utxos = LinkedHashMap::new();
        utxos.insert(
            OutPoint {
                hash: funding.hash(),
                index: 0,
            },
            funding.outputs[0].clone(),
        );

        let spend = Tx {
            version: 2,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: funding.hash(),
                    index: 0,
                },
                unlock_script: Script(vec![OP_2, OP_3, OP_ADD]),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 900,
                lock_script: Script(vec![]),
            }],
            lock_time: 0,
        };
        let block = block_containing(spend);

        assert!(
            block
                .validate(
                    CHRONICLE_ACTIVATION_MAINNET as i32,
                    Network::BSV_Mainnet,
                    &utxos,
                    &HashSet::new()
                )
                .is_ok(),
            "Chronicle rules should apply at the activation height"
        );
        assert!(
            block
                .validate(
                    CHRONICLE_ACTIVATION_MAINNET as i32 - 1,
                    Network::BSV_Mainnet,
                    &utxos,
                    &HashSet::new()
                )
                .is_err(),
            "the block below activation used to pass, because the height never \
             reached the Chronicle gate"
        );
    }
    use crate::messages::{OutPoint, TxIn, TxOut};
    use crate::script::Script;
    use crate::util::Hash256;
    use hex;
    use std::io::Cursor;

    #[test]
    fn read_bytes() {
        let b = hex::decode("010000004860eb18bf1b1620e37e9490fc8a427514416fd75159ab86688e9a8300000000d5fdcc541e25de1c7a5addedf24858b8bb665c9f36ef744ee42c316022c90f9bb0bc6649ffff001d08d2bd610101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0704ffff001d010bffffffff0100f2052a010000004341047211a824f55b505228e4c3d5194c1fcfaa15a456abdf37f9b9d97a4040afc073dee6c89064984f03385237d92167c13e236446b417ab79a0fcae412ae3316b77ac00000000").unwrap();
        let block = Block::read(&mut Cursor::new(&b)).unwrap();
        assert!(
            block
                == Block {
                    header: BlockHeader {
                        version: 1,
                        prev_hash: Hash256::decode(
                            "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048",
                        )
                        .unwrap(),
                        merkle_root: Hash256::decode(
                            "9b0fc92260312ce44e74ef369f5c66bbb85848f2eddd5a7a1cde251e54ccfdd5",
                        )
                        .unwrap(),
                        timestamp: 1231469744,
                        bits: 486604799,
                        nonce: 1639830024,
                    },
                    txns: vec![Tx {
                        version: 1,
                        inputs: vec![TxIn {
                            prev_output: OutPoint {
                                hash: Hash256([0; 32]),
                                index: 4294967295,
                            },
                            unlock_script: Script(vec![4, 255, 255, 0, 29, 1, 11]),
                            sequence: 4294967295,
                        }],
                        outputs: vec![TxOut {
                            satoshis: 5000000000,
                            lock_script: Script(vec![
                                65, 4, 114, 17, 168, 36, 245, 91, 80, 82, 40, 228, 195, 213, 25,
                                76, 31, 207, 170, 21, 164, 86, 171, 223, 55, 249, 185, 217, 122,
                                64, 64, 175, 192, 115, 222, 230, 200, 144, 100, 152, 79, 3, 56, 82,
                                55, 217, 33, 103, 193, 62, 35, 100, 70, 180, 23, 171, 121, 160,
                                252, 174, 65, 42, 227, 49, 107, 119, 172,
                            ]),
                        }],
                        lock_time: 0,
                    }],
                }
        );
    }

    #[test]
    fn write_read() {
        let mut v = Vec::new();
        let block = Block {
            header: BlockHeader {
                version: 77,
                prev_hash: Hash256::decode(
                    "abcdabcdabcdabcd1234123412341234abcdabcdabcdabcd1234123412341234",
                )
                .unwrap(),
                merkle_root: Hash256::decode(
                    "1234567809876543123456780987654312345678098765431234567809876543",
                )
                .unwrap(),
                timestamp: 7,
                bits: 8,
                nonce: 9,
            },
            txns: vec![Tx {
                version: 7,
                inputs: vec![TxIn {
                    prev_output: OutPoint {
                        hash: Hash256([7; 32]),
                        index: 3,
                    },
                    unlock_script: Script(vec![9, 8, 7]),
                    sequence: 42,
                }],
                outputs: vec![TxOut {
                    satoshis: 23,
                    lock_script: Script(vec![1, 2, 3, 4, 5]),
                }],
                lock_time: 4,
            }],
        };
        block.write(&mut v).unwrap();
        assert!(v.len() == block.size());
        assert!(Block::read(&mut Cursor::new(&v)).unwrap() == block);
    }

    #[test]
    fn serde_bytes_match_serializable() {
        let wire = hex::decode("010000004860eb18bf1b1620e37e9490fc8a427514416fd75159ab86688e9a8300000000d5fdcc541e25de1c7a5addedf24858b8bb665c9f36ef744ee42c316022c90f9bb0bc6649ffff001d08d2bd610101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0704ffff001d010bffffffff0100f2052a010000004341047211a824f55b505228e4c3d5194c1fcfaa15a456abdf37f9b9d97a4040afc073dee6c89064984f03385237d92167c13e236446b417ab79a0fcae412ae3316b77ac00000000").unwrap();
        let block = Block::read(&mut Cursor::new(&wire)).unwrap();

        // The serde representation is exactly the raw wire-format bytes.
        let json = serde_json::to_string(&block).unwrap();
        let encoded: Vec<u8> = serde_json::from_str(&json).unwrap();
        assert_eq!(encoded, wire);

        // ... and it round-trips back to an identical Block.
        let decoded: Block = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, block);
    }
}
