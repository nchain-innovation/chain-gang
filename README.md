# Chain-Gang

This is a Rust library that enables monitoring of the P2P messages of Bitcoin derived blockchains.

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
* Wallet key derivation and mnemonic parsing
* Various Bitcoin primitives
* Genesis upgrade support

`Chain-gang` is based on `Rust-SV` An open source library to build Bitcoin SV applications and infrastructure in Rust. The documentation for `Rust-SV` can be found here: 
[Rust-SV Documentation](https://docs.rs/sv/)


# Installation

To call the library from a Rust project add the following line to to Cargo.toml:
```toml
chain-gang = { path = "../chain-gang" }
``` 

## Feature Flags

The library uses the following feature flag
* `interface` - this provides a blockchain interface for accessing the current blockchain status, via servers such as WhatsOnChain.
* `python` - this provides a python interface to the chain_gang library.

Therefore to build the library with the `interface` feature
```bash
cargo build --features "interface"
```

Therefore to build the library with the `python` feature
```bash
cargo build --features "python"
```



# Known limitations

This library should not be used for consensus code because its validation checks are incomplete.

# License

rust-sv and therefore chain-gang is licensed under the MIT license.


