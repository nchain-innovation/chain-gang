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

Constants in Rust: `SIGHASH_CHRONICLE = 0x20` in `src/transaction/sighash.rs`.

Python: `SIGHASH.CHRONICLE` and `SIGHASH.ALL_FORKID_CHRONICLE` in `python/src/tx_engine/tx/sighash.py`.

## Signing (low-S policy)

Per the Chronicle spec, the low-S requirement is removed for transactions with `version > 1`. For signing:

| Flags | Behavior |
|-------|----------|
| Without `SIGHASH_CHRONICLE` | Signatures are normalized to low-S (BIP146) |
| With `SIGHASH_CHRONICLE` | Raw signer output is preserved (high-S allowed) |

Rust: `uses_low_s_signing()` and `generate_signature()` in `src/transaction/mod.rs`.

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

Rust: `uses_two_phase_eval()`, `eval_two_phase()` in `src/script/interpreter.rs`; routed from `Tx::validate()` in `src/messages/tx.rs`.

## Implementation status

- [x] OTDA sighash routing (`SIGHASH_CHRONICLE`)
- [x] OTDA preimage for Chronicle signatures
- [x] Optional high-S signing when `SIGHASH_CHRONICLE` is set
- [x] High-S acceptance during script verification for `tx.version > 1`
- [x] Chronicle opcodes (OP_VER, OP_SUBSTR, OP_LEFT, OP_RIGHT, OP_LSHIFTNUM, OP_RSHIFTNUM)
- [x] Two-phase unlock/lock script evaluation (`tx.version > 1`)
- [ ] Version-gated malleability relaxation (`tx.version > 1`)
- [ ] 32 MB script number limit

## References

- [Chronicle Release (docs)](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release)
- [Chronicle spec (bitcoin-sv-specs)](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/chronicle-spec.md)
