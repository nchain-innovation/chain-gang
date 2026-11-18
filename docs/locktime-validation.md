# nLockTime: field range, finality and admission are separate

Follow-up to #252, found during actual mempool/block validation of #250/#251.

`nLockTime` is a full `uint32_t` field. chain-gang previously rejected values above `INT32_MAX` in stateless transaction validation. That limit does not exist in the pinned BSV v1.2.3 node's structural or standardness checks. The fix removes only that unsupported bound; it does not relax contextual finality or promise mempool/miner acceptance.

## Owning node code

Pinned source `6504a3aff65ba97c0f6c80962b033e35ecbfed4b`:

- [`primitives/transaction.h:276`](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/primitives/transaction.h#L276): unsigned field.
- [`validation.cpp:519–639`](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/validation.cpp#L519-L639): shared structural checks, with no signed locktime cap.
- [`script/standard.cpp:427–500`](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/standard.cpp#L427-L500): standardness rules, also without that cap.
- [`validation.cpp:227–250`](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/validation.cpp#L227-L250): zero locktime is final; otherwise compare height/time using strict `<`; all input sequences `0xffffffff` override an unelapsed locktime.
- [`validation.cpp:1091–1139`](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/validation.cpp#L1091-L1139), [`policy/policy.h:247–255`](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/policy/policy.h#L247-L255): mempool contextual finality and non-final pool handling.
- [`validation.cpp:6039–6078`](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/validation.cpp#L6039-L6078): block finality uses MTP after CSV/BIP113 activation, block timestamp before it.
- [`chainparams.cpp:1046`](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/chainparams.cpp#L1046), [`chainparams.cpp:1470`](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/chainparams.cpp#L1470): CSV/BIP113 mainnet height 419328 versus regtest 576. Mainnet requires standard transactions by default; regtest does not. A low-height permissive regtest result alone must not be extrapolated to mainnet admission.

## Runtime controls

Besides the flexible application-script node, a separate Apple Container node uses `-acceptnonstdtxn=0 -acceptnonstdoutputs=0`, default script/time policies, and height >576. Standard P2PK funding and spending outputs, adequate fees:

- `0x7fffffff`, `0x80000000`, `0xffffffff` with final sequences enter the ordinary mempool and are mined.
- Maximum locktime with sequence 1 enters the non-final pool but its proposed block is rejected. Re-signing with final sequence moves it to the ordinary mempool and it is mined.
- MTP equality/non-final sequence is rejected from a block after BIP113; the all-final control succeeds.
- A nonstandard OP_TRUE output is rejected by the strict policy profile yet accepted in a valid block, checking that standardness was actually enabled.

This is evidence about the pinned node's shared validation code and controlled policy settings, not a mainnet broadcast or a promise that a particular miner accepts the transaction. Fees, script policies, UTXO availability/maturity, conflicts, replacement throttling and operational settings still matter.

`Tx::validate*` checks values, supplied inputs and scripts; it is not a stateful mempool implementation or a complete contextual block validator. In particular a non-final transaction can pass stateless validation and legitimately enter a node's non-final pool. Callers must use the node/appropriate chain context for admission and finality. No replacement policy is inferred solely from a sighash or sequence field.

## Follow-up transaction/script branch audit

The combined branch adds exact `500000000` selector controls, unsigned sequence updates, finalization shape/rate checks, and actual old-UTXO CLTV/CSV probes after both regtest BIP65 and CSV activation. Five-byte operands, negative operands, CSV type/disable/version branches, and post-Genesis NOP behavior are compared with Node. `tests/chronicle_legacy_locks.rs` locks down 89 public transaction API combinations, including mixed input eras. See the combined branch's `docs/transaction-script-branch-audit.md` and receipt archive for execution evidence.

The same limitation applies to `Block::validate*`: its existing partial checks do not include transaction finality and its API lacks previous-block MTP. An audit probe with `height == nLockTime == 600`, sequence 1 and a valid OP_TRUE script unexpectedly passes that API, while Node's `IsFinalTx` rejects equality. This is an existing missing contextual check, not a reason to restore the signed locktime cap or to substitute the block header's time for MTP. The API docs now state this explicitly. Implementing a complete chain/block validator is outside these transaction/script patches.
