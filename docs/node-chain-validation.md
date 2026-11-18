# Actual mempool, non-final-pool and block validation

This extends the `verifyscript` differential evidence for #248/#250/#251. It uses actual confirmed funding outputs, `sendrawtransaction`, both transaction pools, mining, and fully serialized PoW-solved candidate blocks sent through `submitblock`. It does not inject script verification flags or synthetic UTXOs into the node.

## Reproduction and scope

The complete integration harness requires the combined code from #248, #250 and #251, including the #252 locktime fix. #250 alone lacks the explicit chain/UTXO context API used by the boundary phase. The reproducible combined branch is `linzhm/chain-gang:test/chronicle-integration`; the receipt metadata records the library snapshot actually built.

Install that checkout's Python package in a Python >=3.12 environment. Set `BSV_SCRIPT_LAB` to a checkout of `linzhm/bsv-script-lab` at **f8f184bb6a35139ce8d1971a6f514cffc666e4d3**. The harness checks the commit. Use Apple Container with Rosetta; no Docker daemon is used. Each profile requires a **fresh task-owned node**, with no public P2P connections or published RPC ports.

```sh
export BSV_SCRIPT_LAB=/path/to/pinned/bsv-script-lab
export BSV_CONTAINER=cg-chain-your-unique-name
export CHAIN_OUT=/absolute/path/to/results
BSV_PROFILE=flexible sh scripts/node-chain/launch.sh
# Wait until bitcoin-cli getblockcount succeeds, then:
python scripts/node-chain/run_all.py
```

Repeat on separate fresh containers with `BSV_PROFILE=standard`, `default`, or `rate`, running `standard_policy.py`, `default_policy.py`, or `replacement_rate.py` respectively. Use a separate CHAIN_OUT for each. Test-only RPC credentials are local to these containers. Stop the task-owned containers when done; no script deletes existing containers/data.

All profiles pin Node v1.2.3 (`6504a3a`) and image digest `d96fe4786a00e5fc10bc9e0e76a6006376f9472d4eb74d07e4c1e0d019493d33`. Genesis is configured at 1, Chronicle at 200; the node's separate regtest CSV/BIP113 height is **576**, not Chronicle's height. Mining fee is explicitly 0.000005 BSV/kB. Private-chain activation and regtest PoW/coinbase parameters are not mainnet conditions.

| Profile | Purpose and policy |
|---|---|
| flexible | Full boundary/application/sequence matrix; explicit generous script/time limits; accepts nonstandard transactions |
| standard | `acceptnonstdtxn=0`, `acceptnonstdoutputs=0`, otherwise default script/time policy; standard P2PK controls; height >576 |
| default | `acceptnonstdtxn=0`, default nonstandard-output permission and script/time limits; complete `63/e3` application spends at height >576 |
| rate | Explicit non-final replacement cap of two updates per 60 minutes; independent of the default cap; shows throttle rejection without changing block validity |

Even the default-profile experiment does not establish acceptance by every real miner. Script/time limits, fee configuration, network state and operator policies can differ. There was no mainnet broadcast or use of real funds.

## Coverage

- Before/at/after Chronicle: BIP143 control, OTDA `61`, complete companion `63/e3`, old/new confirmed UTXOs, old disabled opcode versus new opcode in a version-1 spend, and old NOP policy/consensus distinction.
- All twelve paired signature types: valid full application spend is accepted and mined; stale signature after changing its own sequence is rejected both by admission and explicit block submission.
- Complete unpaired `63/e3`: wrong R1 witness, changed locktime/own sequence/parent, and adding a paired output all rejected; uncommitted other-input sequence changes remain valid and are mined.
- Non-final-pool updates: unchanged/decreased sequences, mixed increase/decrease, reordered/added/removed inputs, monotone updates, partial and complete finalization.
- Final ordinary transactions, including low sequence + zero locktime, are not replaced merely by fee or sequence increases. An otherwise valid conflicting transaction can nevertheless be accepted in a valid block; pool preference is not consensus invalidity.
- Signed `63/e3` updates: changing a committed sequence without re-signing fails; changing the uncommitted other sequence still must satisfy pool monotonicity; re-signed updates and finalization succeed.
- Height and timestamp lock boundaries, full uint32 nLockTime, final sequence override, and BIP68 sequence-bit probes **both before and after CSV activation**. Post-Genesis BIP68 is not inferred from a low-height regtest where CSV is inactive.
- Zero-fee transaction under zero relay minimum can enter the secondary pool but is not selected at the configured mining fee; direct consensus-valid block inclusion succeeds.

Each record captures transaction/parents, RPC response, both pool snapshots, local library result and chain height. Block cases retain candidate hex, hash, active-tip transition and confirmations. Rejected blocks must leave chain tip and UTXO state unchanged; accepted blocks must spend the actual inputs. Node-accepted ordinary/non-final transactions must pass local stateless validation. Mempool policy failures need not be library consensus failures.

## Recorded results

Across the four profiles: **105 admission checks** (46 in the ordinary/secondary mempool, 18 in the non-final pool, 41 rejected), and **86 block checks** (51 accepted/included, 35 rejected). Funding/bootstrap blocks are not counted. Four additional environment/lifecycle observations are recorded separately.

The combined Rust suite passes **404 tests, 0 failures, 3 ignored**; all-feature/target clippy passes. #251 alone passes **399 tests, 0 failures, 3 ignored**, including the new 24-combination unsigned-locktime/sequence regression. That regression failed on the prior code before the fix. The prior script-level matrix remains enabled; this adds real admission/block coverage rather than replacing it.

`tests/receipts/node-chain/summary.json` contains per-case results and hashes; `full-receipts.json.gz` contains the complete raw transactions, parents and PoW-solved block candidates. Policy-only rejection and consensus rejection are separately labeled by the recorded node reason. The archive also retains the pre-fix strict-policy locktime evidence.

## Findings and corrected expectations

1. **Library bug #252:** nLockTime was incorrectly capped at INT32_MAX. The shared node structure/standardness/finality code supports uint32; the strict-policy post-BIP113 profile independently accepts and mines the high-bit/final-sequence controls. #251 removes only the unsupported cap and documents stateless-validation limits. See its `docs/locktime-validation.md` for exact owning source links.
2. **MTP is not interchangeable with block time before BIP113.** At low regtest heights, pool policy uses MTP while block finality still uses header time. The harness explicitly covers both sides of regtest height 576. This is not a Chronicle bug or proof of different mainnet behavior today.
3. **Zero relay fee is different from minimum mining fee.** The initial assumption that zero-fee admission must fail was wrong for the configured node. Secondary-pool admission, mining omission and direct block acceptance are separately asserted.
4. **Non-final pool snapshots can retain stale entries.** In v1.2.3, direct block inclusion/conflict can leave an old non-final entry after the input is spent. The harness records that behavior and verifies real chain UTXOs; it does not equate a pool listing with a spendable input. `time_locked_mempool.cpp:611–665` maintains maturity/expiry asynchronously; immediate emptiness is not assumed. This is recorded node lifecycle behavior, not a chain-gang discrepancy exemption or a node patch.

Final receipts and a compressed archive of raw transactions/candidate blocks accompany this document. They supersede exploratory runs with incorrect test assumptions. No timeout, RPC failure or unexamined discrepancy is counted as acceptance or consensus rejection. These finite cases establish the listed behaviors of the pinned node, not complete consensus/mempool implementation equivalence.
