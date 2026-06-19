//! BIP-32 watch-only (xpub) wallet helpers.

use crate::network::Network;
use crate::script::Script;
use crate::util::ChainGangError;
use crate::wallet::extended_key::{
    derive_extended_key, ExtendedKey, ExtendedKeyType,
};
use crate::wallet::wallet::{p2pkh_script, public_key_to_address};

/// Default BIP-44 gap limit for address discovery.
pub const DEFAULT_GAP_LIMIT: u32 = 20;

/// Builds a relative BIP-32 path from an account-level extended public key: `M/{change}/{index}`.
pub fn watch_bip32_path(change: u32, index: u32) -> String {
    format!("M/{}/{}", change, index)
}

/// Builds a relative BIP-44 path from an account-level extended public key.
pub fn watch_bip44_path(external: bool, index: u32) -> String {
    let change = if external { 0 } else { 1 };
    watch_bip32_path(change, index)
}

/// Watch-only HD wallet rooted at an extended **public** key (typically account-level `xpub`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdWatchWallet {
    master: ExtendedKey,
}

impl HdWatchWallet {
    /// Creates a watch-only wallet from an encoded extended public key (`xpub` / `tpub`).
    pub fn from_xpub(xpub: &str) -> Result<Self, ChainGangError> {
        Self::from_master(ExtendedKey::decode(xpub)?)
    }

    /// Creates a watch-only wallet from a master extended public key.
    pub fn from_master(master: ExtendedKey) -> Result<Self, ChainGangError> {
        if master.key_type()? != ExtendedKeyType::Public {
            return Err(ChainGangError::BadArgument(
                "HdWatchWallet requires an extended public key".to_string(),
            ));
        }
        Ok(HdWatchWallet { master })
    }

    pub fn master(&self) -> ExtendedKey {
        self.master
    }

    pub fn network(&self) -> Result<Network, ChainGangError> {
        self.master.network()
    }

    /// Derives an extended public key along a BIP-32 path (`M/...`).
    pub fn derive_path(&self, path: &str) -> Result<ExtendedKey, ChainGangError> {
        derive_extended_key(&self.master, path)
    }

    /// Compressed public key bytes at `path`.
    pub fn public_key_at_path(&self, path: &str) -> Result<Vec<u8>, ChainGangError> {
        Ok(self.derive_path(path)?.public_key()?.to_vec())
    }

    /// P2PKH address at `path`.
    pub fn address_at_path(&self, path: &str) -> Result<String, ChainGangError> {
        let pk = self.public_key_at_path(path)?;
        public_key_to_address(&pk, self.network()?)
    }

    /// P2PKH locking script at `path`.
    pub fn locking_script_at_path(&self, path: &str) -> Result<Script, ChainGangError> {
        let pk = self.public_key_at_path(path)?;
        Ok(p2pkh_script(&crate::util::hash160(&pk).0))
    }

    /// P2PKH address at `M/{change}/{index}` relative to an account-level `xpub`.
    pub fn address_at(&self, external: bool, index: u32) -> Result<String, ChainGangError> {
        let change = if external { 0 } else { 1 };
        self.address_at_path(&watch_bip32_path(change, index))
    }

    /// P2PKH address at a relative BIP-44 receive/change chain index.
    pub fn address_at_bip44(&self, external: bool, index: u32) -> Result<String, ChainGangError> {
        self.address_at_path(&watch_bip44_path(external, index))
    }

    /// Scans external or change chain indices until `gap_limit` consecutive unused addresses.
    pub fn scan_addresses<F>(
        &self,
        external: bool,
        gap_limit: u32,
        is_used: F,
    ) -> Result<Vec<String>, ChainGangError>
    where
        F: Fn(&str) -> bool,
    {
        scan_address_indices(|i| self.address_at(external, i), gap_limit, is_used)
    }
}

/// Scans address indices until `gap_limit` consecutive unused addresses.
pub fn scan_address_indices<G, H>(
    address_at: G,
    gap_limit: u32,
    is_used: H,
) -> Result<Vec<String>, ChainGangError>
where
    G: Fn(u32) -> Result<String, ChainGangError>,
    H: Fn(&str) -> bool,
{
    let mut used_addresses = Vec::new();
    let mut gap = 0u32;
    let mut index = 0u32;

    while gap < gap_limit {
        let addr = address_at(index)?;
        if is_used(&addr) {
            used_addresses.push(addr);
            gap = 0;
        } else {
            gap += 1;
        }
        index += 1;
    }

    Ok(used_addresses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::hd_wallet::{bip32_path, HdWallet};
    use hex;

    fn test_seed() -> Vec<u8> {
        hex::decode("000102030405060708090a0b0c0d0e0f").unwrap()
    }

    #[test]
    fn watch_wallet_matches_private_derivation() {
        let hd = HdWallet::from_seed(Network::BSV_Mainnet, &test_seed()).unwrap();
        let account_xpub = hd
            .derive_path("m/0'")
            .unwrap()
            .extended_public_key()
            .unwrap()
            .encode();
        let watch = HdWatchWallet::from_xpub(&account_xpub).unwrap();

        let priv_addr = hd.address_at(0, true, 5).unwrap();
        let watch_addr = watch.address_at(true, 5).unwrap();
        assert_eq!(priv_addr, watch_addr);
    }

    #[test]
    fn watch_path_matches_full_bip32_path() {
        let hd = HdWallet::from_seed(Network::BSV_Mainnet, &test_seed()).unwrap();
        let account_xpub = hd
            .derive_path("m/0'")
            .unwrap()
            .extended_public_key()
            .unwrap()
            .encode();
        let watch = HdWatchWallet::from_xpub(&account_xpub).unwrap();

        let path = bip32_path(0, 0, 3);
        let full_addr = hd.wallet_at_path(&path).unwrap().get_address().unwrap();
        let watch_addr = watch.address_at_path(&watch_bip32_path(0, 3)).unwrap();
        assert_eq!(full_addr, watch_addr);
    }

    #[test]
    fn gap_scan_collects_used_addresses() {
        let hd = HdWallet::from_seed(Network::BSV_Mainnet, &test_seed()).unwrap();
        let account_xpub = hd
            .derive_path("m/0'")
            .unwrap()
            .extended_public_key()
            .unwrap()
            .encode();
        let watch = HdWatchWallet::from_xpub(&account_xpub).unwrap();

        let a0 = watch.address_at(true, 0).unwrap();
        let a2 = watch.address_at(true, 2).unwrap();
        let used = watch
            .scan_addresses(true, 2, |addr| addr == &a0 || addr == &a2)
            .unwrap();
        assert_eq!(used, vec![a0, a2]);
    }
}
