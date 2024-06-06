# Python Feature

This feature provides the ability to call `chain-gang` with a `tx_engine` interface from Python. Where `tx_engine` was a previous Python BSV library used by nChain.

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
The bitcoin script debugger enables the user to examine the stack status as the script is executing as 
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

