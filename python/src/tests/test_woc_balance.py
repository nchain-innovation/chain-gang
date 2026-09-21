""" Test of the WoC balance split across confirmed/unconfirmed endpoints

WhatsOnChain replaced the combined /address/{addr}/balance with separate
confirmed and unconfirmed endpoints. Covers the combining, which needs no
network.
"""
import unittest
from unittest.mock import patch

from tx_engine.interface import woc


class WoCBalanceTest(unittest.TestCase):
    """ Test of the WoC balance combining """

    def test_the_two_halves_combine(self):
        # Shapes as observed against mainnet
        responses = [
            {"address": "1A1z", "script": "8b01df4e", "confirmed": 7297358945,
             "error": "", "associatedScripts": [{"script": "740485f3", "type": "pubkey"}]},
            {"address": "1A1z", "script": "8b01df4e", "unconfirmed": 42, "error": ""},
        ]
        with patch.object(woc, "get_response", side_effect=responses):
            self.assertEqual(
                woc.get_balance("1A1z", testnet=False),
                {"confirmed": 7297358945, "unconfirmed": 42},
            )

    def test_a_body_error_is_not_mistaken_for_a_zero_balance(self):
        # Reported with HTTP 200, so the body is the only signal
        with patch.object(
            woc, "get_response",
            side_effect=[{"confirmed": 0, "error": "invalid address"}],
        ):
            self.assertIsNone(woc.get_balance("nonsense", testnet=False))

    def test_a_failed_request_gives_none(self):
        with patch.object(woc, "get_response", side_effect=[None]):
            self.assertIsNone(woc.get_balance("1A1z", testnet=False))

    def test_a_failed_second_request_gives_none(self):
        # The unconfirmed half failing must not report a confirmed-only balance
        with patch.object(
            woc, "get_response",
            side_effect=[{"confirmed": 100, "error": ""}, None],
        ):
            self.assertIsNone(woc.get_balance("1A1z", testnet=False))

    def test_a_negative_unconfirmed_balance_survives(self):
        # Spending a confirmed UTXO from the mempool makes this negative
        with patch.object(
            woc, "get_response",
            side_effect=[{"confirmed": 100, "error": ""},
                         {"unconfirmed": -5000, "error": ""}],
        ):
            self.assertEqual(
                woc.get_balance("1A1z", testnet=False),
                {"confirmed": 100, "unconfirmed": -5000},
            )


if __name__ == "__main__":
    unittest.main()
