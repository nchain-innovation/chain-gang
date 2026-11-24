import unittest
from tx_engine import Script, Tx, TxIn, TxOut
from tx_engine.interface.verify_script import activation_height, effective_chronicle_tx_version


class ChronicleContextTests(unittest.TestCase):
    def test_explicit_upgradeable_nop_policy(self):
        parent = Tx(1, [TxIn('22' * 32, 0, Script())],
                    [TxOut(100, Script.parse_string('OP_SUBSTR OP_1'))], 0)
        for version in (1, 2):
            spend = Tx(version, [TxIn(parent.id(), 0, Script())],
                       [TxOut(90, Script.parse_string('OP_1'))], 0)
            context = ([parent], 200, 200, [(parent.id(), 0, 199)])
            with self.subTest(version=version):
                # Historical default preserves its policy/consensus behavior.
                spend.validate_with_chronicle_context(*context, consensus_only=True)
                with self.assertRaises(ValueError):
                    spend.validate_with_chronicle_context(*context, consensus_only=False)
                for consensus_only in (False, True):
                    spend.validate_with_chronicle_context(
                        *context, consensus_only=consensus_only,
                        discourage_upgradable_nops=False)
                    with self.assertRaises(ValueError):
                        spend.validate_with_chronicle_context(
                            *context, consensus_only=consensus_only,
                            discourage_upgradable_nops=True)

        # In a Chronicle UTXO, the opcode executes and the NOP policy is irrelevant.
        parent = Tx(1, [TxIn('33' * 32, 0, Script())],
                    [TxOut(100, Script.parse_string('OP_1 OP_1 OP_LSHIFTNUM OP_2 OP_EQUAL'))], 0)
        spend = Tx(2, [TxIn(parent.id(), 0, Script())],
                   [TxOut(90, Script.parse_string('OP_1'))], 0)
        spend.validate_with_chronicle_context(
            [parent], 200, 200, [(parent.id(), 0, 200)], consensus_only=True,
            discourage_upgradable_nops=True)

    def test_nop_policy_in_unlocking_and_skipped_branch(self):
        for unlocking, expected in [('b0', False), ('0063b068', True)]:
            parent = Tx(1, [TxIn('44' * 32, 0, Script())],
                        [TxOut(100, Script.parse_string('OP_1'))], 0)
            unlock_script = Script()
            unlock_script.append_data(bytes.fromhex(unlocking))
            spend = Tx(2, [TxIn(parent.id(), 0, unlock_script)],
                       [TxOut(90, Script.parse_string('OP_1'))], 0)
            args = ([parent], 200, 200, [(parent.id(), 0, 200)])
            if expected:
                spend.validate_with_chronicle_context(
                    *args, consensus_only=True, discourage_upgradable_nops=True)
            else:
                with self.assertRaises(ValueError):
                    spend.validate_with_chronicle_context(
                        *args, consensus_only=True, discourage_upgradable_nops=True)

    def test_explicit_output_era_and_metadata(self):
        parent = Tx(1, [TxIn('11' * 32, 0, Script())],
                    [TxOut(100, Script.parse_string('OP_1 OP_2MUL OP_2 OP_EQUAL'))], 0)
        for version in (1, 2):
            spend = Tx(version, [TxIn(parent.id(), 0, Script())],
                       [TxOut(90, Script.parse_string('OP_1'))], 0)
            spend.validate_with_chronicle_context(
                [parent], 200, 200, [(parent.id(), 0, 200)], consensus_only=True)
            for heights in ([], [(parent.id(), 0, 100)], [(parent.id(), 0, 201)]):
                with self.assertRaises(ValueError):
                    spend.validate_with_chronicle_context(
                        [parent], 200, 200, heights, consensus_only=True)

    def test_duplicate_creation_heights(self):
        parent = Tx(1, [TxIn('11' * 32, 0, Script())], [TxOut(100, Script.parse_string('OP_1'))], 0)
        spend = Tx(1, [TxIn(parent.id(), 0, Script())], [TxOut(90, Script.parse_string('OP_1'))], 0)
        old, new = (parent.id(), 0, 199), (parent.id(), 0, 200)
        spend.validate_with_chronicle_context([parent], 200, 200, [old, old])
        for heights in ([old, new], [new, old]):
            with self.subTest(heights=heights), self.assertRaisesRegex(ValueError, 'Conflicting UTXO creation heights'):
                spend.validate_with_chronicle_context([parent], 200, 200, heights)

    def test_released_defaults_and_signed_version(self):
        for network, height in [('mainnet', 943816), ('testnet', 1713168),
                                ('stn', 250), ('regtest', 15000)]:
            self.assertEqual(activation_height(network), height)
        self.assertEqual(effective_chronicle_tx_version(0xffffffff, 0, 'regtest'), 0xffffffff)
        self.assertEqual(effective_chronicle_tx_version(2, 14999, 'regtest'), 1)


if __name__ == '__main__':
    unittest.main()
