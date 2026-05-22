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

## Implementation status

- [x] OTDA sighash routing (`SIGHASH_CHRONICLE`)
- [x] OTDA preimage for Chronicle signatures
- [x] Optional high-S signing when `SIGHASH_CHRONICLE` is set
- [x] High-S acceptance during script verification for `tx.version > 1`
- [ ] Chronicle opcodes (OP_VER, OP_SUBSTR, etc.)
- [ ] Version-gated malleability relaxation (`tx.version > 1`)
- [ ] Two-phase unlock/lock script evaluation
- [ ] 32 MB script number limit

## References

- [Chronicle Release (docs)](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release)
- [Chronicle spec (bitcoin-sv-specs)](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/chronicle-spec.md)
