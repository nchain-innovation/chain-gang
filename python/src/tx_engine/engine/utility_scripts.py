from typing import Optional

from tx_engine import Script
from tx_engine.engine.util import (
    GROUP_ORDER_INT,
    HALF_GROUP_ORDER_INT,
    Gx_bytes,
    encode_num
)


def reverse_endianness(length: int) -> Script:
    """
    Input parameters:
        - Stack: x
        - Altstack: []
    Output:
        - x with endianness reversed
    Remark: the variable length is the length of x
    """
    out = Script()
    for i in range(length - 1):
        out += Script.parse_string('OP_1 OP_SPLIT')
    for i in range(length - 1):
        out += Script.parse_string('OP_SWAP OP_CAT')
    return out


def paddingToBytes(n: int, byte_order: str) -> Script:
    """
    Input parameters:
        - Stack: x
        - Altstack = []
    Output:
        - x padded to a length of n bytes
    Remark: the byte order variable tells the function if the number to be padded is in big endian (add zeros at the beginning) or in little endian (add zeros at the end)
    """
    maxZeros = '0x' + '0' * 2 * n

    out = Script.parse_string(maxZeros)
    match byte_order:
        case 'little':
            out += Script.parse_string('OP_CAT ' + str(n) + ' OP_SPLIT OP_DROP')
        case 'big':
            out += Script.parse_string('OP_SWAP OP_CAT OP_SIZE ' + str(n) + ' OP_SUB OP_SPLIT OP_NIP')
        case _:
            raise ValueError('Unrecognised byte_order variable')

    return out


def xToPubKey() -> Script:
    """
    Input parameters:
        - Stack: x y
        - Altstack = []
    Output:
        - PubKey associated to x y (in compressed form)
    Assumption on data:
        - x and y are the coordinates of a point on the secp256k1 curve
        - x and y are passed as integers, i.e., minimally encoded and in little endian
    """
    out = Script.parse_string('OP_2 OP_MOD OP_IF OP_3 OP_ELSE OP_2 OP_ENDIF OP_SWAP') + paddingToBytes(n=32, byte_order='little') + reverseEndianness(32) + Script.parse_string('OP_CAT')
    return out


def ensure_is_positive() -> Script:
    """
    Input parameters:
        - Stack: x
        - Altstack: []
    Output:
        - x (in little endian) minimally encoded as a positive number
    Assumption on data:
        - x is an a number in little endian
    """
    return Script.parse_string('0x00 OP_CAT OP_BIN2NUM')


def toModuloClass(loadConstantFrom: str, cleanConstant: Optional[bool] = None, modulo: Optional[int] = None) -> Script:
    """
    From WP1611.
    Input parameters:
        If loadConstantsFrom == 'altstack':
            - Stack: x
            - Altstack: modulo
        If loadConstantsFrom == 'bottom':
            - Stack: modulo x
            - Altstack = []
    Output:
        - x % modulo (element in modulo class, i.e., x % modulo >= 0)
    RERMARKS:
        - It transforms a negative number to its equivalent positive representation.
        - If loading the modulo from altstack or bottom, the function assumes the modulo is correct.
    """

    match loadConstantFrom:
        case 'altstack':
            out = Script.parse_string('OP_FROMALTSTACK OP_MOD OP_FROMALTSTACK OP_ADD OP_FROMALTSTACK OP_MOD')
        case 'bottom':
            assert (cleanConstant is not None)
            if cleanConstant:
                out = Script.parse_string('OP_DEPTH OP_1SUB OP_ROLL OP_TUCK OP_MOD OP_OVER OP_ADD OP_SWAP OP_MOD')
            else:
                out = Script.parse_string('OP_DEPTH OP_1SUB OP_PICK OP_TUCK OP_MOD OP_OVER OP_ADD OP_SWAP OP_MOD')
        case 'modulo':
            out = Script.parse_string(str(modulo)) + Script.parse_string('OP_MOD') + Script.parse_string(str(modulo)) + Script.parse_string('ADD') + Script.parse_string(str(modulo)) + Script.parse_string('OP_MOD')
        case _:
            raise ValueError('Option for constant loading is not supported')

    return out


def toCanonical() -> Script:
    """
    Input parameters:
        - Stack: s (s component of an ECDSA signature)
        - Altstack = []
    Output:
        - s in canonical format (s < GROUP_ORDER_INT/2)
    """
    return Script.parse_string('OP_DUP ' + str(HALF_GROUP_ORDER_INT) + ' OP_GREATERTHAN OP_IF ' + str(GROUP_ORDER_INT) + ' OP_SWAP OP_SUB OP_ENDIF')


def constructDer() -> Script:
    """
    Input parameters:
        - Stack: r s (components of a signature)
        - Altstack: []
    Output:
        - Der(r,s)
    Assumption on data:
        - s is in canonical format
    """
    out = Script.parse_string('OP_SWAP OP_SIZE OP_SWAP OP_CAT OP_2 OP_SWAP OP_CAT OP_SWAP OP_SIZE OP_SWAP OP_CAT OP_2 OP_SWAP OP_CAT OP_CAT OP_SIZE OP_SWAP OP_CAT 0x30 OP_SWAP OP_CAT 0x41 OP_CAT')
    return out


def duplicateElement(constant: bytes, n: int) -> Script:
    '''
    Duplicate a constant n times.
    '''
    out = Script()
    while n > 0:
        if out == Script():
            n -= 1
            out += Script(cmds=[constant])
        elif out == Script(cmds=[constant]):
            n -= 1
            out += Script.parse_string('OP_DUP')
        elif out == Script(cmds=[constant]) + Script.parse_string('OP_DUP'):
            if n >= 2:
                n -= 2
                out += Script.parse_string('OP_2DUP')
            else:
                n -= 1
                out += Script.parse_string('OP_DUP')
        elif out == Script(cmds=[constant]) + Script.parse_string('OP_DUP OP_2DUP'):
            while n >= 3:
                n -= 3
                out += Script.parse_string('OP_3DUP')
            if n == 2:
                n -= 2
                out += Script.parse_string('OP_2DUP')
            if n == 1:
                n -= 1
                out += Script.parse_string('OP_DUP')
    return out


def loadAltStack(constant: bytes, n: int) -> Script:
    '''
    Load altstack with n copies of the constant.
    '''
    out = Script()
    out += duplicateElement(constant=constant, n=n)
    out += Script.parse_string(' '.join(['OP_TOALTSTACK'] * n))
    return out


def pick(position: int, nElements: int) -> Script:
    '''
    Script to pick nElements starting from position.
    Position is the stack position, so we star counting from 0.
    Example:
        nElements = 2, position = 2 --> OP_2 OP_PICK OP_2 OP_PICK
    '''
    return Script.parse_string(' '.join([str(position), 'OP_PICK'] * nElements))


def roll(position: int, nElements: int) -> Script:
    '''
    Script to roll nElements starting from position.
    Position is the stack position, so we star counting from 0.
    Example:
        nElements = 2, position = 2 --> OP_2 OP_ROLL OP_2 OP_ROLL
    '''
    return Script.parse_string(' '.join([str(position), 'OP_ROLL'] * nElements))

def nums_to_script(nums: list[int]) -> Script:

    '''
        Takes a list of number and returns the script pushing those numbers to the stack
    '''

    out = Script()
    for n in nums:
        if n == -1:
            out += Script.parse_string('-1')
        elif 0 <= n <= 16:
            out += Script.parse_string(str(n))
        else:
            out.append_pushdata(encode_num(n))

    return out