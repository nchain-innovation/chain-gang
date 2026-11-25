# Chronicle signature differential validation

Addresses #245, #247; independent of PR #240.

Oracle: actual BSV v1.2.3 (6504a3a) `verifyscript`, running linux/amd64 with Rosetta in Apple Container 1.7.1. Image pinned to `docker.io/bitcoinsv/bitcoin-sv@sha256:d96fe4786a00e5fc10bc9e0e76a6006376f9472d4eb74d07e4c1e0d019493d33`. This is live node execution, not a reimplementation.

Start an isolated node (no public P2P connections):

```sh
container run --name cg-chronicle-node --platform linux/amd64 --rosetta --cpus 2 --memory 2G -d docker.io/bitcoinsv/bitcoin-sv@sha256:d96fe4786a00e5fc10bc9e0e76a6006376f9472d4eb74d07e4c1e0d019493d33 bitcoind -regtest -server -listen=0 -dnsseed=0 -discover=0 -connect=0 -rpcuser=cgtest -rpcpassword=local-regtest-only -minminingtxfee=0.000005 -genesisactivationheight=1 -chronicleactivationheight=1 -excessiveblocksize=100000000 -maxstackmemoryusageconsensus=100000000
```

The static credentials above are for this isolated test container only. No host RPC port is published.

Export fixtures by setting `CHRONICLE_DIFF_OUTPUT` to an absolute file path when running `cargo test --test chronicle_script_code`. Run `python3 scripts/chronicle-node-diff.py cases.json results.json`. The runner compares each node result with the library verdict and expected outcome, rejects timeouts/skips, and returns nonzero on any mismatch. The committed receipt includes the exact transactions, flags, scripts and responses.

Scope: script consensus verification with explicit flags and supplied previous outputs. This does not claim to mine blocks, test mempool admission, or prove full Chronicle equivalence. Activation/UTXO context and numeric limit corrections are separate work (#241/#246).
