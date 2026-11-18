# OTDA constant-digest compatibility

The original transaction digest algorithm returns `uint256(1)` when the input
index is outside the input array, or when `SIGHASH_SINGLE` has no output at that
input index. This applies to legacy and Chronicle dispatch, with or without
ANYONECANPAY. FORKID without CHRONICLE continues to use BIP-143: a missing paired
output leaves hashOutputs zero and still hashes a real preimage.

Reference: bitcoin-sv commit
[6504a3aff65ba97c0f6c80962b033e35ecbfed4b, SignatureHashOriginal and dispatch](https://github.com/bitcoin-sv/bitcoin-sv/blob/6504a3aff65ba97c0f6c80962b033e35ecbfed4b/src/script/interpreter.cpp#L2086-L2123).

## API compatibility and safety

- Digest APIs now return the consensus sentinel where they previously returned
  `BadArgument`. Its raw bytes are `01` followed by 31 zero bytes; display hex
  reverses them. The verifier uses these same consensus semantics.
- Preimage APIs still return `BadArgument`, with an explicit no-serialized-preimage
  message. The protocol returns the sentinel directly; returning the sentinel
  bytes as a preimage would cause callers to hash the wrong value.
- `Wallet::sign_tx_input` and its indexed variant reject these cases before
  changing the transaction. Their forwarding APIs inherit this policy.
- `create_sighash` remains a digest helper. `generate_signature` and
  `Wallet::sign_sighash` remain explicit low-level signing operations; callers
  must enforce their own signing policy. A sentinel signature commits to no
  transaction fields and can be reused wherever the same sentinel applies.
- Ordinary OTDA digests, BIP-143 digests, and public function signatures are
  unchanged. No new error enum variant is introduced.

`tests/otda_constant_digest.rs` exercises public digest/preimage APIs, algorithm
and flag combinations, bounds, raw byte order, real signature consensus
validation, wallet refusal without mutation, and invalidation once a paired
output is added. These are source-derived boundary regressions, supplementing
the existing vendored BSV node vectors. They are not a live-node differential
execution test.
