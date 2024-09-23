use crate::{
    messages::Tx, // TxIn, TxOut},
    network::Network,
    python::{
        base58_checksum::{decode_base58_checksum, encode_base58_checksum},
        hashes::hash160,
        py_tx::tx_as_pytx,
        PyScript, PyTx,
    },
    script::{
        op_codes::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160},
        Script,
    },
    transaction::{
        generate_signature,
        p2pkh::create_unlock_script,
        sighash::{sighash, SigHashCache, SIGHASH_ALL, SIGHASH_FORKID},
    },
    util::{Error, Result},
};
use k256::ecdsa::{SigningKey, VerifyingKey};
use k256::elliptic_curve::generic_array::GenericArray;
use num_bigint::{BigInt, Sign};
use pyo3::prelude::*;
use typenum::U32; 

use pyo3::types::PyType;
use pyo3::types::{PyLong, PyDict};


use rand_core::OsRng;
use pbkdf2::pbkdf2;
use hmac::Hmac;
use sha2::Sha256;
use std::num::NonZeroU32; 

pub const MAIN_PRIVATE_KEY: u8 = 0x80;
pub const TEST_PRIVATE_KEY: u8 = 0xef;

const MAIN_PUBKEY_HASH: u8 = 0x00;
const TEST_PUBKEY_HASH: u8 = 0x6f;

// TODO: note only tested for compressed key
// Given a WIF, return bytes rather than SigningKey
pub fn wif_to_bytes(wif: &str) -> Result<Vec<u8>> {
    let (_, private_key) = wif_to_network_and_private_key(wif)?;
    let private_key_as_bytes = private_key.to_bytes();
    Ok(private_key_as_bytes.to_vec())
}

// Given bytes generate a WIF (for a private key)
pub fn bytes_to_wif(key_as_bytes: &[u8], prefix_as_bytes: u8) -> String {
    let mut wif_bytes = Vec::new();
    wif_bytes.push(prefix_as_bytes);
    wif_bytes.extend_from_slice(&key_as_bytes);
    wif_bytes.push(0x01);

    // Encode in Base58 with checksum
    encode_base58_checksum(&wif_bytes)

}

pub fn generate_wif(password: &str, nonce: &str, network: &str) -> String{
    let pw_bytes = password.as_bytes();
    let salt_bytes = nonce.as_bytes();
    let iterations = NonZeroU32::new(100_000).unwrap();
    let mut dk = [0u8; 32]; // 256-bit key
    pbkdf2::<Hmac<Sha256>>(pw_bytes, salt_bytes, iterations.into(), &mut dk).expect("HMAC can be initialized with any key length");

    // Choose prefix bytes based on network (mainnet or testnet)
    let prefix_as_bytes = match network {
        "test" => TEST_PRIVATE_KEY,
        _ => MAIN_PRIVATE_KEY,
    };


    let mut wif_bytes = Vec::new();
    wif_bytes.push(prefix_as_bytes);
    wif_bytes.extend_from_slice(&dk);
    wif_bytes.push(0x01);

    // Encode in Base58 with checksum
    encode_base58_checksum(&wif_bytes)
}

fn wif_to_network_and_private_key(wif: &str) -> Result<(Network, SigningKey)> {
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

fn network_and_private_key_to_wif(network: Network, private_key: SigningKey) -> Result<String> {
    let prefix: u8 = match network {
        Network::BSV_Mainnet => MAIN_PRIVATE_KEY,
        Network::BSV_Testnet => TEST_PRIVATE_KEY,
        _ => {
            let err_msg = format!("{} does not correspond to a known network.", network);
            return Err(Error::BadData(err_msg));
        }
    };

    let pk_data = private_key.to_bytes();
    let mut data = Vec::new();
    data.push(prefix);
    data.extend_from_slice(&pk_data);
    data.push(0x01);
    Ok(encode_base58_checksum(data.as_slice()))
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
    data.extend(hash160(public_key));
    Ok(encode_base58_checksum(&data))
}

pub fn address_to_public_key_hash(address: &str) -> Result<Vec<u8>> {
    let decoded = decode_base58_checksum(address)?;
    Ok(decoded[1..].to_vec())
}

/// Takes a hash160 and returns the p2pkh script
/// OP_DUP OP_HASH160 <hash_value> OP_EQUALVERIFY OP_CHECKSIG
pub fn p2pkh_pyscript(h160: &[u8]) -> PyScript {
    let mut script = Script::new();
    script.append_slice(&[OP_DUP, OP_HASH160]);
    script.append_data(h160);
    script.append_slice(&[OP_EQUALVERIFY, OP_CHECKSIG]);
    PyScript::new(&script.0)
}

pub fn str_to_network(network: &str) -> Option<Network> {
    match network {
        "BSV_Mainnet" => Some(Network::BSV_Mainnet),
        "BSV_Testnet" => Some(Network::BSV_Testnet),
        "BSV_STN" => Some(Network::BSV_STN),
        "BTC_Mainnet" => Some(Network::BTC_Mainnet),
        "BTC_Testnet" => Some(Network::BTC_Testnet),
        "BCH_Mainnet" => Some(Network::BCH_Mainnet),
        "BCH_Testnet" => Some(Network::BCH_Testnet),
        _ => None,
    }
}

pub fn wallet_from_int(network: &str, int_rep: BigInt) -> Result<PyWallet>{
    if let Some(netwrk) = str_to_network(network){

        let mut big_int_bytes = int_rep.to_bytes_be().1;
        if big_int_bytes.len() > 32 {
            let msg = "Private key must be 32 bytes long".to_string();
            return Err(Error::BadData(msg).into());
        }

        while big_int_bytes.len() < 32 {
            big_int_bytes.insert(0, 0);
        }
         // Convert the 32-byte array to a slice
        let key_bytes: &[u8; 32] = &big_int_bytes.try_into().expect("Expected 32-byte array");
        let key_array: &GenericArray<u8,U32> = GenericArray::from_slice(key_bytes);
        let private_key = SigningKey::from_bytes(key_array).expect("Invalid private key");

        let public_key = *private_key.verifying_key(); 
        Ok(PyWallet{
            private_key,
            public_key,
            network: netwrk,
        })
    }else{
        let msg = format!("Unknown network {}", network);
        Err(Error::BadData(msg).into())
    }
}
/// This class represents the Wallet functionality,
/// including handling of Private and Public keys
/// and signing transactions

#[pyclass(name = "Wallet")]
pub struct PyWallet {
    private_key: SigningKey,
    public_key: VerifyingKey,
    network: Network,
}

impl PyWallet {
    fn public_key_serialize(&self) -> [u8; 33] {
        let vk_bytes = self.public_key.to_sec1_bytes();
        let vk_vec = vk_bytes.to_vec();
        vk_vec.try_into().unwrap()
    }

    // sign_transaction_with_inputs(input_txs, tx, self.private_key)
    fn sign_tx_input(
        &mut self,
        tx_in: &Tx,
        tx: &mut Tx,
        index: usize,
        sighash_type: u8,
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

        let mut cache = SigHashCache::new();

        let sighash = sighash(
            tx,
            index,
            &prev_lock_script.0,
            prev_amount,
            sighash_type,
            &mut cache,
        )?;
        // Get private key
        let private_key_as_bytes: [u8; 32] = self.private_key.to_bytes().into();

        // Sign sighash
        let signature = generate_signature(&private_key_as_bytes, &sighash, sighash_type)?;
        // Create unlocking script for input
        //let public_key = self.public_key.serialize();
        let public_key = self.public_key_serialize();

        tx.inputs[index].unlock_script = create_unlock_script(&signature, &public_key);
        Ok(())
    }
}

#[pymethods]
impl PyWallet {
    // Given the wif_key, set up the wallet

    #[new]
    fn new(wif_key: &str) -> PyResult<Self> {
        let (network, private_key) = wif_to_network_and_private_key(wif_key)?;
        let public_key = *private_key.verifying_key();

        Ok(PyWallet {
            private_key,
            public_key,
            network,
        })
    }

    /// Sign a transaction with the provided previous tx, Returns new signed tx
    fn sign_tx(&mut self, index: usize, input_pytx: PyTx, pytx: PyTx) -> PyResult<PyTx> {
        // Convert PyTx -> Tx
        let input_tx = input_pytx.as_tx();
        let mut tx = pytx.as_tx();
        let sighash_type = SIGHASH_ALL | SIGHASH_FORKID;
        self.sign_tx_input(&input_tx, &mut tx, index, sighash_type)?;
        let updated_txpy = tx_as_pytx(&tx);
        Ok(updated_txpy)
    }

    /// Sign a transaction input with the provided previous tx and sighash flags, Returns new signed tx
    fn sign_tx_sighash(
        &mut self,
        index: usize,
        input_pytx: PyTx,
        pytx: PyTx,
        sighash_type: u8,
    ) -> PyResult<PyTx> {
        // Convert PyTx -> Tx
        let input_tx = input_pytx.as_tx();
        let mut tx = pytx.as_tx();
        self.sign_tx_input(&input_tx, &mut tx, index, sighash_type)?;
        let updated_txpy = tx_as_pytx(&tx);
        Ok(updated_txpy)
    }

    fn get_locking_script(&self) -> PyResult<PyScript> {
        let serial = self.public_key_serialize();
        Ok(p2pkh_pyscript(&hash160(&serial)))
    }

    fn get_public_key_as_hexstr(&self) -> String {
        let serial = self.public_key_serialize();
        serial
            .into_iter()
            .map(|x| format!("{:02x}", x))
            .collect::<Vec<_>>()
            .join("")
    }

    fn get_address(&self) -> Result<String> {
        public_key_to_address(&self.public_key_serialize(), self.network)
    }

    fn to_wif(&self) -> PyResult<String> {
        Ok(network_and_private_key_to_wif(
            self.network,
            self.private_key.clone(),
        )?)
    }

    fn get_network(&self) -> String {
        format!("{}", self.network)
    }
    
    fn to_int(&self, py: Python<'_>) -> PyResult<Py<PyLong>> {
        // Convert the private key into bytes
        let private_key_bytes = self.private_key.to_bytes();
        // Convert GenericArray<u8, _> to [u8; 32]
        let private_key_array: [u8; 32] = private_key_bytes.as_slice().try_into().expect("Private key size mismatch");

        // convert to a BitInt (signed for now)
        let big_int_signed_rep = BigInt::from_bytes_be(Sign::Plus, &private_key_array);

        // Convert the large integer to a string (Python handles large integers from strings well)
        let result_str = big_int_signed_rep.to_string();

        // Create a new PyDict for globals
        let globals = PyDict::new_bound(py);
        // Use Python's built-in int() constructor to convert the string to a Python integer
        let py_int = py.eval_bound(&format!("int('{}')", result_str), Some(&globals), None)?;

        // Cast to PyLong and return
        Ok((*py_int.downcast::<PyLong>()?).clone().into())
    }


    fn to_hex(&self) -> String{
        // Convert the private key into bytes
        let private_key_bytes = self.private_key.to_bytes();
        // Convert GenericArray<u8, _> to [u8; 32]
        let private_key_array: [u8; 32] = private_key_bytes.as_slice().try_into().expect("Private key size mismatch");
        let scalar_hex = hex::encode(private_key_array);
        scalar_hex
    }


    #[classmethod]
    fn generate_keypair(_cls: &Bound<'_, PyType>, network: &str) -> PyResult<Self> {
        if let Some(netwrk) = str_to_network(network) {
            let private_key = SigningKey::random(&mut OsRng);
            let public_key = *private_key.verifying_key();

            Ok(PyWallet {
                private_key,
                public_key,
                network: netwrk,
            })
        } else {
            let msg = format!("Unknown network {}", network);
            Err(Error::BadData(msg).into())
        }
    }

    #[classmethod]
    fn wallet_from_bytes(_cls: &Bound<'_, PyType>, network: &str, key_bytes: &[u8]) -> PyResult<Self>{
        if let Some(netwrk) = str_to_network(network){
            // Ensure the length of key_bytes is 32 bytes
            if key_bytes.len() != 32 {
                let msg = "Private key must be 32 bytes long".to_string();
                return Err(Error::BadData(msg).into());
            }
            // Convert &[u8] to a GenericArray<u8, 32>
            let key_array: &GenericArray<u8,U32> = GenericArray::from_slice(&key_bytes);
            let private_key = SigningKey::from_bytes(key_array).expect("Invalid private key");
            let public_key = *private_key.verifying_key(); 
            Ok(PyWallet{
                private_key,
                public_key,
                network: netwrk,
            })
        }else{
            let msg = format!("Unknown network {}", network);
            Err(Error::BadData(msg).into())
        }
    }

    #[classmethod]
    fn wallet_from_hexstr(_cls: &Bound<'_, PyType>, network: &str, hexstr: &str) -> PyResult<Self>{
        if let Some(netwrk) = str_to_network(network){
            // Attempt to decode the hex string
            let key_bytes = match hex::decode(hexstr) {
                Ok(bytes) => bytes,
                Err(e) => return Err(Error::BadData(e.to_string()).into()),
            };

            // Ensure the length of the bytes is exactly 32
            if key_bytes.len() != 32 {
                let msg = "Private key must be 32 bytes long".to_string();
                return Err(Error::BadData(msg).into());
            }
            
            // Convert &[u8] to a GenericArray<u8, 32>
            let key_array: &GenericArray<u8,U32> = GenericArray::from_slice(&key_bytes);
            let private_key = SigningKey::from_bytes(key_array).expect("Invalid private key");
            let public_key = *private_key.verifying_key(); 
            Ok(PyWallet{
                private_key,
                public_key,
                network: netwrk,
            })
        }else{
            let msg = format!("Unknown network {}", network);
            Err(Error::BadData(msg).into())
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes_to_hexstr(bytes: &[u8]) -> String {
        bytes
            .into_iter()
            .map(|x| format!("{:02x}", x))
            .collect::<Vec<_>>()
            .join("")
    }

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
        assert_eq!(
            wallet.get_address().unwrap(),
            "mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5"
        );
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
        let cmds = bytes_to_hexstr(&ls.cmds);
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

    #[test]
    fn addr_to_public_key_hash() {
        let address = "mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5";
        let public_key =
            hex::decode("036a1a87d876e0fab2f7dc19116e5d0e967d7eab71950a7de9f2afd44f77a0f7a2")
                .unwrap();
        let hash_public_key = hash160(&public_key);

        let pk = address_to_public_key_hash(address).unwrap();
        let pk_hexstr = bytes_to_hexstr(&pk);
        let hash_pk = bytes_to_hexstr(&hash_public_key);
        assert_eq!(pk_hexstr, hash_pk);
    }
    /*
    #[test]
    fn generate_key() {
        let w = PyWallet::generate_key(Network::BSV_Testnet).unwrap();
        dbg!(&w);
    }
    */

    // TODO: Wallet signing test
    /*
    #[test]
    fn sign_tx() {
        let wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3";
        let w = PyWallet::new(wif);
        let wallet = w.unwrap();

        // tx =
        //
    }
    */
}
