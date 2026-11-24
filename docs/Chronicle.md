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

## Signature hashing and signing

The signature hash type selects the digest algorithm. `SIGHASH_FORKID` without
`SIGHASH_CHRONICLE` uses BIP143. A hash type with `SIGHASH_CHRONICLE` uses
the Original Transaction Digest Algorithm (OTDA). The legacy no-FORKID path
also uses the original algorithm. Chronicle signature-type availability depends
on activation, so a digest helper alone does not establish that a spend is valid.

Rust exposes `SIGHASH_CHRONICLE` and `uses_low_s_signing` through
`chain_gang::chronicle`. Python exposes `SIGHASH.CHRONICLE` and
`SIGHASH.ALL_FORKID_CHRONICLE`. See the [Python signing example](Chronicle-Python.md#signing-with-otda-chronicle-sighash).

Signing normalization and verification are distinct. The signing helper uses
the hash type to select its low-S policy. Verification applies the active
transaction and output rules, including the signed transaction version. An
external signature must satisfy the rules of the spending context.

## Script execution and opcodes

Unlock and lock scripts execute in separate phases, carrying the main stack
from unlock to lock while each phase has its own alternate stack and conditional
state. Functional unlocking scripts and malleability relaxations depend on the
active rules and signed transaction version. The explicit context API selects
the spent output era independently of the transaction version.

Chronicle restores `OP_2MUL`, `OP_2DIV`, `OP_VER`, `OP_VERIF`,
`OP_VERNOTIF`, `OP_SUBSTR`, `OP_LEFT`, `OP_RIGHT`, `OP_LSHIFTNUM`, and
`OP_RSHIFTNUM` for outputs created in the appropriate era. `OP_VER` provides
the transaction version as four little-endian bytes. The version conditionals
compare those bytes to the stack operand, with `OP_VERNOTIF` inverting the
result. The node and tests define the behavior of inactive branches and older
outputs.

`max_script_num_length`, `MAX_SCRIPT_NUM_LENGTH_GENESIS`, and
`MAX_SCRIPT_NUM_LENGTH_CHRONICLE` are available from `chain_gang::chronicle`.
For the explicit context API the consensus number limit is selected from the
spent output era, rather than from the spending transaction version.

## Activation helpers and debugging

`activation_height` returns the configured library default for a BSV network.
`chronicle_rules_active` and `effective_chronicle_tx_version` are legacy
version-oriented helpers, useful to existing callers that understand their
assumptions. Use the explicit validation context to evaluate a real spend with
mixed output eras. `TxVersionChecker` and `ZVersionChecker` support script
debugging without a complete transaction or UTXO history.

## Verification and remaining work

Regression cases cover active/inactive Chronicle, old/new outputs, versions 1/2/negative, signature-type activation and numeric boundaries. Live `verifyscript` receipts and reproduction are in [Chronicle context validation](chronicle-context-validation.md). The node was also advanced across a configurable regtest activation, so the activation checks use node-derived flags rather than only injected flags.

This context change does not itself fix restored-opcode byte semantics (#242–#244, PR #248), scriptCode/complete OTDA execution (#239/#245, PR #250), or all historical interpreter/policy differences. Passing the tests does not establish full node equivalence.

## References

- [Chronicle release documentation](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release)
- [Chronicle specification](https://github.com/bitcoin-sv-specs/protocol/blob/master/updates/chronicle-spec.md)
- [Python Chronicle guide](Chronicle-Python.md)
