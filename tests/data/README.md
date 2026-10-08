# Vendored consensus test vectors

Reference data from [bitcoin-sv](https://github.com/bitcoin-sv/bitcoin-sv),
used to check chain-gang against the node rather than against someone's reading
of the consensus rules (CS-491).

| File | Source | SHA-256 |
| --- | --- | --- |
| `sighash.json` | `src/test/data/sighash.json` | `9c1afcaf81e8482f818345efa8a3f0610f6541b975023b58550d50ad2a557f63` |
| `script_tests_codeseparator.json` | the seven rows of `src/test/data/script_tests.json` that mention `CODESEPARATOR`, verbatim and in order | `bf346e717b03c07cf905eb4dcb344cee7e97c05dd539ffec3797643b4e4d8e9d` |
| `script_tests.json` | `src/test/data/script_tests.json` | `a77f8b94412ef61e9ee59980ebc682a64212b47a16f06d87f809d91770ba496d` |
| `base58_encode_decode.json` | `src/test/data/base58_encode_decode.json` | `2c56f0292ffe76083430557700a095e72e1e0483343667159700e6942795810f` |
| `base58_keys_valid.json` | `src/test/data/base58_keys_valid.json` | `b0531bd28c931a105539d1cc26b9b8d0a866c91fe24bb46c96ac13149aa4ae78` |
| `base58_keys_invalid.json` | `src/test/data/base58_keys_invalid.json` | `5e49887829b551d20190154695579c4c23e62fbd2c1de069a2299161daa8eeae` |

Taken from commit
[`879fc8b42168dd0e608dafd51b39c6dabad37d4d`](https://github.com/bitcoin-sv/bitcoin-sv/tree/879fc8b42168dd0e608dafd51b39c6dabad37d4d/src/test/data)
(2026-04-28). `script_tests_codeseparator.json` is taken from `script_tests.json`
above, which is now vendored whole as well. To refresh,
re-download at a named commit and update the table. The digests above are what
the files were when the counts pinned in the tests were measured, so a different
file means those counts have to be measured again.

## Licence

© BSV Association. These files are part of bitcoin-sv and are covered by the **Open
BSV License Version 5**, not by chain-gang's MIT licence. The full text is in
[`LICENSE-bitcoin-sv`](LICENSE-bitcoin-sv), copied from the same commit.

The two licences are not the same — the Open BSV License is revocable and
conditioned on use in connection with the BSV blockchains — so this directory is
listed under `exclude` in `Cargo.toml`. Everything published to crates.io stays
MIT; these vectors exist only in a git checkout. The tests skip when the
directory is absent, which is what an unpacked crate sees; set
`CHAIN_GANG_VECTORS_OPTIONAL=1` to allow that rather than fail.

`sighash.json` originates in Bitcoin Core, which published it under MIT, but
bitcoin-sv's copy has diverged — it is more than twice the size of Core's and
carries the BIP-143 column Core's has no equivalent for — so it is treated as
bitcoin-sv's work and licensed accordingly. `script_tests.json` in the same
directory upstream says so in its own header: "Distributed under the Open BSV
software license".

`base58_encode_decode.json` is byte-for-byte Bitcoin Core's (MIT; identical in
Core v0.14.0 and v0.15.0). The two `base58_keys_*` files have diverged from
Core's, so they are treated as bitcoin-sv's like the others. All of them stay
in this directory, out of the published crate.

## What uses them

- `sighash.json` — `src/transaction/sighash_vectors.rs`, run by
  `cargo test --lib sighash_vectors`.
- `script_tests_codeseparator.json` — `tests/script_vectors_codeseparator.rs`,
  run by `cargo test --test script_vectors_codeseparator`. Six of the seven rows
  carry real signatures over the node test framework's crediting and spending
  transactions, so they say where the node starts each check's script code
  (CS-492, CS-488).
- `script_tests.json` — `tests/script_vectors.rs`, run by
  `cargo test --test script_vectors`. Every row goes through `Tx::validate`
  (or `Tx::validate_consensus`) over the node test framework's crediting and
  spending transactions, with the row's flags mapped onto chain-gang's era,
  policy and FORKID settings; accept/reject is compared, not the error code.
  1335 of the 1483 rows agree. The other 148 are listed in the test under
  their reasons: 104 whose verdict turns on a flag chain-gang does not take
  one by one (`MODELLING_GAPS`), and 44 where chain-gang appears to be wrong
  (`KNOWN_DIFFERENCES`).
- `base58_*.json` — `tests/base58_vectors.rs` (`cargo test --test
  base58_vectors`) through the Rust address and WIF functions, and
  `python/src/tests/test_base58_vectors.py` through the Python ones.
