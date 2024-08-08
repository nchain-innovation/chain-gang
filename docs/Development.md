# Development

Notes on the development of `chain-gang` and the `tx_engine` Python interface.

![Usecase](diagrams/overview.png)

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

## Re-install of the Python code
To force the reinstall of the Python code :
1) `maturin build` in the virtual env 
2) `pip3 install --force-reinstall` in the virtual env .. 

For example
``` bash
pip3 install --force-reinstall target/wheels/tx_engine-0.3.8-cp310-cp310-macosx_11_0_arm64.whl
```

## Maturin-Action
https://github.com/PyO3/maturin-action

GitHub Action to install and run a custom maturin command with built in support for cross compilation.
Used to publish the `tx-engine` on `PyPi`.

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

To force a release the git version needs to be tagged.
1) Update `cargo.toml` version push code up to repo. Otherwise GitHub won't figure out that the software has been updated.
2) Update git `tag` and push. Otherwise the GitHub action `release` will not be triggered.

```bash
git push
git tag -a v0.5.5 -m "Interface, RPCInterface, verify script and flags, TxIn & TxOut - script in constructor"
git push --tags
```

# Jupyter Notebooks and Development
This is the build process for tx-engine for use with Jupyter Notebooks

1) Install Maturin
2) Configure Python VENV
3) Activate Python VENV
Then:

``` bash
cd <chain-gang folder>
maturin develop
cd ~
python3 -m pip install ipykernel
python3 -m ipykernel install —user —name penv —display-name “Python with tx_engine”
``` 


## Jupyter Notebooks with PyPi
To use Jupyter 

1) install pvenv
```
source ~/penv/bin/activate
python3 -m pip install tx-engine
python3 -m pip install ipykernel
python3 -m ipykernel install —user —name penv —display-name “Python with tx_engine”
```

After this, in Jupyter a new kernel will show up under the name "Python with tx_engine"
