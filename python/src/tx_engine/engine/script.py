
import re
from io import BytesIO

from .util import int_to_little_endian, read_varint, little_endian_to_int, encode_varint
from .op_codes import OP_PUSHDATA1, OP_PUSHDATA2, OP_PUSHDATA4
from .decode_op import decode_op
from .engine_types import Commands


def read_data(s: BytesIO, cmds: Commands, read_ints: int) -> int:
    """ read n bytes
    """
    data_length = little_endian_to_int(s.read(read_ints))
    cmds.append(s.read(data_length))
    return data_length + 1


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
        result: bytes = b""
        for cmd in self.cmds:
            if isinstance(cmd, int):
                result += int_to_little_endian(cmd, 1)
            else:
                length = len(cmd)
                if length <= 75:  # Encode as number
                    result += int_to_little_endian(length, 1)
                elif length <= 255:
                    result += int_to_little_endian(OP_PUSHDATA1, 1)
                    result += int_to_little_endian(length, 1)
                elif length <= 65535:
                    result += int_to_little_endian(OP_PUSHDATA2, 1)
                    result += int_to_little_endian(length, 2)
                else:
                    result += int_to_little_endian(OP_PUSHDATA4, 1)
                    result += int_to_little_endian(length, 4)
                result += cmd
        return result

    def serialize(self) -> bytes:
        """ Returns the serialized script, with the prepended length
        """
        result = self.raw_serialize()
        total = len(result)
        return encode_varint(total) + result

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
            if current_byte < 0x4c:
                cmds.append(s.read(current_byte))
                count += current_byte
            elif current_byte == OP_PUSHDATA1:
                count += read_data(s, cmds, 1)
            elif current_byte == OP_PUSHDATA2:
                count += read_data(s, cmds, 2)
            elif current_byte == OP_PUSHDATA4:
                count += read_data(s, cmds, 4)
            else:
                cmds.append(current_byte)
        if count != length:
            raise SyntaxError(f"parsing script failed count = {count} length = {length}")
        return cls(cmds)

    @classmethod
    # def parse_string(cls, s: str) -> Script:
    # def parse_string(cls, s: str):
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
        # Filter out all small bytes as they are push data op codes 1-75
        # Removes byte len that is followed by number of bytes
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
