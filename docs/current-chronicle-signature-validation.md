# Current Chronicle signature-result boundary (#253)

This follow-up to #250 tests current transactions and newly-created post-Chronicle outputs only. It does not extend historical compatibility or implement a node's pool/chain machinery.

## Confirmed discrepancies

For a version-2 transaction spending `<valid compressed key> CHECKSIG NOT`, the canonical signature `300602010002010161` has R=0, S=1 and OTDA/FORKID ALL type 0x61. Node v1.2.3 accepts and mines it: verification returns false, Chronicle relaxes NULLFAIL for this version, and NOT returns true. chain-gang previously propagated a k256 scalar-parsing error, rejecting the transaction. The same issue affects CHECKMULTISIG, zero S, out-of-range scalars and canonically shaped off-curve keys.

Conversely, an empty signature previously returned false before checking the public-key encoding. `CHECKSIG NOT` with an empty signature and a malformed key could therefore pass the library while the node rejected it. Both directions were reproduced; regression tests fail on the preceding library.

Owning source (node commit `6504a3aff65ba97c0f6c80962b033e35ecbfed4b`):

- [DER predicate](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/interpreter.cpp#L177-L247): encoding allows zero or mathematically out-of-range positive scalars.
- [Encoding and version gates](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/interpreter.cpp#L265-L326): strict DER/hash-type/key shape checks remain; Chronicle does not waive them.
- [Signature verification](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/interpreter.cpp#L2125-L2162): mathematical failure returns false.
- [OTDA routing](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/interpreter.cpp#L2084-L2122): FORKID without Chronicle uses BIP143; Chronicle uses the original digest algorithm.

## Minimal repair

Check canonical DER and public-key shape independently of the cryptography backend. After successful encoding checks, backend rejection of a scalar/point is a false verification result. The interpreter still enforces CHECKSIGVERIFY/CHECKMULTISIGVERIFY, version-1 NULLFAIL and final stack results. Transaction and precomputed-digest checkers share the predicate. Historical no-FORKID transaction-checker behavior is preserved.

The change does not accept invalid signatures as true, relax malformed encoding, or change transaction finality. A new control has a successfully evaluated CHECKSIG NOT script but an unelapsed nLockTime: it enters the non-final pool, its block is rejected, and the same transaction becomes includable after height advancement.

## Real-node tests

Apple Container / Rosetta, pinned v1.2.3 image from `node-chain-validation.md`; fresh `default` profile, Genesis1/Chronicle200, all test outputs created after height600, default script/time settings, `acceptnonstdtxn=0`. Default nonstandard-output permission remains enabled; this is not the strict-output profile and not a guarantee about every miner's policy.

Run `current_rules.py` with the usual BSV_CONTAINER / BSV_SCRIPT_LAB / CHAIN_OUT environment after `BSV_PROFILE=default sh scripts/node-chain/launch.sh`. The smaller `current_signature_probe.py` reproduces the original difference against an unfixed extension. The complete matrix covers:

- canonical zero/out-of-range R/S, wrong mathematical signatures, off-curve keys;
- CHECKSIG/CHECKMULTISIG versus their VERIFY forms, v1 NULLFAIL controls;
- malformed DER, incorrect key lengths/prefixes, empty signatures, undefined hash type and missing FORKID;
- six OTDA types (`61/62/63/e1/e2/e3`) in complete companion spends with two inputs/two outputs: stale locktime signature rejection, other-input sequence commitments, re-signed non-final updates, finalization and mining;
- the separate nLockTime finality boundary described above.

**77 admission checks:** 23 ordinary/secondary, 18 non-final, 36 rejected. **55 block checks:** 25 accepted/included, 30 rejected. All assertions pass. Script-negative block checks assert `blk-bad-inputs`; the unelapsed lock control asserts `bad-txns-nonfinal`. Only the separate zero-lock script cases establish the script rejection reason; future-lock stale-signature cases are not claimed as independent block-level signature tests.

Rust #250: **404 passed, 0 failed, 3 ignored**. Combined implementation: **409 passed, 0 failed, 3 ignored**. Both all-feature/target clippy checks pass; 55 Python Chronicle/sighash/transaction regressions pass. The original two focused Rust failures and node-before-fix receipt are retained in the archive.

`tests/receipts/node-chain/current-rules-summary.json` contains individual results and hashes; `current-rules.json.gz` contains transactions, parents and candidate blocks. It records the built library's base commit plus checker-file hash, since the runtime used the combined source with this patch applied before committing. No timeout or startup error is counted as validation success. No mainnet broadcast, historical-test expansion or full-node equivalence claim.
