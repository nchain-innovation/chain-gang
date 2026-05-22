# Chronicle (Python)

Python guide for [Chronicle](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release) in **tx-engine**. For the full specification (sighash routing, opcode table, malleability matrix, Rust API), see [Chronicle.md](Chronicle.md).

## Opt in

Chronicle script rules apply when the spending transaction has **`version > 1`** (typically `version=2`). Set this on the tx you build and validate — not on the funding UTXO.

Activation heights (consensus): MainNet **943,835**, TestNet **1,713,022**.

## Which API to use

| Goal | API | Notes |
|------|-----|-------|
| Debug a script step-by-step | `Context(...)` | Optional `tx_version`, `lock_script`; no block height |
| Check a full spend offline | `Tx.validate(utxos)` | Chronicle when `version > 1`; height ignored |
| Check as a node would at height *H* | `Tx.validate_at_height(utxos, H, network)` | Rejects `version > 1` before activation on that network |
| Sign a Chronicle input | `Wallet.sign_tx_sighash(..., SIGHASH.ALL_FORKID_CHRONICLE)` | OTDA sighash; high-S allowed on verify for `version > 1` |
| Compare to live node RPC | `verify_script.ScriptFlags` | Reference only — not used by local eval |

`network` for height-aware validation: `BSV_Mainnet`, `BSV_Testnet`, or `BSV_STN`.

## Two-phase unlock (functional scriptSig)

Chronicle evaluates **scriptSig** and **scriptPubKey** in separate phases. The unlock script may leave items on the stack for the lock script — impossible under legacy single-phase eval.

```python
from tx_engine import Context, Script, Tx, TxIn, TxOut


def display_tx_hash(tx: Tx) -> str:
    return bytes(reversed(bytes(tx.hash()))).hex()


fund = Tx(
    version=1,
    tx_ins=[],
    tx_outs=[TxOut(amount=1_000, script_pubkey=Script.parse_string("OP_5 OP_EQUAL"))],
)
spend = Tx(
    version=2,
    tx_ins=[
        TxIn(
            display_tx_hash(fund),
            0,
            Script.parse_string("OP_2 OP_3 OP_ADD"),  # unlock: pushes 5
        )
    ],
    tx_outs=[TxOut(amount=900, script_pubkey=Script([]))],
)

spend.validate([fund])  # None on success
```

Same script pair in the debugger:

```python
unlock = Script.parse_string("OP_2 OP_3 OP_ADD")
lock = Script.parse_string("OP_5 OP_EQUAL")

Context(script=unlock, lock_script=lock, tx_version=2).evaluate()  # True
```

With `version=1`, the same spend fails validation because legacy rules require the unlock script to be push-only and use single-phase eval.

## Context and Chronicle opcodes

Pass `tx_version` for opcodes that depend on the executing transaction version (e.g. `OP_VER`, `OP_VERIF`):

```python
from tx_engine import Context, Script

script = Script.parse_string("OP_VER OP_2 OP_NUMEQUAL")
Context(script=script, tx_version=2).evaluate()   # True
Context(script=script, tx_version=1).evaluate()   # False
```

**Two-phase eval** requires both `tx_version > 1` and `lock_script`. **Relaxed clean stack** (multiple true stack items allowed) applies when `tx_version > 1` even without `lock_script`.

`Context` does not enforce block-height activation — only transaction version and script pairing.

## Height-aware validation

Use `validate_at_height` when you know the confirming block and need consensus-faithful gating (e.g. reject a `version=2` spend before MainNet block 943,835):

```python
from tx_engine.interface.verify_script import CHRONICLE_ACTIVATION_MAINNET

# Succeeds at or after activation
spend.validate_at_height([fund], CHRONICLE_ACTIVATION_MAINNET, "BSV_Mainnet")

# Raises ValueError before activation
spend.validate_at_height([fund], CHRONICLE_ACTIVATION_MAINNET - 1, "BSV_Mainnet")
```

`Tx.validate([fund])` would accept the same spend at any height — use that only when height is unknown (signing, mempool simulation).

## Signing with OTDA (Chronicle sighash)

Chronicle signatures use the Original Transaction Digest Algorithm (OTDA), selected by the `SIGHASH_CHRONICLE` flag (`0x20`):

```python
from tx_engine import Script, Tx, TxIn, TxOut, Wallet
from tx_engine.tx.sighash import SIGHASH

wallet = Wallet.from_int("BSV_Mainnet", private_key_int)

fund = Tx(
    version=1,
    tx_ins=[],
    tx_outs=[TxOut(amount=10, script_pubkey=wallet.get_locking_script())],
)
spend = Tx(
    version=2,
    tx_ins=[TxIn(display_tx_hash(fund), 0, Script([]))],
    tx_outs=[TxOut(amount=5, script_pubkey=Script([]))],
)

signed = wallet.sign_tx_sighash(
    0,
    fund,
    spend,
    int(SIGHASH.ALL_FORKID_CHRONICLE),
)
signed.validate([fund])
```

- **`SIGHASH.ALL_FORKID`** (no `CHRONICLE`) → BIP143 preimage  
- **`SIGHASH.ALL_FORKID_CHRONICLE`** → OTDA preimage  

For `version > 1`, verification accepts high-S signatures when signed with the Chronicle sighash flag. Default `sign_tx()` uses `SIGHASH.ALL_FORKID` (low-S signing policy).

## Activation helpers (no full tx required)

`tx_engine.interface.verify_script` mirrors `chain_gang::chronicle` for tooling and RPC integration:

```python
from tx_engine.interface.verify_script import (
    CHRONICLE_ACTIVATION_MAINNET,
    activation_height,
    chronicle_rules_active,
    effective_chronicle_tx_version,
)

chronicle_rules_active(2)  # True (version-only)
chronicle_rules_active(2, CHRONICLE_ACTIVATION_MAINNET - 1, "BSV_Mainnet")  # False
chronicle_rules_active(2, CHRONICLE_ACTIVATION_MAINNET, "BSV_Mainnet")  # True

activation_height("BSV_Mainnet")  # 943835
effective_chronicle_tx_version(2, CHRONICLE_ACTIVATION_MAINNET - 1, "BSV_Mainnet")  # 1
```

## Node RPC vs local validation

`ScriptFlags` documents bits for a **bitcoin-sv node** `verifyscript` call. Chain-gang does not take a flag bitmask for local eval — set `tx.version` and call `Tx.validate()` or `Context(tx_version=...)`.

The node applies Chronicle at consensus heights automatically. Locally, use `validate_at_height` to match that behavior.

## Script number limit

```python
from tx_engine.engine.util import max_script_num_length, MAX_SCRIPT_NUM_LENGTH_CHRONICLE

max_script_num_length(1)   # 750_000 (legacy post-genesis)
max_script_num_length(2)   # 32_000_000 (Chronicle)
```

## Tests

Python Chronicle coverage lives under `python/src/tests/`:

- `test_chronicle_tx_validate.py` — `Tx.validate`, two-phase spends, clean stack  
- `test_chronicle_context.py` — `Context`, `OP_VER`, script number limits  
- `test_chronicle_opcodes.py` — Chronicle opcode behavior  
- `test_verify_script.py` — activation helpers and `ScriptFlags`  
- `test_sighash.py` — OTDA / `SIGHASH.ALL_FORKID_CHRONICLE`  

## See also

- [Chronicle.md](Chronicle.md) — full upgrade spec and Rust API  
- [README.md](../README.md) — Python class reference  
- [Chronicle release (BSV docs)](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release)
