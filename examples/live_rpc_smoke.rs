//! Live smoke test / usage example for `RpcInterface` against a regtest node.
//!
//! Exercises every public call the interface exposes, printing the result of
//! each so developers can see the shapes returned by a real bitcoind/BSV node.
//!
//! Defaults target node1 of the nchain regtest network with the default
//! `bitcoin`/`bitcoin` credentials; override via env vars:
//!
//! ```bash
//! RPC_ADDRESS=127.0.0.1:18332 RPC_USER=bitcoin RPC_PASSWORD=bitcoin \
//!   cargo run --example live_rpc_smoke --features interface
//! ```
//!
//! Optionally set `RPC_ADDRESS_ARG` to a wallet address to see non-empty
//! `get_balance`/`get_utxo` output.

use std::time::Duration;

use chain_gang::interface::{BlockchainInterface, RpcInterface};
use chain_gang::network::Network;

fn env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = env("RPC_ADDRESS", "127.0.0.1:18332");
    let user = env("RPC_USER", "bitcoin");
    let password = env("RPC_PASSWORD", "bitcoin");

    // Construct the client. "http://" is assumed when no scheme is given.
    // A bounded retry policy can be set with `.with_retries(..)`.
    let rpc = RpcInterface::new(&address, &user, &password, Network::BSV_Testnet)?
        .with_retries(5, Duration::from_millis(250));

    println!("== connecting to {address} ==\n");

    // --- BlockchainInterface trait methods -------------------------------

    println!("-- trait: BlockchainInterface --");
    rpc.status().await?;
    println!("status()                  : OK");

    let count = rpc.get_block_count().await?;
    println!("get_block_count()         : {count}");

    // get_latest_block_header() -> typed BlockHeader
    let header = rpc.get_latest_block_header().await?;
    println!("get_latest_block_header() : version={} merkle_root={:?} timestamp={}",
        header.version, header.merkle_root, header.timestamp);

    // get_block_headers() is intentionally unsupported over RPC.
    match rpc.get_block_headers().await {
        Ok(_) => println!("get_block_headers()       : (unexpected Ok)"),
        Err(e) => println!("get_block_headers()       : Err (expected) -> {e}"),
    }

    // --- Read-only query helpers -----------------------------------------

    println!("\n-- read-only helpers --");
    let best = rpc.get_best_block_hash().await?;
    println!("get_best_block_hash()     : {best}");

    let hash_at_tip = rpc.get_block_hash(count).await?;
    println!("get_block_hash({count})       : {hash_at_tip}");

    // Use block #1 for a stable, mature coinbase transaction to inspect.
    let sample_height = 1.min(count);
    let sample_hash = rpc.get_block_hash(sample_height).await?;

    let block = rpc.get_block(&sample_hash).await?;
    let txids = block
        .get("tx")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    println!("get_block(#{sample_height})           : {} tx(s), size={}",
        txids.len(),
        block.get("size").map(|v| v.to_string()).unwrap_or_else(|| "?".into()));

    let block_header = rpc.get_block_header(&sample_hash).await?;
    println!("get_block_header(#{sample_height})    : height={} confirmations={}",
        block_header.get("height").map(|v| v.to_string()).unwrap_or_else(|| "?".into()),
        block_header.get("confirmations").map(|v| v.to_string()).unwrap_or_else(|| "?".into()));

    if let Some(txid) = txids.first().and_then(|v| v.as_str()) {
        let raw = rpc.get_raw_transaction(txid).await?;
        println!("get_raw_transaction(txid) : {} hex chars", raw.len());

        // Typed round-trip via getrawtransaction -> Tx::read.
        match rpc.get_tx(txid).await {
            Ok(tx) => println!("get_tx(txid)              : typed Tx, {} in / {} out",
                tx.inputs.len(), tx.outputs.len()),
            Err(e) => println!("get_tx(txid)              : Err -> {e}"),
        }

        match rpc.get_tx_out(txid, 0).await {
            Ok(v) => {
                let value = v.get("value").map(|v| v.to_string()).unwrap_or_else(|| "null (spent?)".into());
                println!("get_tx_out(txid, 0)       : value={value}");
            }
            Err(e) => println!("get_tx_out(txid, 0)       : Err -> {e}"),
        }

        // Merkle proof that the tx is in the block.
        match rpc.get_merkle_proof(&sample_hash, txid).await {
            Ok(proof) => println!("get_merkle_proof(...)     : {} hex chars", proof.len()),
            Err(e) => println!("get_merkle_proof(...)     : Err -> {e}"),
        }
    }

    let mempool = rpc.get_raw_mempool().await?;
    println!("get_raw_mempool()         : {} entries",
        mempool.as_array().map(|a| a.len()).unwrap_or(0));

    // --- Wallet-scoped calls (only see addresses the node's wallet knows) --

    println!("\n-- wallet-scoped (get_balance / get_utxo) --");
    if let Ok(addr) = std::env::var("RPC_ADDRESS_ARG") {
        let balance = rpc.get_balance(&addr).await?;
        println!("get_balance({addr}) : {balance:?}");
        let utxo = rpc.get_utxo(&addr).await?;
        println!("get_utxo({addr})    : {} entries", utxo.len());
        for entry in utxo.iter().take(5) {
            println!("    {entry:?}");
        }
    } else {
        println!("(set RPC_ADDRESS_ARG=<wallet address> to exercise get_balance/get_utxo)");
        println!("note: these are wallet-scoped -- an arbitrary address returns empty");
    }

    println!("\n== all live calls completed ==");
    Ok(())
}
