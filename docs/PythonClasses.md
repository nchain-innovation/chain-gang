# Python Classes

This provides an overview of the Python Classes their properties and methods, 
including:

* [Script](#script)
* [Context](#context)
* [Tx](#tx)
* [TxIn](#txin)
* [TxOut](#txout)


## Script

The Script class represents bitcoin script.
For more details about bitcoin script see https://wiki.bitcoinsv.io/index.php/Script.

Script has the following property:
* `cmds` - byte arrary of bitcoin operations

Script has the following methods:

* `__init__(self, cmds: bytes) -> Script` - Constructor that takes an array of bytes 
* `raw_serialize(self) -> bytes` - Return the serialised script without the length prepended
* `serialize(self) -> bytes` - Return the serialised script with the length prepended
*  `get_commands(self) -> bytes` - Return a copy of the commands in this script
* `__add__(self, other: Script) -> Script` - Enable script addition e.g. `c_script = a_script + b_script`

Script has the following class methods:
* `Script.parse_string(in_string: str) -> Script` - Converts a string of OP_CODES into a Script
* `Script.parse(in_bytes: bytes) -> Script` - Converts an array of bytes into a Script


## Context

The `context` is the environment in which bitcoin scripts are executed.

Context has the following properties:
* `cmds` - the commands to execute
* `ip_limit` - the number of commands to execute before stopping (optional)
* z - 
* `stack` - main data stack
* `alt_stack` - seconary stack
* `raw_stack` - which contains the `stack` prior to converting to numbers
* `raw_alt_stack` - as above for the `alt_stack`

Context has the following methods:

* `__init__(self, script: Script, cmds: Commands = None, ip_limit: int , z: bytes)` - constructor
* `evaluate_core(self, quiet: bool = False) -> bool` - evaluates the script/cmds using the the interpreter and returns the stacks (`raw_stack`, `raw_alt_stack`). if quiet is true, dont print exceptions
* `evaluate(self, quiet: bool = False) -> bool` - executes the script and decode stack elements to numbers (`stack`, `alt_stack`). Checks `stack` is true on return. if quiet is true, dont print exceptions.
* `get_stack(self) -> Stack` - Return the `stack` as human readable
* `get_altstack(self) -> Stack`-  Return the `alt_stack` as human readable





Example from unit tests of using `evaluate_core` and `raw_stack`:
```python
script = Script([OP_PUSHDATA1, 0x02, b"\x01\x02"])
context = Context(script=script)
self.assertTrue(context.evaluate_core())
self.assertEqual(context.raw_stack, [[1, 2]])
```

### Quiet Evalutation
 Both `evaluate` and `evaluate_core` have a parameter `quiet`.
 If the `quiet` parameter is set to `True` the `evaluate` function does not print out exceptions when executing code.  This `quiet` parameter is currently only used in unit tests.



## Tx

Tx represents a bitcoin transaction.

Script has the following properties:
* `version` - unsigned integer
* `tx_ins` - array of `TxIn` classes,
* `tx_outs` - array of `TxOut` classes
* `locktime` - unsigned integer

Script has the following methods:

* `__init__(version: int, tx_ins: [TxIn], tx_outs: [TxOut], locktime: int) -> Tx` - Constructor that takes the fields 
* `id(self) -> str` - Return human-readable hexadecimal of the transaction hash
* `hash(self) -> bytes` - Return transaction hash as bytes
* `is_coinbase(self) -> bool` - Returns true if it is a coinbase transaction
* `serialize(self) -> bytes` - Returns Tx as bytes
 * `generate_signature_for_input(self, n_input: int, script_code: bytes, satoshis: int, private_key: bytes, sighash_type: int) -> bytes` - Sign the transaction input to spend it
    
Script has the following class methods:

* `Tx.parse(in_bytes: bytes) -> Tx`  - Parse bytes to produce Tx


## TxIn
TxIn represents is a bitcoin transaction input.

TxIn has the following properties:

* `prev_tx` - bytes
* `prev_index` - unsigned int
* `sequence` -  int
* `script_sig` - Script

TxIn has the following method:

* `__init__(prev_tx: bytes, prev_index: int, script_sig: bytes, sequence: int) -> TxIn` - Constructor that takes the fields 

## TxOut
TxOut represents a bitcoin transaction output.

TxOut has the following properties:

* `amount` - int
* `script_pubkey` - Script


TxOut has the following method:

* `__init__(amount: int, script_pubkey: bytes) -> TxOut` - Constructor that takes the fields 
