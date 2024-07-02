# Python Classes

This provides an overview of the Python Classes their properties and methods, 
including:

* [Script](#script)
* [Context](#context)
* [Tx](#tx)
* [TxIn](#txin)
* [TxOut](#txout)
* [Wallet](#wallet)
* [Interface Factory](#interface-factory)
* [Blockchain Interface](#blockchain-interface)
* [Other Functions](#other-functions)

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


## Wallet
This class represents the Wallet functionality, including handling of private and public keys and signing transactions.

Wallet class has the following methods:

* `__init__(wif_key: str) -> Wallet` - Constructor that takes a private key in WIF format
* `sign_tx_with_input(self, index: int, input_tx: Tx, tx: Tx) -> Tx` - Sign a transaction input with the provided previous tx and sighash flags, Returns new signed tx
* `sign_tx_with_input_and_sighash(self, index: int, input_tx: Tx, tx: Tx, sighash_type: int) -> Tx` - Sign a transaction input with the provided previous tx and sighash flags, Returns new signed tx
* `get_locking_script(self) -> Script` - Returns a locking script based on the public key
* `get_public_key_as_hexstr(self) -> String` - Return the public key as a hex string
* `get_address(self) -> String` - Return the address based on the public key
* `to_wif(self) -> String` - Return the private key in WIF format


## Interface Factory
The InterfaceFactory is class for creating interfaces to the BSV blockchain (`BlockchainInterface`).

The InterfaceFactory class one method:

* `set_config(self, config: ConfigType) -> BlockchainInterface` - This reads the configuration `interface_type` field and returns the configured `BlockchainInterface`

## Blockchain Interface

The BlockchainInterface class provides an interface to the BSV network.

BlockchainInterface class has the following methods:

* `__init__(self)` - Constructor that takes no parameters
* `set_config(self, config)` - configures the interface based on the provide config
* `get_addr_history(self, address)` - Return the transaction history with this address
* `is_testnet(self) -> bool` - Return true if this interface is connected to BSV Testnet
* `get_utxo(self, address)` - Return the utxo associated with this address
* `get_balance(self, address)` - Return the balance associated with this address
* `get_block_count(self)` - Return the height of the chain
* `get_best_block_hash(self)` - Return the hash of the latest block
* `get_merkle_proof(self, block_hash: str, tx_id: str) -> str` - Given the block hash and tx_id return the merkle proof
* `get_transaction(self, txid: str)` - Return the transaction (as Dictionary) associated with this txid
* `get_raw_transaction(self, txid: str) -> Optional[str]` - Return the transaction (as kexstring) associated with this txid, use cached copy if available.
* `broadcast_tx(self, transaction: str)` - broadcast this tx to the network
* `get_block(self, blockhash: str) -> Dict` - Return the block given the block hash
* `get_block_header(self, blockhash: str) -> Dict` - Returns te block_header for a given block hash

### WoC Interface
The `WoCInterface` is a `BlockchainInterface` that communicates with the WhatsOnChain API. 
Note that if you are using this you will need to install the python library `requests`.

```bash
pip3 install requests 
```

### Mock Interface 
The `Mock Interface` is a `BlockchainInterface` that is used for unit testing.


# Other Functions

* `address_to_public_key_hash(address: str) -> bytes` - Given the address return the hash160 of the public key
* `hash160(data: bytes) -> bytes` - Returns the hash160 of the provided data (usually the public key)
* `p2pkh_script(h160: bytes) -> Script` - Takes the hash160 of the public key and returns the locking script
* `public_key_to_address(public_key: bytes, network: str) -> String` - Given the public key and the network (either `BSV_Mainnet` or `BSV_Testnet`) return the address


![Bitcoin Keys](diagrams/keys.png)
