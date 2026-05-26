# chain-gang

Rust library for Bitcoin-derived blockchains, with optional Python bindings (**tx-engine** on [PyPI](https://pypi.org/project/tx-engine/)).

| Package | Registry | README |
|---------|----------|--------|
| **chain-gang** (Rust) | [crates.io](https://crates.io/crates/chain-gang) / [docs.rs](https://docs.rs/chain-gang) | [docs/README-chain-gang.md](docs/README-chain-gang.md) |
| **tx-engine** (Python) | [PyPI](https://pypi.org/project/tx-engine/) | [README-pypi.md](README-pypi.md) |

Full documentation index: [docs/README.md](docs/README.md).

## Supported chains

| Name | Code | Networks |
| --- | --- | --- |
| Bitcoin SV | `BSV` | BSV_Mainnet, BSV_Testnet, BSV_STN |
| Bitcoin | `BTC` | BTC_Mainnet, BTC_Testnet |
| Bitcoin Cash | `BCH` | BCH_Mainnet, BCH_Testnet |

**Python `Context` debugger:** pass optional `tx_version` and `lock_script` for Chronicle two-phase eval, relaxed clean stack, and `OP_VER`. `Context` does not enforce block-height activation; for full transaction checks use `Tx.validate()` (version-only) or `Tx.validate_at_height()` with a BSV network name.

**All chains:** P2P messages, address encoding, node connections, mainnet/testnet.

**BSV only:** transaction signing, script evaluation, wallet/key derivation, Genesis upgrade, [Chronicle upgrade](docs/Chronicle.md) (OTDA sighash, opcodes, two-phase eval, `tx.version > 1` rules).

## Rust usage

Add to `Cargo.toml`:

```toml
chain-gang = "0.7"
```

Build with optional features:

```bash
cargo build --features interface   # WhatsOnChain / HTTP blockchain interface
cargo build --features python     # PyO3 bindings for tx-engine
```

Chronicle helpers: `chain_gang::chronicle` — see [docs/Chronicle.md](docs/Chronicle.md) and [docs.rs/chain_gang/chronicle](https://docs.rs/chain-gang/latest/chain_gang/chronicle/index.html).

## Python bindings (tx-engine)

```bash
pip install tx-engine   # Python 3.11+
```

PyPI shows [README-pypi.md](README-pypi.md). Class reference: [docs/Python-API.md](docs/Python-API.md). Chronicle examples: [docs/Chronicle-Python.md](docs/Chronicle-Python.md). Local dev: [python/README.md](python/README.md).

## Development

See [docs/Development.md](docs/Development.md).

## License

MIT — see [LICENSE](LICENSE).
