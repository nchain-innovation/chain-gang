from tx_engine.engine.util import encode_num
import logging
from typing import Union
from tx_engine.engine.op_code_names import OP_CODE_NAMES


LOGGER = logging.getLogger(__name__)

# Create a dictionary of OP_DUP -> 118, DUP -> 118
OPS_STANDARD = {v: k for (k, v) in OP_CODE_NAMES.items()}
SHORT_TO_LONG_OP = {k.split("_")[1]: v for (k, v) in OPS_STANDARD.items()}
ALL_OPS = {**OPS_STANDARD, **SHORT_TO_LONG_OP}


def decode_op(op: str) -> Union[int, bytes]:
    """ Given an op as string convert it to parsable value
        e.g. "OP_2" -> 0x52
    """
    op = op.strip()
    LOGGER.info(f"decode_op '{op, len(op)}'")
    if op[:2] == "0x":
        b: bytes = bytes.fromhex(op[2:])
        LOGGER.info(f"decode_op {op} -> {b.hex()}")
        return b

    elif op in ALL_OPS:
        n: int = ALL_OPS[op]
        LOGGER.info(f"decode_op {op} -> {n}")
        return n

    else:
        n = eval(op)
        LOGGER.info(f"eval(op) = {n}")
        if isinstance(n, int):
            x: bytes = encode_num(n)
            LOGGER.info(f"decode_op {op} -> {x.hex()}")
            return x
        elif isinstance(n, str):
            y = n.encode("utf-8")
            LOGGER.info(f"decode_op {op} -> {y}")
            return y
        elif isinstance(n, bytes):
            LOGGER.info(f"decode_op {op} -> {n, len(n)}")
            return n
        else:
            # have not captured conversion
            assert 1 == 2  # should not get here


def cmd_repr(cmd: int) -> Union[str, bytes]:
    """ Return a string (and bytes) representation of the command
        e.g. 0x5 -> OP_10
    """
    if isinstance(cmd, int):
        try:
            return OP_CODE_NAMES[cmd]
        except KeyError:
            return str(cmd)
    else:
        return cmd
