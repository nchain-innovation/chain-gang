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


# Maturin
`Maturin` is a tool for building and publishing Rust-based Python packages with minimal configuration. 

`maturin build` - builds the wheels and stores them in a folder
`maturin develop` - builds and installs in the current virtualenv.


# Python VENV

Use the following commands to setup the virtual environment

```bash
$ cd ~
$ python3 -m venv penv
$ source ~/penv/bin/activate
```

To use the venv type the following:

```bash
$ source ~/penv/bin/activate
```

For background information see
https://packaging.python.org/en/latest/guides/installing-using-pip-and-virtual-environments/#creating-a-virtual-environment


# Unit Tests
The unit tests need to operate in the Python virtual environment

```bash
$ source ~/penv/bin/activate
$ cd python
$ ./tests.sh
```

For more information on the tests see [here](src/tests/README.md)

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