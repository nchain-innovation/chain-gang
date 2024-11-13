""" Transaction tests
"""
import unittest
from tx_engine import Tx, Script, Context, sig_hash, sig_hash_preimage, SIGHASH, hash256d


class SigHashTest(unittest.TestCase):
    """ SigHash Tests
    """
    def test_sig_hash(self):
        own_tx_as_hex_str = "010000000117cbf49978ccc843405f8956007563b30e61d341fb6e6d9b11775a8e38d161d2000000006b483045022100aacb290ed3aeb43fc91a179d6a3ffef4c5efcca612c901c719e198c2ee685e2702200505bd74db673c6d723a141bd8ac469327ea4bb7987110042f23f8c3d7f91e3d412103dcf21dbdbaa744333af236c3382c85d6308e6d05599df5d3cb19e0f19a205d43ffffffff02e8030000000000001976a9144d7eb7ce5ce099dd218383f3f81d3c2f1e48113f88acf0810100000000001976a914d86625de492d8bd8bbc4930f2bef4328e37f1f5388ac00000000"
        own_tx = Tx.parse_hexstr(own_tx_as_hex_str)

        script_sig = Script.parse_string("0x3045022100aacb290ed3aeb43fc91a179d6a3ffef4c5efcca612c901c719e198c2ee685e2702200505bd74db673c6d723a141bd8ac469327ea4bb7987110042f23f8c3d7f91e3d41 0x03dcf21dbdbaa744333af236c3382c85d6308e6d05599df5d3cb19e0f19a205d43")
        script_pubkey = Script.parse_string("OP_DUP OP_HASH160 0xd86625de492d8bd8bbc4930f2bef4328e37f1f53 OP_EQUALVERIFY OP_CHECKSIG")
        combined_script = script_sig + script_pubkey

        z = sig_hash(own_tx, 0, script_pubkey, 99904, SIGHASH.ALL_FORKID)

        x = Context(script=combined_script, z=z)
        self.assertTrue(x.evaluate())

    def test_sig_hash_preimage(self):
        own_tx_as_hex_str = "010000000117cbf49978ccc843405f8956007563b30e61d341fb6e6d9b11775a8e38d161d2000000006b483045022100aacb290ed3aeb43fc91a179d6a3ffef4c5efcca612c901c719e198c2ee685e2702200505bd74db673c6d723a141bd8ac469327ea4bb7987110042f23f8c3d7f91e3d412103dcf21dbdbaa744333af236c3382c85d6308e6d05599df5d3cb19e0f19a205d43ffffffff02e8030000000000001976a9144d7eb7ce5ce099dd218383f3f81d3c2f1e48113f88acf0810100000000001976a914d86625de492d8bd8bbc4930f2bef4328e37f1f5388ac00000000"
        own_tx = Tx.parse_hexstr(own_tx_as_hex_str)

        script_pubkey = Script.parse_string("OP_DUP OP_HASH160 0xd86625de492d8bd8bbc4930f2bef4328e37f1f53 OP_EQUALVERIFY OP_CHECKSIG")

        sig_hash_value = sig_hash(own_tx, 0, script_pubkey, 99904, SIGHASH.ALL_FORKID)
        sig_hash_preimage_value = sig_hash_preimage(own_tx, 0, script_pubkey, 99904, SIGHASH.ALL_FORKID)
        digest = hash256d(sig_hash_preimage_value)
        self.assertEqual(sig_hash_value, digest)


if __name__ == "__main__":
    unittest.main()
