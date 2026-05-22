""" Chronicle opcode tests
"""
import unittest
from tx_engine import Script, Context
from tx_engine.engine.op_codes import (
    OP_PUSHDATA1,
    OP_SUBSTR,
    OP_LEFT,
    OP_RIGHT,
    OP_LSHIFTNUM,
    OP_RSHIFTNUM,
    OP_4,
    OP_5,
    OP_3,
    OP_2,
    OP_16,
    OP_1,
    OP_EQUAL,
    OP_EQUALVERIFY,
)


class ChronicleOpcodeTest(unittest.TestCase):
    """ Chronicle opcode tests using the Rust script interpreter
    """

    def test_substr_left_right(self):
        script = Script([
            OP_PUSHDATA1, 14, b"BSV Blockchain",
            OP_4, OP_5, OP_SUBSTR,
            OP_PUSHDATA1, 5, b"Block", OP_EQUALVERIFY,
            OP_PUSHDATA1, 14, b"BSV Blockchain",
            OP_3, OP_LEFT,
            OP_PUSHDATA1, 3, b"BSV", OP_EQUALVERIFY,
            OP_PUSHDATA1, 14, b"BSV Blockchain",
            OP_5, OP_RIGHT,
            OP_PUSHDATA1, 5, b"chain", OP_EQUAL,
        ])
        self.assertTrue(Context(script=script).evaluate())

    def test_lshiftnum_rshiftnum(self):
        script = Script.parse_string("OP_4 OP_2 OP_LSHIFTNUM OP_16 OP_EQUAL")
        self.assertTrue(Context(script=script).evaluate())
        script = Script.parse_string("OP_4 OP_2 OP_RSHIFTNUM OP_1 OP_EQUAL")
        self.assertTrue(Context(script=script).evaluate())


if __name__ == "__main__":
    unittest.main()
