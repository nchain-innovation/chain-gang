# Requirements for tx_engine

Hi John, this is the repo I'm working on: https://bitbucket.stressedsharks.com/users/f.barbacovi_nchain.com/repos/script_libraries/browse, 

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

Built using Python3 version 3.11.2


## Changes from pure Python tx_engine

* `encode_num` is now `insert_num`

* `OP_PUSHDATA1` now only takes bytes as parameters for number of bytes to be pushed onto the stack
```python
script = Script([OP_PUSHDATA1, b'\x02', b"\x01\x02"])
script = Script.parse_string("OP_PUSHDATA1, 0x02, b'\x02\x01'")
```

## Context
Context now has 

* `raw_stack` - which contains the stack before trying to convert to numbers
```python
script = Script([OP_PUSHDATA1, 0x02, b"\x01\x02"])
context = Context(script=script)
self.assertTrue(context.evaluate_core())
self.assertEqual(context.raw_stack, [[1,2]])
```

# Script evaluations 
* `evaluate_core` - executes the script, does not decode stack to nums
* `evaluate` - executes the script and decode stack elements to numbers

 Both `evaluate` and `evaluate_core` have a parameter `quiet` which if set to true does not print out exceptions when excuting code.
 This is currently only used in unit tests.