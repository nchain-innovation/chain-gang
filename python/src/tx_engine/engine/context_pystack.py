""" This is the execution context for the script
"""

from typing import Optional, List

from tx_engine.tx_engine import py_script_eval, py_script_eval_pystack, Script, Stack, decode_num_stack

from .op_codes import OP_PUSHDATA1, OP_PUSHDATA2, OP_PUSHDATA4

class Context_PyStack:
    """ This class captures an execution context for the script
    """
    def __init__(self, script: None | Script = None, ip_start: None | int = None, ip_limit: None |int = None, z: None | bytes = None):
        """ Intial setup
        """
        # ip_limit -> this now means where to stop execution.
        # it'll have to change from line number to byte count into the script.
        self.ip_start: Optional[int]
        self.ip_limit: Optional[int]
        self.z: Optional[bytes]
        self.stack: Stack = Stack() 
        self.alt_stack: Stack = Stack()

        # script.get_commamds() returns bytes I believe.
        if script:
            self.cmds = script.get_commands()
        #elif cmds:
        #    self.cmds = cmds[:]
        else:
            self.cmds = []

        self.ip_start = ip_start if ip_start else None
        self.ip_limit = ip_limit if ip_limit else None
        self.z = z if z else None

    def set_commands(self, script: Script) -> None:
        """ Set the commands
        """
        #self.cmds = cmds[:]
        self.cmds = script.get_commands()

    def reset_stacks(self) -> None:
        """ Reset the stacks
        """
        self.stack = Stack()
        self.alt_stack = Stack()

    def evaluate_core(self, quiet: bool = False) -> bool:
        """ evaluate_core calls the interpreter and returns the stacks
            if quiet is true, dont print exceptions
        """

        try:
            (self.stack, self.alt_stack, finish_loc) = py_script_eval_pystack(self.cmds, self.ip_start, self.ip_limit, self.z,  self.stack, self.alt_stack)
        except Exception as e:
            if not quiet:
                print(f"script_eval exception '{e}'")
            return False
        return True
    

    def evaluate(self, quiet: bool = False) -> bool:
        """ evaluate calls Evaluate_core and checks the stack has the correct value on return
            if quiet is true, dont print exceptions
        """
        if not self.evaluate_core(quiet):
            return False
        
        if self.stack.size() == 0:
            return False
        
        # if the size is 1, then check the top element for either empty or zero
        if self.stack.size() == 1:
            # no entry or 0 => false.
            if self.get_stack() == Stack([[]]) or self.get_stack() == Stack([[0]]):
                return False
        # if the top element is 0, OP_0 or empty it's a fail
        if self.get_stack()[0] == [0] or  self.get_stack()[0] == []:
            return False
        return True

    def get_stack(self) -> List:
        """ Return the data stack as human readable
        """
        #self.stack = [decode_element(s) for s in self.raw_stack]
        return self.stack
    
    def get_altstack(self):
        """ Return the get_altstack as human readable
        """
        #self.alt_stack = [decode_element(s) for s in self.raw_alt_stack]
        return self.alt_stack
    
    def set_ip_start(self, start: int) -> None:
        self.ip_start = start

    def set_ip_limit(self, limit: int) -> None:
        self.ip_limit = limit
