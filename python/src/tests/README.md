# Tests
This directory contains the Python unit tests for the Rust BSV Bitcoin script interpreter.

The unit tests need to operate in the Python virtual environment.


## To run the tests
To run all tests:
```bash
$ source ~/penv/bin/activate
$ cd python
$ ./tests.sh
```

To run a test suite:
```bash
$ source ~/penv/bin/activate
$ cd python/src/tests
$ python3 test_op.py
```

To run an individual test:
```bash
$ source ~/penv/bin/activate
$ cd python/src/tests
$ python3 test_op.py ScriptOPTests.test_nop
```
