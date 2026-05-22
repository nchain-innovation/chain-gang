""" Tests for verify_script Chronicle helpers and RPC flag docs
"""
import unittest

from tx_engine.interface.verify_script import (
    CHRONICLE_ACTIVATION_MAINNET,
    DEFAULT_NODE_VERIFYSCRIPT_FLAGS,
    ScriptFlags,
    activation_height,
    chronicle_rules_active,
    effective_chronicle_tx_version,
    verifyscript_params,
)


class VerifyScriptTest(unittest.TestCase):
    """ Chronicle helpers and verifyscript RPC payload builder
    """

    def test_chronicle_rules_version_only(self):
        self.assertTrue(chronicle_rules_active(2))
        self.assertFalse(chronicle_rules_active(1))

    def test_chronicle_rules_mainnet_height(self):
        self.assertFalse(
            chronicle_rules_active(2, CHRONICLE_ACTIVATION_MAINNET - 1, "mainnet")
        )
        self.assertTrue(
            chronicle_rules_active(2, CHRONICLE_ACTIVATION_MAINNET, "BSV_Mainnet")
        )

    def test_effective_tx_version_pre_activation(self):
        self.assertEqual(
            effective_chronicle_tx_version(
                2, CHRONICLE_ACTIVATION_MAINNET - 1, "mainnet"
            ),
            1,
        )
        self.assertEqual(
            effective_chronicle_tx_version(2, CHRONICLE_ACTIVATION_MAINNET, "mainnet"),
            2,
        )

    def test_activation_height_networks(self):
        self.assertEqual(activation_height("BSV_Mainnet"), CHRONICLE_ACTIVATION_MAINNET)
        self.assertIsNone(activation_height("BTC_Mainnet"))

    def test_verifyscript_params_omits_flags_by_default(self):
        params = verifyscript_params("abc", 0, "00", 1000)
        self.assertNotIn("flags", params)
        self.assertEqual(params["txo"]["height"], -1)

    def test_verifyscript_params_with_node_flags(self):
        flags = int(ScriptFlags.SCRIPT_VERIFY_P2SH | ScriptFlags.SCRIPT_GENESIS)
        params = verifyscript_params("abc", 0, "00", 1000, script_flags=flags)
        self.assertEqual(params["flags"], flags)

    def test_default_node_flags_is_post_genesis_set(self):
        self.assertNotEqual(DEFAULT_NODE_VERIFYSCRIPT_FLAGS, 0)
        self.assertTrue(
            DEFAULT_NODE_VERIFYSCRIPT_FLAGS & ScriptFlags.SCRIPT_ENABLE_SIGHASH_FORKID
        )


if __name__ == "__main__":
    unittest.main()
