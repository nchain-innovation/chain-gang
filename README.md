# TX Engine

This library provides a Python interface for building BitcoinSV scripts and transactions.

The classes `Script`, `Context`, `Tx`, `TxIn` and `TxOut` are imported from the top level of `tx_engine`.

For documentation of the Python Classes see  [here](docs/PythonClasses.md)

# Python Installation
As this library is hosted on PyPi (https://pypi.org/project/tx-engine/) it can be installed using the following command:

```bash
pip install tx-engine
```

## Example Tx class usage
So to parse a hex string to Tx:

```python
from tx_engine import Tx

src_tx = "0100000001c7151ebaf14dbfe922bd90700a7580f6db7d5a1b898ce79cb9ce459e17f12909000000006b4830450221008b001e8d8110804ac66e467cd2452f468cba4a2a1d90d59679fe5075d24e5f5302206eb04e79214c09913fad1e3c0c2498be7f457ed63323ac6f2d9a38d53586a58d41210395deb00349c0ae73412a55bec70a7793fc6860a193d29dd61d73c6271ffcbd4cffffffff0103000000000000001976a91496795fb99fd6c0f214f7a0e96019f642225f52d288ac00000000"
tx_bytes = bytes.fromhex(src_tx)

tx = tx.parse(tx_bytes)
print(tx)

PyTx { version: 1, tx_ins: [PyTxIn { prev_tx: "0929f1179e45ceb99ce78c891b5a7ddbf680750a7090bd22e9bf4df1ba1e15c7", prev_index: 0, sequence: 4294967295, script_sig: [0x48 0x30450221008b001e8d8110804ac66e467cd2452f468cba4a2a1d90d59679fe5075d24e5f5302206eb04e79214c09913fad1e3c0c2498be7f457ed63323ac6f2d9a38d53586a58d41 0x21 0x0395deb00349c0ae73412a55bec70a7793fc6860a193d29dd61d73c6271ffcbd4c] }], tx_outs: [PyTxOut { amount: 3, script_pubkey: [OP_DUP OP_HASH160 0x14 0x96795fb99fd6c0f214f7a0e96019f642225f52d2 OP_EQUALVERIFY OP_CHECKSIG] }], locktime: 0 }
```


## Tx

Tx represents a bitcoin transaction.

Tx has the following properties:
* `version` - unsigned integer
* `tx_ins` - array of `TxIn` classes,
* `tx_outs` - array of `TxOut` classes
* `locktime` - unsigned integer

Tx has the following methods:

* `__init__(version: int, tx_ins: [TxIn], tx_outs: [TxOut], locktime: int=0) -> Tx` - Constructor that takes the fields 
* `id(self) -> str` - Return human-readable hexadecimal of the transaction hash
* `hash(self) -> bytes` - Return transaction hash as bytes
* `is_coinbase(self) -> bool` - Returns true if it is a coinbase transaction
* `serialize(self) -> bytes` - Returns Tx as bytes
* `copy(self) -> Tx` - Returns a copy of the Tx
* `to_string(self) -> String` - return the Tx as a string. Note also that you can just print the tx (`print(tx)`).
* `validate(self, [Tx]) -> Result` - provide the input txs, returns None on success and throws a RuntimeError exception on failure. Note can not validate coinbase or pre-genesis transactions.

    
Tx has the following class methods:

* `Tx.parse(in_bytes: bytes) -> Tx`  - Parse bytes to produce Tx

So to parse a hex string to Tx:
```Python
from tx_engine import Tx

src_tx = "0100000001c7151ebaf14dbfe922bd90700a7580f6db7d5a1b898ce79cb9ce459e17f12909000000006b4830450221008b001e8d8110804ac66e467cd2452f468cba4a2a1d90d59679fe5075d24e5f5302206eb04e79214c09913fad1e3c0c2498be7f457ed63323ac6f2d9a38d53586a58d41210395deb00349c0ae73412a55bec70a7793fc6860a193d29dd61d73c6271ffcbd4cffffffff0103000000000000001976a91496795fb99fd6c0f214f7a0e96019f642225f52d288ac00000000"
tx_bytes = bytes.fromhex(src_tx)

tx = Tx.parse(tx_bytes)
print(tx)

PyTx { version: 1, tx_ins: [PyTxIn { prev_tx: "0929f1179e45ceb99ce78c891b5a7ddbf680750a7090bd22e9bf4df1ba1e15c7", prev_index: 0, sequence: 4294967295, script_sig: [0x48 0x30450221008b001e8d8110804ac66e467cd2452f468cba4a2a1d90d59679fe5075d24e5f5302206eb04e79214c09913fad1e3c0c2498be7f457ed63323ac6f2d9a38d53586a58d41 0x21 0x0395deb00349c0ae73412a55bec70a7793fc6860a193d29dd61d73c6271ffcbd4c] }], tx_outs: [PyTxOut { amount: 3, script_pubkey: [OP_DUP OP_HASH160 0x14 0x96795fb99fd6c0f214f7a0e96019f642225f52d2 OP_EQUALVERIFY OP_CHECKSIG] }], locktime: 0 }
```

## Tx utility functions.

There are some utility functions that can be called with a transaction. They return the sighash pre-image of a transaction and the double sha256 of the sighash pre-image
of a transaction. This is the value that is signed when creating a transactions.

```Python
from tx_engine import Tx, TxIn, TxOut, sig_hash_preimage, sig_hash, Script, SIGHASH

src_tx = "0100000001c7151ebaf14dbfe922bd90700a7580f6db7d5a1b898ce79cb9ce459e17f12909000000006b4830450221008b001e8d8110804ac66e467cd2452f468cba4a2a1d90d59679fe5075d24e5f5302206eb04e79214c09913fad1e3c0c2498be7f457ed63323ac6f2d9a38d53586a58d41210395deb00349c0ae73412a55bec70a7793fc6860a193d29dd61d73c6271ffcbd4cffffffff0103000000000000001976a91496795fb99fd6c0f214f7a0e96019f642225f52d288ac00000000"
tx_bytes = bytes.fromhex(src_tx)

tx = Tx.parse(tx_bytes)

sig_hash_val = sig_hash(
        tx=tx,
        index=0,
        script_pubkey=Script(),
        prev_amount=prev_value,
        sighash_value=SIGHASH.ALL_FORKID
    )

preimage_sighash = sig_hash_preimage(tx=tx,
        index=0,
        script_pubkey=pushtx_lock,
        prev_amount=prev_value,
        sighash_value=SIGHASH.ALL_FORKID)
```

# Example Script execution

```python
>>> from tx_engine import Script, Context

>>> s = Script.parse_string("OP_10 OP_5 OP_DIV")
>>> c = Context(script=s)
>>> c.evaluate()
True
>>> c.get_stack()
[2]
```


## Context

The `context` is the environment in which bitcoin scripts are executed.

* `evaluate_core` - executes the script, does not decode stack to numbers
* `evaluate` - executes the script and decode stack elements to numbers

### Context Stacks
`Context` now has: 
* `raw_stack` - which contains the `stack` prior to converting to numbers
* `raw_alt_stack` - as above for the `alt_stack`

Example from unit tests of using`raw_stack`:
```python
script = Script([OP_PUSHDATA1, 0x02, b"\x01\x02"])
context = Context(script=script)
self.assertTrue(context.evaluate_core())
self.assertEqual(context.raw_stack, [[1,2]])
```

### Quiet Evalutation
 Both `evaluate` and `evaluate_core` have a parameter `quiet`.
 If the `quiet` parameter is set to `True` the `evaluate` function does not print out exceptions when executing code.  This `quiet` parameter is currently only used in unit tests.

### Inserting Numbers into Script

* `encode_num()` is now `insert_num()`


# Script Debugger
The script debugger enables the user to examine the stack status as the script is executing as 
well as writing interactive script.

Example debugger usage:
```bash
% cd python/src
% python3 dbg.py -f ../examples/add.bs
Script debugger
For help, type "help".
Loading filename: ../examples/add.bs
altstack = [], stack = []
(gdb) list
0: OP_1
1: OP_2
2: OP_ADD
altstack = [], stack = []
(gdb) s
0: OP_1
altstack = [], stack = [1]
(gdb) s
1: OP_2
altstack = [], stack = [1, 2]
(gdb) s
2: OP_ADD
altstack = [], stack = [3]
(gdb) 
```

The debugger supports the following commands:

* `h`, `help` - Prints a list of commands
* `q`, `quit`, `exit` -- Quits the program
* `file` [filename] - Loads the specified script file for debugging
* `list` - List the current script file contents
* `run` - Runs the current loaded script until breakpoint or error
* `i` [script] -- Execute script interactively
* `hex` - Display the main stack in hexidecimal values
* `dec` - Display the main stack in decimal values
* `reset` - Reset the script to the staring position
* `s`, `step` - Step over the next instruction
* `c` - Continue the current loaded script until breakpoint or error
* `b` - Adds a breakpoint on the current operation
* `b` [n] - Adds a breakpoint on the nth operation
* `info break` - List all the current breakpoints
* `d` [n] - Deletes breakpoint number n

