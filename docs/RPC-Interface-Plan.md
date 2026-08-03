# Adding a native Rust `RpcInterface` (bitcoind JSON-RPC)

## Motivation

The bitcoind JSON-RPC capability currently lives **only** in Python
(`python/src/tx_engine/interface/rpc_interface.py`), and it is pure Python — it
does not call into Rust. This plan adds a native Rust `RpcInterface` under
`src/interface/` so that Rust clients can talk to a bitcoind/BSV node over
JSON-RPC, alongside the existing `WocInterface` and `UaaSInterface`.

## Confirmed design decisions

| Decision        | Choice                                                                    |
| --------------- | ------------------------------------------------------------------------- |
| Method surface  | Shared `BlockchainInterface` trait **+ read-only query calls** (no wallet/regtest mutation ops) |
| Return types    | Typed structs for trait methods; `serde_json::Value` for looser calls     |
| Consumers       | **Rust only** — Python `rpc_interface.py` left untouched                   |
| Transport       | Hand-rolled JSON-RPC on the existing `reqwest` dependency                  |
| Retries         | Bounded + configurable (no infinite loop)                                 |
| Construction    | Typed constructor                                                         |
| Tests           | Mock-server unit tests **+ live regtest integration tests**               |

Because the change is Rust-only and additive, **no existing Rust code changes
behaviour**. It sits under the existing `interface` feature → a **minor**
version bump (0.10.0 → 0.11.0), no breaking changes.

## 1. What gets built

### 1.1 New file: `src/interface/rpc_interface.rs`

JSON-RPC plumbing (bitcoind/BSV speaks JSON-RPC 1.0 over HTTP Basic auth):

```rust
#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,            // "1.0"
    id: &'a str,                // "chain-gang"
    method: &'a str,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcErrorObject>,
}

#[derive(Deserialize, Debug)]
struct RpcErrorObject { code: i32, message: String }
```

The client struct + typed constructor:

```rust
pub struct RpcInterface {
    client: reqwest::Client,
    url: String,                 // e.g. "http://127.0.0.1:8332"
    user: String,
    password: String,
    network_type: Network,
    max_retries: u32,            // default 5
    retry_delay: Duration,       // default 250ms
}

impl RpcInterface {
    pub fn new(address: &str, user: &str, password: &str, network: Network)
        -> Result<Self, ChainGangError> { /* validates URL */ }

    pub fn with_retries(mut self, max: u32, delay: Duration) -> Self { /* override */ }
}
```

One generic call path all methods funnel through — bounded retry + error mapping
live here:

```rust
async fn call<T: DeserializeOwned>(&self, method: &str, params: serde_json::Value)
    -> Result<T, ChainGangError>
{
    // loop up to max_retries:
    //   client.post(&url).basic_auth(user, Some(password)).json(&req).send().await
    //   on reqwest connection error -> sleep(retry_delay), retry
    //   on HTTP body -> parse RpcResponse<T>
    //   error object present -> Err(ChainGangError::RpcError{code, message})
    //   else -> Ok(result)
}
```

Verification points to confirm against docs at implementation time (not from
memory): `reqwest` 0.13 `RequestBuilder::basic_auth`/`json` signatures, and BSV
node RPC semantics/param ordering.

### 1.2 Trait implementation (`impl BlockchainInterface for RpcInterface`)

| Trait method             | RPC call(s)                                                                     | Returns             |
| ------------------------ | ------------------------------------------------------------------------------- | ------------------- |
| `set_network`            | —                                                                               | sets `network_type` |
| `status`                 | `getblockchaininfo`                                                             | `Ok(())` on success |
| `get_balance(addr)`      | `listunspent 0 9999999 [addr]` → sum                                            | `Balance`           |
| `get_utxo(addr)`         | `listunspent` → map                                                            | `Utxo`              |
| `broadcast_tx(tx)`       | `sendrawtransaction [hex]`                                                      | `String` (txid)     |
| `get_tx(txid)`           | `getrawtransaction [txid]` → hex → `Tx::read`                                   | `Tx`                |
| `get_latest_block_header`| `getblockcount` → `getblockhash` → `getblockheader hash false` → `BlockHeader::read` | `BlockHeader`  |
| `get_block_headers`      | no clean RPC analog — return `Err(InvalidOperation)` (see caveat)               | `String`            |

### 1.3 Extra read-only methods (return `serde_json::Value`, or narrow typed)

`get_raw_transaction`, `get_tx_out`, `get_block`, `get_block_header`,
`get_raw_mempool`, `get_merkle_proof` (`gettxoutproof`), plus small typed
helpers `get_block_count() -> u64`, `get_block_hash(i) -> String`,
`get_best_block_hash() -> String`.

Excluded per scope (wallet/regtest mutation): `getnewaddress`,
`generatetoaddress`, `sendtoaddress`, `importaddress`, `getmininginfo`,
`getwalletinfo`, `getinfo`, `listaddressgroupings`, `verifyscript`, `generate`.

### 1.4 `src/interface/mod.rs`

```rust
pub mod rpc_interface;
pub use rpc_interface::RpcInterface;
```

### 1.5 `src/util/errors.rs`

```rust
#[error("RPC error {code}: {message}")]
RpcError { code: i32, message: String },
```

### 1.6 `Cargo.toml`

No new runtime deps. Add a `[dev-dependencies]` block (currently absent) to
drive async tests:

```toml
[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
httpmock = "0.7"   # or wiremock — for mock-server unit tests
```

The library imposes no runtime on Rust clients; they bring their own.

## 2. Behavioural caveats to document

1. **`get_balance`/`get_utxo` are wallet-scoped, not chain-scoped.** RPC
   `listunspent` only sees addresses the node's wallet knows. Unlike
   `WocInterface` (which indexes the whole chain), an arbitrary address returns
   empty until `importaddress` + rescan. Same trait method, different precondition.
2. **`get_block_headers` has no direct RPC equivalent.** Return
   `Err(ChainGangError::InvalidOperation(...))` rather than faking it.
3. **JSON-RPC 1.0.** bitcoind returns `id`/`result`/`error`; keep parsing lenient.

## 3. Testing plan

- **Unit (mock HTTP, no node):** request serialization (method/params/auth),
  `result` vs `error` parsing → `RpcError`, retry-then-succeed, UTXO/balance
  mapping and satoshi conversion, hex→`Tx`/`BlockHeader` round-trips.
- **Integration (live regtest, `#[ignore]` or `rpc-integration` feature):** creds
  via env vars; the harness can call `generatetoaddress`/`importaddress` through
  the raw `call()` to arrange state, then assert the public read methods. Not run
  in normal CI.

## 4. Sequenced, atomic commits (trunk-based)

Short-lived branch `feature/rust-rpc-interface`:

1. `feat: add RpcError variant to ChainGangError`
2. `feat: add JSON-RPC transport core to RpcInterface`
3. `feat: implement BlockchainInterface trait for RpcInterface`
4. `feat: add read-only RPC query methods`
5. `feat: export RpcInterface from interface module`
6. `chore: add tokio + httpmock dev-dependencies`
7. `test: add mock-server unit tests for RpcInterface`
8. `test: add gated regtest integration tests`
9. `docs: document RpcInterface + wallet-scope caveats`
10. `chore(release): bump 0.10.0 → 0.11.0`

## 5. Open points for the build phase

- `get_block_headers`: `Err(InvalidOperation)` vs. best-effort single-header
  (leaning toward the honest error).
- Default retry values (proposed 5 × 250ms — bounded).
- Whether the constructor pings the node eagerly or stays lazy (leaning lazy,
  like `WocInterface`).
