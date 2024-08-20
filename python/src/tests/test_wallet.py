

import unittest
import sys
sys.path.append("..")


from tx_engine import Wallet, hash160, Tx, TxIn, TxOut, Script


class WalletTest(unittest.TestCase):
    def test_wallet_wif(self):
        wif = "cSW9fDMxxHXDgeMyhbbHDsL5NNJkovSa2LTqHQWAERPdTZaVCab3"
        wallet = Wallet(wif)
        self.assertEqual(wallet.get_address(), "mgzhRq55hEYFgyCrtNxEsP1MdusZZ31hH5")

    def test_sign_tx(self):
        funding_tx = "0100000001baa9ec5094816f5686371e701b3a4dcadc93df44d151496a58089018706b865c000000006b483045022100b53c9ab501032a626050651fb785967e1bdf03bca0cb17cb4f2c75a45a56d17d0220292a27ce9001efb9c41ab9a06ecaaefad91138e94d4407ee14952456274357a24121024f8d67f0a5ec11e72cc0f2fa5c272b69fd448b933f92a912210f5a35a8eb2d6affffffff0276198900000000001976a914661657ba0a6b276bb5cb313257af5cc416450c0888ac64000000000000001976a9147d981c463355c618e9666044315ef1ffc523e87088ac00000000"
        fund_tx = Tx.parse(bytes.fromhex(funding_tx))

        wif_key = "cVvay9F4wkxrC6cLwThUnRHEajQ8FNoDEg1pbsgYjh7xYtkQ9LVZ"
        wallet = Wallet(wif_key)
        self.assertEqual(wallet.get_address(), "mry2yrN53spb4qXC2WFnaNh2uSHk5XDdN6")
        pk = bytes.fromhex(wallet.get_public_key_as_hexstr())
        hash_pk = hash160(pk)
        self.assertEqual(hash_pk.hex(), "7d981c463355c618e9666044315ef1ffc523e870")
        # Matches funding_tx output 1

        # print(f"fund_tx.id() = {fund_tx.id()}")
        # fund_tx.id() = b8d763d3ca229fa43f5b9c886d7848244c9329c5aa349a650faa38859f459c03

        #  fn new(prev_tx: [u8; 32], prev_index: u32, script: &[u8], sequence: u32) -> Self
        vins = [TxIn(prev_tx=fund_tx.id(), prev_index=1, script=Script([]), sequence=0xFFFFFFFF)]
        amt = 50

        # print(f"wallet.get_locking_script().get_commands() = {wallet.get_locking_script().get_commands().hex()}")
        # wallet.get_locking_script().get_commands() = 76a9147d981c463355c618e9666044315ef1ffc523e87088ac
        # fn new(amount: i64, script_pubkey: &[u8]) -> Self
        vouts = [TxOut(amount=amt, script_pubkey=wallet.get_locking_script())]

        # fn new(version: u32, tx_ins: Vec<PyTxIn>, tx_outs: Vec<PyTxOut>, locktime: u32) -> Self
        tx = Tx(version=1, tx_ins=vins, tx_outs=vouts, locktime=0)

        # fn sign_tx(&mut self, index: usize, input_pytx: PyTx, pytx: PyTx) -> PyResult<PyTx>
        new_tx = wallet.sign_tx(0, fund_tx, tx)
        expected = "0100000001039c459f8538aa0f659a34aac529934c2448786d889c5b3fa49f22cad363d7b8010000006b483045022100a0334ea6f3a4fbb8e55ffe38763905a7fc69721a3fc888eaccd6b4379859f57302205baa86118837948582a4365ea67819f9df1c8218477dbd478d30895d65060121412102074255deb137868690e021edc515ab06f33513a287952ff44492390aaca8dae0ffffffff0132000000000000001976a9147d981c463355c618e9666044315ef1ffc523e87088ac00000000"
        self.assertEqual(new_tx.serialize().hex(), expected)
        result = new_tx.validate([fund_tx])
        self.assertIsNone(result)

    def test_sign_tx_twice(self):
        funding_tx = "0100000001baa9ec5094816f5686371e701b3a4dcadc93df44d151496a58089018706b865c000000006b483045022100b53c9ab501032a626050651fb785967e1bdf03bca0cb17cb4f2c75a45a56d17d0220292a27ce9001efb9c41ab9a06ecaaefad91138e94d4407ee14952456274357a24121024f8d67f0a5ec11e72cc0f2fa5c272b69fd448b933f92a912210f5a35a8eb2d6affffffff0276198900000000001976a914661657ba0a6b276bb5cb313257af5cc416450c0888ac64000000000000001976a9147d981c463355c618e9666044315ef1ffc523e87088ac00000000"
        fund_tx = Tx.parse(bytes.fromhex(funding_tx))

        wif_key = "cVvay9F4wkxrC6cLwThUnRHEajQ8FNoDEg1pbsgYjh7xYtkQ9LVZ"
        wallet = Wallet(wif_key)
        self.assertEqual(wallet.get_address(), "mry2yrN53spb4qXC2WFnaNh2uSHk5XDdN6")
        pk = bytes.fromhex(wallet.get_public_key_as_hexstr())
        hash_pk = hash160(pk)
        self.assertEqual(hash_pk.hex(), "7d981c463355c618e9666044315ef1ffc523e870")
        # Matches funding_tx output 1

        #  fn new(prev_tx: [u8; 32], prev_index: u32, script: &[u8], sequence: u32) -> Self
        vins = [TxIn(prev_tx=fund_tx.id(), prev_index=1, script=Script([]), sequence=0xFFFFFFFF)]
        amt = 50

        # fn new(amount: i64, script_pubkey: &[u8]) -> Self
        vouts = [TxOut(amount=amt, script_pubkey=wallet.get_locking_script())]

        # fn new(version: u32, tx_ins: Vec<PyTxIn>, tx_outs: Vec<PyTxOut>, locktime: u32) -> Self
        tx = Tx(version=1, tx_ins=vins, tx_outs=vouts, locktime=0)

        # fn sign_tx(&mut self, index: usize, input_pytx: PyTx, pytx: PyTx) -> PyResult<PyTx>
        new_tx = wallet.sign_tx(0, fund_tx, tx)
        expected = "0100000001039c459f8538aa0f659a34aac529934c2448786d889c5b3fa49f22cad363d7b8010000006b483045022100a0334ea6f3a4fbb8e55ffe38763905a7fc69721a3fc888eaccd6b4379859f57302205baa86118837948582a4365ea67819f9df1c8218477dbd478d30895d65060121412102074255deb137868690e021edc515ab06f33513a287952ff44492390aaca8dae0ffffffff0132000000000000001976a9147d981c463355c618e9666044315ef1ffc523e87088ac00000000"
        self.assertEqual(new_tx.serialize().hex(), expected)
        result = new_tx.validate([fund_tx])
        self.assertIsNone(result)

        # Create same tx again
        tx2 = Tx(version=1, tx_ins=vins, tx_outs=vouts, locktime=0)
        new_tx2 = wallet.sign_tx(0, fund_tx, tx2)
        self.assertEqual(new_tx2.serialize().hex(), expected)
        result = new_tx2.validate([fund_tx])
        self.assertIsNone(result)

    def test_generate_keypair(self):
        w1 = Wallet.generate_keypair("BSV_Testnet")
        w2 = Wallet.generate_keypair("BSV_Testnet")
        self.assertNotEqual(w1.get_address(), w2.get_address())


if __name__ == "__main__":
    unittest.main()
