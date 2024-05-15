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
    OP_EQUAL,
    # BSV codes
    OP_AND,
    OP_OR,
    OP_XOR,
    OP_RSHIFT,
    OP_LSHIFT,
    OP_CAT,
    OP_SPLIT,
)


class BitTwiddlingTests(unittest.TestCase):

    def test_and(self):
        """ Check of bitwise AND
        """
        script = Script([b"\x00\x01", b"\x00\x03", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00\x01"])

        script = Script([b"\x01\xF0", b"\x00\x10", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00\x10"])

        script = Script([b"\x01\x00\x00\xFF", b"\x01\x00\x01\x10", OP_AND])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01\x00\x00\x10"])

    def test_or(self):
        """ Check of bitwise OR
        """
        script = Script([b"\x00\x01", b"\x00\x03", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00\x03"])

        script = Script([b"\x01\xF0", b"\x00\x10", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01\xF0"])

        script = Script([b"\x01\x00\x00\xFF", b"\x01\x00\x01\x10", OP_OR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01\x00\x01\xFF"])

    def test_xor(self):
        """ Check of bitwise XOR
        """
        script = Script([b"\x00\x01", b"\x00\x03", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00\x02"])

        script = Script([b"\x00\x00", b"\x01\x00", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01\x00"])

        script = Script([b"\x01\x00\x00", b"\x00\x00\x00", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x01\x00\x00"])

        script = Script([b"\x01", b"\x01", OP_XOR])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00"])

    def test_rshift(self):
        """ Check of right shift
        """
        script = Script([b"\x00\x11", OP_1, OP_RSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x00\x08"])

        script = Script([b"\x10\x11\x00\x10", OP_1, OP_RSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x08\x08\x80\x08"])

    def test_lshift(self):
        """ Check of left shift
        """
        script = Script([b'\x00\x01', OP_1, OP_LSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b'\x00\x02'])

        script = Script([b'\x00\x02', OP_2, OP_LSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b'\x00\x08'])

        script = Script([b'\x80\x00', OP_1, OP_LSHIFT])
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        # I left this in to show the change from the old behaviour to the new.
        self.assertNotEqual(context.stack, [b'\x01\x00\x00'])
        self.assertEqual(context.stack, [b'\x00\x00'])

    def test_cat(self):
        """ Check of OP_CAT
        """
        script = Script(
            [b"\x81\x02", b"\x83\x04", OP_CAT, b"\x81\x02\x83\x04", OP_EQUAL]
        )
        context = Context(script=script)
        self.assertTrue(context.evaluate())

    def test_split(self):
        """ Check of OP_SPLIT
        """
        script = Script(
            [b"\x81\x02\x83\x04", OP_2, OP_SPLIT]
        )
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x81\x02", b"\x83\x04"])

        script = Script(
            [b"\x81\x02\x83\x04", OP_1, OP_SPLIT]
        )
        context = Context(script=script)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(context.stack, [b"\x81", b"\x02\x83\x04"])


if __name__ == "__main__":
    logging.basicConfig(level="WARN")
    unittest.main()
