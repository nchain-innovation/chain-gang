""" Chronicle Python Context parity tests
"""
import unittest

from tx_engine import Context, Script
from tx_engine.engine.util import max_script_num_length, MAX_SCRIPT_NUM_LENGTH_CHRONICLE


class ChronicleContextTest(unittest.TestCase):
    """ Chronicle rules exposed through Python Context and util helpers
    """

    def test_op_ver_with_tx_version(self):
        script = Script.parse_string("OP_VER OP_2 OP_NUMEQUAL")
        self.assertTrue(Context(script=script, tx_version=2).evaluate())
        self.assertFalse(Context(script=script, tx_version=1).evaluate())

    def test_two_phase_carries_stack(self):
        unlock = Script.parse_string("OP_2 OP_3 OP_ADD")
        lock = Script.parse_string("OP_5 OP_EQUAL")
        self.assertTrue(
            Context(script=unlock, lock_script=lock, tx_version=2).evaluate()
        )

    def test_relaxed_clean_stack(self):
        script = Script.parse_string("OP_1 OP_2 OP_1")
        self.assertFalse(Context(script=script, tx_version=1).evaluate())
        self.assertTrue(Context(script=script, tx_version=2).evaluate())

    def test_max_script_num_length(self):
        self.assertEqual(max_script_num_length(1), 750_000)
        self.assertEqual(max_script_num_length(2), MAX_SCRIPT_NUM_LENGTH_CHRONICLE)
        self.assertEqual(max_script_num_length(None), 750_000)
        self.assertEqual(max_script_num_length(2, pregenesis=True), 4)


if __name__ == "__main__":
    unittest.main()
