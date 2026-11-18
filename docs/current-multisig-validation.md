# Current CHECKSIG / CHECKMULTISIG interaction audit (#254)

This follow-up to #250 uses current Chronicle rules and newly created outputs. No historical activation/UTXO cases were added.

## Confirmed bug, in both directions

CHECKMULTISIG cleans scriptCode using **every supplied signature before matching any key**. A later signature can affect this cleanup even if early termination prevents its encoding or signature from being checked. The node searches for `CScript(signature)`, the complete serialized data push, at opcode boundaries. The library searched for raw signature bytes and skipped empty signatures.

A version-2 example uses locking script `<3001> DROP 2 <K> <K> 2 CHECKMULTISIG NOT` and unlocking script `0 <3001> <current-FORKID-signature>`. The malformed entry is deliberately unvisited if the first pair fails. Node deletes `02 30 01` from scriptCode; the old library deletes nothing:

- Signing the uncleaned code: node's first pair fails, it exits early, and NOT succeeds. Node accepts/mines; old library matches the first pair, reaches the malformed entry and rejects.
- Signing the cleaned code: node reaches the malformed entry and rejects, while the old library incorrectly succeeds through early termination.

Both directions were reproduced through real admission; the positive transaction was mined and the negative candidate block was rejected. Empty-entry variants use a malformed later key instead and prove that the node's cleanup of the empty signature's `OP_0` push also matters.

This is not acceptance of historical signature types: the checked signature uses current FORKID/BIP143 or FORKID/OTDA. The other entry remains subject to strict encoding if it is actually visited. Non-empty FORKID signatures must **not** be deleted, including OTDA signatures.

## Owning source and minimal fix

Pinned BSV Node `6504a3aff65ba97c0f6c80962b033e35ecbfed4b`:

- [CleanupScriptCode](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/interpreter.cpp#L255-L262).
- [CHECKSIG code and NULLFAIL](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/interpreter.cpp#L1460-L1510).
- [CHECKMULTISIG cleanup, ordered matching, early exit and NULLFAIL](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/interpreter.cpp#L1578-L1665).
- [FindAndDelete](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/script.h#L202-L222).

The shared helper now constructs the signature's serialized push, removes exact opcode-boundary occurrences, and treats the empty signature as OP_0. It preserves nonminimal push aliases and matching bytes inside another push. Encoding checks remain lazy: unused keys/signatures are not prevalidated. NULLFAIL, NULLDUMMY, VERIFY behavior and digest algorithms are unchanged. Existing historical regression vectors still run, but no historical feature/test expansion was made.

## Source branches covered by real transactions

- Cleanup for both empty and malformed unvisited entries; both rejection directions; locking and unlocking phases; BIP143 and OTDA, including ANYONECANPAY/SINGLE.
- Repeated pushes, PUSHDATA1-length operands, nonminimal push aliases, nested data bytes and skipped branches. Alias/payload controls must not be deleted.
- First visited key versus unused malformed keys; zero-of-zero and zero-of-invalid-keys; missing dummy; too many/negative signatures; four-byte count limit; early exit skipping a malformed signature; version-1 NULLFAIL includes all supplied signatures after failure.
- CHECKMULTISIGVERIFY cannot turn false into success. Nonempty dummy/nonminimal count controls distinguish current version2 relaxation from version1 policy, with actual consensus-valid block controls.
- Valid mixed BIP143/OTDA multisig signatures and CHECKSIGVERIFY/CHECKMULTISIGVERIFY in either unlock/lock order. Each signature is separately corrupted as a negative control. An executed separator removes prior pushes from unlock-phase scriptCode while that scriptCode still appends the locking script.
- Unpaired `63/e3` constant-digest signatures combined with BIP143 in one CHECKMULTISIG. A constant signature is also pushed inside the locking script: the BIP143 signature must commit to that push. Incorrectly deleting the FORKID push is a negative control.

An exploratory direct five-byte count push was rejected while creating its output by `GetSigOpCount` policy, before the target spend. Final count tests insert a NOP between count push and CHECKMULTISIG so static output scanning does not reject the fixture, while runtime stack/count semantics stay identical. This setup failure is retained separately, not counted as an executed negative spend. See [static counting](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/script.cpp#L30-L115).

## Results and reproduction

**81 admission checks:** 39 ordinary/secondary, 42 rejected. **81 block checks:** 41 accepted/included, 40 rejected. The two additional consensus-valid cases are policy rejections. All expected outcomes pass; script-invalid blocks assert `blk-bad-inputs`. No future locktime can hide a script failure in this matrix: tested spends use locktime0 and final sequences.

This branch: **406 Rust tests passed, 3 ignored**; combined branch: **411 passed, 3 ignored**. Both all-feature/target clippy checks pass, as do 55 Python Chronicle/sighash/transaction tests. The 81 actual transactions and parent outputs are also replayed in ordinary Rust CI, comparing policy and consensus outcomes separately. A minimal public-API regression failed before this fix and passes after it.

Use Apple Container/Rosetta and the pinned image/default profile described in `node-chain-validation.md`. On a fresh node run `current_multisig.py`, then `current_multisig_constant.py` against the same container and CHAIN_OUT. Both use the pinned bsv-script-lab dependency. `multisig_cleanup_probe.py` reproduces the original small case; set `CLEANUP_SIGN_CLEANED=1` for the opposite direction. All outputs are created after height600 with Genesis1/Chronicle200. Script/time settings are default; default nonstandard-output permission remains enabled.

`tests/receipts/node-chain/current-multisig-summary.json` has per-case outcomes/hashes; `current-multisig.json.gz` has raw transactions, parents, candidate blocks and before-fix evidence. `tests/data/chronicle_multisig_node.json` is the Rust replay fixture derived from actual node outcomes. Runtime metadata records the base checkout plus source-file hashes for the patch built before commit. No timeout/startup error counts as a passing result, no mainnet broadcast occurred, and these cases do not prove complete implementation equivalence or universal miner acceptance.
