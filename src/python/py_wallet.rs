use crate::{
    network::Network,
    python::{py_tx::tx_as_pytx, PyScript, PyTx},
    script::{
        op_codes::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160},
        Script,
    },
    transaction::sighash::{SIGHASH_ALL, SIGHASH_FORKID},
    util::ChainGangError,
    wallet::{
        base58_checksum::{decode_base58_checksum, encode_base58_checksum},
        wallet::{wif_to_network_and_private_key, Wallet, MAIN_PRIVATE_KEY, TEST_PRIVATE_KEY},
    },
};
use k256::{ecdsa::SigningKey, elliptic_curve::Generate};
use pyo3::{
    prelude::*,
    types::{PyBytes, PyInt, PyType},
};

use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha2::Sha256;
use std::num::NonZeroU32;

/// Decode a compressed WIF private key to 32 raw bytes.
///
/// Uncompressed WIF (51 characters, no trailing `0x01` suffix) is not supported here;
/// `bytes_to_wif` always emits compressed WIF.
pub fn wif_to_bytes(wif: &str) -> Result<Vec<u8>, ChainGangError> {
    let (_, private_key) = wif_to_network_and_private_key(wif)?;
    let private_key_as_bytes = private_key.to_bytes();
    Ok(private_key_as_bytes.to_vec())
}

/// Encode 32 private-key bytes as a compressed WIF (appends `0x01` suffix).
pub fn bytes_to_wif(key_as_bytes: &[u8], prefix_as_bytes: u8) -> String {
    let mut wif_bytes = Vec::new();
    wif_bytes.push(prefix_as_bytes);
    wif_bytes.extend_from_slice(key_as_bytes);
    wif_bytes.push(0x01);

    // Encode in Base58 with checksum
    encode_base58_checksum(&wif_bytes)
}

/// Derives a WIF private key from `password` and `nonce` for `network`.
///
/// Takes a [`Network`] rather than its name: the network used to arrive as a
/// string, and anything the match did not recognise fell back to the mainnet
/// prefix, so a mistyped or unsupported name silently produced a mainnet key.
pub fn generate_wif(
    password: &str,
    nonce: &str,
    network: Network,
) -> Result<String, ChainGangError> {
    let prefix = wif_prefix(network)?;

    let iterations = NonZeroU32::new(100_000).unwrap();
    let mut dk = [0u8; 32]; // 256-bit key
    pbkdf2::<Hmac<Sha256>>(
        password.as_bytes(),
        nonce.as_bytes(),
        iterations.get(),
        &mut dk,
    )
    .expect("HMAC can be initialized with any key length");

    Ok(bytes_to_wif(&dk, prefix))
}

/// WIF version byte for `network`, or an error where the crate defines none.
///
/// The single place the mapping lives, so the two WIF constructors cannot drift
/// apart on what a network means.
fn wif_prefix(network: Network) -> Result<u8, ChainGangError> {
    match network {
        Network::BSV_Mainnet => Ok(MAIN_PRIVATE_KEY),
        // Regtest uses testnet's WIF version byte, as it does for addresses
        Network::BSV_Testnet | Network::BSV_Regtest => Ok(TEST_PRIVATE_KEY),
        Network::BSV_STN
        | Network::BTC_Mainnet
        | Network::BTC_Testnet
        | Network::BCH_Mainnet
        | Network::BCH_Testnet => Err(ChainGangError::BadData(format!(
            "{network} does not correspond to a known network."
        ))),
    }
}

pub fn network_and_private_key_to_wif(
    network: Network,
    private_key: SigningKey,
) -> Result<String, ChainGangError> {
    let prefix = wif_prefix(network)?;
    Ok(bytes_to_wif(&private_key.to_bytes(), prefix))
}

pub fn address_to_public_key_hash(address: &str) -> Result<Vec<u8>, ChainGangError> {
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
    network.parse().ok()
}

/// Builds a wallet from a raw private key, for the `from_bytes`, `from_hexstr`
/// and `from_int` constructors.
///
/// The key must be 32 bytes and a valid secp256k1 scalar: not zero, and below
/// the curve order. Anything else is an error. These constructors used to
/// `expect` it, so a key of zero, or one at or above the order, panicked, and
/// Python saw a `PanicException`, which derives from `BaseException` and slips
/// past `except Exception` (#34).
fn wallet_from_key_bytes(network: &str, key_bytes: &[u8]) -> Result<PyWallet, ChainGangError> {
    let netwrk = str_to_network(network)
        .ok_or_else(|| ChainGangError::BadData(format!("Unknown network {}", network)))?;
    if key_bytes.len() != 32 {
        let msg = "Private key must be 32 bytes long".to_string();
        return Err(ChainGangError::BadData(msg));
    }
    let private_key = SigningKey::from_slice(key_bytes).map_err(|_| {
        ChainGangError::BadData(
            "Private key must be greater than zero and less than the secp256k1 curve order"
                .to_string(),
        )
    })?;
    let public_key = *private_key.verifying_key();
    Ok(PyWallet {
        wallet: Wallet::new(private_key, public_key, netwrk),
    })
}
/// This class represents the Wallet functionality,
/// including handling of Private and Public keys
/// and signing transactions

#[pyclass(name = "Wallet")]
pub struct PyWallet {
    wallet: Wallet,
}

impl PyWallet {
    pub(crate) fn from_wallet(wallet: Wallet) -> Self {
        PyWallet { wallet }
    }
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

    fn sign_tx_sighash_checksig_index(
        &mut self,
        index: usize,
        input_pytx: PyTx,
        pytx: PyTx,
        sighash_type: u8,
        checksig_index: usize,
    ) -> PyResult<PyTx> {
        // Convert PyTx -> Tx
        let input_tx = input_pytx.as_tx();
        let mut tx = pytx.as_tx();
        self.wallet.sign_tx_input_checksig_index(
            &input_tx,
            &mut tx,
            index,
            sighash_type,
            checksig_index,
        )?;
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

    fn get_address(&self) -> PyResult<String> {
        Ok(self.wallet.get_address()?)
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

    fn to_int(&self, py: Python<'_>) -> PyResult<Py<PyInt>> {
        // `int.from_bytes(key, "big")`. Under the stable ABI the wheels are built
        // for there is no C call that makes a 256-bit int, and this is what PyO3's
        // own big-integer conversion does there too. It used to format the key as
        // a decimal string and `eval` the expression `int('...')` (#34).
        let key: [u8; 32] = self.wallet.private_key.to_bytes().into();
        let int = py
            .get_type::<PyInt>()
            .call_method1("from_bytes", (PyBytes::new(py, &key), "big"))?;
        Ok(int.cast_into::<PyInt>()?.unbind())
    }

    fn to_hex(&self) -> String {
        let private_key_array: [u8; 32] = self.wallet.private_key.to_bytes().into();
        hex::encode(private_key_array)
    }

    #[classmethod]
    fn generate_keypair(_cls: &Bound<'_, PyType>, network: &str) -> PyResult<Self> {
        if let Some(netwrk) = str_to_network(network) {
            let private_key = SigningKey::generate();
            let public_key = *private_key.verifying_key();
            let wallet = Wallet::new(private_key, public_key, netwrk);
            Ok(PyWallet { wallet })
        } else {
            let msg = format!("Unknown network {}", network);
            Err(ChainGangError::BadData(msg).into())
        }
    }

    #[classmethod]
    fn from_bytes(_cls: &Bound<'_, PyType>, network: &str, key_bytes: &[u8]) -> PyResult<Self> {
        Ok(wallet_from_key_bytes(network, key_bytes)?)
    }

    #[classmethod]
    fn from_hexstr(_cls: &Bound<'_, PyType>, network: &str, hexstr: &str) -> PyResult<Self> {
        let key_bytes = hex::decode(hexstr).map_err(|e| ChainGangError::BadData(e.to_string()))?;
        Ok(wallet_from_key_bytes(network, &key_bytes)?)
    }

    #[classmethod]
    fn from_int(
        _cls: &Bound<'_, PyType>,
        network: &str,
        int_rep: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let int = int_rep
            .cast::<PyInt>()
            .map_err(|_| pyo3::exceptions::PyTypeError::new_err("Expected an int"))?;
        // `int.to_bytes(32, "big")` raises OverflowError for a negative value or
        // one of 2**256 or more, which no private key is. This used to go through
        // the decimal string and drop the sign, so -5 made the wallet for key 5.
        let key = int.call_method1("to_bytes", (32, "big")).map_err(|_| {
            ChainGangError::BadData(
                "Private key must be a non-negative integer below 2**256".to_string(),
            )
        })?;
        Ok(wallet_from_key_bytes(
            network,
            key.cast::<PyBytes>()?.as_bytes(),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::hash160;
    use k256::SecretKey;

    /// secp256k1's group order, the first value that is not a private key.
    const CURVE_ORDER: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xfe, 0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36,
        0x41, 0x41,
    ];

    /// Out-of-range keys are errors, not panics (#34). The range is 1 to the
    /// curve order less one, and both ends are accepted.
    #[test]
    fn key_bytes_outside_the_scalar_range_are_errors() {
        let mut one = [0u8; 32];
        one[31] = 1;
        let mut order_less_one = CURVE_ORDER;
        order_less_one[31] -= 1;
        assert!(wallet_from_key_bytes("BSV_Mainnet", &one).is_ok());
        assert!(wallet_from_key_bytes("BSV_Mainnet", &order_less_one).is_ok());

        for (label, key) in [
            ("zero", [0u8; 32]),
            ("the curve order", CURVE_ORDER),
            ("all ones", [0xff; 32]),
        ] {
            match wallet_from_key_bytes("BSV_Mainnet", &key) {
                Err(e) => assert!(e.to_string().contains("curve order"), "{label}: {e}"),
                Ok(_) => panic!("{label} is not a private key"),
            }
        }
        assert!(wallet_from_key_bytes("BSV_Mainnet", &one[..31]).is_err());
        assert!(wallet_from_key_bytes("Nonsense", &one).is_err());
    }

    #[test]
    fn generate_wif_uses_the_networks_own_prefix() {
        // The network used to arrive as a string, where anything but the exact
        // "BSV_Testnet" produced a mainnet key. A Network cannot be mistyped.
        let main = generate_wif("pw", "nonce", Network::BSV_Mainnet).unwrap();
        let test = generate_wif("pw", "nonce", Network::BSV_Testnet).unwrap();
        let regtest = generate_wif("pw", "nonce", Network::BSV_Regtest).unwrap();

        assert_ne!(main, test, "mainnet and testnet must not encode alike");
        assert_eq!(regtest, test, "regtest shares testnet's WIF prefix");
        // The key material is the same; only the version byte differs
        assert_eq!(wif_to_bytes(&main).unwrap(), wif_to_bytes(&test).unwrap());
    }

    #[test]
    fn generate_wif_is_deterministic() {
        assert_eq!(
            generate_wif("pw", "nonce", Network::BSV_Testnet).unwrap(),
            generate_wif("pw", "nonce", Network::BSV_Testnet).unwrap()
        );
        assert_ne!(
            generate_wif("pw", "nonce", Network::BSV_Testnet).unwrap(),
            generate_wif("pw", "other", Network::BSV_Testnet).unwrap()
        );
    }

    #[test]
    fn generate_wif_rejects_networks_with_no_prefix() {
        // Previously these produced a mainnet key without complaint
        for net in [
            Network::BSV_STN,
            Network::BTC_Mainnet,
            Network::BTC_Testnet,
            Network::BCH_Mainnet,
            Network::BCH_Testnet,
        ] {
            assert!(
                generate_wif("pw", "nonce", net).is_err(),
                "{net} should be rejected rather than defaulted"
            );
        }
    }

    #[test]
    fn the_two_wif_constructors_agree() {
        // Both go through wif_prefix, so they cannot drift on what a network means
        for net in [
            Network::BSV_Mainnet,
            Network::BSV_Testnet,
            Network::BSV_Regtest,
        ] {
            let from_password = generate_wif("pw", "nonce", net).unwrap();
            let key_bytes = wif_to_bytes(&from_password).unwrap();
            let signing = SigningKey::from_slice(&key_bytes).unwrap();
            assert_eq!(
                network_and_private_key_to_wif(net, signing).unwrap(),
                from_password,
                "{net} encodes differently through the two constructors"
            );
        }
    }

    #[test]
    fn regtest_wif_matches_testnet() {
        // Regtest shares testnet's WIF version byte, as it does for addresses.
        // Before the match arms were made explicit, regtest fell into the
        // catch-all and this returned an error instead.
        let key = SecretKey::from_slice(&[7u8; 32]).unwrap();
        let signing = SigningKey::from(key);
        let testnet =
            network_and_private_key_to_wif(Network::BSV_Testnet, signing.clone()).unwrap();
        let regtest =
            network_and_private_key_to_wif(Network::BSV_Regtest, signing.clone()).unwrap();
        assert_eq!(regtest, testnet);
        assert_ne!(
            regtest,
            network_and_private_key_to_wif(Network::BSV_Mainnet, signing).unwrap()
        );
    }

    #[test]
    fn networks_without_a_wif_prefix_are_rejected() {
        let signing = SigningKey::from(SecretKey::from_slice(&[7u8; 32]).unwrap());
        for net in [Network::BSV_STN, Network::BTC_Mainnet, Network::BCH_Testnet] {
            assert!(
                network_and_private_key_to_wif(net, signing.clone()).is_err(),
                "{net} should be rejected"
            );
        }
    }

    fn bytes_to_hexstr(bytes: &[u8]) -> String {
        bytes
            .iter()
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
