#!/usr/bin/python3
import unittest
import sys
sys.path.append("..")

from tx_engine import Script, Context, encode_num
from tx_engine.engine.op_codes import (
    OP_1, OP_16, OP_1NEGATE,
    OP_DEPTH, OP_1SUB, OP_PICK, OP_EQUALVERIFY, OP_ROT, OP_TOALTSTACK,
    OP_ROLL, OP_TUCK, OP_OVER, OP_ADD, OP_MOD, OP_FROMALTSTACK, OP_SWAP,
    OP_EQUAL,
)


class ParseTest(unittest.TestCase):
    def test_comma_separated(self):
        s = "OP_PUSHDATA1 0x1A 'abcdefghijklmnopqrstuvwxyz',OP_SHA1, OP_PUSHDATA1, 0x14, 0x32d10c7b8cf96570ca04ce37f2a19d84240d3a89, OP_EQUAL"
        script = Script.parse_string(s)
        context = Context(script=script)
        self.assertTrue(context.evaluate())

    def test_space_separated_1(self):
        s_sig = "OP_PUSHDATA1 0x41 0x040b4c866585dd868a9d62348a9cd008d6a312937048fff31670e7e920cfc7a7447b5f0bba9e01e6fe4735c8383e6e7a3347a0fd72381b8f797a19f694054e5a69"
        s_pk = "OP_HASH160 OP_PUSHDATA1 0x14 0xff197b14e502ab41f3bc8ccb48c4abac9eab35bc OP_EQUAL"
        s1 = Script.parse_string(s_sig)
        s2 = Script.parse_string(s_pk)
        combined_sig = s1 + s2
        context = Context(script=combined_sig)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.stack, [1])

    def test_space_separated_2(self):
        s_sig = "0x040b4c866585dd868a9d62348a9cd008d6a312937048fff31670e7e920cfc7a7447b5f0bba9e01e6fe4735c8383e6e7a3347a0fd72381b8f797a19f694054e5a69"
        s_pk = "OP_HASH160 0xff197b14e502ab41f3bc8ccb48c4abac9eab35bc OP_EQUAL"
        s1 = Script.parse_string(s_sig)
        s2 = Script.parse_string(s_pk)
        combined_sig = s1 + s2
        context = Context(script=combined_sig)
        self.assertTrue(context.evaluate())
        self.assertEqual(context.stack, [1])

    def test_simple(self):
        s = "1 0x025624,OP_MUL,0x025624,OP_EQUAL"
        script = Script.parse_string(s)
        context = Context(script=script)
        self.assertTrue(context.evaluate())

    def test_simple_add(self):
        s1 = "OP_1"
        script1 = Script.parse_string(s1)
        s2 = "OP_2"
        script2 = Script.parse_string(s2)
        script3 = script1 + script2
        context = Context(script=script3)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.raw_stack, [[1], [2]])

    def test_numbers(self):
        script1 = Script.parse_string("1")
        self.assertEqual(script1.cmds, [OP_1])

        script1 = Script.parse_string("16")
        self.assertEqual(script1.cmds, [OP_16])

        script1 = Script.parse_string("-1")
        self.assertEqual(script1.cmds, [OP_1NEGATE])

        script1 = Script.parse_string("17")
        self.assertEqual(script1.cmds, [1, 17])

        script1 = Script.parse_string("1000")
        self.assertEqual(script1.cmds, [2, 232, 3])

    def test_federico1(self):
        input_script = Script.parse_string("19 1 0 0 1")
        self.assertEqual(input_script.cmds, [1, 19, OP_1, 0, 0, OP_1])

    def test_federico2(self):
        script1 = Script.parse_string('19 1 0 0 1 OP_DEPTH OP_1SUB OP_PICK 0x13 OP_EQUALVERIFY OP_ROT OP_ADD OP_TOALTSTACK OP_ADD OP_DEPTH OP_1SUB OP_ROLL OP_TUCK OP_MOD OP_OVER OP_ADD OP_OVER OP_MOD OP_FROMALTSTACK OP_ROT OP_TUCK OP_MOD OP_OVER OP_ADD OP_SWAP OP_MOD 1 OP_EQUALVERIFY 1 OP_EQUAL')
        self.assertEqual(script1.cmds, [1, 19, OP_1, 0, 0, OP_1, OP_DEPTH, OP_1SUB, OP_PICK, 1, 19, OP_EQUALVERIFY, OP_ROT, OP_ADD, OP_TOALTSTACK, OP_ADD, OP_DEPTH, OP_1SUB, OP_ROLL, OP_TUCK, OP_MOD, OP_OVER, OP_ADD, OP_OVER, OP_MOD, OP_FROMALTSTACK, OP_ROT, OP_TUCK, OP_MOD, OP_OVER, OP_ADD, OP_SWAP, OP_MOD, OP_1, OP_EQUALVERIFY, OP_1, OP_EQUAL])
        context = Context(script=script1)
        context.evaluate_core()
        # Should leave [1] on the stack
        self.assertEqual(context.raw_stack, [[1]])

    def test_federico3(self):
        script1 = Script.parse_string('OP_2 OP_1 OP_SUB')
        context = Context(script=script1)
        context.evaluate()
        self.assertEqual(context.stack, [1])

    def test_federico4(self):
        x = encode_num(53758635199196621832532654341949827999954483761840054390272371671254106983912)
        self.assertEqual(x, b'\xe8ME\xca\xabI\x1a7:$#+\x91\xe2\xab`%\xce`3Y\xc0\x064\xde\x0f\x8fU+O\xdav')


if __name__ == '__main__':
    unittest.main()
