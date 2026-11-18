import unittest
from tx_engine import Script, Tx, TxIn, TxOut
from tx_engine.interface.verify_script import activation_height, effective_chronicle_tx_version


class ChronicleContextTests(unittest.TestCase):
    def test_explicit_output_era_and_metadata(self):
        parent = Tx(1, [TxIn('11' * 32, 0, Script())],
                    [TxOut(100, Script.parse_string('OP_1 OP_2MUL OP_2 OP_EQUAL'))], 0)
        for version in (1, 2):
            spend = Tx(version, [TxIn(parent.id(), 0, Script())],
                       [TxOut(90, Script.parse_string('OP_1'))], 0)
            spend.validate_with_chronicle_context([parent], 200, 200,
                                                 [(parent.id(), 0, 200)], consensus_only=True)
            for heights in ([], [(parent.id(), 0, 100)], [(parent.id(), 0, 201)]):
                with self.assertRaises(ValueError):
                    spend.validate_with_chronicle_context([parent], 200, 200, heights,
                                                         consensus_only=True)

    def test_released_defaults_and_signed_version(self):
        for network, height in [('mainnet', 943816), ('testnet', 1713168),
                                ('stn', 250), ('regtest', 15000)]:
            self.assertEqual(activation_height(network), height)
        self.assertEqual(effective_chronicle_tx_version(0xffffffff, 0, 'regtest'), 0xffffffff)
        self.assertEqual(effective_chronicle_tx_version(2, 14999, 'regtest'), 1)


if __name__ == '__main__':
    unittest.main()
