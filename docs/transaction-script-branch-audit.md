# Transaction/script branch audit against BSV Node v1.2.3

Follow-up review of #248/#250/#251. Owning source is pinned at `6504a3aff65ba97c0f6c80962b033e35ecbfed4b`, not inferred from passing library tests. Actual execution uses Apple Container, the same image digest as `node-chain-validation.md`, and the combined library snapshot `e129e5a0a914614473556bab3f25906d55a4a680`.

Scope is transaction and script behavior. Blocks and pools serve as test oracles; this does not implement a node's chain manager, network relay, scheduler, or full admission policy inside chain-gang.

## Source branches and corresponding additional probes

All paths below are in https://github.com/bitcoin-sv/bitcoin-sv/tree/v1.2.3/src .

| Owning branch | Previously missing probe | New assertion |
|---|---|---|
| `validation.cpp:227–250` IsFinalTx | Exact 500000000 height/time selector | 499999999 + non-final sequence stays non-final; 500000000/500000001 are elapsed timestamps; final-sequence control succeeds |
| `time_locked_mempool.cpp:517–590` | Unsigned sequence high bit | 0x7fffffff -> 0x80000000 succeeds; backward fails |
| `txmempool.cpp:2257–2283`, `time_locked_mempool.cpp:114–157` | Lowering locktime into a final transaction without all-final inputs | Zero/elapsed locktime with increased but non-final sequence is a pool conflict; all-final exact-input update succeeds |
| `time_locked_mempool.cpp:33–97` | More than one old non-final transaction | Combined update rejects, separate finalizations succeed |
| `validateUpdateNL` and finalization fast path | Final candidate with subset/reordered/superset inputs | Reject with specific bad-txn-update or conflict reason, old entry retained; exact-input finalization succeeds |
| `validation.cpp:952–965, 1133–1138` | Non-final child versus final child of non-final parent | Explicit chain rejection versus missing inputs; finalized parent + child package succeeds |
| `time_locked_mempool.cpp:517–590` | Replacement throttle at finalization | All-final candidate still fails exhausted rate check, although its valid block succeeds |
| `validation.cpp:1120–1125` | Non-final pool disabled | Admission rejects non-final transaction; same transaction after height maturity is admitted/mined |
| `validation.cpp:2667–2706` | Unconfirmed / same-block output creation era | Parent/child package fails before activation, succeeds at activation; child-before-parent rejects; mixed old/new inputs recompute flags independently in either order |
| `validation.cpp:4036–4055` | Reusing cached interpretation across boundary | Candidate-height crossing clears old pending parent; rollback restores old script rejection; reconsideration restores valid activation package |
| `script/interpreter.cpp:520–594` | Old CLTV/CSV after Chronicle versus new-output NOP | Insufficient/negative/final-sequence old cases fail, satisfied old cases succeed; negative operands on new outputs are consensus-valid but policy-discouraged |
| `script/interpreter.cpp:2166–2250` | Five-byte operands and CSV special branches | CLTV high-bit equality is script-valid/non-final; CSV type mismatch, version 1, disabled input fail; wide masked operand and disabled operand succeed |

`branch_audit.py`, `legacy_locks.py`, `disabled_nonfinal.py`, and the extended `replacement_rate.py` contain the executable cases. Source-specific policy negatives assert RPC reason substrings, not merely absence from the pool. Finality negatives assert `bad-txns-nonfinal`; old-script block negatives assert `blk-bad-inputs`. Block tip/UTXO preservation and actual transaction inclusion remain checked.

## Reproduction

Use fresh containers with the `audit`, `legacy`, `disabled`, and `rate` profiles respectively. Run the corresponding script with `BSV_CONTAINER`, `BSV_SCRIPT_LAB`, and a distinct `CHAIN_OUT`, as in `node-chain-validation.md`. The launcher waits for RPC readiness; startup errors are not validation outcomes.

- `audit`: Genesis 1, Chronicle 200, non-final periodic check 1000 ms. Includes one bounded maturity observation; scheduler timing is not treated as script consensus.
- `legacy`: Genesis **150**, Chronicle 200. Fund old outputs below 150 and spend above regtest CSV 576 and BIP65 1351. Mark old outpoints explicitly in the library context. This avoids erroneously proving CLTV/CSV behavior while they are inactive.
- `disabled`: non-final pool limit zero, otherwise the existing private-chain activation settings.
- `rate`: explicit two-updates-per-60-minutes cap, not the default rate.

The final receipt archive is `tests/receipts/node-chain/branch-audit.json.gz`; its adjacent summary provides counts and hashes. Earlier diagnostic runs are excluded from passing totals (one startup failure and one test's misspelled expected block rejection string were corrected).

## Findings and limits

No additional transaction/script disagreement was found in these executed branches. #251 now adds 89 public-API regression combinations: 17 lock-opcode cases across five spending/UTXO contexts, plus four mixed-input cases. Consensus script validation is intentionally distinguished from node finality and policy.

The source audit did identify an existing API documentation gap: `Block::validate*` does not check transaction nLockTime finality and has no previous-block MTP parameter. A focused probe expecting rejection at height == locktime failed on the library, while the node rejects that boundary. #251 now documents this limitation explicitly; its Chronicle context selects script rules and does **not** make the API a complete block-acceptance validator. No inferred timestamp substitution or unrelated block-validator rewrite was introduced.

Replacement-rate decay over wall time, persistence/restart, expiry, peer relay and broad reorganization behavior are outside this transaction/script audit. The bounded activation rollback and maturity observations above do not establish complete coverage of those systems. Real miner acceptance is still configuration-dependent; no mainnet transactions were sent.

## Recorded results

This audit run records **68 admission checks** (20 ordinary/secondary, 14 non-final, 34 rejected) and **44 block checks** (24 accepted/included, 20 rejected), plus five environment/lifecycle observations. These are this run's action counts; the rate profile reruns existing controls and is not counted as wholly new unique coverage. #251 Rust: **401 passed, 0 failed, 3 ignored**, clippy passed.
