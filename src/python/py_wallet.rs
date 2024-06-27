use pyo3::prelude::*;
use ripemd::Ripemd160;
use secp256k1::{All, PublicKey, Secp256k1, SecretKey};
use sha2::{Digest, Sha256};
use std::slice;
//use rand::rngs::OsRng;

use crate::{
    network::Network,
    python::{
        base58_checksum::{decode_base58_checksum, encode_base58_checksum},
        PyScript,
    },
    script::{
        op_codes::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160},
        Script,
    },
    util::{Error, Result},
};

const MAIN_PRIVATE_KEY: u8 = 0x80;
const TEST_PRIVATE_KEY: u8 = 0xef;

const MAIN_PUBKEY_HASH: u8 = 0x00;
const TEST_PUBKEY_HASH: u8 = 0x6f;

fn hash160(data: &[u8]) -> Vec<u8> {
    let sha256 = Sha256::digest(data);
    Ripemd160::digest(sha256).to_vec()
}

// TODO: note only tested for compressed key
fn wif_to_network_and_private_key(wif: &str) -> Result<(Network, SecretKey)> {
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
    let private_key = SecretKey::from_slice(&private_key_as_bytes)?;
    Ok((network, private_key))
}

fn network_and_private_key_to_wif(network: Network, private_key: SecretKey) -> Result<String> {
    let prefix: u8 = match network {
        Network::BSV_Mainnet => MAIN_PRIVATE_KEY,
        Network::BSV_Testnet => TEST_PRIVATE_KEY,
        _ => {
            let err_msg = format!("{} does not correspond to a known network.", network);
            return Err(Error::BadData(err_msg));
        }
    };

    dbg!(&private_key.len());

    let pk_data = unsafe { slice::from_raw_parts(private_key.as_ptr(), private_key.len()) };
    let mut data = Vec::new();
    data.push(prefix);
    data.extend_from_slice(pk_data);
    data.push(0x01);
    Ok(encode_base58_checksum(data.as_slice()))
}

// Given public_key and network return address as a string
fn public_key_to_address(public_key: &[u8], network: Network) -> Result<String> {
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
    data.extend(hash160(public_key));
    Ok(encode_base58_checksum(&data))
}

/// Takes a hash160 and returns the p2pkh script
/// OP_DUP OP_HASH160 <hash_value> OP_EQUALVERIFY OP_CHECKSIG
// The script (signature public_key --- bool)
fn p2pkh_script(h160: &[u8]) -> PyScript {
    let mut script = Script::new();
    script.append_slice(&[OP_DUP, OP_HASH160]);
    script.append_data(h160);
    script.append_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);
    PyScript::new(&script.0)
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
        let public_key = PublicKey::from_secret_key(&secp, &private_key);
        // public_key -> address
        let address = public_key_to_address(&public_key.serialize(), network)?;
        Ok(PyWallet {
            secp,
            private_key,
            address,
            network,
        })
    }

    /*  
    fn generate_key(network: Network) -> PyResult<Self> {
        let secp = Secp256k1::new();
        let mut rng = OsRng::new()?;
        let (private_key, public_key) = secp.generate_keypair(&mut rng);

        let address = public_key_to_address(&public_key.serialize(), network)?;
        Ok(PyWallet {
            secp,
            private_key,
            address,
            network,
        })
    }
    */
    

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

    fn get_locking_script(&self) -> PyResult<PyScript> {
        let h160 = decode_base58_checksum(&self.address)?;
        // Drop the first byte
        Ok(p2pkh_script(&h160[1..]))
    }

    fn get_public_key_as_hexstr(&self) -> String {
        let public_key = PublicKey::from_secret_key(&self.secp, &self.private_key);
        let serial = public_key.serialize();
        let hexstr = serial
            .into_iter()
            .map(|x| format!("{:02x}", x))
            .collect::<Vec<_>>()
            .join("");
        hexstr
    }

    fn to_wif(&self) -> PyResult<String> {
        Ok(network_and_private_key_to_wif(
            self.network,
            self.private_key,
        )?)
    }


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

        let wallet = w.unwrap();
        assert_eq!(wallet.address, "mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5");
        assert_eq!(wallet.network, Network::BSV_Testnet);
    }

    #[test]
    fn wif_wallet_roundtrip() {
        let wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3";
        let w = PyWallet::new(wif);

        let wallet = w.unwrap();
        let wif2 = wallet.to_wif().unwrap();
        assert_eq!(wif, wif2);
    }

    #[test]
    fn locking_script() {
        let wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3";
        let w = PyWallet::new(wif);
        let wallet = w.unwrap();

        let ls = wallet.get_locking_script().unwrap();
        let cmds = ls
            .cmds
            .into_iter()
            .map(|x| format!("{:02x}", x))
            .collect::<Vec<_>>()
            .join("");
        let locking_script = "76a91410375cfe32b917cd24ca1038f824cd00f739185988ac";
        assert_eq!(cmds, locking_script);
    }

    #[test]
    fn public_key() {
        let wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3";
        let w = PyWallet::new(wif);
        let wallet = w.unwrap();

        let pk = wallet.get_public_key_as_hexstr();

        let public_key = "036a1a87d876e0fab2f7dc19116e5d0e967d7eab71950a7de9f2afd44f77a0f7a2";
        assert_eq!(pk, public_key);
    }


    /*
    #[test]
    fn generate_key() {
        let w = PyWallet::generate_key(Network::BSV_Testnet).unwrap();
        dbg!(&w);
    }
    */
}
