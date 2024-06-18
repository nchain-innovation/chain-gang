from typing import Union

from .util import encode_num
from .op_code_names import OP_CODE_NAMES


# Create a dictionary of OP_DUP -> 118, DUP -> 118
OPS_STANDARD = {v: k for (k, v) in OP_CODE_NAMES.items()}
SHORT_TO_LONG_OP = {k.split("_")[1]: v for (k, v) in OPS_STANDARD.items()}
ALL_OPS = {**OPS_STANDARD, **SHORT_TO_LONG_OP}


def decode_op(op: str) -> Union[int, bytes]:
    """ Given an op as string convert it to parsable value
        e.g. "OP_2" -> 0x52
    """
    op = op.strip()
    if op[:2] == "0x":
        b: bytes = bytes.fromhex(op[2:])
        return b

    elif op in ALL_OPS:
        n: int = ALL_OPS[op]
        return n

    else:
        n = eval(op)
        if isinstance(n, int):
            x: bytes = encode_num(n)
            return x
        elif isinstance(n, str):
            y = n.encode("utf-8")
            return y
        elif isinstance(n, bytes):
            return n
        else:
            # have not captured conversion
            assert 1 == 2  # should not get here
