# Vendored consensus test vectors

Reference data from [bitcoin-sv](https://github.com/bitcoin-sv/bitcoin-sv),
used to check chain-gang against the node rather than against someone's reading
of the consensus rules (CS-491).

| File | Source | SHA-256 |
| --- | --- | --- |
| `sighash.json` | `src/test/data/sighash.json` | `9c1afcaf81e8482f818345efa8a3f0610f6541b975023b58550d50ad2a557f63` |

Taken from commit
[`879fc8b42168dd0e608dafd51b39c6dabad37d4d`](https://github.com/bitcoin-sv/bitcoin-sv/tree/879fc8b42168dd0e608dafd51b39c6dabad37d4d/src/test/data)
(2026-04-28). To refresh, re-download at a named commit and update the table.
The digest above is what the file was when the expected counts in
`src/transaction/sighash_vectors.rs` were measured, so a different file means
those counts have to be measured again.

## Licence

© BSV Association. This file is part of bitcoin-sv and is covered by the **Open
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

## What uses it

`src/transaction/sighash_vectors.rs`, run by `cargo test --lib sighash_vectors`.

## Not vendored: script_tests.json

The ticket also proposes bitcoin-sv's `script_tests.json`, which is what would
settle CS-488 and cover the separator bugs end to end, because there the
interpreter decides which `OP_CODESEPARATOR` executed and what each `CHECKSIG`
signs. It is deliberately absent, on measurement rather than principle.

Of its 1483 test rows, only **149** carry no flag chain-gang lacks. The rest
need consensus flags the interpreter does not model — it has `NO_FLAGS` and
`PREGENESIS_RULES`, against 1092 rows needing `P2SH`, 1043 `STRICTENC`, 133
`UTXO_AFTER_GENESIS`, 116 `MINIMALDATA`, 42 `UTXO_AFTER_CHRONICLE` and a tail of
`DERSIG`, `MINIMALIF`, `NULLFAIL`, `CLEANSTACK`, `SIGPUSHONLY`, `LOW_S` and
others across 29 distinct combinations. Wiring the file up now would skip about
nine rows in ten, so the useful order is interpreter flag support first, then
these vectors. Fetch it from the commit above when that lands.
