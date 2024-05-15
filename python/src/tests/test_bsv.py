#!/usr/bin/python3
import sys

sys.path.append("..")

import unittest
import logging
from tx_engine.engine.script import Script
from tx_engine.engine.context import Context

from tx_engine.engine.op_codes import (
    OP_1,
    OP_2,
    OP_3,
    OP_4,
    OP_16,
    OP_EQUAL,
    OP_AND,
    OP_OR,
    OP_XOR,
    OP_2MUL,
    OP_2DIV,
    OP_MOD,
    OP_DIV,
    OP_MUL,
    OP_RSHIFT,
    OP_LSHIFT,
    OP_CAT,
    OP_BIN2NUM,
    OP_NUM2BIN,
    OP_INVERT,
    OP_1NEGATE,
)


class BSVTests(unittest.TestCase):
    """ Tests for BSV specific OPs
        These can be found https://github.com/shadders/uahf-spec/blob/reenable-op-codes/reenable-op-codes.md
    """

    def test_and(self):
        """ Simple check of bitwise AND
        """
        script = Script([b"\x00", b"\x00", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00"])

        script = Script([b"\x00", b"\x01", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00"])

        script = Script([b"\x01", b"\x00", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00"])

        script = Script([b"\x01", b"\x01", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01"])

    def test_or(self):
        """ Simple check of bitwise OR
        """
        script = Script([b"\x00", b"\x00", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00"])

        script = Script([b"\x00", b"\x01", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01"])

        script = Script([b"\x01", b"\x00", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01"])

        script = Script([b"\x01", b"\x01", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01"])

    def test_xor(self):
        """ Simple check of bitwise XOR
        """
        script = Script([b"\x00", b"\x00", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00"])

        script = Script([b"\x00", b"\x01", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01"])

        script = Script([b"\x01", b"\x00", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01"])

        script = Script([b"\x01", b"\x01", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00"])

    def test_2mul(self):
        """ Simple check of 2MUL
        """
        script = Script([OP_16, OP_2MUL])
        context = Context(script=script)
        # False as disabled in intepreter
        self.assertFalse(context.evaluate_core())

    def test_2div(self):
        """ Simple check of 2DIV
        """
        script = Script([OP_16, OP_2DIV])
        context = Context(script=script)
        # False as disabled in intepreter
        self.assertFalse(context.evaluate_core())

    def test_mod(self):
        """ Simple check of MOD
        """
        script = Script([OP_3, OP_2, OP_MOD])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.get_stack(), [1])

    def test_div(self):
        """ Simple check of DIV
        """
        script = Script([OP_4, OP_2, OP_DIV])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.get_stack(), [2])

    def test_mul(self):
        """ Simple check of MUL
        """
        script = Script([OP_4, OP_2, OP_MUL])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.get_stack(), [8])

    def test_rshift(self):
        """ Simple check of right shift
        """
        script = Script([b'\x00\x80', OP_1, OP_RSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b'\x00\x40'])

    def test_lshift(self):
        """ Simple check of left shift
        """
        script = Script([b'\x00\x40', OP_1, OP_LSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b'\x00\x80'])

    def test_cat(self):
        """ Simple check of cat
        """
        str1 = "one"
        str2 = "two"
        str3 = "onetwo"
        script = Script(
            [str.encode(str1), str.encode(str2), OP_CAT, str.encode(str3), OP_EQUAL]
        )
        context = Context(script=script)
        self.assertTrue(context.evaluate())

    def test_bin2num(self):
        """ Simple check of bin2num
            Definition found in https://github.com/shadders/uahf-spec/blob/reenable-op-codes/reenable-op-codes.md
        """
        script = Script([b"\x00\x00\x00\x00\x02", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x02])
        self.assertEqual(context.stack, [b"\x00\x00\x00\x00\x02"])

        script = Script([b"\x02\x00\x00\x00\x00", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x02])
        self.assertEqual(context.stack, [b"\x02"])

        script = Script([b"\x00", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [b"\x00"])
        self.assertEqual(context.stack, [b""])

        script = Script([b"\x00\x00\x00\x00\x00\x00", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0])
        self.assertEqual(context.stack, [b""])

        script = Script([b"\x00\x00\x00\x00\x00\x00\x01", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x01])
        self.assertTrue(context.stack, [b"\x00\x00\x00\x00\x00\x00\x01"])

        script = Script([b"\x01\x00\x00\x00\x00\x00\x00", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x01])
        self.assertTrue(context.stack, [b"\x01"])

        # 0x80 00 05 OP_BIN2NUM -> 0x85
        script = Script([b"\x80\x00\x05", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x85])
        self.assertEqual(context.stack, [b"\x80\x00\x05"])

        script = Script([b"\x05\x00\x80", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x85])
        self.assertEqual(context.stack, [b"\x85"])

        script = Script([b"\x80\x00\x00\x00\x00\x00\x01", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x81])
        self.assertEqual(context.stack, [b"\x80\x00\x00\x00\x00\x00\x01"])

        script = Script([b"\x80\x00\x00\x00\x00\x01\x01", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x8101])
        self.assertEqual(context.stack, [b"\x80\x00\x00\x00\x00\x01\x01"])

        script = Script([b"\x01\x00\x00\x00\x00\x00\x80", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x81])
        self.assertEqual(context.stack, [b"\x81"])

        script = Script([b"\x01\x00\x00\x00\x00\x01\x80", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x8101])
        self.assertEqual(context.stack, [b"\x01\x00\x00\x00\x00\x81"])

        # a OP_BIN2NUM -> failure, pre genesis as limited to 4 bytes
        script = Script([b"\x01\x00\x00\x01\x00\x00\x00\x00\x01\x01", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.stack, [0x1000001000000000101])
        self.assertEqual(context.stack, [b"\x01\x00\x00\x01\x00\x00\x00\x00\x01\x01"])

    def test_num2bin_1(self):
        """ Check of num2bin
        """
        script = Script([b"\x02", OP_4, OP_NUM2BIN])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00\x00\x00\x02"])

    def test_num2bin_2(self):
        """ Check of num2bin
        """
        script = Script([b"\x85", OP_4, OP_NUM2BIN])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x80\x00\x00\x05"])

    def test_num2bin_3(self):
        """ Check of num2bin
        """
        # 0x0100 1 OP_NUM2BIN -> failure
        script = Script([b"\x01\x01", OP_1, OP_NUM2BIN])
        context = Context(script=script)
        self.assertFalse(context.evaluate_core())

    def test_bin2num_round_trip_1(self):
        """ Convert a byte array to number and back to byte array to see if it removes the leading 0s
        """
        # Check the ablity to remove leading 0s
        script = Script([b"\x01\x00\x00\x00\x00\x00\x00", OP_BIN2NUM, OP_2, OP_NUM2BIN])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00\x01"])

    def test_bin2num_round_trip_2(self):
        """ Convert a byte array to number and back to byte array to see if it removes the leading 0s
        """
        # Check the ablity to remove leading 0s, repeat with one byte
        script = Script([b"\x01\x00\x00\x00\x00\x00\x00", OP_BIN2NUM, OP_1, OP_NUM2BIN])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01"])

    def test_BitWiseInvert(self):
        """ Test bitwise invert on a byte
        """
        script = Script([b"\x00", OP_INVERT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        stack = [x.hex() for x in context.stack]
        self.assertEqual(stack, ["ff"])

        script = Script([b"\xFF", OP_INVERT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        stack = [x.hex() for x in context.stack]
        self.assertEqual(stack, ["00"])

    def test_BitWiseInvert2(self):
        """ Test bitwise invert on a bytearray
        """
        # script = Script.parse_string("0xddffa0cea09612, OP_INVERT")
        script = Script([b"\xDD\xFF\xA0\xCE\xA0\x96\x12", OP_INVERT])
        context = Context(script=script)

        self.assertTrue(context.evaluate_core())
        stack = [x.hex() for x in context.stack]
        self.assertEqual(stack, ["22005f315f69ed"])

    def test_1negate(self):
        """ Test OP_1NEGATE
        """
        script = Script([OP_1NEGATE])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x81"])


if __name__ == "__main__":
    # logging.basicConfig(level="WARN")
    logging.basicConfig(level="INFO")
    unittest.main()
