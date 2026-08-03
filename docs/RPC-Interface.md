# RPC Interface (Rust)

`RpcInterface` is a native Rust client that talks to a bitcoind / BSV node over
JSON-RPC. It sits alongside `WocInterface` and `UaaSInterface` and implements
the shared [`BlockchainInterface`] trait, so it is a drop-in alternative for the
trait methods, plus it exposes a set of read-only node-query helpers.

Requires the `interface` feature:

```bash
cargo build --features interface
```

## Constructing a client

```rust
use chain_gang::interface::{BlockchainInterface, RpcInterface};
use chain_gang::network::Network;

# async fn example() -> Result<(), chain_gang::util::ChainGangError> {
let rpc = RpcInterface::new(
    "127.0.0.1:8332",   // "http://" is assumed if no scheme is given
    "rpc_user",
    "rpc_password",
    Network::BSV_Testnet,
)?;

let height = rpc.get_block_count().await?;
println!("height = {height}");
# Ok(())
# }
```

Credentials are sent as HTTP Basic auth on every request. The client is async
and expects a Tokio runtime (as `reqwest` does).

### Retry policy

Transient connection/timeout failures are retried a bounded number of times
(default: 5 attempts, 250 ms apart). Override with `with_retries`:

```rust
use std::time::Duration;
let rpc = RpcInterface::new("127.0.0.1:8332", "u", "p", Network::BSV_Testnet)?
    .with_retries(3, Duration::from_millis(500));
```

## Trait methods (`BlockchainInterface`)

| Method | RPC call(s) |
| --- | --- |
| `status` | `getblockchaininfo` |
| `get_balance(address)` | `listunspent` |
| `get_utxo(address)` | `listunspent` + `getblockcount` |
| `broadcast_tx(tx)` | `sendrawtransaction` |
| `get_tx(txid)` | `getrawtransaction` |
| `get_latest_block_header()` | `getblockcount` → `getblockhash` → `getblockheader` |
| `get_block_headers()` | unsupported — returns an error (no RPC analog) |

## Read-only query helpers

`get_block_count`, `get_best_block_hash`, `get_block_hash`,
`get_raw_transaction`, `get_tx_out`, `get_block`, `get_block_header`,
`get_raw_mempool`, `get_merkle_proof`.

Loosely-shaped responses (`get_tx_out`, `get_block`, `get_block_header`,
`get_raw_mempool`) return `serde_json::Value` for the caller to interpret.

Wallet and regtest **mutation** calls (`getnewaddress`, `generatetoaddress`,
`sendtoaddress`, `importaddress`, …) are intentionally out of scope.

## Behavioural caveats

- **`get_balance` / `get_utxo` are wallet-scoped, not chain-scoped.** RPC
  `listunspent` only returns outputs for addresses the node's wallet is
  watching. Unlike `WocInterface` (which indexes the whole chain), an arbitrary
  address returns empty until the node has `importaddress`-ed it and rescanned.
- **`get_block_headers` is not available over RPC** and returns
  `ChainGangError::InvalidOperation`.
- Balance/UTXO height and satoshi conversions mirror the Python `RPCInterface`
  for behavioural parity.

## Errors

JSON-RPC error objects are surfaced as
`ChainGangError::RpcError { code, message }`. Transport failures surface as
`ChainGangError::ReqwestError` once retries are exhausted.

## Testing

- Unit tests (mock HTTP server, no node) run with the normal test command:
  `cargo test --features interface`.
- Live integration tests are opt-in and ignored by default — see
  [`tests/rpc_integration.rs`](../tests/rpc_integration.rs):

  ```bash
  export RPC_ADDRESS=127.0.0.1:18332
  export RPC_USER=bitcoin
  export RPC_PASSWORD=bitcoin
  cargo test --features interface --test rpc_integration -- --ignored
  ```

[`BlockchainInterface`]: README-chain-gang.md
