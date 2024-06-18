#!/usr/bin/python3
import unittest
import sys
sys.path.append("..")

from tx_engine import Script, Context


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
        s_sig = "0x41 0x040b4c866585dd868a9d62348a9cd008d6a312937048fff31670e7e920cfc7a7447b5f0bba9e01e6fe4735c8383e6e7a3347a0fd72381b8f797a19f694054e5a69"
        s_pk = "OP_HASH160 0x14 0xff197b14e502ab41f3bc8ccb48c4abac9eab35bc OP_EQUAL"
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


if __name__ == '__main__':
    unittest.main()
