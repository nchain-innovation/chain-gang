""" Test of the WoC interface height normalisation

Covers the part that needs no network: rewriting WhatsOnChain's unconfirmed
marker to this package's.
"""
import unittest
from typing import Any, Dict, List

from tx_engine.interface.blockchain_interface import UNCONFIRMED_HEIGHT
from tx_engine.interface.woc_interface import _normalise_unconfirmed


class WoCHeightTest(unittest.TestCase):
    """ Test of the WoC unconfirmed height normalisation """

    def test_height_zero_becomes_the_unconfirmed_sentinel(self):
        # Trimmed from a real /address/{addr}/unspent/all response. Transaction
        # fa7f15f8..d860 was in WhatsOnChain's own mempool/raw list, with no
        # blockhash, blockheight or confirmations.
        utxo: List[Dict[str, Any]] = [
            {"height": 964754, "tx_pos": 1, "tx_hash": "aa", "value": 1},
            {"height": 0, "tx_pos": 1, "tx_hash": "fa7f15f8", "value": 2954693},
        ]
        _normalise_unconfirmed(utxo)
        self.assertEqual(utxo[1]["height"], UNCONFIRMED_HEIGHT)
        self.assertLess(utxo[1]["height"], 0)
        # the confirmed entry and every other field are untouched
        self.assertEqual(utxo[0]["height"], 964754)
        self.assertEqual(utxo[1]["value"], 2954693)

    def test_a_mempool_entry_without_height_is_unconfirmed(self):
        # CS-462: WhatsOnChain omits height entirely on a mempool entry, and
        # sends an extra hex field. Payload copied from the report.
        utxo: List[Dict[str, Any]] = [
            {"tx_pos": 0, "tx_hash": "73c933af", "value": 44,
             "isSpentInMempoolTx": False, "hex": "76a914", "status": "unconfirmed"},
        ]
        _normalise_unconfirmed(utxo)
        self.assertEqual(utxo[0]["height"], UNCONFIRMED_HEIGHT)
        self.assertEqual(utxo[0]["value"], 44)

    def test_an_entry_with_neither_height_nor_status_still_gets_a_height(self):
        # Every entry must come out with a height key, or callers KeyError
        utxo: List[Dict[str, Any]] = [
            {"tx_pos": 0, "tx_hash": "aa", "value": 1},
        ]
        _normalise_unconfirmed(utxo)
        self.assertEqual(utxo[0]["height"], UNCONFIRMED_HEIGHT)

    def test_status_marks_an_unconfirmed_entry_even_at_a_real_height(self):
        # status is the documented signal on /unspent/all; height is the fallback
        utxo: List[Dict[str, Any]] = [
            {"height": 964754, "tx_pos": 0, "tx_hash": "aa", "value": 1,
             "isSpentInMempoolTx": False, "status": "unconfirmed"},
        ]
        _normalise_unconfirmed(utxo)
        self.assertEqual(utxo[0]["height"], UNCONFIRMED_HEIGHT)

    def test_a_confirmed_status_keeps_its_height(self):
        utxo: List[Dict[str, Any]] = [
            {"height": 964754, "tx_pos": 0, "tx_hash": "aa", "value": 1,
             "isSpentInMempoolTx": False, "status": "confirmed"},
        ]
        _normalise_unconfirmed(utxo)
        self.assertEqual(utxo[0]["height"], 964754)

    def test_is_idempotent(self):
        utxo: List[Dict[str, Any]] = [
            {"height": UNCONFIRMED_HEIGHT, "tx_pos": 0, "tx_hash": "aa", "value": 1}
        ]
        _normalise_unconfirmed(utxo)
        self.assertEqual(utxo[0]["height"], UNCONFIRMED_HEIGHT)

    def test_empty_utxo_set(self):
        self.assertEqual(_normalise_unconfirmed([]), [])

    def test_matches_the_rust_constant(self):
        # Both sides must agree, or the two clients disagree about the same chain
        self.assertEqual(UNCONFIRMED_HEIGHT, -1)


if __name__ == "__main__":
    unittest.main()
