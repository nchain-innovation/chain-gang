from typing import List

from .engine_types import StackElement

# Maximum script number length before Genesis (equal to CScriptNum::MAXIMUM_ELEMENT_SIZE)
MAX_SCRIPT_NUM_LENGTH_BEFORE_GENESIS = 4
# Maximum script number length after Genesis
MAX_SCRIPT_NUM_LENGTH_AFTER_GENESIS = 750 * 1000
# Maximum size that we are using
MAXIMUM_ELEMENT_SIZE = MAX_SCRIPT_NUM_LENGTH_AFTER_GENESIS


def encode_num(num: int) -> bytes:
    """ Encode a number, return a bytearray in little endian
    """
    if num == 0:
        return b""
    abs_num = abs(num)
    negative = num < 0
    result = bytearray()
    while abs_num:
        result.append(abs_num & 0xFF)
        abs_num >>= 8
    if result[-1] & 0x80:
        if negative:
            result.append(0x80)
        else:
            result.append(0)
    elif negative:
        result[-1] |= 0x80
    return bytes(result)


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

    if isinstance(element, int):
        return element

    elif isinstance(element, bytes):
        try:
            b = element
            big_endian = b[::-1]
        except TypeError:
            # TypeError: 'int' object is not subscriptable
            return int(element)
        if big_endian[0] & 0x80:
            negative = True
            result = big_endian[0] & 0x7F
        else:
            negative = False
            result = big_endian[0]
        for c in big_endian[1:]:
            result <<= 8
            result += c
        if negative:
            return -result
        else:
            return result
    else:
        raise ValueError(f"Value is of unknown type: {element} {type(element)}")


def insert_num(val: int) -> List[int]:
    """ This function is used to insert numbers into script
    """
    val_as_bytes = bytearray(encode_num(val))
    length = len(val_as_bytes)
    assert length < 0x4c, "Length of number too long, need to encode using OP_PUSHDATA"
    # Insert the length
    val_as_bytes.insert(0, length)
    return list(val_as_bytes)
