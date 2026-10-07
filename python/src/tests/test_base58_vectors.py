""" bitcoin-sv's base58 address and private key vectors, through the Python API

The files are the node's own, vendored in tests/data at the repository root
(see tests/data/README.md there); tests/base58_vectors.rs runs the same files
through the Rust API. They are only in a git checkout, so set
CHAIN_GANG_VECTORS_OPTIONAL=1 to skip rather than fail where they are absent.
"""
import json
import os
import unittest

from tx_engine import address_to_public_key_hash, bytes_to_wif, wif_to_bytes, Wallet


DATA = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "tests", "data")


def load(name):
    path = os.path.join(DATA, name)
    if not os.path.exists(path):
        if os.environ.get("CHAIN_GANG_VECTORS_OPTIONAL") == "1":
            raise unittest.SkipTest(f"{path} is absent")
        raise AssertionError(f"{path} is missing; set CHAIN_GANG_VECTORS_OPTIONAL=1 to skip vendored vectors")
    with open(path, encoding="utf-8") as f:
        return json.load(f)


class Base58KeysValidTest(unittest.TestCase):
    """ base58_keys_valid.json: every address and private key reads correctly """

    def setUp(self):
        self.rows = load("base58_keys_valid.json")
        self.assertEqual(len(self.rows), 50)

    def test_addresses_decode_to_their_hash(self):
        for encoded, payload, meta in self.rows:
            if meta["isPrivkey"]:
                continue
            with self.subTest(address=encoded):
                self.assertEqual(address_to_public_key_hash(encoded), bytes.fromhex(payload))

    def test_private_keys_decode_to_their_key_and_network(self):
        for encoded, payload, meta in self.rows:
            if not meta["isPrivkey"]:
                continue
            network = "BSV_Testnet" if meta["isTestnet"] else "BSV_Mainnet"
            with self.subTest(wif=encoded):
                self.assertEqual(wif_to_bytes(encoded), bytes.fromhex(payload))
                self.assertEqual(Wallet(encoded).get_network(), network)

    def test_compressed_private_keys_encode_back(self):
        # bytes_to_wif and Wallet.to_wif always produce the compressed form
        for encoded, payload, meta in self.rows:
            if not (meta["isPrivkey"] and meta["isCompressed"]):
                continue
            network = "BSV_Testnet" if meta["isTestnet"] else "BSV_Mainnet"
            with self.subTest(wif=encoded):
                self.assertEqual(bytes_to_wif(bytes.fromhex(payload), network), encoded)
                self.assertEqual(Wallet(encoded).to_wif(), encoded)


class Base58KeysInvalidTest(unittest.TestCase):
    """ base58_keys_invalid.json: none of these is an address or a private key """

    def setUp(self):
        self.rows = load("base58_keys_invalid.json")
        self.assertEqual(len(self.rows), 50)

    def test_none_is_a_private_key(self):
        for (encoded, *_) in self.rows:
            with self.subTest(string=encoded):
                with self.assertRaises(Exception):
                    Wallet(encoded)

    @unittest.expectedFailure
    def test_none_is_an_address(self):
        # address_to_public_key_hash checks only the base58 checksum. It does
        # not check the version byte or that 20 bytes follow it, so 40 of
        # these 50 come back as a "hash", of 32, 33 or 50 bytes among others.
        # Remove expectedFailure once it validates addresses.
        accepted = []
        for (encoded, *_) in self.rows:
            try:
                address_to_public_key_hash(encoded)
                accepted.append(encoded)
            except Exception:
                pass
        self.assertEqual(accepted, [])


if __name__ == "__main__":
    unittest.main()
