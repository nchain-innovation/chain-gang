
import re

from .util import int_to_little_endian
from .op_codes import OP_PUSHDATA1, OP_PUSHDATA2, OP_PUSHDATA4
from .decode_op import decode_op
from .engine_types import Commands


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
        """ Return the commands in this script
        """
        return self.cmds[:]


""" Script
    Script + Script = Script
    Script.parse_string
    Script.raw_serialize
"""
