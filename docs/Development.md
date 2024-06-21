# Development

Notes on the development of `chain-gang` and the `tx_engine` Python interface.

# Project directory structure
```
├── README.md
├── docs
│   ├── Development.md
│   └── diagrams
├── python
│   ├── README.md
│   ├── examples
│   ├── lint.sh
│   ├── src
│   └── tests.sh
├── src
└── target
```
* `docs` - contains the documents and diagrams associated with this project
* `python/src` - contains the source code for the `tx_engine` Python interface
* `python/examples` - contains example scripts for the python script debugger
* `src` - contains the Rust source code for the `chain-gang` library
* `target` - contains the build artefacts for the project


# Tx_engine Requirements
Tx_engine was developed from the following "requirements".

* Script
* Script + Script = Script
* Script.parse_string
* Script.raw_serialize
* The debugger (or, more generally, the Context class)


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

# Maturin
`Maturin` is a tool for building and publishing Rust-based Python packages. 

* `maturin build` - builds the wheels and stores them in a folder
* `maturin develop` - builds and installs in the current `virtualenv`.
* `maturin publish` - builds and uploads to `pypi` - this appears to work, however we don't want to build at this time
* `maturin sdist` - creates a source code distribution, as a `.tar.gz` file, - appears to work with both Rust and Python source code
* `maturin upload`

Maturin User Guide [here](https://www.maturin.rs/)

* `maturin publish --interpreter -r https://test.pypi.org/legacy/`
maturin failed
Caused by: Failed to get registry https://test.pypi.org/legacy/ in .pypirc. Note: Your index didn't start with http:// or https://, which is required for non-pypirc indices.

tried 
`pip install --index-url https://test.pypi.org/simple/ --upgrade pip`
still failed

Tried disabling WARP, still failed


## Maturin-Action
https://github.com/PyO3/maturin-action

GitHub Action to install and run a custom maturin command with built in support for cross compilation

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

For background information see:
https://packaging.python.org/en/latest/guides/installing-using-pip-and-virtual-environments/#creating-a-virtual-environment


 # Github & PyPi

 To force a release the git version needs to be tagged

```bash
 git tag -a v0.2.0 -m "Python interface"
 git push --tags
 ```