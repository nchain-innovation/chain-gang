# Tools

This directory contains tools that use the Python tx-engine library.

* `generate_key.py` - is a script which generates a random key pair for BSV testnet

The script debugger that used to be here, `dbg.py`, is now its own project:
[nchain-innovation/script_debugger](https://github.com/nchain-innovation/script_debugger).

For more details see below.

# Key Generator
This generates a random keypair and prints the WIF (Wallet Independent Format) and associated address.

```bash
% python3 generate_key.py
wif = cS9gc7npzkPfDpBmLBcqtxhHKWB58KPJGD13RBryzXWKmXgWEZCQ
address = mzE7XmZ5PWxHQkKjVrnwDKeQeV7Q8BZBiY
```

