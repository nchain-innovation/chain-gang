from typing import Optional

from tx_engine.tx_engine import py_script_eval, py_decode_num, Script

from .engine_types import Commands, Stack, StackElement
from .op_codes import OP_PUSHDATA1, OP_PUSHDATA2, OP_PUSHDATA4


def decode_element(elem: StackElement) -> int:
    try:
        retval = py_decode_num(bytes(elem))
    except RuntimeError as e:
        print(f"runtime error {e}")
        retval = elem
        print(f"elem={elem}, retval={retval}, type={type(retval)}")  # type: ignore[str-bytes-safe]
    return retval


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

        self.ip_limit = ip_limit if ip_limit else None
        self.z = z if z else None

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
        except Exception as e:
            if not quiet:
                print(f"cmds_as_bytes exception '{e}'")
            return False
        try:
            (self.raw_stack, self.raw_alt_stack) = py_script_eval(cmds, self.ip_limit, self.z)
        except Exception as e:
            if not quiet:
                print(f"script_eval exception '{e}'")
            return False
        else:
            return True

    def evaluate(self, quiet: bool = False) -> bool:
        """ evaluate calls Evaluate_core and checks the stack has the correct value on return
            if quiet is true, dont print exceptions
        """
        if not self.evaluate_core(quiet):
            return False
        self.stack = [decode_element(s) for s in self.raw_stack]
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
