# OTDA complete-spend differential validation

## Why the earlier digest-only fix was insufficient

PR #240 correctly returned Node's `uint256(1)` for unpaired OTDA SINGLE. Nevertheless, the complete `0x63`/`0xe3` companion spends in [bsv-script-lab CI](https://github.com/linzhm/bsv-script-lab/actions/runs/44369295614/job/135031389747) failed. Replaying its exact commit `f8f184bb6a35139ce8d1971a6f514cffc666e4d3` against #240 (`ffa7f0c`) reproduces both failures while BSV Node accepts both.

The companion path also verifies a BIP-143 signature. Its executed separator leaves scriptCode `63ad67ab68ac`, including an **unexecuted** separator. Chronicle's two-phase evaluator deleted that separator before the digest algorithm could choose how to handle it. BIP-143 must preserve it; OTDA's serializer deletes opcode separators itself. This is the same defect tracked in #245 and fixed in #249, not a new constant-digest byte-order defect.

The replacement combines the digest/API/wallet protection from #240 and scriptCode/checker corrections from #249. Keeping the earlier PR ready independently was not sufficient for the application-level acceptance criterion. Neither branch history is rewritten.

## Coverage and results

Real BSV Node v1.2.3, source `6504a3aff65ba97c0f6c80962b033e35ecbfed4b`, image `docker.io/bitcoinsv/bitcoin-sv@sha256:d96fe4786a00e5fc10bc9e0e76a6006376f9472d4eb74d07e4c1e0d019493d33`, running in Apple Container with Rosetta, isolated regtest and no public P2P connections:

- **1,236/1,236** commitment cases match Node, Rust and independent expected verdicts: **544 positive, 692 negative**.
- Twelve types (`41/42/43/c1/c2/c3/61/62/63/e1/e2/e3`), input positions 0/1, CHECKSIGVERIFY in unlocking/locking phases, mixed OTDA/BIP-143 CHECKMULTISIGVERIFY, executed/skipped separators and pushed `ab` data.
- Mutations of paired/unpaired output amount and script, appending an output, own/other input outpoint and sequence, other unlocking script, previous-output amount, version, locktime and signature. Missing paired outputs are included for all four SINGLE types.
- Complete application script: **88/88** matching checks (**49 positive, 39 negative**) across seven functional tests. Includes paired twelve-type authorization, unpaired companion flows, mixed inputs, ordinal/size constraints, malformed P-256 scalars/inverse/point fields and wallet-generated witnesses. Timing-only test is excluded.
- The exact two full companion transactions and parent transactions are vendored in `tests/data/otda_companion.json`. Rust regressions exercise success, unrelated-output mutation, adding a paired output, and altered locktime.
- Existing digest/preimage/wallet-policy boundary tests and 1,000 source-node sighash vectors remain enabled. Constant digest has no serialized preimage; default wallet transaction signing still refuses it before mutation.

`tests/receipts/otda-complete-node.json` records the pinned node, original failures, counts, per-case request hash and actual verdict. Generated full requests/responses can be regenerated with the scripts below. No mismatch allowlist; timeout/skipped/RPC failure does not count as rejection.

## Reproduce

Use Apple Container to start the pinned image with `--arch amd64 --rosetta`, an isolated regtest datadir, no P2P peers and no published RPC ports. The application scripts need more than the default 3ms standard validation budget. Our node uses:

```
-regtest -server -listen=0 -dnsseed=0 -discover=0 -connect=0
-rpcuser=cgtest -rpcpassword=local-regtest-only
-genesisactivationheight=1 -chronicleactivationheight=200
-excessiveblocksize=100000000 -minminingtxfee=0.000005
-maxstackmemoryusageconsensus=1000000000 -maxstackmemoryusagepolicy=1000000000
-maxscriptnumlengthpolicy=0 -maxscriptsizepolicy=0 -maxtxsizepolicy=100000000
-maxstdtxvalidationduration=10000 -maxnonstdtxvalidationduration=20000
-maxtxnvalidatorasynctasksrunduration=30000
```

The script RPC requests explicitly set Chronicle/UTXO flags. Activation is intentionally not inferred from this regtest tip.

```sh
CHRONICLE_DIFF_OUTPUT=/tmp/otda-matrix.json cargo test --test otda_commitment_matrix
BSV_CONTAINER=cg-chronicle-context-v3 python3 scripts/chronicle-node-diff.py /tmp/otda-matrix.json /tmp/otda-results.json
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
```

Install this checkout's Python package into a Python >=3.12 virtual environment. Check out `linzhm/bsv-script-lab` at the pinned commit above, then:

```sh
BSV_CONTAINER=cg-chronicle-context-v3 python scripts/otda-lab-diff.py fixed /tmp/application-results.json --lab /path/to/bsv-script-lab
```

For the historical comparison, install #240 into a **different** virtual environment and run the same script with `baseline`; it checks four unpaired types and records Rust rejection/Node acceptance for `63/e3`. The fixture uses synthetic transactions and test-only keys, not real funds.

## Limits

This validates real signatures and application scripts against `verifyscript` with supplied UTXOs and explicit flags. It does **not** demonstrate block acceptance, mempool admission, activation-height selection, all script flags, or all Chronicle semantics. Context/activation and number-limit defects (#241/#246) and restored opcodes (#242–#244) remain separate changes. A finite matrix is not a proof of full consensus equivalence.

The differential runners also require returned effective flags to equal explicitly requested flags. Explicit-flag application cases omit `txo.height`, which would otherwise override the UTXO-era flag on Node. Height-derived tests intentionally omit flags instead.
