""" Chronicle integration tests through Tx.validate and Context
"""
import unittest

from tx_engine import Context, Script, Tx, TxIn, TxOut


def display_tx_hash(tx: Tx) -> str:
    """Return the display-order txid hex string used by TxIn.prev_tx."""
    return bytes(reversed(bytes(tx.hash()))).hex()


class ChronicleTxValidateTest(unittest.TestCase):
    """ End-to-end Chronicle behavior via native Tx.validate
    """

    def _fund_and_spend(
        self,
        lock_script: Script,
        unlock_script: Script,
        spend_version: int,
    ) -> tuple[Tx, Tx]:
        fund = Tx(
            version=1,
            tx_ins=[],
            tx_outs=[TxOut(amount=1_000, script_pubkey=lock_script)],
        )
        spend = Tx(
            version=spend_version,
            tx_ins=[
                TxIn(
                    display_tx_hash(fund),
                    0,
                    unlock_script,
                )
            ],
            tx_outs=[TxOut(amount=900, script_pubkey=Script([]))],
        )
        return fund, spend

    def test_two_phase_functional_unlock_validates(self):
        fund, spend = self._fund_and_spend(
            Script.parse_string("OP_5 OP_EQUAL"),
            Script.parse_string("OP_2 OP_3 OP_ADD"),
            2,
        )
        self.assertIsNone(spend.validate([fund]))

    def test_version_one_rejects_functional_unlock(self):
        fund, spend = self._fund_and_spend(
            Script.parse_string("OP_5 OP_EQUAL"),
            Script.parse_string("OP_2 OP_3 OP_ADD"),
            1,
        )
        with self.assertRaises(ValueError):
            spend.validate([fund])

    def test_relaxed_clean_stack_validates_for_version_two(self):
        fund, spend = self._fund_and_spend(
            Script.parse_string("OP_1 OP_1"),
            Script.parse_string("OP_1"),
            2,
        )
        self.assertIsNone(spend.validate([fund]))

    def test_relaxed_clean_stack_rejects_version_one(self):
        fund, spend = self._fund_and_spend(
            Script.parse_string("OP_1 OP_1"),
            Script.parse_string("OP_1"),
            1,
        )
        with self.assertRaises(ValueError):
            spend.validate([fund])

    def test_context_two_phase_matches_tx_validate_path(self):
        unlock = Script.parse_string("OP_2 OP_3 OP_ADD")
        lock = Script.parse_string("OP_5 OP_EQUAL")
        self.assertTrue(
            Context(script=unlock, lock_script=lock, tx_version=2).evaluate()
        )


if __name__ == "__main__":
    unittest.main()
