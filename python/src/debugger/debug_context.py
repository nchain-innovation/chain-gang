import logging
from typing import Optional

from debugger.stack_frame import StackFrame
from tx_engine.engine.script import Script

from debugger.breakpoints import Breakpoints

LOGGER = logging.getLogger(__name__)


class DebuggingContext:
    """ This is the state of the current debugging session
    """
    def __init__(self):
        self.noisy: bool = True  # print operations as they are executed
        self.sf = StackFrame()

    def get_stack(self):
        return self.sf.context.get_stack()[:]

    def get_raw_stack(self):
        # return the stack without the values converted to human readable form
        return self.sf.context.raw_stack[:]

    def get_altstack(self):
        return self.sf.context.get_altstack()[:]

    @property
    def breakpoints(self) -> Breakpoints:
        return self.sf.breakpoints

    @property
    def ip(self) -> Optional[int]:
        return self.sf.ip

    def run_to_breakpoint(self):
        pass

    def step(self) -> bool:
        """ Step over the next instruction
            Return True if operation was successful
        """
        if self.noisy:
            self.sf.print_cmd()

        # Next instruction
        assert isinstance(self.sf.ip, int)
        self.sf.ip += 1

        self.sf.context.ip_limit = self.sf.ip
        return self.sf.context.evaluate_core()

    def reset(self) -> None:
        """ Reset the script ready to run - interface to Debugger
        """
        LOGGER.info("debug_context - reset")

        self.sf.reset_core()
        self.sf.reset_stacks()

    def can_run(self) -> bool:
        """ Return true if script has not finished
        """
        return self.sf.can_run()

    def run(self, stop_on_fn_end: bool = False) -> None:
        """ Run the script
        """
        LOGGER.info(f"evaluate_core - {self.sf.ip}")
        """ Return false if failed
        """
        succ = self.sf.context.evaluate_core()
        print(f"succ={succ}")

        """
        while self.can_run():
            if self.sf.hit_breakpoint():
                if self.noisy:
                    self.sf.print_breakpoint()
                break
            (succ, exited_function) = self.step()
            if not succ:
                print("Operation failed.")
                break
            if exited_function and stop_on_fn_end:
                if self.noisy:
                    print("Exited function.")
                break
        """

    def get_number_of_operations(self) -> int:
        return len(self.sf.script_state.get_commands())

    def interpret_line(self, user_input: str) -> None:
        """ Interpret the provided line in separate context.
        """
        try:
            script = Script.parse_string(user_input)
        except NameError as e:
            print(e)
            return

        self.sf.context.set_commands(script.get_commands())
        if not self.sf.context.evaluate_core():
            print("Operation failed")

    def has_script(self) -> bool:
        """ Return True if we have a script loaded.
        """
        return self.sf.script_state.script is not None

    def is_not_runable(self) -> bool:
        """ Return True if script is not runable
        """
        return self.sf.ip is None

    def list(self) -> None:
        """ List the commands
        """
        self.sf.script_state.list()

    def load_script_file(self, fname) -> None:
        """ Load script file from fname
            Reset the breakpoints as these will no longer be relevant
            Load any use(d) library files
        """
        self.sf.script_state.load_file(fname)
        self.sf.breakpoints.reset_all()

    def load_source(self, source) -> None:
        """ Load script file from fname
            Reset the breakpoints as these will no longer be relevant
            Load any use(d) library files
        """
        self.sf.script_state.load_source(source)
        self.sf.breakpoints.reset_all()
