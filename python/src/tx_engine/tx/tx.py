from io import BytesIO

from typing import List

from tx_engine.engine.script import Script
from tx_engine.tx.sighash import SIGHASH
from tx_engine.engine.util import little_endian_to_int, read_varint



class TxIn:
    """ This class represents is a bitcoin transaction input
    """
    def __init__(self, prev_tx: bytes, prev_index: int, script_sig=None, sequence=0xFFFFFFFF, sighash: SIGHASH = SIGHASH.ALL_FORKID):
        # self.prev_tx: bytes = prev_tx
        self.prev_index: int = prev_index
        # self.script_sig = Script() if script_sig is None else script_sig
        # self.sequence: int = sequence


class TxOut:
    """ This class represents a bitcoin transaction output
    """
    def __init__(self, amount: int, script_pubkey: Script):
        self.amount: int = amount
        self.script_pubkey: Script = script_pubkey


class Tx:
    """ This class represents a bitcoin transaction
        We need this to
        * parse a bytestream - python
        * serialise a transaction - rust
        * sign tx - rust
        * verify tx - rust
    """
    def __init__(self, version: int, tx_ins: List[TxIn], tx_outs: List[TxOut], locktime: int):
        self.version: int = version
        self.tx_ins: List[TxIn] = tx_ins
        self.tx_outs: List[TxOut] = tx_outs
        self.locktime: int = locktime

    @classmethod
    def parse(cls, s: BytesIO):
        """Takes a byte stream and parses the transaction at the start
        return a Tx object
        """
        # s.read(n) will return n bytes
        # version is an integer in 4 bytes, little-endian
        version = little_endian_to_int(s.read(4))

        # num_inputs is a varint, use read_varint(s)
        num_inputs = read_varint(s)
        # Parse num_inputs number of TxIns
        inputs = [TxIn.parse(s) for _ in range(num_inputs)]
        # num_outputs is a varint, use read_varint(s)
        num_outputs = read_varint(s)
        # Parse num_outputs number of TxOuts
        outputs = [TxOut.parse(s) for _ in range(num_outputs)]
        # locktime is an integer in 4 bytes, little-endian
        locktime = little_endian_to_int(s.read(4))
        # Return an instance of the class (see __init__ for args)
        return cls(version, inputs, outputs, locktime)
