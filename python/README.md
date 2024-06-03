# Python Feature

This feature provides the ability to call `chain-gang` with a `tx_engine` like interface from Python. Where `tx_engine` was a previous Python BSV library used by nChain.

This interface has been tested using Python3 version 3.11.2

# Example usage

```python
>>> from tx_engine.engine.context import Context
>>> from tx_engine.engine.script import Script

>>> s = Script.parse_string("OP_10 OP_5 OP_DIV")
>>> c = Context(script=s)
>>> c.evaluate()
True
>>> c.get_stack()
[2]
```

## Changes from Python tx_engine


## Context

The `context` is the environment in which bitcoin scripts are executed.

* `evaluate_core` - executes the script, does not decode stack to nums
* `evaluate` - executes the script and decode stack elements to numbers

 Both `evaluate` and `evaluate_core` have a parameter `quiet`, which if set to true does not print out exceptions when executing code.
 `Quiet` is currently only used in unit tests.


`Context` now has: 

* `raw_stack` - which contains the `stack` prior to converting to numbers
* `raw_alt_stack` - as above for the `alt_stack`
```python
script = Script([OP_PUSHDATA1, 0x02, b"\x01\x02"])
context = Context(script=script)
self.assertTrue(context.evaluate_core())
self.assertEqual(context.raw_stack, [[1,2]])
```


* `encode_num()` is now `insert_num()`

