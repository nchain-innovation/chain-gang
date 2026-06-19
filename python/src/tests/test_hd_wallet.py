""" HdWallet and BIP-32 Python binding tests
"""
import unittest

from tx_engine import HdWallet, HdWatchWallet, bip32_path, bip44_path, bsv_coin_type, derive_extended_key, mnemonic_to_seed


ABANDON_MNEMONIC = (
    "abandon abandon abandon abandon abandon abandon abandon "
    "abandon abandon abandon abandon about"
)


class HdWalletTest(unittest.TestCase):
    """ HdWallet Python API
    """

    def test_mnemonic_to_seed_vector(self):
        seed = mnemonic_to_seed(ABANDON_MNEMONIC, "")
        self.assertEqual(len(seed), 64)

    def test_hd_wallet_from_mnemonic_address(self):
        hd = HdWallet.from_mnemonic("BSV_Mainnet", ABANDON_MNEMONIC)
        addr = hd.address_at_bip44(bsv_coin_type(), 0, True, 0)
        self.assertTrue(len(addr) > 20)

    def test_wallet_at_path_signs(self):
        hd = HdWallet.from_mnemonic("BSV_Mainnet", ABANDON_MNEMONIC)
        wallet = hd.wallet_at_path(bip32_path(0, 0, 0))
        self.assertEqual(wallet.get_network(), "BSV_Mainnet")
        self.assertTrue(wallet.get_address())

    def test_derive_extended_key_matches_hd(self):
        hd = HdWallet.from_mnemonic("BSV_Mainnet", ABANDON_MNEMONIC)
        path = bip44_path(bsv_coin_type(), 0, True, 1)
        xprv = hd.derive_xprv(path)
        self.assertEqual(derive_extended_key(hd.master_xprv(), path), xprv)

    def test_watch_wallet_from_account_xpub(self):
        hd = HdWallet.from_mnemonic("BSV_Mainnet", ABANDON_MNEMONIC)
        account_xpub = hd.derive_xpub("m/0'")
        watch = HdWatchWallet.from_xpub(account_xpub)
        self.assertEqual(
            watch.address_at(True, 0),
            hd.address_at(0, True, 0),
        )
        self.assertEqual(watch.derive_xpub("M/0/0"), hd.derive_xpub(bip32_path(0, 0, 0)))


if __name__ == "__main__":
    unittest.main()
