# Development

Notes on the development of `chain-gang` and the `tx_engine` Python interface.


# Tx_engine Requirements
Tx_engine was developed from the following "requirements".

This is the repo I'm working on: https://bitbucket.stressedsharks.com/users/f.barbacovi_nchain.com/repos/script_libraries/browse, 

(disregarding the fact that tx_engine is in there)

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


# Tx_engine Unit Tests
The unit tests need to operate in the Python virtual environment

```bash
$ source ~/penv/bin/activate
$ cd python
$ ./tests.sh
```

For more information on the tests see [here](../python/src/tests/README.md)

# Linting tx_engine

To perform static code analysis on the Python source code run the following:

```bash
$ cd python
$ ./lint.sh
```

