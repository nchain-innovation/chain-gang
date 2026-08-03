//! Live integration tests for `RpcInterface` against a regtest bitcoind/BSV node.
//!
//! These are opt-in and ignored by default. Provide connection details via
//! environment variables and run explicitly:
//!
//! ```bash
//! export RPC_ADDRESS=127.0.0.1:18332
//! export RPC_USER=bitcoin
//! export RPC_PASSWORD=bitcoin
//! cargo test --features interface --test rpc_integration -- --ignored
//! ```
//!
//! Only read-only calls are exercised, so they are safe to run against any
//! regtest node (the genesis block at height 0 is sufficient).

#![cfg(feature = "interface")]

use chain_gang::interface::{BlockchainInterface, RpcInterface};
use chain_gang::network::Network;

/// Build an interface from env vars, or return `None` (skip) if unset.
fn interface_from_env() -> Option<RpcInterface> {
    let address = std::env::var("RPC_ADDRESS").ok()?;
    let user = std::env::var("RPC_USER").ok()?;
    let password = std::env::var("RPC_PASSWORD").ok()?;
    Some(RpcInterface::new(&address, &user, &password, Network::BSV_Testnet).unwrap())
}

macro_rules! skip_if_unconfigured {
    () => {
        match interface_from_env() {
            Some(iface) => iface,
            None => {
                eprintln!("skipping: set RPC_ADDRESS/RPC_USER/RPC_PASSWORD to run");
                return;
            }
        }
    };
}

#[tokio::test]
#[ignore]
async fn status_ok() {
    let iface = skip_if_unconfigured!();
    iface.status().await.expect("node should respond to status");
}

#[tokio::test]
#[ignore]
async fn block_count_and_tip_hash_agree() {
    let iface = skip_if_unconfigured!();

    let count = iface.get_block_count().await.expect("getblockcount");
    let best = iface
        .get_best_block_hash()
        .await
        .expect("getbestblockhash");
    let at_height = iface.get_block_hash(count).await.expect("getblockhash");

    assert_eq!(best.len(), 64, "hash should be 32-byte hex");
    assert_eq!(best, at_height, "tip hash should match hash at tip height");
}

#[tokio::test]
#[ignore]
async fn latest_block_header_parses() {
    let iface = skip_if_unconfigured!();
    let header = iface
        .get_latest_block_header()
        .await
        .expect("get_latest_block_header");
    // A parsed header always has a non-zero merkle root on a real chain tip.
    println!("tip header: {header:?}");
}

#[tokio::test]
#[ignore]
async fn raw_mempool_is_reachable() {
    let iface = skip_if_unconfigured!();
    let mempool = iface.get_raw_mempool().await.expect("getrawmempool");
    assert!(mempool.is_array(), "getrawmempool should return an array");
}

#[tokio::test]
#[ignore]
async fn get_block_headers_is_unsupported() {
    let iface = skip_if_unconfigured!();
    // Documented behaviour: no RPC analog, so this is a deliberate error.
    assert!(iface.get_block_headers().await.is_err());
}
