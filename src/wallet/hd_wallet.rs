//! BIP-32 hierarchical deterministic wallet helpers.

use crate::network::Network;
use crate::script::Script;
use crate::util::ChainGangError;
use crate::wallet::extended_key::{
    derive_extended_key, master_extended_key_from_seed, ExtendedKey, ExtendedKeyType,
};
use crate::wallet::mnemonic::mnemonic_to_seed_validated;
use crate::wallet::wallet::Wallet;

/// BSV mainnet coin type per SLIP-44.
pub const BSV_COIN_TYPE: u32 = 236;

/// Builds a BIP-32 account path: `m/{account}'/{change}/{index}`.
///
/// `change` is `0` for external (receive) keys and `1` for internal (change) keys.
pub fn bip32_path(account: u32, change: u32, index: u32) -> String {
    format!("m/{}'/{}/{}", account, change, index)
}

/// Builds a BIP-44 style path: `m/44'/{coin_type}'/{account}'/{change}/{index}`.
pub fn bip44_path(coin_type: u32, account: u32, external: bool, index: u32) -> String {
    let change = if external { 0 } else { 1 };
    format!("m/44'/{}'/{}'/{}/{}", coin_type, account, change, index)
}

/// HD wallet rooted at a BIP-32 master extended **private** key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdWallet {
    master: ExtendedKey,
}

impl HdWallet {
    /// Creates an HD wallet from a master extended private key.
    pub fn from_master(master: ExtendedKey) -> Result<Self, ChainGangError> {
        if master.key_type()? != ExtendedKeyType::Private {
            return Err(ChainGangError::BadArgument(
                "HdWallet requires a master extended private key".to_string(),
            ));
        }
        Ok(HdWallet { master })
    }

    /// Creates an HD wallet from a BIP-32 seed.
    pub fn from_seed(network: Network, seed: &[u8]) -> Result<Self, ChainGangError> {
        let master = master_extended_key_from_seed(network, seed)?;
        Ok(HdWallet { master })
    }

    /// Creates an HD wallet from a validated BIP-39 mnemonic (English word list).
    pub fn from_mnemonic(
        network: Network,
        mnemonic: &str,
        passphrase: &str,
        word_list: &[String],
    ) -> Result<Self, ChainGangError> {
        let seed = mnemonic_to_seed_validated(mnemonic, passphrase, word_list)?;
        Self::from_seed(network, &seed)
    }

    /// Master extended private key (`m`).
    pub fn master(&self) -> ExtendedKey {
        self.master
    }

    /// Network encoded in the master extended key.
    pub fn network(&self) -> Result<Network, ChainGangError> {
        self.master.network()
    }

    /// Derives an extended key along a BIP-32 path (`m/...` or `M/...`).
    pub fn derive_path(&self, path: &str) -> Result<ExtendedKey, ChainGangError> {
        derive_extended_key(&self.master, path)
    }

    /// Derives a signing [`Wallet`] at a private BIP-32 path.
    pub fn wallet_at_path(&self, path: &str) -> Result<Wallet, ChainGangError> {
        Wallet::from_extended_key(&self.derive_path(path)?)
    }

    /// P2PKH address at `m/{account}'/{change}/{index}`.
    pub fn address_at(&self, account: u32, external: bool, index: u32) -> Result<String, ChainGangError> {
        let change = if external { 0 } else { 1 };
        self.wallet_at_path(&bip32_path(account, change, index))?.get_address()
    }

    /// Locking script at `m/{account}'/{change}/{index}`.
    pub fn locking_script_at(
        &self,
        account: u32,
        external: bool,
        index: u32,
    ) -> Result<Script, ChainGangError> {
        let change = if external { 0 } else { 1 };
        Ok(self
            .wallet_at_path(&bip32_path(account, change, index))?
            .get_locking_script())
    }

    /// P2PKH address at a BIP-44 path for the given coin type.
    pub fn address_at_bip44(
        &self,
        coin_type: u32,
        account: u32,
        external: bool,
        index: u32,
    ) -> Result<String, ChainGangError> {
        self.wallet_at_path(&bip44_path(coin_type, account, external, index))?.get_address()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{
        OutPoint, Tx, TxIn, TxOut, COINBASE_OUTPOINT_HASH, COINBASE_OUTPOINT_INDEX,
    };
    use crate::transaction::sighash::{SIGHASH_ALL, SIGHASH_FORKID};
    use hex;

    fn test_seed() -> Vec<u8> {
        hex::decode("000102030405060708090a0b0c0d0e0f").unwrap()
    }

    #[test]
    fn from_mnemonic_matches_from_seed() {
        let wordlist = crate::wallet::load_wordlist(crate::wallet::Wordlist::English);
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let seed = crate::wallet::mnemonic_to_seed_validated(mnemonic, "", &wordlist).unwrap();
        let from_seed = HdWallet::from_seed(Network::BSV_Mainnet, &seed).unwrap();
        let from_mnemonic =
            HdWallet::from_mnemonic(Network::BSV_Mainnet, mnemonic, "", &wordlist).unwrap();
        assert_eq!(from_seed.master(), from_mnemonic.master());
    }

    #[test]
    fn wallet_at_path_matches_extended_private_key() {
        let hd = HdWallet::from_seed(Network::BSV_Mainnet, &test_seed()).unwrap();
        let path = bip32_path(0, 0, 5);
        let wallet = hd.wallet_at_path(&path).unwrap();
        let key = hd.derive_path(&path).unwrap();
        assert_eq!(wallet.public_key_serialize(), key.public_key().unwrap());
    }

    #[test]
    fn bip44_path_derives_distinct_addresses() {
        let hd = HdWallet::from_seed(Network::BSV_Mainnet, &test_seed()).unwrap();
        let a0 = hd.address_at_bip44(BSV_COIN_TYPE, 0, true, 0).unwrap();
        let a1 = hd.address_at_bip44(BSV_COIN_TYPE, 0, true, 1).unwrap();
        assert_ne!(a0, a1);
    }

    #[test]
    fn wallet_at_path_signs_p2pkh_spend() {
        let hd = HdWallet::from_seed(Network::BSV_Mainnet, &test_seed()).unwrap();
        let wallet = hd.wallet_at_path(&bip32_path(0, 0, 0)).unwrap();
        let lock = wallet.get_locking_script();

        let fund = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: COINBASE_OUTPOINT_HASH,
                    index: COINBASE_OUTPOINT_INDEX,
                },
                unlock_script: Script::new(),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 10_000,
                lock_script: lock,
            }],
            lock_time: 0,
        };

        let mut spend = Tx {
            version: 1,
            inputs: vec![TxIn {
                prev_output: OutPoint {
                    hash: fund.hash(),
                    index: 0,
                },
                unlock_script: Script::new(),
                sequence: 0xffffffff,
            }],
            outputs: vec![TxOut {
                satoshis: 9_000,
                lock_script: Script::new(),
            }],
            lock_time: 0,
        };

        let sighash_flags = SIGHASH_ALL | SIGHASH_FORKID;
        wallet
            .sign_tx_input(&fund, &mut spend, 0, sighash_flags)
            .unwrap();
        assert!(!spend.inputs[0].unlock_script.0.is_empty());
    }
}
