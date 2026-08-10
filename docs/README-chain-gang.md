# Chain-Gang

This is a Rust library that enables interacting with Bitcoin derived blockchains.

This library currently supports the following blockchains:

| Name | Code | Networks |
| --- | --- | --- |
| Bitcoin SV | `BSV` | BSV_Mainnet, BSV_Testnet, BSV_STN |
| Bitcoin  | `BTC` | BTC_Mainnet, BTC_Testnet |
| Bitcoin Cash | `BCH` | BCH_Mainnet, BCH_Testnet |


Features (all blockchains)
* P2P protocol messages (construction and serialization)
* Address encoding and decoding
* Node connections and basic message handling
* Mainnet and testnet support

BSV only Features
* Transaction signing 
* Script evaluation 
* Wallet key derivation, BIP-32 HD wallets (`HdWallet`), and mnemonic parsing
* Various Bitcoin primitives
* Genesis upgrade support
* [Chronicle upgrade](https://github.com/nchain-innovation/chain-gang/blob/main/docs/Chronicle.md) (OTDA sighash, opcodes, two-phase eval, `tx.version > 1` rules)

`Chain-gang` is based on `Rust-SV` An open source library to build Bitcoin SV applications and infrastructure in Rust. The documentation for `Rust-SV` can be found here: 
[Rust-SV Documentation](https://docs.rs/sv/)


# Installation

To call the library from a Rust project add the following line to Cargo.toml:
```toml
chain-gang = "0.10"
```

## Feature Flags

The `chain-gang` library uses the following feature flags:

* `interface` - this provides a blockchain interface for accessing the current blockchain status, via servers such as WhatsOnChain.
* `python` - this provides a Python interface to the `chain_gang` library.

To build the library with the `interface` feature
```bash
cargo build --features "interface"
```

To build the library with the `python` feature
```bash
cargo build --features "python"
```
For more details of the `python` feature see [python/README.md](https://github.com/nchain-innovation/chain-gang/blob/main/python/README.md). Full documentation index: [docs/README.md](https://github.com/nchain-innovation/chain-gang/blob/main/docs/README.md). HD wallets: [BIP-32.md](https://github.com/nchain-innovation/chain-gang/blob/main/docs/BIP-32.md).

## HD wallets (BIP-32)

```rust,ignore
use chain_gang::network::Network;
use chain_gang::wallet::{HdWallet, bip44_path, BSV_COIN_TYPE};

let hd = HdWallet::from_seed(Network::BSV_Mainnet, &seed)?;
let addr = hd.address_at_bip44(BSV_COIN_TYPE, 0, true, 0)?;
```

See [BIP-32.md](https://github.com/nchain-innovation/chain-gang/blob/main/docs/BIP-32.md) for mnemonics, watch-only `xpub`, and gap-limit scanning.

# Known limitations

This library should not be used for consensus code because its validation checks are incomplete.

# License

`chain-gang` is licensed under the [MIT license](https://github.com/nchain-innovation/chain-gang/blob/main/LICENSE).

It is based on `Rust-SV`, which is also licensed under the MIT license (see [LICENSE-rust-sv](https://github.com/nchain-innovation/chain-gang/blob/main/LICENSE-rust-sv)).

