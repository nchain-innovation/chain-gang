""" Test if bools and OP_IF
"""
import unittest
from tx_engine import Script, Context, Context_PyStack
from tx_engine.engine.op_codes import OP_0, OP_1, OP_DUP, OP_IF, OP_ELSE, OP_ENDIF


class IfTest(unittest.TestCase):
    def test_true(self):
        """ Simple check of true and false
        """
        context = Context(script=Script([OP_1]))
        # should evaluate to True
        self.assertTrue(context.evaluate())
        context_py_stack = Context_PyStack(script=Script([OP_1]))
        self.assertTrue(context_py_stack.evaluate())

    def test_false(self):
        """ Simple check of true and false
        """
        context = Context(script=Script([OP_0]))
        # should evaluate to False (OP_0)
        self.assertFalse(context.evaluate())

        context_py_stack = Context_PyStack(script=Script([OP_0]))
        self.assertFalse(context_py_stack.evaluate())

    def test_if_endif(self):
        """ Simple OP_IF.. OP_ENDIF
        """
        script = Script([OP_1, OP_0, OP_IF, OP_0, OP_ENDIF])
        context = Context(script=script)
        # should evaluate to True
        self.assertTrue(context.evaluate())

        context_py_stack = Context_PyStack(script=script)
        self.assertTrue(context_py_stack.evaluate())

        script = Script([OP_0, OP_1, OP_IF, OP_0, OP_ENDIF])
        context = Context(script=script)
        # should evaluate to False (OP_0)
        self.assertFalse(context.evaluate())

        context_py_stack = Context_PyStack(script=script)
        self.assertFalse(context_py_stack.evaluate())

    def test_if_else_endif(self):
        """ Simple OP_IF..ELSE..OP_ENDIF
        """
        script = Script([OP_1, OP_IF, OP_0, OP_ELSE, OP_1, OP_ENDIF])
        context = Context(script=script)
        # should evaluate to False (OP_0)
        self.assertFalse(context.evaluate())

        context_py_stack = Context_PyStack(script=script)
        self.assertFalse(context_py_stack.evaluate())

        script = Script([OP_0, OP_IF, OP_0, OP_ELSE, OP_1, OP_ENDIF])
        # should evaluate to True
        context = Context(script=script)
        self.assertTrue(context.evaluate())

        context_py_stack = Context_PyStack(script=script)
        self.assertTrue(context_py_stack.evaluate())

    def test_nested_if_else_endif(self):
        """ Simple nested OP_IF..ELSE..OP_ENDIF
        """
        script = Script(
            [OP_1, OP_DUP, OP_IF, OP_IF, OP_0, OP_ELSE, OP_1, OP_ENDIF, OP_ENDIF]
        )
        context = Context(script=script)
        # should evaluate to False (OP_0)
        self.assertFalse(context.evaluate())

        context_py_stack = Context_PyStack(script=script)
        self.assertFalse(context_py_stack.evaluate())

        script = Script(
            [OP_0, OP_1, OP_IF, OP_IF, OP_0, OP_ELSE, OP_1, OP_ENDIF, OP_ENDIF]
        )
        context = Context(script=script)
        # should evaluate to True
        self.assertTrue(context.evaluate())

        context_py_stack = Context_PyStack(script=script)
        self.assertTrue(context_py_stack.evaluate())


if __name__ == "__main__":
    unittest.main()
