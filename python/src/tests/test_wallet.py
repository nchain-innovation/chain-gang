

import unittest
import sys
sys.path.append("..")


from tx_engine import Wallet


class WalletTest(unittest.TestCase):
    def test_wallet_wif(self):
        wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3"
        wallet = Wallet(wif)
        self.assertEqual(wallet.get_address(), "mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5")


if __name__ == "__main__":
    unittest.main()
