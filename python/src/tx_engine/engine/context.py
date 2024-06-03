from typing import Optional

from tx_engine.chain_gang import py_script_eval, py_decode_num

from .script import Script, cmds_as_bytes
from .engine_types import Commands, Stack, StackElement


def decode_element(elem: StackElement) -> int:
    try:
        retval = py_decode_num(bytes(elem))
    except RuntimeError as e:
        print(f"runtime error {e}")
        retval = elem
        print(f"elem={elem}, retval={retval}, type={type(retval)}")  # type: ignore[str-bytes-safe]
    return retval


class Context:
    """ This class captures an execution context for the script
    """
    def __init__(self, script: None | Script = None, cmds: None | Commands = None, ip_limit: None | int = None, z: None | bytes = None):
        self.cmds: Commands
        self.ip_limit: Optional[int]
        self.z: Optional[bytes]
        self.stack: Stack = []
        self.alt_stack: Stack = []
        self.raw_stack: Stack = []
        self.raw_alt_stack: Stack = []

        if script:
            self.cmds = script.get_commands()
        elif cmds:
            self.cmds = cmds[:]
        else:
            self.cmds = []

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

    def reset_stacks(self) -> None:
        self.stack = []
        self.alt_stack = []
        self.raw_stack = []
        self.raw_alt_stack = []

    def evaluate_core(self, quiet: bool = False) -> bool:
        """ evaluate_core calls the interpreter and returns the stacks
            if quiet is true, dont print exceptions
        """
        try:
            cmds = cmds_as_bytes(self.cmds)
            # print(f"cmds={cmds.hex()}")
        except Exception as e:
            if not quiet:
                print(f"cmds_as_bytes exception '{e}'")
            return False
        try:
            (self.raw_stack, self.raw_alt_stack) = py_script_eval(cmds, self.ip_limit)
        except Exception as e:
            if not quiet:
                print(f"script_eval exception '{e}'")
            # print(f"cmds={self.cmds}")
            return False
        else:
            # print(f"before self.stack={self.stack}")
            return True

    def evaluate(self, quiet: bool = False) -> bool:
        """ evaluate calls Evaluate_core and checks the stack has the correct value on return
            if quiet is true, dont print exceptions
        """
        if not self.evaluate_core(quiet):
            return False
        self.stack = [decode_element(s) for s in self.raw_stack]
        # print(f"after self.stack={self.stack}")
        self.alt_stack = [decode_element(s) for s in self.raw_alt_stack]

        if len(self.stack) == 0:
            return False
        if self.stack[-1] == 0:  # was b""
            return False
        return True

    def get_stack(self) -> Stack:
        """ Return the data stack as human readable
        """
        self.stack = [decode_element(s) for s in self.raw_stack]

        return self.stack

    def get_altstack(self):
        """ Return the get_altstack as human readable
        """
        self.alt_stack = [decode_element(s) for s in self.raw_alt_stack]
        return self.alt_stack
