import logging
from typing import List
from tx_engine import Script
from debugger.decode_op import cmd_repr
from debugger.util import has_extension


LOGGER = logging.getLogger(__name__)


def print_cmd(i: int, cmd, indent: int = 0) -> int:
    """ Prints the command and manages the indent
    """
    cmd = cmd_repr(cmd)
    if isinstance(cmd, str):
        if cmd in ("OP_ELSE", "OP_ENDIF"):
            indent -= 2
        print(f"{i}: {' '*indent}{cmd}")
        if cmd in ("OP_IF", "OP_NOTIF", "OP_ELSE"):
            indent += 2
    else:
        print(f"{i}: {' '*indent}{int.from_bytes(cmd, byteorder='little')} (0x{cmd.hex()}, {cmd})")
    return indent


class ScriptState():
    """ This holds the interpreted script, provides load and list methods.
    """
    def __init__(self):
        self.script = None

    def load_file(self, filename: str) -> None:
        """ Load loaded file, but don't parse it
        """
        try:
            # load it
            with open(filename, "r") as f:
                contents = f.readlines()
        except FileNotFoundError as e:
            print(e)
        else:
            if has_extension(filename, "bs"):
                self.parse_script(contents)

    def parse_script(self, contents: List[str]) -> None:
        if contents:
            # parse contents
            line: str = " ".join(contents)
            self.script = Script.parse_string(line)

    def list(self) -> None:
        """ Prints out script
        """
        if self.script:
            cmds = self.script.get_commands()
            indent = 0
            for i, cmd in enumerate(cmds):
                indent = print_cmd(i, cmd, indent)
        else:
            print("No file loaded.")

    def get_commands(self):
        if self.script:
            return self.script.get_commands()
        else:
            return []

    def set_commands(self, cmds):
        if self.script is None:
            self.script = Script()
        self.script.cmds = cmds
