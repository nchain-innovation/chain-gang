""" Tests of the debugger context
"""
import unittest

from tx_engine import Context, Script, Stack

from tx_engine.engine.op_codes import (
    OP_1,
    OP_2,
    OP_3,
    OP_4,
)


class DebugTest(unittest.TestCase):
    """ Tests of the debugger context
    """

    def test_ip_limit_1(self):
        script = Script([OP_1, OP_2, OP_3, OP_4])
        context = Context(script=script, ip_limit=1)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), Stack([[1]]))

    def test_ip_limit_2(self):
        script = Script([OP_1, OP_2, OP_3, OP_4])
        context = Context(script=script, ip_limit=2)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), Stack([[1], [2]]))

    def test_ip_limit_3(self):
        script = Script([OP_1, OP_2, OP_3, OP_4])
        context = Context(script=script, ip_limit=3)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), Stack([[1], [2], [3]]))

    def test_ip_limit_with_z_matches_without_z(self):
        """Regression: py_script_eval_pystack must pass start_at/break_at consistently when z is set."""
        script = Script([OP_1, OP_2, OP_3, OP_4])
        z = bytes(32)
        without_z = Context(script=script, ip_limit=2)
        with_z = Context(script=script, ip_limit=2, z=z)
        self.assertTrue(without_z.evaluate_core())
        self.assertTrue(with_z.evaluate_core())
        self.assertEqual(without_z.get_stack(), with_z.get_stack())


if __name__ == "__main__":
    unittest.main()
