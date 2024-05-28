from typing import List
from io import BytesIO

from .engine_types import StackElement

# Maximum script number length before Genesis (equal to CScriptNum::MAXIMUM_ELEMENT_SIZE)
MAX_SCRIPT_NUM_LENGTH_BEFORE_GENESIS = 4
# Maximum script number length after Genesis
MAX_SCRIPT_NUM_LENGTH_AFTER_GENESIS = 750 * 1000
# Maximum size that we are using
MAXIMUM_ELEMENT_SIZE = MAX_SCRIPT_NUM_LENGTH_AFTER_GENESIS


from chain_gang import py_encode_num, py_decode_num, py_encode_varint


def int_to_little_endian(n: int, length: int) -> bytes:
    """ endian_to_little_endian takes an integer and returns the little-endian
        byte sequence of length
    """
    return n.to_bytes(length, "little")


def little_endian_to_int(b: bytes) -> int:
    """ little_endian_to_int takes byte sequence as a little-endian number.
        Returns an integer
    """
    return int.from_bytes(b, "little")


def encode_num(num: int) -> bytes:
    """ Encode a number, return a bytearray in little endian
    """
    return py_encode_num(num)


def is_minimally_encoded(element, max_element_size=MAXIMUM_ELEMENT_SIZE) -> bool:
    """ Determines if an element is minimally encoded, returns True if it is.
        Code copied from SV codebase for details see:
            file: int_serialization.h, function: IsMinimallyEncoded, line: 98
    """
    if isinstance(element, int):
        return True
    size = len(element)
    if size > max_element_size:
        return False
    if size > 0:
        elem = element[::-1]
        if elem[0] & 0x7f == 0:
            if size <= 1 or (elem[1] & 0x80 == 0):
                return False
    return True


def decode_num(element: StackElement, check_encoding=False) -> int:
    """ Take a byte(array), return a number
    """
    if element == b"":
        return 0

    if check_encoding and not is_minimally_encoded(element):
        if isinstance(element, bytes):
            raise ValueError(f"Value is not minimally encoded: {element.hex()}")
        else:
            raise ValueError(f"Value is not minimally encoded: {element}")
    return py_decode_num(element)


def insert_num(val: int) -> List[int]:
    """ This function is used to insert numbers into script
    """
    val_as_bytes = bytearray(encode_num(val))
    length = len(val_as_bytes)
    assert length < 0x4c, "Length of number too long, need to encode using OP_PUSHDATA"
    # Insert the length
    val_as_bytes.insert(0, length)
    return list(val_as_bytes)


def read_varint(s: BytesIO) -> int:
    """ read_varint reads a variable integer from a stream
    """
    i = s.read(1)[0]
    if i == 0xFD:
        # 0xfd means the next two bytes are the number
        return little_endian_to_int(s.read(2))
    elif i == 0xFE:
        # 0xfe means the next four bytes are the number
        return little_endian_to_int(s.read(4))
    elif i == 0xFF:
        # 0xff means the next eight bytes are the number
        return little_endian_to_int(s.read(8))
    else:
        # anything else is just the integer
        return i


def encode_varint(i: int) -> bytes:
    """encodes an integer as a varint"""
    return py_encode_varint(i)
