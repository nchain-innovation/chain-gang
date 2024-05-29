
import re
from io import BytesIO

from .util import read_varint, little_endian_to_int, encode_varint
from .op_codes import OP_PUSHDATA1, OP_PUSHDATA2, OP_PUSHDATA4
from .decode_op import decode_op

from .engine_types import Commands

from tx_engine.chain_gang import py_script_serialise


def read_pushdata(s: BytesIO, cmds: Commands, read_ints: int) -> int:
    """ read n bytes, append to cmd and return length - used by OP_PUSHDATA
    """
    data_length = little_endian_to_int(s.read(read_ints))
    cmds.append(s.read(data_length))
    return data_length + 1


def cmds_as_bytes(cmds: Commands) -> bytes:
    """ Given commands return bytes - prior to passing to Rust
    """
    retval = bytearray()
    for c in cmds:
        if isinstance(c, int):
            retval += c.to_bytes()
        elif isinstance(c, list):
            retval += cmds_as_bytes(c)
        else:
            # If we have a byte array without a preceeding length, add it, if less than 0x4c
            # Otherwise would expect OP_PUSHDATA preceeding
            if len(c) < 0x4c:
                if len(retval) == 0:
                    retval += len(c).to_bytes()
                elif not retval[-1] in [OP_PUSHDATA1, OP_PUSHDATA2, OP_PUSHDATA4] and retval[-1] != len(c):
                    retval += len(c).to_bytes()
            retval += c
    return bytes(retval)


class Script:
    """ This class represents bitcoin script
    """
    def __init__(self, cmds: Commands | None = None):
        self.cmds: Commands
        if cmds is None:
            self.cmds = []
        else:
            self.cmds = cmds

    def __add__(self, other):
        """ Enable the addition of two scripts
        """
        return Script(self.cmds + other.cmds)

    def raw_serialize(self) -> bytes:
        """ Returns the serialized script, without the prepended length
        """
        cmds = cmds_as_bytes(self.cmds)
        return py_script_serialise(cmds)

    def serialize(self) -> bytes:
        """ Returns the serialized script, with the prepended length
        """
        result = self.raw_serialize()
        total = len(result)
        return bytearray(encode_varint(total)) + bytearray(result)

    @classmethod
    # def parse(cls, s: BytesIO) -> Script:
    def parse(cls, s: BytesIO):
        """ Parse the provided byte stream
        """
        length = read_varint(s)
        cmds: Commands = []
        count = 0
        while count < length:
            current = s.read(1)
            count += 1
            current_byte = current[0]
            if current_byte == OP_PUSHDATA1:
                count += read_pushdata(s, cmds, 1)
            elif current_byte == OP_PUSHDATA2:
                count += read_pushdata(s, cmds, 2)
            elif current_byte == OP_PUSHDATA4:
                count += read_pushdata(s, cmds, 4)

            elif current_byte < 0x4c:
                # The next opcode bytes is data to be pushed onto the stack
                cmds.append(s.read(current_byte))
                count += current_byte
            else:
                cmds.append(current_byte)

        if count != length:
            raise SyntaxError(f"parsing script failed count = {count} length = {length}")
        return cls(cmds)

    @classmethod
    # def parse_string(cls, s: str) -> Script:
    def parse_string(cls, s):
        """ Converts a string to a Script
        """
        stripped: str = s.strip()
        split = re.split(" |,|\n", stripped)
        ss = list(filter(lambda x: x.strip() != '', split))
        decoded = list(map(decode_op, ss))
        retval = cls._remove_byte_lengths(decoded)
        return cls(retval)

    @classmethod
    def _remove_byte_lengths(cls, decoded: Commands):
        """ Filter out all small bytes as they are push data op codes 1-75 (< 0x4c)
            Removes byte len that is followed by number of bytes
        """
        retval = []
        for i, element in enumerate(decoded):
            if i + 1 < len(decoded):
                next = decoded[i + 1]
                if i > 0:
                    # Check previous element
                    if decoded[i - 1] in (OP_PUSHDATA1, OP_PUSHDATA2, OP_PUSHDATA4):
                        retval.append(element)
                        continue
                # fmt:off
                if (isinstance(element, bytes) and isinstance(next, bytes) and  # noqa: W504
                        len(next) == int.from_bytes(element, byteorder="little") and  # noqa: W504
                        (element > b"\x00" and element <= b"\x4b")):
                    pass
                else:
                    retval.append(element)
                # fmt:on
            else:
                retval.append(element)
        return retval

    def get_commands(self) -> Commands:
        """ Return a copy of the commands in this script
        """
        return self.cmds[:]
