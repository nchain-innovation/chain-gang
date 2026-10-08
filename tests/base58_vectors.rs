//! bitcoin-sv's base58 vectors: raw base58, addresses and WIF private keys.
//!
//! The three files in `tests/data` are the node's own (see
//! `tests/data/README.md`), and these tests follow the node's
//! `base58_tests.cpp`:
//!
//! - `base58_encode_decode.json`: raw base58 both ways. chain-gang hands raw
//!   base58 to the `bs58` crate, so this pins that dependency's behaviour,
//!   leading zero bytes included.
//! - `base58_keys_valid.json`: addresses (P2PKH and P2SH, mainnet and testnet)
//!   decode to their hash and type and encode back, and WIF private keys
//!   decode to their key and network. A private key must not also read as an
//!   address.
//! - `base58_keys_invalid.json`: none of these is a mainnet address or a
//!   mainnet private key. The node runs that test under mainnet parameters
//!   only, so that is all it claims.

use chain_gang::address::{addr_decode, addr_encode, AddressType};
use chain_gang::network::Network;
use chain_gang::util::Hash160;
use chain_gang::wallet::wallet::wif_to_network_and_private_key;
use std::path::PathBuf;

/// Set to `1` to skip when the vendored data is absent, as it is in a crate
/// unpacked from crates.io. See `src/transaction/sighash_vectors.rs`.
const OPTIONAL_ENV: &str = "CHAIN_GANG_VECTORS_OPTIONAL";

/// The rows of a vendored file, or None where the data is absent and that is
/// allowed.
fn load(name: &str) -> Option<Vec<Vec<serde_json::Value>>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join(name);
    if !path.exists() {
        assert!(
            std::env::var(OPTIONAL_ENV).as_deref() == Ok("1"),
            "{} is missing; set {OPTIONAL_ENV}=1 to skip vendored vectors",
            path.display()
        );
        return None;
    }
    Some(serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap())
}

fn network(is_testnet: bool) -> Network {
    if is_testnet {
        Network::BSV_Testnet
    } else {
        Network::BSV_Mainnet
    }
}

#[test]
fn base58_encode_decode() {
    let Some(rows) = load("base58_encode_decode.json") else {
        return;
    };
    assert_eq!(rows.len(), 12);
    for row in &rows {
        let (hex_data, encoded) = (row[0].as_str().unwrap(), row[1].as_str().unwrap());
        let data = hex::decode(hex_data).unwrap();
        assert_eq!(
            bs58::encode(&data).into_string(),
            encoded,
            "encode {hex_data}"
        );
        assert_eq!(
            bs58::decode(encoded).into_vec().unwrap(),
            data,
            "decode {encoded:?}"
        );
    }
    assert!(bs58::decode("invalid").into_vec().is_err());
}

#[test]
fn base58_keys_valid() {
    let Some(rows) = load("base58_keys_valid.json") else {
        return;
    };
    assert_eq!(rows.len(), 50);
    let mut failures = Vec::new();
    for row in &rows {
        let encoded = row[0].as_str().unwrap();
        let payload = hex::decode(row[1].as_str().unwrap()).unwrap();
        let meta = &row[2];
        let net = network(meta["isTestnet"].as_bool().unwrap());

        if meta["isPrivkey"].as_bool().unwrap() {
            match wif_to_network_and_private_key(encoded) {
                Ok((got_net, key)) => {
                    if got_net != net {
                        failures.push(format!("{encoded}: network {got_net}, expected {net}"));
                    }
                    if key.to_bytes().as_slice() != payload.as_slice() {
                        failures.push(format!("{encoded}: wrong key"));
                    }
                }
                Err(e) => failures.push(format!("{encoded}: private key rejected: {e}")),
            }
            if addr_decode(encoded, net).is_ok() {
                failures.push(format!("{encoded}: private key also reads as an address"));
            }
        } else {
            let addr_type = match meta["addrType"].as_str().unwrap() {
                "pubkey" => AddressType::P2PKH,
                "script" => AddressType::P2SH,
                other => panic!("unexpected addrType {other}"),
            };
            let hash = Hash160(payload.clone().try_into().expect("20-byte payload"));
            match addr_decode(encoded, net) {
                Ok((got_hash, got_type)) => {
                    if got_hash != hash || got_type != addr_type {
                        failures.push(format!(
                            "{encoded}: decoded to {got_type:?} {}",
                            hex::encode(got_hash.0)
                        ));
                    }
                }
                Err(e) => failures.push(format!("{encoded}: address rejected: {e}")),
            }
            let reencoded = addr_encode(&hash, addr_type, net);
            if reencoded != encoded {
                failures.push(format!("{encoded}: encodes as {reencoded}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn base58_keys_invalid() {
    let Some(rows) = load("base58_keys_invalid.json") else {
        return;
    };
    assert_eq!(rows.len(), 50);
    let mut failures = Vec::new();
    for row in &rows {
        let encoded = row[0].as_str().unwrap();
        if let Ok(decoded) = addr_decode(encoded, Network::BSV_Mainnet) {
            failures.push(format!("{encoded:?}: accepted as an address: {decoded:?}"));
        }
        if let Ok((net, _)) = wif_to_network_and_private_key(encoded) {
            if net == Network::BSV_Mainnet {
                failures.push(format!("{encoded:?}: accepted as a mainnet private key"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
