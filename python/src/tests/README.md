# Tests
This directory contains the Python unit tests for the Rust BSV Bitcoin script interpreter.

## To run the tests
To run all tests:
```bash
% cd python
% ./tests.sh
```

To run a test suite:
```bash
% cd python/src/tests
% python3 test_op.py
```

To run an individual test:
```bash
% cd python/src/tests
% python3 test_op.py ScriptOPTests.test_nop
```
