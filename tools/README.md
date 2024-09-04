# Tools

This directory contains tools that use the Python tx-engine library.

* `dbg.py` - is a script debugger in the style of GDB
* `generate_key.py` - is a script which generates a random key pair for BSV testnet

For more details see the following sections

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


## Key Generator
This generates a random keypair and prints the WIF (Wallet Independent Format) and associated address.

```bash
% python3 generate_key.py
wif = cS9gc7npzkPfDpBmLBcqtxhHKWB58KPJGD13RBryzXWKmXgWEZCQ
address = mzE7XmZ5PWxHQkKjVrnwDKeQeV7Q8BZBiY
```

