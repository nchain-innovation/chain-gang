from typing import Optional

from .copy_library import copy_library

copy_library()

# import copied library
from chain_gang import script_eval, py_decode_num

from .script import Script
from .engine_types import Commands, Stack


def cmds_as_bytes(cmds: Commands) -> bytes:
    """ Given commands return bytes
    """
    # print(f"cmds = {cmds}")
    retval = bytearray()
    for c in cmds:
        # print(f"c = {c}")
        if isinstance(c, int):
            retval += c.to_bytes()
        elif isinstance(c, list):
            retval += cmds_as_bytes(c)
        else:
            retval += c
    return bytes(retval)


class Context:
    """ This class captures an execution context for the script
    """
    def __init__(self, script: None | Script = None, cmds: None | Commands = None, stack: None | Stack = None, ip_limit: None | int = None, z: None | bytes = None):
        self.cmds: Commands
        self.stack: Stack
        self.ip_limit: Optional[int]
        self.z: Optional[bytes]
        self.altstack: Stack = []

        if script:
            self.cmds = script.get_commands()
        elif cmds:
            self.cmds = cmds[:]
        else:
            self.cmds = []

        if stack:
            self.stack = stack
        else:
            self.stack = []

        if ip_limit:
            self.ip_limit = ip_limit
        else:
            self.ip_limit = None

        if z:
            self.z = z
        else:
            self.z = None

    def set_commands(self, cmds: Commands) -> None:
        self.cmds = cmds[:]

    def evaluate_core(self) -> bool:
        """ evaluate_core calls the interpreter and returns the stacks
        """
        try:
            # cmds = bytes(self.cmds)
            cmds = cmds_as_bytes(self.cmds)
        except Exception as e:
            print(f"exception1 '{e}'")
            return False
        try:
            (self.stack, self.alt_stack) = script_eval(cmds)
        except Exception as e:
            print(f"exception2 '{e}'")
            return False
        else:
            self.stack = [py_decode_num(bytes(s)) for s in self.stack]
            self.alt_stack = [py_decode_num(bytes(s)) for s in self.alt_stack]
            return True

    def evaluate(self) -> bool:
        """ evaluate calls Evaluate_core and checks the stack has the correct value on return
        """
        if not self.evaluate_core():
            return False
        if len(self.stack) == 0:
            return False
        if self.stack[-1] == 0:  # was b""
            return False
        return True

    def get_stack(self) -> Stack:
        """ Return the data stack as human readable
        """
        return self.stack

    def get_altstack(self):
        """ Return the get_altstack as human readable
        """
        return self.alt_stack
