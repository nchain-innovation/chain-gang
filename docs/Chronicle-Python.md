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

## Signing with OTDA (Chronicle sighash)

The wallet exposes transaction signing with an explicit sighash type. Given an
existing wallet, a funding transaction `fund`, and a spending transaction
`spend` whose input 0 references `fund`, the following selects OTDA.

```python
from tx_engine.tx.sighash import SIGHASH

signed = wallet.sign_tx_sighash(
    0, fund, spend, int(SIGHASH.ALL_FORKID_CHRONICLE)
)
signed.validate([fund])
```

`SIGHASH.ALL_FORKID` selects BIP143, while
`SIGHASH.ALL_FORKID_CHRONICLE` selects OTDA. Signature-type availability and
verification depend on the spending context. For mixed-era or historical
outputs, validate with explicit activation and output creation heights.

## Debugging scripts and activation helpers

`Context` is useful for debugging a pair of scripts with a transaction version.
It does not infer the creation height of a spent output.

```python
from tx_engine import Context, Script

unlock = Script.parse_string("OP_2 OP_3 OP_ADD")
lock = Script.parse_string("OP_5 OP_EQUAL")
assert Context(script=unlock, lock_script=lock, tx_version=2).evaluate()
```

The activation helper reports the library's released-node default. Private
chains can configure a different height and should pass it explicitly to the
context API.

```python
from tx_engine.interface.verify_script import activation_height

assert activation_height("BSV_Mainnet") == 943_816
```

For context-free inspection, `tx_engine.engine.util` provides
`max_script_num_length` and `MAX_SCRIPT_NUM_LENGTH_CHRONICLE`. These helper
results do not replace the spent output era needed by transaction validation.

## Script number limits

`max_script_num_length(1)` and `max_script_num_length(2)` in
`tx_engine.engine.util` report the 750,000-byte post-Genesis and
32,000,000-byte Chronicle limits for version-based debugging. With explicit
transaction validation context, the spent output's era determines the
consensus limit.

## References

- [Chronicle rule selection and Rust API](Chronicle.md)
- [Python API](Python-API.md)
- [Chronicle release documentation](https://docs.bsvblockchain.org/network-topology/nodes/sv-node/chronicle-release)
