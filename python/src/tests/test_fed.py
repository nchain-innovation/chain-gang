#!/usr/bin/python3
import unittest
import sys
sys.path.append("..")

from tx_engine import Script, Context, encode_num
from tx_engine.engine.op_codes import (
    OP_1,
    OP_DEPTH, OP_1SUB, OP_PICK, OP_EQUALVERIFY, OP_ROT, OP_TOALTSTACK,
    OP_ROLL, OP_TUCK, OP_OVER, OP_ADD, OP_MOD, OP_FROMALTSTACK, OP_SWAP,
    OP_MUL, OP_2,
    OP_EQUAL,
)


class FedTest(unittest.TestCase):

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

    def test_federico5(self):
        script1 = Script.parse_string('OP_2 OP_1 OP_MUL')
        self.assertEqual(script1.cmds, [OP_2, OP_1, OP_MUL])
        context = Context(script=script1)
        context.evaluate()
        self.assertEqual(context.stack, [2])

        script1 = Script.parse_string('OP_MUL')
        self.assertEqual(script1.cmds, [OP_MUL])


if __name__ == '__main__':
    unittest.main()
