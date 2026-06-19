# BIP-32 (Python)

Short Python-focused guide for HD wallets in **tx-engine**. Full spec and Rust examples: [BIP-32.md](BIP-32.md). Class reference: [Python-API.md](Python-API.md#hdwallet).

## Mnemonic → addresses

```python
from tx_engine import HdWallet, bip44_path, bsv_coin_type

mnemonic = (
    "abandon abandon abandon abandon abandon abandon abandon "
    "abandon abandon abandon abandon about"
)

hd = HdWallet.from_mnemonic("BSV_Mainnet", mnemonic, passphrase="")

# First BIP-44 external address (m/44'/236'/0'/0/0)
print(hd.address_at_bip44(bsv_coin_type(), 0, True, 0))

# Signing wallet at that path
wallet = hd.wallet_at_path(bip44_path(bsv_coin_type(), 0, True, 0))
print(wallet.get_address())
```

## Watch-only `xpub`

Export the account `xpub` from a full wallet, then derive addresses without private keys:

```python
from tx_engine import HdWallet, HdWatchWallet

hd = HdWallet.from_mnemonic("BSV_Mainnet", mnemonic)
account_xpub = hd.derive_xpub("m/0'")

watch = HdWatchWallet.from_xpub(account_xpub)
print(watch.address_at(True, 0))   # M/0/0 relative to account xpub
print(watch.derive_xpub("M/0/1"))  # extended public key at child path
```

## Gap-limit discovery

Scan receive indices until `gap_limit` consecutive unused addresses (typical value: 20):

```python
known_on_chain = {"1A...", "1B..."}

used = watch.scan_addresses(
    True,   # external (receive) chain
    20,     # gap limit
    lambda addr: addr in known_on_chain,
)
```

Full wallets can scan per account:

```python
used = hd.scan_external_addresses(account=0, gap_limit=20, is_used=lambda a: a in known_on_chain)
```

## Module helpers

| Function | Purpose |
|----------|---------|
| `mnemonic_to_seed(mnemonic, passphrase)` | 64-byte seed (does not validate words) |
| `derive_extended_key(xprv, path)` | Derive `xprv` along `m/...` path |
| `bip32_path`, `bip44_path` | Build standard paths |
| `watch_bip32_path`, `watch_bip44_path` | Paths relative to account `xpub` |
| `bsv_coin_type()` | Returns `236` |
