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

## Implementation status

- [x] OTDA sighash routing (`SIGHASH_CHRONICLE`)
- [x] OTDA preimage for Chronicle signatures
- [ ] Chronicle opcodes (OP_VER, OP_SUBSTR, etc.)
- [ ] Version-gated malleability relaxation (`tx.version > 1`)
- [ ] Two-phase unlock/lock script evaluation
- [ ] 32 MB script number limit

## References

- [Chronicle Release (docs)](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release)
- [Chronicle spec (bitcoin-sv-specs)](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/chronicle-spec.md)
