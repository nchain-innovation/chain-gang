import unittest
import sys
sys.path.append("..")

from tx_engine import Script, Context, p2pkh_script, hash160
from tx_engine.engine.op_codes import OP_PUSHDATA4, OP_DUP, OP_HASH160


class ScriptTest(unittest.TestCase):

    def test_joined_scripts(self):
        s_sig = "0x3044022018f6d074f8179c49de073709c598c579a917d99b5ca9e1cff0a8655f8a815557022036a758595c64b90c1c8042739b1980b44325c3fbba8510d63a3141f11b3cee3301 0x040b4c866585dd868a9d62348a9cd008d6a312937048fff31670e7e920cfc7a7447b5f0bba9e01e6fe4735c8383e6e7a3347a0fd72381b8f797a19f694054e5a69"
        s_pk = "OP_DUP OP_HASH160 0xff197b14e502ab41f3bc8ccb48c4abac9eab35bc OP_EQUALVERIFY"
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

    def test_new_script(self):
        s1 = Script([])
        self.assertTrue(isinstance(s1, Script))
        s2 = Script()
        self.assertTrue(isinstance(s2, Script))
        self.assertEqual(s1, s2)

    def test_script_to_string(self):
        # With round trip back to script
        public_key = bytes.fromhex("036a1a87d876e0fab2f7dc19116e5d0e967d7eab71950a7de9f2afd44f77a0f7a2")
        script1 = p2pkh_script(hash160(public_key))
        as_str = script1.to_string()
        self.assertEqual(as_str, "[OP_DUP OP_HASH160 0x14 0x10375cfe32b917cd24ca1038f824cd00f7391859 OP_EQUALVERIFY OP_CHECKSIG]")

    def test_script_to_string_op_pushdata1(self):
        as_str1 = "OP_PUSHDATA1 0x02 0x01f0 OP_PUSHDATA1 0x02 0x0010 OP_AND"
        script1 = Script.parse_string(as_str1)
        as_str2 = script1.to_string()
        as_str3 = "[" + as_str1 + "]"
        self.assertEqual(as_str2, as_str3)

    def test_script_to_string_op_pushdata2(self):
        as_str1 = "OP_PUSHDATA2 0x00 0x01 0x01010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101010101"
        script1 = Script.parse_string(as_str1)
        as_str2 = script1.to_string()
        as_str3 = "[" + as_str1 + "]"
        self.assertEqual(as_str2, as_str3)

    def test_script_to_string_op_pushdata4(self):
        script1 = Script([OP_PUSHDATA4, 0x00, 0x00, 0x00, 0x01, b"\x01" * 0x01000000])
        as_str1 = script1.to_string()
        script2 = Script.parse_string(as_str1[1:-1])
        as_str2 = script2.to_string()
        self.assertEqual(as_str2, as_str1)

    def test_append_byte(self):
        script1 = Script()
        script1.append_byte(OP_DUP)
        as_str1 = script1.to_string()
        self.assertEqual(as_str1, "[OP_DUP]")

    def test_append_data(self):
        script1 = Script()
        script1.append_data(bytes.fromhex("01f0"))
        as_str1 = script1.to_string()
        self.assertEqual(as_str1, "[0x01 0xf0]")

    def test_append_pushdata(self):
        script1 = Script()
        script1.append_pushdata(bytes.fromhex("01f0"))
        as_str1 = script1.to_string()
        self.assertEqual(as_str1, "[0x02 0x01f0]")

    def test_index_operator(self):
        public_key = bytes.fromhex("036a1a87d876e0fab2f7dc19116e5d0e967d7eab71950a7de9f2afd44f77a0f7a2")
        script1 = p2pkh_script(hash160(public_key))
        # [OP_DUP OP_HASH160 0x14 0x10375cfe32b917cd24ca1038f824cd00f7391859 OP_EQUALVERIFY OP_CHECKSIG]
        self.assertEqual(script1[0], OP_DUP)
        self.assertEqual(script1[1], OP_HASH160)

    def test_is_p2pkh(self):
        public_key = bytes.fromhex("036a1a87d876e0fab2f7dc19116e5d0e967d7eab71950a7de9f2afd44f77a0f7a2")
        script1 = p2pkh_script(hash160(public_key))
        # [OP_DUP OP_HASH160 0x14 0x10375cfe32b917cd24ca1038f824cd00f7391859 OP_EQUALVERIFY OP_CHECKSIG]
        self.assertTrue(script1.is_p2pkh())

        as_str1 = "OP_PUSHDATA1 0x02 0x01f0 OP_PUSHDATA1 0x02 0x0010 OP_AND"
        script2 = Script.parse_string(as_str1)
        self.assertFalse(script2.is_p2pkh())


if __name__ == "__main__":
    unittest.main()
