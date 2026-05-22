# Documentation

Index for **chain-gang** (Rust library) and **tx-engine** (Python bindings).

## Python (`tx-engine`)

| Document | Description |
|----------|-------------|
| [README.md](../README.md) | Install from PyPI, class API reference, Chronicle overview |
| [python/README.md](../python/README.md) | Python package layout, local dev, and tests |
| [Chronicle-Python.md](Chronicle-Python.md) | **Python Chronicle guide** — examples for Context, Tx, signing, helpers |
| [Chronicle.md](Chronicle.md) | Chronicle upgrade: sighash, opcodes, validation modes, Rust API |

**Quick links**

- Install: `pip install tx-engine` (Python 3.11+)
- Chronicle validation: [Chronicle-Python.md](Chronicle-Python.md) — `Tx.validate()`, `Tx.validate_at_height()`, `Context`
- Node RPC flag reference: `tx_engine.interface.verify_script`

## Rust (`chain-gang`)

| Document | Description |
|----------|-------------|
| [README-chain-gang.md](README-chain-gang.md) | Crate overview, feature flags, installation |
| [Chronicle.md](Chronicle.md) | Same Chronicle spec; Rust examples use `chain_gang::chronicle` |
| [docs.rs](https://docs.rs/chain-gang) | Generated Rust API (`chain_gang::chronicle`, `Tx::validate_at_height`, etc.) |

## Development and releases

| Document | Description |
|----------|-------------|
| [Development.md](Development.md) | Project layout, building, testing, wheels |
| [Releases.md](Releases.md) | Version history |

## Chronicle upgrade

Start with [Chronicle-Python.md](Chronicle-Python.md) (Python) or [Chronicle.md](Chronicle.md) (full spec / Rust). Summary of validation modes:

| Goal | Python | Rust |
|------|--------|------|
| Offline / mempool-style check | `Tx.validate(utxos)` with `version > 1` | `tx.validate(...)` |
| Consensus at known block height | `Tx.validate_at_height(utxos, height, "BSV_Mainnet")` | `tx.validate_at_height(..., height, Network::BSV_Mainnet)` |
| Script debugger / partial eval | `Context(tx_version=2, lock_script=...)` | `TxVersionChecker` / `ZVersionChecker` |
| Compare to live node rules | `verify_script.ScriptFlags` | N/A (use node RPC) |

Activation heights: MainNet **943,835**, TestNet **1,713,022** — see [Chronicle.md#activation](Chronicle.md#activation).
