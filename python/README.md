# tx-engine (Python)

Python bindings for the [chain-gang](../) Rust library, published on PyPI as [`tx-engine`](https://pypi.org/project/tx-engine/).

## Documentation

- [docs/README.md](../docs/README.md) — documentation index
- [README.md](../README.md) — install and Python class API reference
- [docs/Chronicle-Python.md](../docs/Chronicle-Python.md) — Chronicle Python guide (examples)
- [docs/Chronicle.md](../docs/Chronicle.md) — Chronicle upgrade (full spec)

## Layout

```
python/
├── src/tx_engine/   # package source (Script, Tx, Context, Wallet, …)
├── src/tests/       # unit tests
├── examples/        # script debugger examples
├── lint.sh          # flake8
└── tests.sh         # run unit tests
```

The native extension is built from the repo root via [maturin](https://www.maturin.rs/) (`pyproject.toml`).

## Local development

From the repository root:

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install maturin
maturin develop --features python
```

Run tests and lint:

```bash
cd python
./tests.sh
./lint.sh
```

Requires **Python 3.11** or newer.
