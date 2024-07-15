import unittest
import sys
sys.path.append("..")
import logging

from tx_engine import p2pkh_script, hash160, address_to_public_key_hash, public_key_to_address

"""
Quick test of the following functionality
* p2pkh_script
* h160
* address_to_public_key_hash
"""

log = logging.getLogger(__name__)
log.setLevel(logging.DEBUG)
stream_handler = logging.StreamHandler(sys.stdout)
log.addHandler(stream_handler)


class SignTest(unittest.TestCase):
    def test_p2pkh(self):
        public_key = bytes.fromhex("036a1a87d876e0fab2f7dc19116e5d0e967d7eab71950a7de9f2afd44f77a0f7a2")
        script = p2pkh_script(hash160(public_key))
        locking_script = "1976a91410375cfe32b917cd24ca1038f824cd00f739185988ac"
        self.assertEqual(script.serialize().hex(), locking_script)

    def test_address_to_public_key_hash(self):
        address = "mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5"
        calculated_public_key = address_to_public_key_hash(address).hex()
        public_key = bytes.fromhex("036a1a87d876e0fab2f7dc19116e5d0e967d7eab71950a7de9f2afd44f77a0f7a2")
        public_key_hash = hash160(public_key)

        self.assertEqual(calculated_public_key, public_key_hash.hex())

    def test_public_key_to_address(self):
        public_key = bytes.fromhex("036a1a87d876e0fab2f7dc19116e5d0e967d7eab71950a7de9f2afd44f77a0f7a2")
        address = "mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5"
        result = public_key_to_address(public_key, "BSV_Testnet")
        self.assertEqual(result, address)


if __name__ == "__main__":
    unittest.main()
