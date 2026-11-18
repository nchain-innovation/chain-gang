# Chronicle validation from Python

Use `Tx.validate_with_chronicle_context` when output creation era or activation matters. It wraps the same Rust validation as the explicit-context API.

```python
spending_tx.validate_with_chronicle_context(
    parents,
    block_height=200,
    activation_height=200,  # match your node's configuration
    utxo_heights=[(parent.id(), 0, 100) for parent in parents],
    consensus_only=True,
)
```

The height tuples are `(display_txid_hex, output_index, creation_height)` and must cover every spent outpoint. Missing/future heights raise `ValueError`. For pre-Genesis outputs also pass `pregenesis_outpoints=[(txid, index), ...]`. For policy checks use `consensus_only=False` and `script_num_policy_limit=<node configuration>`; 0 selects the era consensus limit. Other node policy settings are not all represented by this library.

Activation, output era and version are independent. Chronicle signature types require activation; restored opcodes and the 32,000,000-byte numeric limit depend on output creation era; relaxed malleability additionally requires a **signed** version greater than 1. See [rule selection](Chronicle.md).

`Tx.validate(parents)` assumes active Chronicle and post-Chronicle post-Genesis outputs. `validate_at_height(parents, height, network)` uses the default activation but cannot infer historical output creation heights. `Context(tx_version=...)` remains a context-free debugging interface. None is a substitute for explicit historical metadata.

The helpers in `tx_engine.interface.verify_script` expose released-node default heights: mainnet 943,816; testnet 1,713,168; STN 250; regtest 15,000. Its `ScriptFlags` enum is for node RPC, not a local interpreter configuration. `chronicle_rules_active` describes the legacy version-dependent relaxation, not all Chronicle features.

Complete OTDA digest/preimage/safe-wallet behavior and companion-spend verification are tracked separately in PR #250. This API does not claim full mempool or block acceptance equivalence.
