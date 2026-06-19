# BIP-32 HD wallets

Guide for hierarchical deterministic (HD) wallets in **chain-gang** (Rust) and **tx-engine** (Python). Implements [BIP-32](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki) key derivation, [BIP-39](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki) mnemonics, and [BIP-44](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki) style paths. BSV mainnet coin type **236** ([SLIP-44](https://github.com/satoshilabs/slips/blob/master/slip-0044.md)).

Python class reference: [Python-API.md](Python-API.md#hdwallet). Rust API: `chain_gang::wallet`.

## Concepts

| Piece | Role |
|-------|------|
| Seed | 64-byte output of BIP-39 PBKDF2 (or any ≥16-byte seed for BIP-32 master) |
| `xprv` / `xpub` | Base58 extended private / public keys |
| Path `m/...` | Private derivation from master `xprv` (hardened steps allowed) |
| Path `M/...` | Public derivation from `xpub` (no hardened children) |
| `HdWallet` | Full wallet — derive signing keys and addresses |
| `HdWatchWallet` | Watch-only from account-level `xpub` — addresses only |

Invalid BIP-32 child indices are skipped automatically (next index is tried per the spec).

## Rust

```rust
use chain_gang::network::Network;
use chain_gang::wallet::{
    bip32_path, bip44_path, load_wordlist, master_extended_key_from_seed, BSV_COIN_TYPE,
    HdWallet, HdWatchWallet, Wordlist,
};

// From BIP-39 mnemonic (English word list)
let wordlist = load_wordlist(Wordlist::English);
let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
let hd = HdWallet::from_mnemonic(Network::BSV_Mainnet, mnemonic, "", &wordlist)?;

// Or from seed bytes
let seed = chain_gang::wallet::mnemonic_to_seed_validated(mnemonic, "", &wordlist)?;
let hd = HdWallet::from_seed(Network::BSV_Mainnet, &seed)?;

// BIP-44 receive address (m/44'/236'/0'/0/0)
let addr = hd.address_at_bip44(BSV_COIN_TYPE, 0, true, 0)?;

// Leaf signing wallet at m/0'/0/0
let wallet = hd.wallet_at_path(&bip32_path(0, 0, 0))?;

// Watch-only from account xpub (m/0')
let account_xpub = hd.derive_path("m/0'")?.extended_public_key()?.encode();
let watch = HdWatchWallet::from_xpub(&account_xpub)?;
let same_addr = watch.address_at(true, 0)?;

// Gap-limit scan (e.g. 20 unused indices after last used)
let used = watch.scan_addresses(true, 20, |a| a == &addr)?;
```

Core helpers without `HdWallet`:

```rust
let master = master_extended_key_from_seed(Network::BSV_Mainnet, &seed)?;
let child = chain_gang::wallet::derive_extended_key(&master, "m/0'/0/0")?;
```

## Python

```python
from tx_engine import (
    HdWallet,
    HdWatchWallet,
    bip44_path,
    bsv_coin_type,
    mnemonic_to_seed,
)

mnemonic = (
    "abandon abandon abandon abandon abandon abandon abandon "
    "abandon abandon abandon abandon about"
)

hd = HdWallet.from_mnemonic("BSV_Mainnet", mnemonic)
addr = hd.address_at_bip44(bsv_coin_type(), 0, True, 0)
wallet = hd.wallet_at_path(bip44_path(bsv_coin_type(), 0, True, 0))

# Watch-only from account xpub
account_xpub = hd.derive_xpub("m/0'")
watch = HdWatchWallet.from_xpub(account_xpub)
assert watch.address_at(True, 0) == addr

# Gap scan: callable returns True if address was used on-chain
used = watch.scan_addresses(True, 20, lambda a: a in my_known_addresses)
```

## Path helpers

| Rust | Python | Example output |
|------|--------|----------------|
| `bip32_path(0, 0, 5)` | `bip32_path(0, 0, 5)` | `m/0'/0/5` |
| `bip44_path(236, 0, true, 5)` | `bip44_path(bsv_coin_type(), 0, True, 5)` | `m/44'/236'/0'/0/5` |
| `watch_bip32_path(0, 5)` | `watch_bip32_path(0, 5)` | `M/0/5` (from account `xpub`) |
| `watch_bip44_path(true, 5)` | `watch_bip44_path(True, 5)` | `M/0/5` |

## See also

- [Python-API.md](Python-API.md) — `Wallet`, `HdWallet`, `HdWatchWallet`
- [README-chain-gang.md](README-chain-gang.md) — crate overview
- BIP-32 test vectors 1–3 are covered in `extended_key` unit tests
