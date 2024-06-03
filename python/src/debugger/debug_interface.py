#!/usr/bin/python3
import logging
import sys
import readline  # noqa: F401
# Note readline is not used but including it enables 'input()' command history.
from typing import List
sys.path.append("..")
from debugger.debug_context import DebuggingContext
from debugger.util import has_extension

LOGGER = logging.getLogger(__name__)


USAGE = """ usage: ./debugger.py -file <input_file.ms>"

This program allows the user to debug a bitcoin script file.
"""


HELP = """
This is the bitcoin script debugger help

h, help -- Prints this message.
q, quit, exit -- Quits the program.

file <filename> -- Loads the specified script file for debugging.
list -- List the current script file contents.
run -- Runs the current loaded script until breakpoint or error.
i <script> -- Execute script interactively in the current context.

hex -- Display the main stack in hexidecimal values.
dec -- Display the main stack in decimal values.

reset -- Reset the script to the staring position.
s -- Step over the next instruction.
n -- Like s but does not step into functions.
f -- Run until the current function is complete.
c -- Continue the current loaded script until breakpoint or error.

b -- Adds a breakpoint on the current operation.
b <n>-- Adds a breakpoint on the nth operation.
info break -- List all the current breakpoints.
d <n> -- Deletes breakpoint number n.

"""


class DebuggerInterface:
    """ Provides the interface to the debugger
    """
    def __init__(self):
        self.context = DebuggingContext()
        # Display the stack in hex
        self.hex_stack = False

    def set_noisy(self, boolean: bool) -> None:
        self.context.noisy = boolean

    def print_status(self) -> None:
        """ Print out the current stack contents
        """
        altstack = self.context.get_altstack()
        stack = self.context.get_stack()
        if self.hex_stack:
            # Print stack in hex form
            hstack = [e.hex() for e in stack]
            print(f"altstack = {altstack}, stack(hex) = {hstack}")
        else:
            print(f"altstack = {altstack}, stack = {stack}")

    def load_script_file(self, fname: str) -> None:
        bits = fname.split(".")

        if len(bits) > 1:
            if bits[-1] not in ("bs"):
                print(f"Wrong file extension: {fname}")
            else:
                if self.context.noisy:
                    print(f"Loading filename: {fname}")
                self.context.load_script_file(fname)
        else:
            print(f"No file extension: {fname}")

    def has_script(self) -> bool:
        """ Return True if we have a script loaded.
        """
        return self.context.has_script()

    def run(self) -> None:
        if self.has_script():
            self.context.reset()
            self.context.run()
        else:
            print("No script loaded.")

    def reset(self) -> None:
        LOGGER.info("reset")
        if self.has_script():
            self.context.reset()
        else:
            print("No script loaded.")

    def _next(self) -> None:
        """ Perform the next operation, all setup has been performed by step
            this needs to be aware of the number of MOP_FNCALLS we have done and exit the same number of times
        """
        first_time = True

        while first_time:
            first_time = False
            if not self.context.step():
                print("Operation failed.")
                break

    def step(self, stepping: bool) -> None:
        """ Step over the next operation.
            The stepping flag is used to indicate if we are stepping into (function_table) functions or not.
        """
        if not self.has_script():
            print("No script loaded.")
            return

        if self.context.is_not_runable():
            self.context.reset()

        if self.context.can_run():
            if stepping:
                self.context.step()
            else:
                self._next()
        else:
            print('At end of script, use "reset" to run again.')

    def continue_to_fn_end(self) -> None:
        if not self.has_script():
            print("No script loaded.")
            return

        if self.context.is_not_runable():
            self.context.reset()
        elif self.context.can_run():
            # Step to step over current breakpoint
            self.context.step()
            self.context.run(stop_on_fn_end=True)
        else:
            print('At end of script, use "reset" to run again.')

    def continue_script(self) -> None:
        """ Continue - but we can't use that word
        """
        if not self.has_script():
            print("No script loaded.")
            return

        if self.context.is_not_runable():
            self.context.reset()

        if self.context.can_run():
            # step to step over current breakpoint
            self.context.step()
            self.context.run()
        else:
            print('At end of script, use "reset" to run again.')

    def add_breakpoint(self, user_input: List[str]) -> None:
        if not self.has_script():
            print("No script loaded.")
            return

        if len(user_input) > 1:
            n = int(user_input[1])
            if n >= self.context.get_number_of_operations():
                print('Breakpoint beyond end of script.')
                return
        else:
            if self.context.is_not_runable():
                print('Script is not running.')
                return
            else:
                if isinstance(self.context.ip, int):
                    n = max(self.context.ip - 1, 0)
                else:
                    n = 0

        bpid = self.context.breakpoints.add(n)
        if bpid is None:
            print("Breakpoint already present at this address.")
        else:
            if self.context.noisy:
                print(f"Added breakpoint {bpid} at {n}")

    def list_breakpoints(self) -> None:
        bps = self.context.breakpoints.get_all()
        if len(bps) == 0:
            print("No breakpoints.")
        else:
            for k, v in bps.items():
                print(f"Breakpoint: {k} operation number: {v}")

    def delete_breakpoint(self, user_input: List[str]) -> None:
        if len(user_input) < 2:
            print("Provide the n of the breakpoint to delete.")
        else:
            n = user_input[1]
            bp = self.context.breakpoints.get_all()
            if n in bp.keys():
                if self.context.noisy:
                    print(f"Deleted breakpoint {n}.")
                self.context.breakpoints.delete(n)
            else:
                print(f"Breakpoint {n} not found.")

    def interpreter_mode(self, user_input: List[str]) -> None:
        if len(user_input) > 1:
            # interpret line
            s = user_input[1:]
            line: str = " ".join(s)
            self.context.interpret_line(line)

    def process_input(self, user_input: List[str]) -> None:
        if user_input[0] in ("h", "help"):
            print(HELP)
        elif user_input[0] == "file":
            if len(user_input) < 2:
                print("The file command requires a filename.")
            else:
                self.load_script_file(user_input[1])
        elif user_input[0] == "list":
            self.context.list()
        elif user_input[0] == "info" and user_input[1] == "break":
            self.list_breakpoints()
        elif user_input[0] == "hex":
            self.hex_stack = True
        elif user_input[0] == "dec":
            self.hex_stack = False
        elif user_input[0] == "reset":
            self.reset()
        elif user_input[0] in ("r", "run"):
            self.run()
        elif user_input[0] == "s":
            self.step(stepping=True)
        elif user_input[0] == "n":
            self.step(stepping=False)
        elif user_input[0] == "f":
            self.continue_to_fn_end()
        elif user_input[0] == "c":
            self.continue_script()
        elif user_input[0] == "b":
            self.add_breakpoint(user_input)
        elif user_input[0] == "d":
            self.delete_breakpoint(user_input)
        elif user_input[0] == "i":
            self.interpreter_mode(user_input)
        else:
            print(f'Unknown command "{user_input[0]}"".')

    def read_eval_print_loop(self) -> None:
        """ Main print-read-eval loop of debugger.
        """
        while True:
            self.print_status()
            user_input = input("(gdb) ")
            split_input: List[str] = user_input.strip().split()
            if len(split_input) == 0:
                pass
            elif split_input[0] in ("q", "quit", "exit"):
                break
            else:
                self.process_input(split_input)

    def load_files_from_list(self, filenames: List[str]) -> None:
        """ Parse the provided list of filenames and load script files.
        """
        for fname in filenames:
            # determine the file extension
            if has_extension(fname, "bs") or has_extension(fname, "ms"):
                self.load_script_file(fname)
            else:
                print(f"Unknown file type: {fname}")
                print(USAGE)
                sys.exit()
