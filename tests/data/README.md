# Vendored consensus test vectors

Reference data from [bitcoin-sv](https://github.com/bitcoin-sv/bitcoin-sv),
used to check chain-gang against the node rather than against someone's reading
of the consensus rules (CS-491).

| File | Source | SHA-256 |
| --- | --- | --- |
| `sighash.json` | `src/test/data/sighash.json` | `9c1afcaf81e8482f818345efa8a3f0610f6541b975023b58550d50ad2a557f63` |
| `script_tests_codeseparator.json` | the seven rows of `src/test/data/script_tests.json` that mention `CODESEPARATOR`, verbatim and in order | `bf346e717b03c07cf905eb4dcb344cee7e97c05dd539ffec3797643b4e4d8e9d` |

Taken from commit
[`879fc8b42168dd0e608dafd51b39c6dabad37d4d`](https://github.com/bitcoin-sv/bitcoin-sv/tree/879fc8b42168dd0e608dafd51b39c6dabad37d4d/src/test/data)
(2026-04-28); the whole `script_tests.json` the rows were taken from has SHA-256
`a77f8b94412ef61e9ee59980ebc682a64212b47a16f06d87f809d91770ba496d`. To refresh,
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

## What uses them

- `sighash.json` — `src/transaction/sighash_vectors.rs`, run by
  `cargo test --lib sighash_vectors`.
- `script_tests_codeseparator.json` — `tests/script_vectors_codeseparator.rs`,
  run by `cargo test --test script_vectors_codeseparator`. Six of the seven rows
  carry real signatures over the node test framework's crediting and spending
  transactions, so they say where the node starts each check's script code
  (CS-492, CS-488).

## Not vendored: the rest of script_tests.json

Only the `CODESEPARATOR` rows are taken, and they need no consensus flags. The
rest of the file is deliberately absent, on measurement rather than principle.

Of its 1483 test rows, only **149** carry no flag chain-gang lacks. The rest
need consensus flags the interpreter does not model — it has `NO_FLAGS` and
`PREGENESIS_RULES`, against 1092 rows needing `P2SH`, 1043 `STRICTENC`, 133
`UTXO_AFTER_GENESIS`, 116 `MINIMALDATA`, 42 `UTXO_AFTER_CHRONICLE` and a tail of
`DERSIG`, `MINIMALIF`, `NULLFAIL`, `CLEANSTACK`, `SIGPUSHONLY`, `LOW_S` and
others across 29 distinct combinations. Wiring the whole file up now would skip
about nine rows in ten, so the useful order is interpreter flag support first,
then these vectors. Fetch it from the commit above when that lands.
