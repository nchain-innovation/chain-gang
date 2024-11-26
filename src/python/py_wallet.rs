use crate::{
    network::Network,
    python::{py_tx::tx_as_pytx, PyScript, PyTx},
    script::{
        op_codes::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160},
        Script,
    },
    transaction::sighash::{SIGHASH_ALL, SIGHASH_FORKID},
    util::{Error, Result},
    wallet::{
        base58_checksum::{decode_base58_checksum, encode_base58_checksum},
        wallet::{wif_to_network_and_private_key, Wallet, MAIN_PRIVATE_KEY, TEST_PRIVATE_KEY},
    },
};
use k256::{ecdsa::SigningKey, elliptic_curve::generic_array::GenericArray};
use num_bigint::{BigInt, Sign};
use pyo3::{
    prelude::*,
    types::{PyDict, PyLong, PyType},
};

use typenum::U32;

use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand_core::OsRng;
use sha2::Sha256;
use std::num::NonZeroU32;

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
    wif_bytes.extend_from_slice(key_as_bytes);
    wif_bytes.push(0x01);

    // Encode in Base58 with checksum
    encode_base58_checksum(&wif_bytes)
}

pub fn generate_wif(password: &str, nonce: &str, network: &str) -> String {
    let pw_bytes = password.as_bytes();
    let salt_bytes = nonce.as_bytes();
    let iterations = NonZeroU32::new(100_000).unwrap();
    let mut dk = [0u8; 32]; // 256-bit key
    pbkdf2::<Hmac<Sha256>>(pw_bytes, salt_bytes, iterations.into(), &mut dk)
        .expect("HMAC can be initialized with any key length");

    // Choose prefix bytes based on network (mainnet or testnet)
    let prefix_as_bytes = match network {
        "BSV_Testnet" => TEST_PRIVATE_KEY,
        _ => MAIN_PRIVATE_KEY,
    };

    let mut wif_bytes = Vec::new();
    wif_bytes.push(prefix_as_bytes);
    wif_bytes.extend_from_slice(&dk);
    wif_bytes.push(0x01);

    // Encode in Base58 with checksum
    encode_base58_checksum(&wif_bytes)
}

pub fn network_and_private_key_to_wif(network: Network, private_key: SigningKey) -> Result<String> {
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

pub fn wallet_from_int(network: &str, int_rep: BigInt) -> Result<PyWallet> {
    if let Some(netwrk) = str_to_network(network) {
        let mut big_int_bytes = int_rep.to_bytes_be().1;
        if big_int_bytes.len() > 32 {
            let msg = "Private key must be 32 bytes long".to_string();
            return Err(Error::BadData(msg));
        }

        while big_int_bytes.len() < 32 {
            big_int_bytes.insert(0, 0);
        }
        // Convert the 32-byte array to a slice
        let key_bytes: &[u8; 32] = &big_int_bytes.try_into().expect("Expected 32-byte array");
        let key_array: &GenericArray<u8, U32> = GenericArray::from_slice(key_bytes);
        let private_key = SigningKey::from_bytes(key_array).expect("Invalid private key");

        let public_key = *private_key.verifying_key();
        let wallet = Wallet::new(private_key, public_key, netwrk);
        Ok(PyWallet { wallet })
    } else {
        let msg = format!("Unknown network {}", network);
        Err(Error::BadData(msg))
    }
}
/// This class represents the Wallet functionality,
/// including handling of Private and Public keys
/// and signing transactions

#[pyclass(name = "Wallet")]
pub struct PyWallet {
    wallet: Wallet,
}

#[pymethods]
impl PyWallet {
    // Given the wif_key, set up the wallet

    #[new]
    fn new(wif_key: &str) -> PyResult<Self> {
        let wallet = Wallet::from_wif(wif_key)?;
        Ok(PyWallet { wallet })
    }

    /// Sign a transaction with the provided previous tx, Returns new signed tx
    fn sign_tx(&mut self, index: usize, input_pytx: PyTx, pytx: PyTx) -> PyResult<PyTx> {
        // Convert PyTx -> Tx
        let input_tx = input_pytx.as_tx();
        let mut tx = pytx.as_tx();
        let sighash_type = SIGHASH_ALL | SIGHASH_FORKID;
        self.wallet
            .sign_tx_input(&input_tx, &mut tx, index, sighash_type)?;
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
        self.wallet
            .sign_tx_input(&input_tx, &mut tx, index, sighash_type)?;
        let updated_txpy = tx_as_pytx(&tx);
        Ok(updated_txpy)
    }

    fn get_locking_script(&self) -> PyResult<PyScript> {
        let script = self.wallet.get_locking_script();
        let pyscript = PyScript::new(&script.0);
        Ok(pyscript)
    }

    fn get_public_key_as_hexstr(&self) -> String {
        let serial = self.wallet.public_key_serialize();
        serial
            .into_iter()
            .map(|x| format!("{:02x}", x))
            .collect::<Vec<_>>()
            .join("")
    }

    fn get_address(&self) -> Result<String> {
        self.wallet.get_address()
    }

    fn to_wif(&self) -> PyResult<String> {
        Ok(network_and_private_key_to_wif(
            self.wallet.network,
            self.wallet.private_key.clone(),
        )?)
    }

    fn get_network(&self) -> String {
        format!("{}", self.wallet.network)
    }

    fn to_int(&self, py: Python<'_>) -> PyResult<Py<PyLong>> {
        // Convert the private key into bytes
        let private_key_bytes = self.wallet.private_key.to_bytes();
        // Convert GenericArray<u8, _> to [u8; 32]
        let private_key_array: [u8; 32] = private_key_bytes
            .as_slice()
            .try_into()
            .expect("Private key size mismatch");

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

    fn to_hex(&self) -> String {
        // Convert the private key into bytes
        let private_key_bytes = self.wallet.private_key.to_bytes();
        // Convert GenericArray<u8, _> to [u8; 32]
        let private_key_array: [u8; 32] = private_key_bytes
            .as_slice()
            .try_into()
            .expect("Private key size mismatch");
        hex::encode(private_key_array)
    }

    #[classmethod]
    fn generate_keypair(_cls: &Bound<'_, PyType>, network: &str) -> PyResult<Self> {
        if let Some(netwrk) = str_to_network(network) {
            let private_key = SigningKey::random(&mut OsRng);
            let public_key = *private_key.verifying_key();
            let wallet = Wallet::new(private_key, public_key, netwrk);
            Ok(PyWallet { wallet })
        } else {
            let msg = format!("Unknown network {}", network);
            Err(Error::BadData(msg).into())
        }
    }

    #[classmethod]
    fn from_bytes(_cls: &Bound<'_, PyType>, network: &str, key_bytes: &[u8]) -> PyResult<Self> {
        if let Some(netwrk) = str_to_network(network) {
            // Ensure the length of key_bytes is 32 bytes
            if key_bytes.len() != 32 {
                let msg = "Private key must be 32 bytes long".to_string();
                return Err(Error::BadData(msg).into());
            }
            // Convert &[u8] to a GenericArray<u8, 32>
            let key_array: &GenericArray<u8, U32> = GenericArray::from_slice(key_bytes);
            let private_key = SigningKey::from_bytes(key_array).expect("Invalid private key");
            let public_key = *private_key.verifying_key();
            let wallet = Wallet::new(private_key, public_key, netwrk);
            Ok(PyWallet { wallet })
        } else {
            let msg = format!("Unknown network {}", network);
            Err(Error::BadData(msg).into())
        }
    }

    #[classmethod]
    fn from_hexstr(_cls: &Bound<'_, PyType>, network: &str, hexstr: &str) -> PyResult<Self> {
        if let Some(netwrk) = str_to_network(network) {
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
            let key_array: &GenericArray<u8, U32> = GenericArray::from_slice(&key_bytes);
            let private_key = SigningKey::from_bytes(key_array).expect("Invalid private key");
            let public_key = *private_key.verifying_key();
            let wallet = Wallet::new(private_key, public_key, netwrk);
            Ok(PyWallet { wallet })
        } else {
            let msg = format!("Unknown network {}", network);
            Err(Error::BadData(msg).into())
        }
    }

    #[classmethod]
    fn from_int(
        _cls: &Bound<'_, PyType>,
        network: &str,
        int_rep: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        // Use with_gil to get a reference to the Python interpreter
        Python::with_gil(|_cls| {
            // Use the bound reference to access the PyAny
            // Downcast the PyAny reference to PyLong
            let py_long = int_rep
                .downcast::<PyLong>()
                .map_err(|_| pyo3::exceptions::PyTypeError::new_err("Expected a PyLong"))?
                .as_ref();

            // Convert the PyLong into a BigInt using to_string
            let big_int_str = py_long.str()?.to_str()?.to_owned();

            // Convert the string to a Rust BigInt (assumption is base-10)
            let big_int = BigInt::parse_bytes(big_int_str.as_bytes(), 10)
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Failed to parse BigInt"))?;

            let test_wallet = wallet_from_int(network, big_int)?;
            Ok(test_wallet)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::hash160;

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

        let wallet1 = w.unwrap();
        assert_eq!(
            wallet1.get_address().unwrap(),
            "mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5"
        );
        assert_eq!(wallet1.wallet.network, Network::BSV_Testnet);
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
        let hash_public_key = hash160(&public_key).0;

        let pk = address_to_public_key_hash(address).unwrap();
        let pk_hexstr = bytes_to_hexstr(&pk);
        let hash_pk = bytes_to_hexstr(&hash_public_key);
        assert_eq!(pk_hexstr, hash_pk);
    }
}
