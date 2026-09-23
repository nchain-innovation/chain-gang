""" Test of the WoC unspent read leaving out outputs spent in the mempool

/unspent/all goes on listing an output after a mempool transaction spends it,
flagged isSpentInMempoolTx. Such an output is not unspent (CS-465). Needs no
network.
"""
import unittest
from unittest.mock import patch

from tx_engine.interface import woc


def page(*entries):
    return {"result": list(entries), "error": "", "nextPageToken": ""}


class WoCUnspentTest(unittest.TestCase):
    """ Test of the mempool-spent filter """

    def test_an_output_already_spent_in_the_mempool_is_left_out(self):
        response = page(
            {"height": 964754, "tx_pos": 0, "tx_hash": "spent", "value": 2000,
             "isSpentInMempoolTx": True, "status": "confirmed"},
            {"height": 964754, "tx_pos": 1, "tx_hash": "kept", "value": 9904,
             "isSpentInMempoolTx": False, "status": "confirmed"},
            {"tx_pos": 0, "tx_hash": "change", "value": 1904,
             "isSpentInMempoolTx": False, "status": "unconfirmed"},
        )
        with patch.object(woc, "get_response", side_effect=[response]):
            hashes = [e["tx_hash"] for e in woc.get_unspent_transactions("1A1z", testnet=False)]
        self.assertEqual(hashes, ["kept", "change"])

    def test_a_chain_of_unconfirmed_spends_leaves_only_its_tip(self):
        response = page(
            {"tx_pos": 0, "tx_hash": "first", "value": 1968,
             "isSpentInMempoolTx": True, "status": "unconfirmed"},
            {"tx_pos": 0, "tx_hash": "second", "value": 1936,
             "isSpentInMempoolTx": True, "status": "unconfirmed"},
            {"tx_pos": 0, "tx_hash": "tip", "value": 1904,
             "isSpentInMempoolTx": False, "status": "unconfirmed"},
        )
        with patch.object(woc, "get_response", side_effect=[response]):
            entries = woc.get_unspent_transactions("1A1z", testnet=False)
        self.assertEqual([e["tx_hash"] for e in entries], ["tip"])

    def test_an_absent_flag_means_not_spent(self):
        response = page(
            {"height": 964754, "tx_pos": 0, "tx_hash": "aa", "value": 1, "status": "confirmed"},
        )
        with patch.object(woc, "get_response", side_effect=[response]):
            self.assertEqual(len(woc.get_unspent_transactions("1A1z", testnet=False)), 1)


if __name__ == "__main__":
    unittest.main()
