use crate::{
    network::Network,
    python::py_wallet::{str_to_network, PyWallet},
    util::ChainGangError,
    wallet::{
        bip32_path, bip44_path, derive_extended_key, load_wordlist, mnemonic_to_seed, watch_bip32_path,
        watch_bip44_path, Wordlist, BSV_COIN_TYPE, ExtendedKey, HdWallet, HdWatchWallet,
    },
};

use pyo3::prelude::*;
use pyo3::types::PyType;

#[pyclass(name = "HdWallet")]
pub struct PyHdWallet {
    inner: HdWallet,
}

#[pymethods]
impl PyHdWallet {
  #[classmethod]
  fn from_seed(_cls: &Bound<'_, PyType>, network: &str, seed: &[u8]) -> PyResult<Self> {
    let net = parse_network(network)?;
    Ok(PyHdWallet {
      inner: HdWallet::from_seed(net, seed)?,
    })
  }

  #[classmethod]
  #[pyo3(signature = (network, mnemonic, passphrase=None))]
  fn from_mnemonic(
    _cls: &Bound<'_, PyType>,
    network: &str,
    mnemonic: &str,
    passphrase: Option<&str>,
  ) -> PyResult<Self> {
    let net = parse_network(network)?;
    let wordlist = load_wordlist(Wordlist::English);
    let pass = passphrase.unwrap_or("");
    Ok(PyHdWallet {
      inner: HdWallet::from_mnemonic(net, mnemonic, pass, &wordlist)?,
    })
  }

  #[classmethod]
  fn from_xprv(_cls: &Bound<'_, PyType>, xprv: &str) -> PyResult<Self> {
    let master = ExtendedKey::decode(xprv)?;
    Ok(PyHdWallet {
      inner: HdWallet::from_master(master)?,
    })
  }

  fn master_xprv(&self) -> PyResult<String> {
    Ok(self.inner.master().encode())
  }

  fn master_xpub(&self) -> PyResult<String> {
    Ok(self.inner.master().extended_public_key()?.encode())
  }

  fn address_at(&self, account: u32, external: bool, index: u32) -> PyResult<String> {
    Ok(self.inner.address_at(account, external, index)?)
  }

  fn address_at_bip44(
    &self,
    coin_type: u32,
    account: u32,
    external: bool,
    index: u32,
  ) -> PyResult<String> {
    Ok(self.inner.address_at_bip44(coin_type, account, external, index)?)
  }

  fn wallet_at_path(&self, path: &str) -> PyResult<PyWallet> {
    Ok(PyWallet::from_wallet(self.inner.wallet_at_path(path)?))
  }

  fn derive_xprv(&self, path: &str) -> PyResult<String> {
    Ok(self.inner.derive_path(path)?.encode())
  }

  fn derive_xpub(&self, path: &str) -> PyResult<String> {
    Ok(self
      .inner
      .derive_path(path)?
      .extended_public_key()?
      .encode())
  }

  fn scan_external_addresses(
    &self,
    account: u32,
    gap_limit: u32,
    is_used: &Bound<'_, PyAny>,
  ) -> PyResult<Vec<String>> {
    Ok(self
      .inner
      .scan_external_addresses(account, gap_limit, |addr| call_is_used(is_used, addr))?)
  }
}

#[pyclass(name = "HdWatchWallet")]
pub struct PyHdWatchWallet {
  inner: HdWatchWallet,
}

#[pymethods]
impl PyHdWatchWallet {
  #[classmethod]
  fn from_xpub(_cls: &Bound<'_, PyType>, xpub: &str) -> PyResult<Self> {
    Ok(PyHdWatchWallet {
      inner: HdWatchWallet::from_xpub(xpub)?,
    })
  }

  fn master_xpub(&self) -> PyResult<String> {
    Ok(self.inner.master().encode())
  }

  fn address_at(&self, external: bool, index: u32) -> PyResult<String> {
    Ok(self.inner.address_at(external, index)?)
  }

  fn address_at_bip44(&self, external: bool, index: u32) -> PyResult<String> {
    Ok(self.inner.address_at_bip44(external, index)?)
  }

  fn address_at_path(&self, path: &str) -> PyResult<String> {
    Ok(self.inner.address_at_path(path)?)
  }

  fn derive_xpub(&self, path: &str) -> PyResult<String> {
    Ok(self.inner.derive_path(path)?.encode())
  }

  fn scan_addresses(
    &self,
    external: bool,
    gap_limit: u32,
    is_used: &Bound<'_, PyAny>,
  ) -> PyResult<Vec<String>> {
    Ok(self
      .inner
      .scan_addresses(external, gap_limit, |addr| call_is_used(is_used, addr))?)
  }
}

fn call_is_used(is_used: &Bound<'_, PyAny>, addr: &str) -> bool {
  is_used
    .call1((addr,))
    .and_then(|v| v.extract::<bool>())
    .unwrap_or(false)
}

fn parse_network(network: &str) -> PyResult<Network> {
  str_to_network(network).ok_or_else(|| {
    ChainGangError::BadArgument(format!("Unknown network {}", network)).into()
  })
}

#[pyfunction(name = "mnemonic_to_seed")]
pub fn py_mnemonic_to_seed(mnemonic: &str, passphrase: &str) -> Vec<u8> {
  mnemonic_to_seed(mnemonic, passphrase).to_vec()
}

#[pyfunction(name = "derive_extended_key")]
pub fn py_derive_extended_key(xprv: &str, path: &str) -> PyResult<String> {
  let master = ExtendedKey::decode(xprv)?;
  Ok(derive_extended_key(&master, path)?.encode())
}

#[pyfunction(name = "bip32_path")]
pub fn py_bip32_path(account: u32, change: u32, index: u32) -> String {
  bip32_path(account, change, index)
}

#[pyfunction(name = "bip44_path")]
pub fn py_bip44_path(coin_type: u32, account: u32, external: bool, index: u32) -> String {
  bip44_path(coin_type, account, external, index)
}

#[pyfunction(name = "bsv_coin_type")]
pub fn py_bsv_coin_type() -> u32 {
  BSV_COIN_TYPE
}

#[pyfunction(name = "watch_bip32_path")]
pub fn py_watch_bip32_path(change: u32, index: u32) -> String {
  watch_bip32_path(change, index)
}

#[pyfunction(name = "watch_bip44_path")]
pub fn py_watch_bip44_path(external: bool, index: u32) -> String {
  watch_bip44_path(external, index)
}
