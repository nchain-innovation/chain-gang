

import unittest

import sys
sys.path.append("..")

from tx_engine.engine.context import Context
from tx_engine.engine.script import Script


from tx_engine.engine.op_codes import (
    OP_1,
    OP_2,
    OP_3,
    OP_4,
)


class DebugTest(unittest.TestCase):
    def test_ip_limit_1(self):
        script = Script([OP_1, OP_2, OP_3, OP_4])
        context = Context(script=script, ip_limit=2)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [1, 2])

    def test_ip_limit_2(self):
        script = Script([OP_1, OP_2, OP_3, OP_4])
        context = Context(script=script, ip_limit=3)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [1, 2, 3])


if __name__ == "__main__":
    unittest.main()
