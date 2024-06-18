import unittest
import sys
sys.path.append("..")

from tx_engine import Script, Context


class ScriptTest(unittest.TestCase):

    def test_joined_scripts(self):

        s_sig = "0x47 0x3044022018f6d074f8179c49de073709c598c579a917d99b5ca9e1cff0a8655f8a815557022036a758595c64b90c1c8042739b1980b44325c3fbba8510d63a3141f11b3cee3301 0x41 0x040b4c866585dd868a9d62348a9cd008d6a312937048fff31670e7e920cfc7a7447b5f0bba9e01e6fe4735c8383e6e7a3347a0fd72381b8f797a19f694054e5a69"
        s_pk = "OP_DUP OP_HASH160 0x14 0xff197b14e502ab41f3bc8ccb48c4abac9eab35bc OP_EQUALVERIFY"
        s1 = Script.parse_string(s_sig)
        s2 = Script.parse_string(s_pk)
        combined_sig = s1 + s2
        context = Context(script=combined_sig)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(len(context.raw_stack), 2)

        assert isinstance(context.raw_stack[0], list)
        self.assertEqual(len(context.raw_stack[0]), 0x47)
        assert isinstance(context.raw_stack[1], list)
        self.assertEqual(len(context.raw_stack[1]), 0x41)

        serial = combined_sig.serialize()
        # Parse the serialised data
        s3 = Script.parse(serial)

        context = Context(script=s3)
        self.assertTrue(context.evaluate_core())
        self.assertEqual(len(context.raw_stack), 2)
        assert isinstance(context.raw_stack[0], list)
        self.assertEqual(len(context.raw_stack[0]), 0x47)
        assert isinstance(context.raw_stack[1], list)
        self.assertEqual(len(context.raw_stack[1]), 0x41)


if __name__ == "__main__":
    unittest.main()
