# Chronicle context and number-limit validation

Fixes #241 and #246 independently of the restored-opcode and complete-OTDA changes. Defaults are taken from released BSV Node v1.2.3, not the older draft bitcoin-sv-specs heights. The rule-selection API is documented in [Chronicle.md](Chronicle.md).

## Actual node evidence

Apple Container, amd64/Rosetta, pinned `docker.io/bitcoinsv/bitcoin-sv@sha256:d96fe4786a00e5fc10bc9e0e76a6006376f9472d4eb74d07e4c1e0d019493d33` (BSV v1.2.3 / `6504a3a`). No public P2P peers or published RPC ports.

- **39/39** explicit-context differential cases: active/inactive chain, old/new outputs, signed versions 1/2/-1, restored-opcode activation, old NOP behavior, Chronicle signature availability and low/high-S verification.
- **39/39** cases with **no caller-supplied script flags**, rerun on 2026-11-24: mined the isolated regtest to height 201 with Chronicle configured at 200; supplied parent block hashes at heights 198/199 and output heights 100/200. Node derived the activation/output flags. All nine `nop-left` cases are retained: old outputs treat OP_LEFT as a NOP, while new outputs execute it and reject the insufficient stack. With `prevblockhash`, the RPC derives block flags, then applies policy resource limits; those limits do not add `DISCOURAGE_UPGRADABLE_NOPS` ([v1.2.3 RPC source](https://github.com/bitcoin-sv/bitcoin-sv/blob/6504a3aff65ba97c0f6c80962b033e35ecbfed4b/src/rpc/misc.cpp#L726)).
- **12/12** exact numeric boundary cases: 749,999/750,000/750,001 bytes for old outputs, 31,999,999/32,000,000/32,000,001 bytes for new outputs, each with spending versions 1 and 2. Operands are constructed by small scripts at runtime, so this is real boundary execution, not a comparison of constants.
- Rust also tests missing/future creation metadata, same-block output heights, explicit numeric policy vs consensus, preservation of negative serialized versions, and context forwarding through block validation. Python tests exercise the actual built extension's context API and released defaults.

Actual request/response receipts are in `tests/receipts/chronicle-{context,height-derived,number}-node.json`. No mismatches are allowed. Timeout/skipped/RPC errors fail comparison.

The 39 height-derived spends were also compared on 2026-11-24 with the executable
[bsv-script-lean specification at `1c051ea`](https://github.com/Calhooon/bsv-script-lean/tree/1c051eaef1077cc95e5d1618686d5efacc4aa097).
Acceptance, error descriptions and complete effective flag words all matched.
To reproduce the RPC parameter selection in its CLI, first derive block flags
from the spend/UTXO heights with `--consensus true`, then execute that derived
word with `--flags` and `--consensus false` for policy resource limits. Both
steps use `--config sv-regtest --genesis-height 1 --chronicle-height 200`.
The node's returned flags are comparison outputs, not inputs to that derivation.

## Reproduction

Use the pinned node image under Apple Container with regtest, `-genesisactivationheight=1 -chronicleactivationheight=200`, `-listen=0 -dnsseed=0 -discover=0 -connect=0`, and test-only RPC credentials `cgtest:local-regtest-only`. For boundary tests configure:

```
-excessiveblocksize=100000000 -minminingtxfee=0.000005
-maxstackmemoryusageconsensus=1000000000 -maxstackmemoryusagepolicy=1000000000
-maxscriptnumlengthpolicy=0 -maxscriptsizepolicy=0 -maxtxsizepolicy=100000000
-maxstdtxvalidationduration=10000 -maxnonstdtxvalidationduration=20000
-maxtxnvalidatorasynctasksrunduration=30000
```

```
CHRONICLE_DIFF_OUTPUT=/tmp/context.json cargo test --test chronicle_context
BSV_CONTAINER=cg-chronicle-context-v3 python3 scripts/chronicle-node-diff.py /tmp/context.json /tmp/context-result.json
CHRONICLE_LARGE_BOUNDARIES=1 CHRONICLE_DIFF_OUTPUT=/tmp/numbers.json cargo test --test chronicle_number_limits
BSV_CONTAINER=cg-chronicle-context-v3 python3 scripts/chronicle-node-diff.py /tmp/numbers.json /tmp/numbers-result.json
```

For height-derived verification, advance this **private test node** to height 201 (e.g. `bitcoin-cli generate 201` from a fresh datadir), then:

```
BSV_CONTAINER=cg-chronicle-context-v3 python3 scripts/chronicle-height-cases.py /tmp/context.json /tmp/heights.json
BSV_CONTAINER=cg-chronicle-context-v3 python3 scripts/chronicle-node-diff.py /tmp/heights.json /tmp/heights-result.json
```

## Scope

Mining here establishes real activation context; synthetic test spends are still checked through `verifyscript` with supplied outputs. We did not mine these spending transactions or establish mempool admission. Genesis metadata/configuration remains an explicit existing responsibility. The old context-free API necessarily assumes output eras; callers needing fidelity must use complete metadata.

The vector adapter models `DISCOURAGE_UPGRADABLE_NOPS` independently of other policy flags. Rows 1093–1097 exercise SUBSTR, LEFT, RIGHT, LSHIFTNUM and RSHIFTNUM before Chronicle restores them for newly created outputs. The 40-case replay of these five opcodes covers transaction versions 1 and 2, both output eras and both settings of the policy flag. All 40 matched BSV Node v1.2.3 under isolated Linux Docker regtest with explicit flags.

```sh
CHRONICLE_NOP_DIFF_OUTPUT=/tmp/nop.json cargo test --test script_vectors discourage_nops_flag_uses_each_utxo_creation_height
BSV_CONTAINER_RUNTIME=docker BSV_CONTAINER=cg-chain-review-node python3 scripts/chronicle-node-diff.py /tmp/nop.json /tmp/nop-result.json
```

The differential runners also require returned effective flags to equal explicitly requested flags. Explicit-flag application cases omit `txo.height`, which would otherwise override the UTXO-era flag on Node. Height-derived requests omit flags so Node selects the rules. The runner checks the returned chain-activation and UTXO-era bits against the two expectations saved alongside each request. Run `python3 -m unittest discover -s python/src/tests -p test_chronicle_replay.py` to check retention of all 39 cases (including nine NOP cases), replay across the 32-request batch boundary, and rejection of missing or flipped Chronicle bits without a node.

The activation-boundary matrix exports 180 additional requests covering spending
heights before, at and after activation, valid execution on new outputs, and the
expected node error for rejected scripts.

```sh
CHRONICLE_NOP_BOUNDARY_DIFF_OUTPUT=/tmp/nop-boundary.json cargo test --test script_vectors discourage_nops_flag_at_chronicle_activation_boundary
python3 scripts/chronicle-node-diff.py /tmp/nop-boundary.json /tmp/nop-boundary-result.json
```
