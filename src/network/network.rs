use crate::messages::{Block, BlockHeader, OutPoint, Tx, TxIn, TxOut};
use crate::network::SeedIter;
use crate::script::Script;
use std::fmt;
// use crate::util::{Error, Hash256, Result};
use crate::util::Hash256;
use hex;

#[allow(non_camel_case_types)]
/// Network type
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Network {
    // BSV
    BSV_Mainnet,
    BSV_Testnet,
    BSV_STN,
    // BTC
    BTC_Mainnet,
    BTC_Testnet,
    // BCH
    BCH_Mainnet,
    BCH_Testnet,
}

/// Display the network name
impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Network::BSV_Mainnet => "BSV_Mainnet",
            Network::BSV_Testnet => "BSV_Testnet",
            Network::BSV_STN => "BSV_STN",
            Network::BTC_Mainnet => "BTC_Mainnet",
            Network::BTC_Testnet => "BTC_Testnet",
            Network::BCH_Mainnet => "BCH_Mainnet",
            Network::BCH_Testnet => "BCH_Testnet",
        };
        write!(f, "{}", s)
    }
}

impl Network {
    /// Converts an integer to a network type
    /// `pub fn from_u8(x: u8) -> Result<Network> {`
    /// Deleted as considered too dangerous!
    /// As it was hardcoding u8 -> network mappings

    /// Returns the default TCP port
    pub fn port(&self) -> u16 {
        match self {
            Network::BSV_Mainnet => 8333,
            Network::BSV_Testnet => 18333,
            Network::BSV_STN => 9333,

            Network::BTC_Mainnet => 8333,
            Network::BTC_Testnet => 18333,
            // Network::BTC_SigNet => 38333,
            // Network::BTC_RegTest => 18444,
            Network::BCH_Mainnet => 8333,
            Network::BCH_Testnet => 18333,
            // Network::BCH_Testnet4 => 28333,
            // Network::BCH_ScaleNet => 38333,
            // Network::BCH_RegTest => 18444,
        }
    }

    /// Return a user_agent string for the network
    pub fn user_agent(&self) -> &str {
        match self {
            Network::BSV_Mainnet | Network::BSV_Testnet | Network::BSV_STN => "/Bitcoin SV:1.0.10/",
            Network::BTC_Mainnet | Network::BTC_Testnet => "/Satoshi:23.0.0/",
            Network::BCH_Mainnet | Network::BCH_Testnet => "/Bitcoin Cash Node:24.1.0(EB32.0)/",
        }
    }

    /// Returns the magic bytes for the message headers
    pub fn magic(&self) -> [u8; 4] {
        match self {
            Network::BSV_Mainnet | Network::BCH_Mainnet => [0xe3, 0xe1, 0xf3, 0xe8],
            Network::BSV_Testnet | Network::BCH_Testnet => [0xf4, 0xe5, 0xf3, 0xf4],
            Network::BSV_STN => [0xfb, 0xce, 0xc4, 0xf9],

            Network::BTC_Mainnet => [0xf9, 0xbe, 0xb4, 0xd9],
            Network::BTC_Testnet => [0x0b, 0x11, 0x09, 0x07],
        }
    }

    /// Returns the genesis block
    pub fn genesis_block(&self) -> Block {
        match self {
            Network::BSV_Mainnet | Network::BTC_Mainnet | Network::BCH_Mainnet => {
                let header = BlockHeader {
                    version: 1,
                    prev_hash: Hash256([0; 32]),
                    merkle_root: Hash256::decode(
                        "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
                    )
                    .unwrap(),
                    timestamp: 1231006505,
                    bits: 0x1d00ffff,
                    nonce: 2083236893,
                };

                let tx = Tx {
                    version: 1,
                    inputs: vec![TxIn {
                        prev_output: OutPoint {
                            hash: Hash256([0; 32]),
                            index: 0xffffffff,
                        },
                        unlock_script: Script(hex::decode("04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73").unwrap()),
                        sequence: 0xffffffff,
                    }],
                    outputs: vec![TxOut {
                        satoshis: 5000000000,
                        lock_script: Script(hex::decode("4104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac").unwrap()),
                    }],
                    lock_time: 0,
                };

                Block {
                    header,
                    txns: vec![tx],
                }
            }
            Network::BSV_Testnet
            | Network::BSV_STN
            | Network::BTC_Testnet
            | Network::BCH_Testnet => {
                let header = BlockHeader {
                    version: 1,
                    prev_hash: Hash256([0; 32]),
                    merkle_root: Hash256::decode(
                        "4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b",
                    )
                    .unwrap(),
                    timestamp: 1296688602,
                    bits: 0x1d00ffff,
                    nonce: 414098458,
                };

                let tx = Tx {
                    version: 1,
                    inputs: vec![TxIn {
                        prev_output: OutPoint {
                            hash: Hash256([0; 32]),
                            index: 0xffffffff,
                        },
                        unlock_script: Script(hex::decode("04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73").unwrap()),
                        sequence: 0xffffffff,
                    }],
                    outputs: vec![TxOut {
                        satoshis: 5000000000,
                        lock_script: Script(hex::decode("4104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac").unwrap()),
                    }],
                    lock_time: 0,
                };

                Block {
                    header,
                    txns: vec![tx],
                }
            }
        }
    }

    /// Returns the genesis block hash
    pub fn genesis_hash(&self) -> Hash256 {
        match self {
            Network::BSV_Mainnet | Network::BTC_Mainnet | Network::BCH_Mainnet => {
                Hash256::decode("000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f")
                    .unwrap()
            }
            Network::BSV_Testnet
            | Network::BSV_STN
            | Network::BTC_Testnet
            | Network::BCH_Testnet => {
                Hash256::decode("000000000933ea01ad0ee984209779baaec3ced90fa3f408719526f8d77f4943")
                    .unwrap()
            }
        }
    }

    /// Returns the version byte flag for P2PKH-type addresses
    pub fn addr_pubkeyhash_flag(&self) -> u8 {
        match self {
            Network::BSV_Mainnet | Network::BTC_Mainnet | Network::BCH_Mainnet => 0x00,
            Network::BSV_Testnet | Network::BTC_Testnet | Network::BCH_Testnet => 0x6f,
            Network::BSV_STN => 0x6f,
        }
    }

    /// Returns the version byte flag for P2SH-type addresses
    pub fn addr_script_flag(&self) -> u8 {
        match self {
            Network::BSV_Mainnet | Network::BTC_Mainnet | Network::BCH_Mainnet => 0x05,
            Network::BSV_Testnet | Network::BTC_Testnet | Network::BCH_Testnet => 0xc4,
            Network::BSV_STN => 0xc4,
        }
    }

    /// Returns a list of DNS seeds for finding initial nodes
    pub fn seeds(&self) -> Vec<String> {
        match self {
            Network::BSV_Mainnet => vec![
                "seed.bitcoinsv.io".to_string(),
                "seed.cascharia.com".to_string(),
                "seed.satoshisvision.network".to_string(),
            ],
            Network::BSV_Testnet => vec![
                "testnet-seed.bitcoinsv.io".to_string(),
                "testnet-seed.cascharia.com".to_string(),
                "testnet-seed.bitcoincloud.net".to_string(),
            ],
            Network::BSV_STN => vec!["stn-seed.bitcoinsv.io".to_string()],

            Network::BTC_Mainnet => vec![
                "seed.bitcoin.sipa.be".to_string(),
                "dnsseed.bluematt.me".to_string(),
                "dnsseed.bitcoin.dashjr.org".to_string(),
                "seed.bitcoinstats.com".to_string(),
                "seed.bitcoin.jonasschnelli.ch".to_string(),
                "seed.btc.petertodd.org".to_string(),
                "seed.bitcoin.sprovoost.nl".to_string(),
                "dnsseed.emzy.de".to_string(),
                "seed.bitcoin.wiz.biz".to_string(),
            ],
            Network::BTC_Testnet => vec![
                "testnet-seed.bitcoin.jonasschnelli.ch".to_string(),
                "seed.tbtc.petertodd.org".to_string(),
                "seed.testnet.bitcoin.sprovoost.nl".to_string(),
                "testnet-seed.bluematt.me".to_string(),
            ],

            Network::BCH_Mainnet => vec![
                "seed.flowee.cash".to_string(),
                "seed-bch.bitcoinforks.org".to_string(),
                "btccash-seeder.bitcoinunlimited.info".to_string(),
                "seed.bchd.cash".to_string(),
                "seed.bch.loping.net".to_string(),
                "dnsseed.electroncash.de".to_string(),
                "bchseed.c3-soft.com".to_string(),
                "bch.bitjson.com".to_string(),
            ],
            Network::BCH_Testnet => vec![
                "testnet-seed.bchd.cash".to_string(),
                "seed.tbch.loping.net".to_string(),
                "testnet-seed.bitcoinunlimited.info".to_string(),
            ],
        }
    }

    /// Creates a new DNS seed iterator for this network
    pub fn seed_iter(&self) -> SeedIter {
        SeedIter::new(&self.seeds(), self.port())
    }
}
