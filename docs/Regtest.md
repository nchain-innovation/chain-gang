# Regtest

Regtest is a private chain you run yourself: blocks are mined on demand at minimum
difficulty, coins are worthless, and nothing leaves your machine. It is the natural
place to test anything that spends, because the alternative is testnet coins,
outbound network access, and waiting on a public chain.

chain-gang reaches regtest through two independent paths:

| | type | what it needs |
|---|---|---|
| **Chain queries** — balances, UTXOs, broadcast | [`RpcInterface`](https://docs.rs/chain-gang/latest/chain_gang/interface/rpc_interface/struct.RpcInterface.html) | the node's JSON-RPC port, and credentials |
| **P2P** — messages, headers, blocks | [`Peer`](https://docs.rs/chain-gang/latest/chain_gang/peer/struct.Peer.html) with `Network::BSV_Regtest` | the node's P2P port |

`WocInterface` cannot: WhatsOnChain is a public explorer, and a private chain has
nothing for it to index. Asking it for regtest returns an error rather than
guessing.

## Starting a node

```bash
docker run -d --name regtest -p 18443:18443 -p 18444:18444 \
  bitcoinsv/bitcoin-sv:latest \
  bitcoind -regtest -server -listen=1 -bind=0.0.0.0:18444 \
    -rpcbind=0.0.0.0 -rpcallowip=0.0.0.0/0 -rpcport=18443 \
    -rpcuser=cguser -rpcpassword=cgpass \
    -minminingtxfee=0.000005 -printtoconsole
```

Two of those are easy to get wrong:

* **`-minminingtxfee` is mandatory.** Without it `bitcoind` refuses to start, with
  `Mandatory policy parameter is not set`. The container exits and the only trace
  is in `docker logs`.
* **`-listen=1` is needed for P2P.** RPC on 18443 works without it; a `Peer`
  connection to 18444 will not.

Then mine enough blocks for a spendable coinbase — 101, since coinbase outputs
mature after 100:

```bash
ADDR=$(bitcoin-cli -regtest getnewaddress)
bitcoin-cli -regtest generatetoaddress 101 "$ADDR"
```

## Chain queries

```rust
use chain_gang::interface::{BlockchainInterface, RpcInterface};
use chain_gang::network::Network;

let node = RpcInterface::new("127.0.0.1:18443", "cguser", "cgpass", Network::BSV_Regtest);

node.status().await?;                       // getblockchaininfo
let balance = node.get_balance(&address).await?;
let utxos = node.get_utxo(&address).await?;
let txid = node.broadcast_tx(&tx).await?;   // sendrawtransaction
```

A bare `host:port` is treated as `http://`; pass a full URL for `https`. Credentials
go to whatever host is configured, as HTTP basic auth on every call, so point this
only at a node you control.

### The node must be watching the address

`get_balance` and `get_utxo` go through `listunspent`, which reports only what the
node's own wallet tracks. **An address the node knows nothing about reads as zero,
with no error** — the node is answering truthfully about a wallet that has never
heard of it. The symptom is an empty balance from a node that is plainly reachable,
which looks like a bug in your code.

Either ask the node to watch it:

```rust
node.import_address(&address, false).await?;   // false: skip the rescan
```

or do the same from the command line:

```bash
bitcoin-cli -regtest importaddress "<address>" "" false
```

`rescan` asks the node to walk the chain for the address's history and blocks the
RPC connection until it finishes. Pass `false` when the address has no history yet —
which on a fresh chain is always — and `true` when importing an address that has
already been paid.

Nothing in `RpcInterface` imports for you. Doing so would mean the library managing
the node's wallet and deciding when to rescan, which belongs to whoever runs the
node.

## P2P

```rust
use chain_gang::messages::{Version, NODE_BITCOIN_CASH, PROTOCOL_VERSION};
use chain_gang::network::Network;
use chain_gang::peer::{Peer, SVPeerFilter};
use chain_gang::util::{rx::Observable, secs_since};

let version = Version {
    version: PROTOCOL_VERSION,
    services: NODE_BITCOIN_CASH,
    timestamp: secs_since(std::time::UNIX_EPOCH) as i64,
    user_agent: "my-app".to_string(),
    ..Default::default()
};

let peer = Peer::connect(ip, 18444, Network::BSV_Regtest, version, SVPeerFilter::new(0));
peer.connected_event().poll_timeout(Duration::from_secs(15))?;
```

There are no DNS seeds on a private chain, so `Network::BSV_Regtest.seeds()` is
empty by design: connect to an address you already know.

**A node bans a peer that sends a bad message start.** If the magic bytes are wrong
the first attempt is dropped and every later attempt is refused without being
looked at, whatever it sends — so a wrong constant looks identical to a network
problem. Clear it with `bitcoin-cli -regtest clearbanned`, or restart the node,
before trying again.

## What regtest is, in constants

| | value | note |
|---|---|---|
| P2P port | 18444 | RPC is 18443 |
| Magic bytes | `da b5 bf fa` | inherited from Bitcoin Cash, **not** Bitcoin Core's `fa bf b5 da` |
| Genesis hash | `0f9188f1…466e2206` | differs from testnet in `bits` and `nonce`: regtest mines at minimum difficulty |
| Address prefixes | `0x6f` / `0xc4` | the same as testnet |
| Extended keys | `tpub` / `tprv` | the same as testnet |
| WIF prefix | `0xef` | the same as testnet |
| DNS seeds | none | a private chain has nothing to discover |
| Chronicle activation | block 0 | as for STN, a private chain runs current rules from its first block |

Because the address and key prefixes are testnet's, anything you already have on
disk for testnet — addresses, WIFs, extended keys — works unchanged on regtest.

## The tests in this repository

`tests/regtest_p2p.rs` drives a real handshake against a node and is `#[ignore]`d,
because CI has no node to talk to:

```bash
cargo test --test regtest_p2p -- --ignored --test-threads=1
```

It exists because a wrong `magic` value cannot be caught any other way: the magic
bytes are not part of the genesis hash, so every offline test passes while no node
will speak to you. That is not hypothetical — `BSV_Regtest` shipped with Bitcoin
Core's magic until this test was written.
