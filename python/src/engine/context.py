import shutil

from engine.script import Script
from engine.engine_types import Commands, Stack
from typing import Optional

from engine.util import decode_num


# Copy library
shutil.copyfile("../../target/debug/libchain_gang.dylib", "./chain_gang.so")

# import copied library
from chain_gang import script_eval


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

    def evaluate(self) -> bool:
        print(f"self.cmds = {self.cmds}")
        try:
            (stack, self.alt_stack) = script_eval(bytes(self.cmds))
        except Exception as e:
            print(f"exception '{e}'")
            return False
        else:
            print(f"stack = {stack}")
            if len(stack) == 0:
                return False
            
            self.stack = [sum(s) for s in stack]
            if stack.pop() == b"":
                return False

            return True

    def get_stack(self) -> Stack:
        """ Return the data stack as human readable
        """
        return list(map(decode_num, self.stack))

    def get_altstack(self):
        """ Return the get_altstack as human readable
        """
        return list(map(decode_num, self.altstack))

