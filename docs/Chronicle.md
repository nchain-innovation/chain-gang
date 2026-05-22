# Chronicle Upgrade

Implementation plan and notes for [Chronicle Release](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release) support.

## Activation

| Network | Block height |
|---------|--------------|
| TestNet | 1,713,022 |
| MainNet | 943,835 |

## Sighash routing

Per [bitcoin-sv `SignatureHash`](https://github.com/bitcoin-sv/bitcoin-sv/blob/master/src/script/interpreter.cpp):

| Flags | Algorithm |
|-------|-----------|
| `SIGHASH_FORKID` without `SIGHASH_CHRONICLE` (`0x20`) | BIP143 |
| `SIGHASH_CHRONICLE` set | OTDA (Original Transaction Digest Algorithm) |
| Neither | OTDA (pre-fork legacy) |

Constants and signing policy: `SIGHASH_CHRONICLE`, `uses_low_s_signing()` — re-exported from `chain_gang::chronicle` (also in `src/transaction/sighash.rs` and `src/transaction/mod.rs`).

Python: `SIGHASH.CHRONICLE` and `SIGHASH.ALL_FORKID_CHRONICLE` in `python/src/tx_engine/tx/sighash.py`.

## Signing (low-S policy)

Per the Chronicle spec, the low-S requirement is removed for transactions with `version > 1`. For signing:

| Flags | Behavior |
|-------|----------|
| Without `SIGHASH_CHRONICLE` | Signatures are normalized to low-S (BIP146) |
| With `SIGHASH_CHRONICLE` | Raw signer output is preserved (high-S allowed) |

Rust: `uses_low_s_signing()` and `generate_signature()` in `src/transaction/mod.rs` (see `chain_gang::chronicle`).

Note: k256's deterministic signer currently returns low-S; the Chronicle path matters when encoding externally produced signatures or future signing backends.

Verification for `tx.version > 1` normalizes S before the k256 prehash verify call so high-S encodings are accepted under Chronicle rules.

## Opcodes

Chronicle reinstates and reassigns opcodes per the [Chronicle spec](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/chronicle-spec.md):

| Opcode | Value | Description |
|--------|-------|-------------|
| `OP_VER` | 0x62 | Push executing transaction version |
| `OP_VERIF` | 0x65 | Conditional on `tx.version >=` stack top |
| `OP_VERNOTIF` | 0x66 | Inverse of `OP_VERIF` |
| `OP_SUBSTR` | 0xb3 | Substring by start index and length (was NOP4) |
| `OP_LEFT` | 0xb4 | Leftmost substring (was NOP5) |
| `OP_RIGHT` | 0xb5 | Rightmost substring (was NOP6) |
| `OP_LSHIFTNUM` | 0xb6 | Signed left shift (was NOP7) |
| `OP_RSHIFTNUM` | 0xb7 | Signed right shift (was NOP8) |

`OP_2MUL` and `OP_2DIV` were already implemented; `OP_VER` requires a transaction context (`Checker::tx_version()`).

## Two-phase script evaluation

For transactions with `version > 1`, unlock and lock scripts are evaluated separately per the [Chronicle spec](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/chronicle-spec.md):

1. Evaluate the **unlock script** first.
2. Keep the **main stack**; clear **conditional** and **alt** stacks.
3. Evaluate the **lock script** with the inherited main stack.

Legacy transactions (`version == 1`) continue to concatenate `unlock + OP_CODESEPARATOR + lock` into a single script.

CHECKSIG `scriptCode` in the unlock phase spans from the last `OP_CODESEPARATOR` in the unlock script through the end of the lock script (code separators stripped). CHECKSIG in the lock phase uses only the lock script from its last `OP_CODESEPARATOR`.

Rust: `uses_two_phase_eval()`, `eval_two_phase()` — re-exported from `chain_gang::chronicle`; routed from `Tx::validate()` in `src/messages/tx.rs`.

## Malleability relaxation

For transactions with `version > 1`, Chronicle relaxes malleability-related script rules per the [Chronicle spec](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/chronicle-spec.md):

| Rule | `version == 1` | `version > 1` |
|------|----------------|---------------|
| Clean stack (exactly one true item) | Enforced | Top item must be true; extra stack items allowed |
| MINIMALIF (`OP_IF` / `OP_NOTIF` operands) | Empty or `0x01` only | Any valid bool encoding |
| MINIMALDATA (push / number encoding) | Enforced | Relaxed |
| NULLFAIL (`OP_CHECKSIG` failure) | Non-empty sig is an error | Pushes false |
| NULLDUMMY (`OP_CHECKMULTISIG` dummy) | Must be empty | Any dummy value |
| Push-only unlock script | Required | Functional opcodes allowed (two-phase eval) |
| Low-S signatures | Enforced at verify | High-S accepted (Phase 2) |

Rules apply when the checker provides a transaction version (`TransactionChecker`). Context-free script evaluation preserves prior behavior.

Rust: `uses_relaxed_malleability()`, `is_push_only()` — re-exported from `chain_gang::chronicle`.

## Script number limit

The maximum encoded script number size increases from 750 KB to 32 MB for Chronicle transactions (`tx.version > 1`). Pre-genesis inputs (`PREGENESIS_RULES`) keep the 4-byte limit; post-genesis `version == 1` transactions keep 750 KB.

Rust: `max_script_num_length()`, `MAX_SCRIPT_NUM_LENGTH_*` — re-exported from `chain_gang::chronicle`; enforced in `core_eval()` via `pop_bigint_checked()` and `OP_BIN2NUM` / `OP_NUM2BIN`.

Python: `MAX_SCRIPT_NUM_LENGTH_CHRONICLE` in `python/src/tx_engine/engine/util.py`.

## Rust API

Import Chronicle helpers from one module:

```rust
use chain_gang::chronicle::{
    eval_two_phase, is_push_only, max_script_num_length, uses_low_s_signing,
    uses_relaxed_malleability, uses_two_phase_eval, SIGHASH_CHRONICLE,
    MAX_SCRIPT_NUM_LENGTH_CHRONICLE, TxVersionChecker,
};
```

Version-only script debugging without a full transaction: `TxVersionChecker` and `ZVersionChecker` (also in `chain_gang::chronicle`).

## Implementation status

- [x] OTDA sighash routing (`SIGHASH_CHRONICLE`)
- [x] OTDA preimage for Chronicle signatures
- [x] Optional high-S signing when `SIGHASH_CHRONICLE` is set
- [x] High-S acceptance during script verification for `tx.version > 1`
- [x] Chronicle opcodes (OP_VER, OP_SUBSTR, OP_LEFT, OP_RIGHT, OP_LSHIFTNUM, OP_RSHIFTNUM)
- [x] Two-phase unlock/lock script evaluation (`tx.version > 1`)
- [x] Version-gated malleability relaxation (`tx.version > 1`)
- [x] 32 MB script number limit (`tx.version > 1`)

## References

- [Chronicle Release (docs)](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release)
- [Chronicle spec (bitcoin-sv-specs)](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/chronicle-spec.md)
