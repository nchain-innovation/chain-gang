use k256::ecdsa::{SigningKey, VerifyingKey};

use crate::{
    messages::Tx,
    network::Network,
    script::{
        op_codes::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160},
        Script,
    },
    transaction::{
        generate_signature,
        p2pkh::create_unlock_script,
        sighash::{sighash, SigHashCache},
    },
    util::{hash160, Error, Hash256, Result},
    wallet::base58_checksum::{decode_base58_checksum, encode_base58_checksum},
};

pub const MAIN_PRIVATE_KEY: u8 = 0x80;
pub const TEST_PRIVATE_KEY: u8 = 0xef;

const MAIN_PUBKEY_HASH: u8 = 0x00;
const TEST_PUBKEY_HASH: u8 = 0x6f;

pub fn wif_to_network_and_private_key(wif: &str) -> Result<(Network, SigningKey)> {
    let decode = decode_base58_checksum(wif)?;
    // Get first byte
    let prefix: u8 = *decode.first().ok_or("Invalid wif length")?;
    let network: Network = match prefix {
        MAIN_PRIVATE_KEY => Network::BSV_Mainnet,
        TEST_PRIVATE_KEY => Network::BSV_Testnet,
        _ => {
            let err_msg = format!(
                "{:02x?} does not correspond to a mainnet nor testnet address.",
                prefix
            );
            return Err(Error::BadData(err_msg));
        }
    };
    // Remove prefix byte and, if present, compression flag.
    let last_byte: u8 = *decode.last().ok_or("Invalid wif length")?;
    let compressed: bool = wif.len() == 52 && last_byte == 1u8;
    let private_key_as_bytes: Vec<u8> = if compressed {
        decode[1..decode.len() - 1].to_vec()
    } else {
        decode[1..].to_vec()
    };
    let private_key = SigningKey::from_slice(&private_key_as_bytes)?;
    Ok((network, private_key))
}

// Given public_key and network return address as a string
pub fn public_key_to_address(public_key: &[u8], network: Network) -> Result<String> {
    let prefix_as_bytes: u8 = match network {
        Network::BSV_Mainnet => MAIN_PUBKEY_HASH,
        Network::BSV_Testnet => TEST_PUBKEY_HASH,
        _ => {
            let err_msg = format!("{} unknnown network.", &network);
            return Err(Error::BadData(err_msg));
        }
    };
    // # 33 bytes compressed, 65 uncompressed.
    if public_key.len() != 33 && public_key.len() != 65 {
        let err_msg = format!(
            "{} is an invalid length for a public key.",
            public_key.len()
        );
        return Err(Error::BadData(err_msg));
    }
    let mut data: Vec<u8> = vec![prefix_as_bytes];
    data.extend(hash160(public_key).0);
    Ok(encode_base58_checksum(&data))
}

pub fn p2pkh_script(h160: &[u8]) -> Script {
    let mut script = Script::new();
    script.append_slice(&[OP_DUP, OP_HASH160]);
    script.append_data(h160);
    script.append_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);
    script
}

pub fn create_sighash(
    tx: &Tx,
    n_input: usize,
    prev_lock_script: &Script,
    prev_amount: i64,
    sighash_flags: u8,
) -> Result<Hash256> {
    let mut cache = SigHashCache::new();

    let sighash = sighash(
        tx,
        n_input,
        &prev_lock_script.0,
        prev_amount,
        sighash_flags,
        &mut cache,
    )?;
    Ok(sighash)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wallet {
    pub private_key: SigningKey,
    pub public_key: VerifyingKey,
    pub network: Network,
}

impl Wallet {
    pub fn from_wif(wif_key: &str) -> Result<Self> {
        let (network, private_key) = wif_to_network_and_private_key(wif_key)?;
        let public_key = *private_key.verifying_key();

        Ok(Wallet {
            private_key,
            public_key,
            network,
        })
    }

    pub fn new(private_key: SigningKey, public_key: VerifyingKey, network: Network) -> Self {
        Wallet {
            private_key,
            public_key,
            network,
        }
    }
    pub fn get_address(&self) -> Result<String> {
        public_key_to_address(&self.public_key_serialize(), self.network)
    }

    pub fn public_key_serialize(&self) -> [u8; 33] {
        let vk_bytes = self.public_key.to_sec1_bytes();
        let vk_vec = vk_bytes.to_vec();
        vk_vec.try_into().unwrap()
    }

    pub fn get_locking_script(&self) -> Script {
        let serial = self.public_key_serialize();
        p2pkh_script(&hash160(&serial).0)
    }

    pub fn create_unlock_script(&self, signature: &[u8]) -> Script {
        let public_key = self.public_key_serialize();
        create_unlock_script(signature, &public_key)
    }

    pub fn sign_sighash(&self, sighash: Hash256, sighash_flags: u8) -> Result<Vec<u8>> {
        // Get private key
        let private_key_as_bytes: [u8; 32] = self.private_key.to_bytes().into();
        let signature = generate_signature(&private_key_as_bytes, &sighash, sighash_flags)?;
        Ok(signature)
    }

    // sign_transaction_with_inputs(input_txs, tx, self.private_key)
    pub fn sign_tx_input(
        &self,
        tx_in: &Tx,
        tx: &mut Tx,
        index: usize,
        sighash_flags: u8,
    ) -> Result<()> {
        // Check correct input tx provided
        let prev_hash = tx.inputs[index].prev_output.hash;
        if prev_hash != tx_in.hash() {
            let err_msg = format!("Unable to find input tx {:?}", &prev_hash);
            return Err(Error::BadData(err_msg));
        }
        // Gather data for sighash
        let prev_index: usize = tx.inputs[index]
            .prev_output
            .index
            .try_into()
            .expect("Unable to convert prev_index into usize");
        let prev_amount = tx_in.outputs[prev_index].satoshis;
        let prev_lock_script = &tx_in.outputs[prev_index].lock_script;

        let sighash = create_sighash(tx, index, prev_lock_script, prev_amount, sighash_flags)?;
        // Sign sighash
        let signature = self.sign_sighash(sighash, sighash_flags)?;

        // Create unlocking script for input
        tx.inputs[index].unlock_script = self.create_unlock_script(&signature);
        Ok(())
    }

    pub fn sign_tx_sighash_flags(
        &mut self,
        index: usize,
        input_tx: Tx,
        tx: Tx,
        sighash_flags: u8,
    ) -> Result<Tx> {
        let mut new_tx = tx.clone();
        self.sign_tx_input(&input_tx, &mut new_tx, index, sighash_flags)?;
        Ok(new_tx)
    }
}
