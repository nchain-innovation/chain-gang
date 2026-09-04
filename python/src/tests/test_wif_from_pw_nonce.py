""" Test of wif_from_pw_nonce network handling

The network name used to be matched as a string, where the binding defaulted an
unrecognised name to testnet while the Rust function underneath defaulted to
mainnet. It is now parsed once, and an unrecognised name is an error.
"""
import unittest

from tx_engine import wif_from_pw_nonce


class WifFromPwNonceTest(unittest.TestCase):
    """ Test of wif_from_pw_nonce network handling """

    def test_default_is_testnet(self):
        # A compressed testnet WIF starts with 'c'
        self.assertEqual(
            wif_from_pw_nonce("password", "nonce"),
            wif_from_pw_nonce("password", "nonce", "BSV_Testnet"),
        )
        self.assertTrue(wif_from_pw_nonce("password", "nonce").startswith("c"))

    def test_mainnet_differs_from_testnet(self):
        main = wif_from_pw_nonce("password", "nonce", "BSV_Mainnet")
        test = wif_from_pw_nonce("password", "nonce", "BSV_Testnet")
        self.assertNotEqual(main, test)
        # A compressed mainnet WIF starts with 'K' or 'L'
        self.assertIn(main[0], "KL")

    def test_regtest_matches_testnet(self):
        self.assertEqual(
            wif_from_pw_nonce("password", "nonce", "BSV_Regtest"),
            wif_from_pw_nonce("password", "nonce", "BSV_Testnet"),
        )

    def test_unrecognised_network_raises(self):
        # Previously this quietly returned a testnet key through the binding,
        # and a mainnet key when the Rust function was called directly
        for name in ["testnet", "BSV_TestNet", "nonsense", ""]:
            with self.assertRaises(Exception, msg=f"{name!r} should be rejected"):
                wif_from_pw_nonce("password", "nonce", name)

    def test_network_without_a_wif_prefix_raises(self):
        # Parses as a Network, but the crate defines no WIF prefix for it
        for name in ["BSV_STN", "BTC_Mainnet", "BCH_Testnet"]:
            with self.assertRaises(Exception, msg=f"{name} should be rejected"):
                wif_from_pw_nonce("password", "nonce", name)


if __name__ == "__main__":
    unittest.main()
