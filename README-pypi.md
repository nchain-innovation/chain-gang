# TX Engine

Python interface for building Bitcoin SV scripts and transactions. The native core is the Rust [chain-gang](https://github.com/nchain-innovation/chain-gang) library.

**Requires Python 3.11+.** Install from [PyPI](https://pypi.org/project/tx-engine/):

```bash
pip install tx-engine
```

## Documentation

| Topic | Link |
|-------|------|
| Documentation index | [docs/README.md](docs/README.md) |
| Python class reference | [docs/Python-API.md](docs/Python-API.md) |
| Chronicle (Python examples) | [docs/Chronicle-Python.md](docs/Chronicle-Python.md) |
| Chronicle (full spec) | [docs/Chronicle.md](docs/Chronicle.md) |
| Local development | [python/README.md](python/README.md) |

## Quick example

```python
from tx_engine import Tx

src_tx = (
    "0100000001c7151ebaf14dbfe922bd90700a7580f6db7d5a1b898ce79cb9ce459e17f1290900000000"
    "6b4830450221008b001e8d8110804ac66e467cd2452f468cba4a2a1d90d59679fe5075d24e5f530220"
    "6eb04e79214c09913fad1e3c0c2498be7f457ed63323ac6f2d9a38d53586a58d41210395deb00349c0ae7"
    "3412a55bec70a7793fc6860a193d29dd61d73c6271ffcbd4cffffffff0103000000000000001976a914"
    "96795fb99fd6c0f214f7a0e96019f642225f52d288ac00000000"
)

tx = Tx.parse_hexstr(src_tx)
print(tx)
```

## Chronicle upgrade

Bitcoin SV [Chronicle](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release) is supported in both the Rust library and Python bindings. **Guide:** [docs/Chronicle-Python.md](docs/Chronicle-Python.md).

Chronicle script rules apply when **`tx.version > 1`**. Use `version: 2` (or higher) on spending transactions to opt in.

| Check | Python API |
|-------|------------|
| Version-only (offline / mempool-style) | `Tx.validate(utxos)` |
| Consensus at a known block height | `Tx.validate_at_height(utxos, block_height, network)` — `network` is `BSV_Mainnet`, `BSV_Testnet`, or `BSV_STN` |
| Script debugger / partial eval | `Context(tx_version=2, lock_script=...)` |

See [docs/Chronicle.md](docs/Chronicle.md) for sighash routing, opcodes, two-phase evaluation, malleability rules, and script number limits.
