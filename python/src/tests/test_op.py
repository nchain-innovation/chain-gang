#!/usr/bin/python3
import sys
sys.path.append("..")

import unittest
from tx_engine import Script, Context

from tx_engine.engine.op_codes import (
    OP_0,
    OP_1,
    OP_2,
    OP_3,
    OP_4,
    OP_5,
    OP_6,
    OP_1NEGATE,
    OP_PUSHDATA1,
    OP_PUSHDATA2,
    OP_PUSHDATA4,
    OP_ADD,
    OP_SWAP,
    OP_2SWAP,
    OP_ROT,
    OP_2ROT,
    OP_NOP,
    OP_RETURN,
    OP_CODESEPARATOR,
    OP_EQUAL,
    OP_RESERVED,
    OP_RESERVED1,
    OP_RESERVED2,
    OP_VER,
    OP_VERIF,
    OP_VERNOTIF,
)
from tx_engine.engine.util import insert_num


class ScriptOPTests(unittest.TestCase):
    """ Regression Tests for standard OPs
    """
    def test_nop(self):
        """Check of nop
        """
        script = Script([OP_1, OP_2, OP_NOP, OP_3, OP_4])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [1, 2, 3, 4])

    def test_return(self):
        """ Check of return
        """
        script = Script([OP_1, OP_2, OP_RETURN, OP_3, OP_4])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [1, 2])

    def test_swap(self):
        """ Check of swap
        """
        script = Script([OP_2, OP_1, OP_SWAP])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [1, 2])

        script = Script([OP_1, OP_2, OP_3, OP_SWAP])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [1, 3, 2])

    def test_swap_bignum(self):
        """ Check of swap with a big number
        """
        # script = Script([encode_num(64), OP_1, OP_SWAP])
        script = Script(insert_num(64) + [OP_1, OP_SWAP])  # type: ignore[arg-type]
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [1, 64])

    def test_2swap(self):
        """ Check of 2swap
        """
        script = Script([OP_1, OP_2, OP_3, OP_4, OP_2SWAP])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [3, 4, 1, 2])

    def test_rot(self):
        """ Check of rot
        """
        script = Script([OP_1, OP_2, OP_3, OP_ROT])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [2, 3, 1])

    def test_2rot(self):
        """ Check of 2rot
        """
        script = Script([OP_1, OP_2, OP_3, OP_4, OP_5, OP_6, OP_2ROT])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [3, 4, 5, 6, 1, 2])

    def test_reserved(self):
        """ Check of reserved words
        """
        script = Script([OP_RESERVED])
        context = Context(script=script)
        self.assertFalse(context.evaluate(quiet=True))

        script = Script([OP_RESERVED1])
        context = Context(script=script)
        self.assertFalse(context.evaluate(quiet=True))

        script = Script([OP_RESERVED2])
        context = Context(script=script)
        self.assertFalse(context.evaluate(quiet=True))

        script = Script([OP_VER])
        context = Context(script=script)
        self.assertFalse(context.evaluate(quiet=True))

        script = Script([OP_VERIF])
        context = Context(script=script)
        self.assertFalse(context.evaluate(quiet=True))

        script = Script([OP_VERNOTIF])
        context = Context(script=script)
        self.assertFalse(context.evaluate(quiet=True))

    def test_1negate(self):
        """ Check of 1negate
        """
        # assert -1 + 1 == 0
        script = Script([OP_1NEGATE, OP_1, OP_ADD, OP_0, OP_EQUAL])
        context = Context(script=script)
        self.assertTrue(context.evaluate())

        # -1 -> 0x81
        script = Script([OP_1NEGATE])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.stack, [-1])

        # Test as text
        script = Script.parse_string("OP_1NEGATE")
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.stack, [-1])

    def test_1negate_large_numbers_part1(self):
        """  Check of 1negate with large numbers
        """
        # OP_1NEGATE,1000,OP_ADD,999,OP_EQUAL
        # script = Script.parse_string("OP_1NEGATE, 1000, OP_ADD, 999, OP_EQUAL")
        script = Script([OP_1NEGATE] + insert_num(1000) + [OP_ADD] + insert_num(999) + [OP_EQUAL])    # type: ignore[arg-type]
        context = Context(script=script)
        self.assertTrue(context.evaluate())

    def test_1negate_large_numbers_part2(self):
        """  Check of 1negate with large numbers
        """
        # script = Script([encode_num(-1), encode_num(1000), OP_ADD, encode_num(999), OP_EQUAL])
        script = Script(insert_num(-1) + insert_num(1000) + [OP_ADD] + insert_num(999) + [OP_EQUAL])  # type: ignore[arg-type]
        context = Context(script=script)
        self.assertTrue(context.evaluate())

    def test_pushdata1_1(self):
        """ Check of pushdata1
        """
        script = Script([OP_PUSHDATA1, b'\x02', b"\x01\x02"])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        # self.assertEqual(context.stack, [b"\x01\x02"])
        self.assertEqual(context.raw_stack, [[1, 2]])

    def test_pushdata1_3(self):
        """ Check of pushdata1
        """
        # Test as text
        script = Script.parse_string("OP_PUSHDATA1, 0x02, b'\x02\x01'")
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[2, 1]])

    def test_pushdata1_4(self):
        """ Check of pushdata1
        """
        script = Script.parse_string("OP_PUSHDATA1, 0x04, b'\x01\x02\x03\x04'")
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1, 2, 3, 4]])

    def test_pushdata2(self):
        """ Check of pushdata2
        """
        script = Script([OP_PUSHDATA2, 0x00, 0x01, b"\x01" * 256])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1] * 256])

    def test_pushdata4(self):
        """ Check of pushdata4
        """
        script = Script([OP_PUSHDATA4, 0x00, 0x00, 0x00, 0x01, b"\x01" * 0x01000000])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1] * 0x01000000])

    def test_codeseparator(self):
        """ Simple check of codeseparator
        """
        script = Script([OP_1, OP_2, OP_CODESEPARATOR, OP_3, OP_4])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [1, 2, 3, 4])


if __name__ == "__main__":
    unittest.main()
