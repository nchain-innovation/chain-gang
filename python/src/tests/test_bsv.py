#!/usr/bin/python3
import sys

sys.path.append("..")

import unittest
import logging
from tx_engine.engine.script import Script
from tx_engine.engine.context import Context

from tx_engine.engine.op_codes import (
    OP_0,
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
    OP_PUSHDATA1,
)


class BSVTests(unittest.TestCase):
    """ Tests for BSV specific OPs
        These can be found https://github.com/shadders/uahf-spec/blob/reenable-op-codes/reenable-op-codes.md
    """

    def test_and_part1(self):
        """ Simple check of bitwise AND
        """
        script = Script([OP_0, OP_0, OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[]])

    def test_and_part2(self):
        """ Simple check of bitwise AND
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x00", OP_PUSHDATA1, 0x01, b"\x01", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0]])

    def test_and_part3(self):
        """ Simple check of bitwise AND
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x01", OP_PUSHDATA1, 0x01, b"\x00", OP_AND])

        # script = Script([b"\x01", b"\x00", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0]])

    def test_and_part4(self):
        """ Simple check of bitwise AND
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x01", OP_PUSHDATA1, 0x01, b"\x01", OP_AND])
        # script = Script([b"\x01", b"\x01", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x01]])

    def test_or_part1(self):
        """ Simple check of bitwise OR
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x00", OP_PUSHDATA1, 0x01, b"\x00", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0]])

    def test_or_part2(self):
        """ Simple check of bitwise OR
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x00", OP_PUSHDATA1, 0x01, b"\x01", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x01]])

    def test_or_part3(self):
        """ Simple check of bitwise OR
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x01", OP_PUSHDATA1, 0x01, b"\x00", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x01]])

    def test_or_part4(self):
        """ Simple check of bitwise OR
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x01", OP_PUSHDATA1, 0x01, b"\x01", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x01]])

    def test_xor_part1(self):
        """ Simple check of bitwise XOR
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x00", OP_PUSHDATA1, 0x01, b"\x00", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x00]])

    def test_xor_part2(self):
        """ Simple check of bitwise XOR
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x00", OP_PUSHDATA1, 0x01, b"\x01", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x01]])

    def test_xor_part3(self):
        """ Simple check of bitwise XOR
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x01", OP_PUSHDATA1, 0x01, b"\x00", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x01]])

    def test_xor_part4(self):
        """ Simple check of bitwise XOR
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x01", OP_PUSHDATA1, 0x01, b"\x01", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x00]])

    def test_2mul(self):
        """ Simple check of 2MUL
        """
        script = Script([OP_16, OP_2MUL])
        context = Context(script=script)
        # False as disabled in intepreter
        self.assertFalse(context.evaluate_core(quiet=True))

    def test_2div(self):
        """ Simple check of 2DIV
        """
        script = Script([OP_16, OP_2DIV])
        context = Context(script=script)
        # False as disabled in intepreter
        self.assertFalse(context.evaluate_core(quiet=True))

    def test_mod(self):
        """ Simple check of MOD
        """
        script = Script([OP_3, OP_2, OP_MOD])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [1])

    def test_div(self):
        """ Simple check of DIV
        """
        script = Script([OP_4, OP_2, OP_DIV])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [2])

    def test_mul(self):
        """ Simple check of MUL
        """
        script = Script([OP_4, OP_2, OP_MUL])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.get_stack(), [8])

    def test_rshift(self):
        """ Simple check of right shift
        """
        script = Script([OP_PUSHDATA1, b'\x02', b'\x00\x80', OP_1, OP_RSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 0x40]])

    def test_lshift(self):
        """ Simple check of left shift
        """
        script = Script([OP_PUSHDATA1, b'\x02', b'\x00\x40', OP_1, OP_LSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 0x80]])

    def test_cat(self):
        """ Simple check of cat
        """
        str1 = str.encode("one")
        str2 = str.encode("two")
        str3 = str.encode("onetwo")
        script = Script(
            [OP_PUSHDATA1, b'\x03', str1, OP_PUSHDATA1, b'\x03', str2, OP_CAT, OP_PUSHDATA1, b'\x06', str3, OP_EQUAL]
        )
        context = Context(script=script)
        self.assertTrue(context.evaluate())

    def test_bin2num_part1(self):
        """ Simple check of bin2num
            Definition found in https://github.com/shadders/uahf-spec/blob/reenable-op-codes/reenable-op-codes.md
        """
        script = Script([OP_PUSHDATA1, b'\x05', b"\x00\x00\x00\x00\x02", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertNotEqual(context.raw_stack, [[0x02]])
        self.assertEqual(context.raw_stack, [[0, 0, 0, 0, 0x02]])

    def test_bin2num_part2(self):
        script = Script([OP_PUSHDATA1, b'\x05', b"\x02\x00\x00\x00\x00", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x02]])
        # self.assertEqual(context.stack, [b"\x02"])

    def test_bin2num_part3(self):
        script = Script([OP_PUSHDATA1, b'\x01', b"\x00", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[]])

    def test_bin2num_part4(self):
        script = Script([OP_PUSHDATA1, 0x06, b"\x00\x00\x00\x00\x00\x00", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[]])

    def test_bin2num_part5(self):
        script = Script([OP_PUSHDATA1, 0x07, b"\x00\x00\x00\x00\x00\x00\x01", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 0, 0, 0, 0, 0, 0x01]])

    def test_bin2num_part6(self):
        script = Script([OP_PUSHDATA1, 0x07, b"\x01\x00\x00\x00\x00\x00\x00", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1]])

    def test_bin2num_part7(self):
        # 0x80 00 05 OP_BIN2NUM -> 0x85
        script = Script([OP_PUSHDATA1, 0x03, b"\x80\x00\x05", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.stack, [327808])

    def test_bin2num_part8(self):
        script = Script([OP_PUSHDATA1, 0x03, b"\x05\x00\x80", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x85]])

    def test_bin2num_part9(self):
        script = Script([OP_PUSHDATA1, 0x07, b"\x80\x00\x00\x00\x00\x00\x01", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]])

    def test_bin2num_part10(self):
        script = Script([OP_PUSHDATA1, 0x07, b"\x80\x00\x00\x00\x00\x01\x01", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x80, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01]])

    def test_bin2num_part11(self):
        script = Script([OP_PUSHDATA1, 0x07, b"\x01\x00\x00\x00\x00\x00\x80", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x81]])

    def test_bin2num_part12(self):
        script = Script([OP_PUSHDATA1, 0x07, b"\x01\x00\x00\x00\x00\x01\x80", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x01, 0x00, 0x00, 0x00, 0x00, 0x81]])

    def test_bin2num_part13(self):
        # a OP_BIN2NUM -> failure, pre genesis as limited to 4 bytes
        script = Script([OP_PUSHDATA1, 0x0A, b"\x01\x00\x00\x01\x00\x00\x00\x00\x01\x01", OP_BIN2NUM])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01]])

    def test_num2bin_1(self):
        """ Check of num2bin
        """
        script = Script([OP_2, OP_4, OP_NUM2BIN])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x00, 0x00, 0x00, 0x02]])

    def test_num2bin_2(self):
        """ Check of num2bin
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x85", OP_4, OP_NUM2BIN])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x80, 0x00, 0x00, 0x05]])

    def test_num2bin_3(self):
        """ Check of num2bin
        """
        # 0x0100 1 OP_NUM2BIN -> failure
        script = Script([OP_PUSHDATA1, 0x02, b"\x01\x01", OP_1, OP_NUM2BIN])
        context = Context(script=script)
        # Causes a script execution failure as n longer than m
        self.assertFalse(context.evaluate_core(quiet=True))

    def test_bin2num_round_trip_1(self):
        """ Convert a byte array to number and back to byte array to see if it removes the leading 0s
        """
        # Check the ablity to remove leading 0s
        script = Script([OP_PUSHDATA1, 0x07, b"\x01\x00\x00\x00\x00\x00\x00", OP_BIN2NUM, OP_2, OP_NUM2BIN])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0, 1]])

    def test_bin2num_round_trip_2(self):
        """ Convert a byte array to number and back to byte array to see if it removes the leading 0s
        """
        # Check the ablity to remove leading 0s, repeat with one byte
        script = Script([OP_PUSHDATA1, 0x07, b"\x01\x00\x00\x00\x00\x00\x00", OP_BIN2NUM, OP_1, OP_NUM2BIN])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1]])

    def test_bitwise_invert_part1(self):
        """ Test bitwise invert on a byte
        """
        script = Script([OP_PUSHDATA1, 0x01, b"\x00", OP_INVERT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        # stack = [x.hex() for x in context.stack]
        self.assertEqual(context.raw_stack, [[0xff]])

    def test_bitwise_invert_part2(self):
        script = Script([OP_PUSHDATA1, 0x01, b"\xFF", OP_INVERT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        # stack = [x.hex() for x in context.stack]
        self.assertEqual(context.raw_stack, [[0x00]])

    def test_bitwise_invert2(self):
        """ Test bitwise invert on a bytearray
        """
        # script = Script.parse_string("0xddffa0cea09612, OP_INVERT")
        script = Script([OP_PUSHDATA1, 0x07, b"\xDD\xFF\xA0\xCE\xA0\x96\x12", OP_INVERT])
        context = Context(script=script)

        self.assertTrue(context.evaluate_core())
        # stack = [x.hex() for x in context.stack]
        self.assertEqual(context.raw_stack, [[0x22, 0x00, 0x5f, 0x31, 0x5f, 0x69, 0xed]])

    def test_1negate(self):
        """ Test OP_1NEGATE
        """
        script = Script([OP_1NEGATE])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[0x81]])


if __name__ == "__main__":
    # logging.basicConfig(level="WARN")
    logging.basicConfig(level="INFO")
    unittest.main()
