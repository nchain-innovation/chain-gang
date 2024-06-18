#!/usr/bin/python3
import sys
sys.path.append("..")

import unittest
from tx_engine import Context, Script


from tx_engine.engine.op_codes import (
    OP_1,
    OP_2,
    OP_3,
    OP_EQUAL,
    # BSV codes
    OP_AND,
    OP_OR,
    OP_XOR,
    OP_RSHIFT,
    OP_LSHIFT,
    OP_CAT,
    OP_SPLIT,
    OP_PUSHDATA1,
)


class BitTwiddlingTests(unittest.TestCase):

    def test_and_part1(self):
        """ Check of bitwise AND
        """
        script = Script([OP_1, OP_3, OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1]])

    def test_and_part2(self):
        """ Check of bitwise AND
        """
        script = Script([OP_PUSHDATA1, 0x02, b"\x00\x01", OP_PUSHDATA1, 0x02, b"\x00\x03", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 1]])

    def test_and_part3(self):
        """ Check of bitwise AND
        """
        script = Script([OP_PUSHDATA1, 0x02, b"\x01\xF0", OP_PUSHDATA1, 0x02, b"\x00\x10", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 0x10]])

    def test_and_part4(self):
        """ Check of bitwise AND
        """
        script = Script([OP_PUSHDATA1, 0x04, b"\x01\x00\x00\xFF", OP_PUSHDATA1, 0x04, b"\x01\x00\x01\x10", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1, 0, 0, 0x10]])

    def test_or_part1(self):
        """ Check of bitwise OR
        """
        script = Script([OP_PUSHDATA1, 0x02, b"\x00\x01", OP_PUSHDATA1, 0x02, b"\x00\x03", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 3]])

    def test_or_part2(self):
        """ Check of bitwise OR
        """
        script = Script([OP_PUSHDATA1, 0x02, b"\x01\xF0", OP_PUSHDATA1, 0x02, b"\x00\x10", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1, 0xF0]])

    def test_or_part3(self):
        """ Check of bitwise OR
        """
        script = Script([OP_PUSHDATA1, 0x04, b"\x01\x00\x00\xFF", OP_PUSHDATA1, 0x04, b"\x01\x00\x01\x10", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1, 0, 1, 0xFF]])

    def test_xor_part1(self):
        """ Check of bitwise XOR
        """
        script = Script([OP_1, OP_3, OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.stack, [2])

    def test_xor_part2(self):
        """ Check of bitwise XOR
        """
        script = Script([OP_PUSHDATA1, 0x02, b"\x00\x01", OP_PUSHDATA1, 0x02, b"\x00\x03", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 2]])

    def test_xor_part3(self):
        """ Check of bitwise XOR
        """

        script = Script([OP_PUSHDATA1, 0x02, b"\x00\x00", OP_PUSHDATA1, 0x02, b"\x01\x00", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1, 0]])

    def test_xor_part4(self):
        """ Check of bitwise XOR
        """
        script = Script([OP_PUSHDATA1, 0x03, b"\x01\x00\x00", OP_PUSHDATA1, 0x03, b"\x00\x00\x00", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1, 0, 0]])

    def test_xor_part5(self):
        """ Check of bitwise XOR
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x01", OP_PUSHDATA1, 0x01, b"\x01", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0]])

    def test_rshift_part1(self):
        """ Check of right shift
        """
        script = Script([OP_PUSHDATA1, 0x02, b"\x00\x11", OP_1, OP_RSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 0x08]])

    def test_rshift_part2(self):
        """ Check of right shift
        """
        script = Script([OP_PUSHDATA1, 0x04, b"\x10\x11\x00\x10", OP_1, OP_RSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x08, 0x08, 0x80, 0x08]])

    def test_lshift_part1(self):
        """ Check of left shift
        """
        script = Script([OP_PUSHDATA1, 0x02, b'\x00\x01', OP_1, OP_LSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 0x02]])

    def test_lshift_part2(self):
        """ Check of left shift
        """
        script = Script([OP_PUSHDATA1, 0x02, b'\x00\x02', OP_2, OP_LSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 0x08]])

    def test_lshift_part3(self):
        """ Check of left shift
        """
        script = Script([OP_PUSHDATA1, 0x02, b'\x80\x00', OP_1, OP_LSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        # I left this in to show the change from the old behaviour to the new.
        self.assertNotEqual(context.raw_stack, [[1, 0, 0]])
        self.assertEqual(context.raw_stack, [[0, 0]])

    def test_cat(self):
        """ Check of OP_CAT
        """
        script = Script(
            [OP_PUSHDATA1, 0x02, b"\x81\x02", OP_PUSHDATA1, 0x02, b"\x83\x04", OP_CAT, OP_PUSHDATA1, 0x04, b"\x81\x02\x83\x04", OP_EQUAL]
        )
        context = Context(script=script)
        self.assertTrue(context.evaluate())

    def test_split_part1(self):
        """ Check of OP_SPLIT
        """
        script = Script(
            [OP_PUSHDATA1, 0x04, b"\x81\x02\x83\x04", OP_2, OP_SPLIT]
        )
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x81, 0x02], [0x83, 0x04]])

    def test_split_part2(self):
        """ Check of OP_SPLIT
        """
        script = Script(
            [OP_PUSHDATA1, 0x04, b"\x81\x02\x83\x04", OP_1, OP_SPLIT]
        )
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x81], [0x02, 0x83, 0x04]])


if __name__ == "__main__":
    unittest.main()
