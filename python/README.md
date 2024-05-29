# Python Feature

This feature provides the ability to call `chain-gang` with a `tx_engine` like interface from Python. Where `tx_engine` was a previous Python BSV library used by nChain.

This interface has been tested using Python3 version 3.11.2


## Changes from pure Python tx_engine

* `encode_num()` is now `insert_num()`


## Context

The `context` is the environment in which bitcoin scripts are executed.

Context now has: 

* `raw_stack` - which contains the `stack` prior to converting to numbers
* `raw_alt_stack` - as above for the `alt_stack`
```python
script = Script([OP_PUSHDATA1, 0x02, b"\x01\x02"])
context = Context(script=script)
self.assertTrue(context.evaluate_core())
self.assertEqual(context.raw_stack, [[1,2]])
```

# Script evaluations 
* `evaluate_core` - executes the script, does not decode stack to nums
* `evaluate` - executes the script and decode stack elements to numbers

 Both `evaluate` and `evaluate_core` have a parameter `quiet`, which if set to true does not print out exceptions when executing code.
 `Quiet` is currently only used in unit tests.


 `Maturin` is a tool for building and publishing Rust-based Python packages with minimal configuration. 


 # Requirements for tx_engine

this is the repo I'm working on: https://bitbucket.stressedsharks.com/users/f.barbacovi_nchain.com/repos/script_libraries/browse, 

disregarding the fact that tx_engine is in there, 

if you enter lib/ellipticcurves/ec_arithmetic_Fq, 
you will see that I actually only use Script, pick and roll 

(which I defined to return OP_i OP_PICK as many times as I need, e.g., pick(position=3,nElements=2) -> OP_3 OP_PICK OP_3 OP_PICK, 

you can find them in tx_engine/engine/utility_scripts.py).

I think that the only thing I need is:
Script
Script + Script = Script
Script.parse_string
Script.raw_serialize
The debugger (or, more generally, the Context class)