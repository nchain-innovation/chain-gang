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

With verbose output
```bash
python3 -m unittest test_fed.py -vvv
```

## Network access
The suite needs no network. The WhatsOnChain tests in `test_woc.py` run against
responses recorded from testnet, kept in `fixtures/woc/`, with `requests.get`
patched. The same checks can also be run against the live WhatsOnChain testnet
API, which is skipped unless `CHAIN_GANG_LIVE_TESTS=1` is set:
```bash
$ cd python/src/tests
$ CHAIN_GANG_LIVE_TESTS=1 python3 -m unittest test_woc -v
```

If WhatsOnChain changes a response, re-record the fixture with `curl` from the
URL it is keyed by in `test_woc.py` (`RECORDED`).
