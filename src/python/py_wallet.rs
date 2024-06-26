use base58::{FromBase58, ToBase58};
use sha2::{Digest, Sha256};
use secp256k1::{PublicKey, Secp256k1, SecretKey, All};
//use secp256k1::SecretKey;
use ripemd::Ripemd160;

use pyo3::prelude::*;

use crate::{
    network::Network,
    // script::Script,
    // messages::{OutPoint, Tx, TxIn, TxOut},
    python::PyScript,
    util::{Error, Result},
};



const MAIN_PRIVATE_KEY: u8 = b'\x80';
const TEST_PRIVATE_KEY: u8 = b'\xef';

const MAIN_PUBKEY_HASH: u8 = b'\x00';
const TEST_PUBKEY_HASH: u8 = b'\x6f';


/// Return first 4 digits of double sha256
fn short_double_sha256_checksum(data: &[u8]) -> Vec<u8> {
    // Double hash of data
    let sha256 = Sha256::digest(data);
    let sha256d = Sha256::digest(sha256);
    // Return first 4 digits
    sha256d.as_slice()[..4].to_vec()
}

fn hash160(data: &[u8]) -> Vec<u8> {
    let sha256 = Sha256::digest(data);
    Ripemd160::digest(sha256).to_vec()
}


/// Given the string return the checked base58 value
fn decode_base58_checksum(input: &str) -> Result<Vec<u8>> {
    let decoded: Vec<u8> = input.from_base58()?;
    // Return all but the last 4
    let shortened: Vec<u8> = decoded.as_slice()[..decoded.len() - 4].to_vec();
    // Return last 4
    let decoded_checksum: Vec<u8> = decoded.as_slice()[decoded.len() - 4..].to_vec();
    let hash_checksum: Vec<u8> = short_double_sha256_checksum(&shortened);
    if hash_checksum != decoded_checksum {
        let err_msg = format!(
            "Decoded checksum {:x?} derived from '{}' is not equal to hash checksum {:x?}.",
            decoded_checksum, input, hash_checksum
        );
        Err(Error::BadData(err_msg))
    } else {
        Ok(shortened)
    }
}

/// Return base58 with checksum
/// Used to turn public key into an address
fn encode_base58_checksum(input: &[u8]) -> String {

    let hash = short_double_sha256_checksum(input);
    let mut data: Vec<u8> = input.to_vec();
    data.extend(hash);
    
    data.to_base58()
}

// TODO: note only tested for compressed key
fn wif_to_network_and_private_key(wif: &str) -> Result<(Network, SecretKey)> {
    let decode = decode_base58_checksum(wif)?;
    // Get first byte
    let prefix: u8 = *decode.first().ok_or("Invalid wif length")?;
    let network: Network;

    match prefix {
        MAIN_PRIVATE_KEY => network = Network::BSV_Mainnet,
        TEST_PRIVATE_KEY => network = Network::BSV_Testnet,
        _ => {
            let err_msg = format!(
                "{:02x?} does not correspond to a mainnet nor testnet address.",
                prefix
            );
            return Err(Error::BadData(err_msg));
        }
    }
    // Remove prefix byte and, if present, compression flag.
    let last_byte: u8 = *decode.last().ok_or("Invalid wif length")?;
    let compressed: bool = wif.len() == 52 && last_byte == 1u8;

    let private_key_as_bytes: Vec<u8> = if compressed {
        decode[1..decode.len() - 1].to_vec()
    } else {
        decode[1..].to_vec()
    };
    let private_key = SecretKey::from_slice(&private_key_as_bytes)?;
    Ok((network, private_key))
}


fn public_key_to_address(public_key: &[u8], network: Network) -> Result<String> {
    let prefix_as_bytes: u8;
    
    match network {
        Network::BSV_Mainnet => prefix_as_bytes = MAIN_PUBKEY_HASH,
        Network::BSV_Testnet => prefix_as_bytes = TEST_PUBKEY_HASH,
        _ => {
            let err_msg = format!("{} unknnown network.", &network);
            return Err(Error::BadData(err_msg));
        }
    };
    // # 33 bytes compressed, 65 uncompressed.
    if public_key.len() != 33 && public_key.len() != 65 {
        let err_msg = format!("{} is an invalid length for a public key.", public_key.len());
        return Err(Error::BadData(err_msg));
    }

    let mut data: Vec<u8> = vec![prefix_as_bytes];
    data.extend(hash160(public_key));
    Ok(encode_base58_checksum(&data))
}


/// This class represents the Wallet functionality, 
/// including handling of Private and Public keys
/// and signing transactions

#[pyclass(name = "Wallet")]
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PyWallet {
    secp: Secp256k1<All>,
    private_key: SecretKey,
    pub address: String,
    pub network: Network,
}

impl PyWallet {
    // sign_transaction_with_inputs(input_txs, tx, self.private_key)
    /*
    fn sign_tx_with_inputs(&self, input_txs: &[Tx], tx: &mut Tx) -> bool {
        //return sign_transaction_with_inputs(input_txs, tx, self.private_key)
        // Sign a transaction with the provided private key
        // Return true if successful

        //# Sign inputs
        /*
        for i, _ in enumerate(tx.tx_ins):
            if not sign_input_bsv_with_inputs(input_txs, tx, i, private_key):
                print(f"failed to sign input {i}")
                return False
        return True
        */
        true
    }
    */
}

#[pymethods]
impl PyWallet {


    /// Given the wif_key, set up the wallet
    #[new]
    fn new(wif_key: &str) -> PyResult<Self> {
        let secp = Secp256k1::new();

        let (network, private_key) = wif_to_network_and_private_key(wif_key)?;
        // self.address = self.private_key.address
        let public_key = PublicKey::from_secret_key(&secp, &private_key);
        // public_key -> address
        let address = public_key_to_address(&public_key.serialize(), network)?;
        // TODO: check serialised public key is valid
        Ok(PyWallet {
            secp,
            private_key,
            address,
            network,
        })
    }

    //fn sign_tx_with_inputs(&self, input_txs: &[Tx], tx: &mut Tx) -> bool {
    //return sign_transaction_with_inputs(input_txs, tx, self.private_key)
    // Sign a transaction with the provided private key
    // Return true if successful

    //# Sign inputs
    /*
    for i, _ in enumerate(tx.tx_ins):
        if not sign_input_bsv_with_inputs(input_txs, tx, i, private_key):
            print(f"failed to sign input {i}")
            return False
    return True
    */
    //    true
    //}

    fn get_locking_script(&self) -> PyScript {
        // p2pkh_script(decode_base58(self.address))
        PyScript::new(&[])
    }

    fn get_locking_script_as_hex(&self) -> String {
        //self.get_locking_script().raw_serialize().hex()
        String::new()
    }

    fn get_public_key_as_hexstr(&self) -> String {
        //self.private_key.public_key.hex()
        String::new()
    }

    /*
    fn generate_key(network: Network) -> Self {
    }
    fn to_wif(&self) -> String {
    }
    */
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_base58_checksum_valid() {
        // Valid data
        let wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3";
        let result = decode_base58_checksum(wif);
        assert!(&result.is_ok());
    }

    #[test]
    fn decode_base58_checksum_invalid() {
        // Invalid data
        let wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab2";
        let result = decode_base58_checksum(wif);
        assert!(&result.is_err());
    }

    #[test]
    fn wif_to_bytes_check() {
        // Valid data
        let wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3";
        let result = wif_to_network_and_private_key(wif);
        assert!(result.is_ok());
        if let Ok((network, _private_key)) = result {
            assert!(network == Network::BSV_Testnet);
        }
    }

    #[test]
    fn wif_to_wallet() {
        let wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3";
        let w = PyWallet::new(wif);
        
        //dbg!(w);
        let wallet = w.unwrap();
        assert_eq!(wallet.address, "mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5");
        assert_eq!(wallet.network, Network::BSV_Testnet);
    }
    
    /*
    root@1336db7830e6:/app/python/util# python3 generate_key.py
    wif = cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3
    private_key = <PrivateKey: mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5>
    address = mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5
    */
}
