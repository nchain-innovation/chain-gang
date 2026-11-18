# Chronicle validation context

BSV Node v1.2.3 selects rules from three different facts. Transaction version alone is insufficient.

| Rule | Selection |
|---|---|
| SIGHASH_CHRONICLE availability | Spending block has activated Chronicle |
| Relaxed malleability/high-S | Chronicle active **and signed int32 transaction version > 1** |
| Restored opcodes | Spent output created at/after Chronicle activation |
| Script-number consensus limit | Spent output era: 4 bytes before Genesis, 750,000 after Genesis, **32,000,000** after Chronicle |
| Script-number policy limit | Explicit node policy; 0 selects the era's consensus limit |

The released node's defaults are mainnet **943,816**, testnet **1,713,168**, STN **250**, regtest **15,000**. Private chains may override activation. These replace obsolete draft-spec values; regtest is not automatically active at height zero. Sources: [chainparams](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/chainparams.cpp), [numeric policy](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/configscriptpolicy.cpp), [interpreter](https://github.com/bitcoin-sv/bitcoin-sv/blob/v1.2.3/src/script/interpreter.cpp).

## Rust API

Use `chronicle::ChronicleValidationContext` with `Tx::validate_with_chronicle_context`. It contains the spending `block_height`, configured `activation_height`, complete `utxo_heights: &HashMap<OutPoint, u64>`, and optional `script_num_policy_limit`. Pass existing Genesis/FORKID selection and pre-Genesis output metadata separately. `consensus_only=true` ignores the numeric policy limit.

Missing output creation heights and heights later than the spending block are errors. A same-block output may have the spending height. `Block::validate_with_chronicle_context` forwards this context and checks that its spending height matches the supplied block height; the caller must include creation metadata for spent same-block outputs too. Existing block validation responsibilities and limitations are unchanged.

The actual transaction version is preserved for OP_VER and signature hashing. Disabling Chronicle does not rewrite the serialized version, and does not reject a transaction solely because its version is 2.

## Compatibility APIs

- `Tx::validate` / `validate_consensus` assume Chronicle is active and post-Genesis outputs are post-Chronicle. Malleability still uses the signed transaction version.
- `validate_at_height` and its consensus counterpart enforce the network activation default but assume post-Genesis outputs belong to the spending era. They cannot distinguish old/new outputs after activation.
- Context-free `Context`, `TxVersionChecker`, `uses_two_phase_eval`, `chronicle_rules_active`, and `effective_chronicle_tx_version` retain legacy version-oriented helper behavior. They are not complete activation/UTXO-era selectors. The effective-version helper must not be used as a serialized OP_VER value.
- `None` for numeric policy means consensus-sized numbers, not the running node's mempool default. Set the node's configured numeric policy explicitly when comparing policy behavior. Other node policies are not all modeled by this library.

These assumptions are retained for callers without metadata; use the explicit API for historical or mixed-era spends. The node RPC `ScriptFlags` bitmask is separate from local interpreter flags.

## Verification and remaining work

Regression cases cover active/inactive Chronicle, old/new outputs, versions 1/2/negative, signature-type activation and numeric boundaries. Live `verifyscript` receipts and reproduction are in [Chronicle context validation](chronicle-context-validation.md). The node was also advanced across a configurable regtest activation, so the activation checks use node-derived flags rather than only injected flags.

This context change does not itself fix restored-opcode byte semantics (#242–#244, PR #248), scriptCode/complete OTDA execution (#239/#245, PR #250), or all historical interpreter/policy differences. Passing the tests does not establish full node equivalence.
