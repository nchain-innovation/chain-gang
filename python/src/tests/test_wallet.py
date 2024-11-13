""" Tests of wallet functionality
"""

import unittest
from tx_engine import Wallet, hash160, Tx, TxIn, TxOut, Script, create_wallet_from_pem_bytes, create_pem_from_wallet


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

    def test_create_testnet_key_from_pem(self):
        pem = '-----BEGIN PRIVATE KEY-----\nMIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQg9UgQ6ADRTosvl43bg5zp\nWU3cFFnuMA0MO5mQpw0yIKmhRANCAAS0+wZKso7C2qmxYsbEvK88us9aop4JTDb9\nnjAqlYPw6ik7Iybiu1aYtVggdWSDfJrEVQcuNdcWGuKohHfU/F6X\n-----END PRIVATE KEY-----\n'
        pem_as_bytes = pem.encode()

        w = create_wallet_from_pem_bytes(pem_as_bytes, network="BSV_Testnet")
        expected_output = "mg7k4cWKZAH6dHFAk4GPjuWFvmFZBHKf7s"
        self.assertEqual(w.get_address(), expected_output)

    def test_testnet_key_from_to_pem(self):
        pem = '-----BEGIN PRIVATE KEY-----\nMIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQg9UgQ6ADRTosvl43bg5zp\nWU3cFFnuMA0MO5mQpw0yIKmhRANCAAS0+wZKso7C2qmxYsbEvK88us9aop4JTDb9\nnjAqlYPw6ik7Iybiu1aYtVggdWSDfJrEVQcuNdcWGuKohHfU/F6X\n-----END PRIVATE KEY-----\n'
        pem_as_bytes = pem.encode()

        w = create_wallet_from_pem_bytes(pem_as_bytes, network="BSV_Testnet")
        test_pem = create_pem_from_wallet(w)
        self.assertEqual(pem, test_pem)

    def test_create_wallet_from_int(self):
        w = Wallet.from_int("BSV_Testnet", 110943977574299588079135027069764758606913326570652510108968462252246438125737)
        self.assertEqual(w.get_address(), "mg7k4cWKZAH6dHFAk4GPjuWFvmFZBHKf7s")

    def test_create_wallet_from_int_mainnet(self):
        w = Wallet.from_int("BSV_Mainnet", 110943977574299588079135027069764758606913326570652510108968462252246438125737)
        self.assertEqual(w.get_address(), "1bnmZRLk8qqrAmZ2VJ1uzHw4merFyKSP3")

    def test_create_wallet_from_hex(self):
        w = Wallet.from_hexstr("BSV_Testnet", "f54810e800d14e8b2f978ddb839ce9594ddc1459ee300d0c3b9990a70d3220a9")
        self.assertEqual(w.get_address(), "mg7k4cWKZAH6dHFAk4GPjuWFvmFZBHKf7s")

    def test_create_wallet_from_hex_mainnet(self):
        w = Wallet.from_hexstr("BSV_Mainnet", "f54810e800d14e8b2f978ddb839ce9594ddc1459ee300d0c3b9990a70d3220a9")
        self.assertEqual(w.get_address(), "1bnmZRLk8qqrAmZ2VJ1uzHw4merFyKSP3")

    def test_wallet_to_int(self):
        w = Wallet("cVoVmd5zY69LEevwGa5iq1Ba3oBc6J8xxUqdKuJCtuFWUJJngPPP")
        self.assertEqual(w.to_int(), 110943977574299588079135027069764758606913326570652510108968462252246438125737)

    def test_wallet_to_hex(self):
        w = Wallet("cVoVmd5zY69LEevwGa5iq1Ba3oBc6J8xxUqdKuJCtuFWUJJngPPP")
        self.assertEqual(w.to_hex(), "f54810e800d14e8b2f978ddb839ce9594ddc1459ee300d0c3b9990a70d3220a9")

    def test_create_wallet_mainnet(self):
        w = Wallet("L5SWJi6972T55DTftAGbTggWRZtCRr3GtShADUqhPnbWDZAciATX")
        self.assertEqual(w.get_address(), "1bnmZRLk8qqrAmZ2VJ1uzHw4merFyKSP3")

    def test_create_mainnet_key_from_pem(self):
        pem = '-----BEGIN PRIVATE KEY-----\nMIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQg9UgQ6ADRTosvl43bg5zp\nWU3cFFnuMA0MO5mQpw0yIKmhRANCAAS0+wZKso7C2qmxYsbEvK88us9aop4JTDb9\nnjAqlYPw6ik7Iybiu1aYtVggdWSDfJrEVQcuNdcWGuKohHfU/F6X\n-----END PRIVATE KEY-----\n'
        w = create_wallet_from_pem_bytes(pem.encode(), network="BSV_Mainnet")
        expected_output = "1bnmZRLk8qqrAmZ2VJ1uzHw4merFyKSP3"
        self.assertEqual(w.get_address(), expected_output)

    def test_mainnet_key_from_to_pem(self):
        pem = '-----BEGIN PRIVATE KEY-----\nMIGEAgEAMBAGByqGSM49AgEGBSuBBAAKBG0wawIBAQQg9UgQ6ADRTosvl43bg5zp\nWU3cFFnuMA0MO5mQpw0yIKmhRANCAAS0+wZKso7C2qmxYsbEvK88us9aop4JTDb9\nnjAqlYPw6ik7Iybiu1aYtVggdWSDfJrEVQcuNdcWGuKohHfU/F6X\n-----END PRIVATE KEY-----\n'
        w = create_wallet_from_pem_bytes(pem.encode(), network="BSV_Mainnet")
        test_pem = create_pem_from_wallet(w)
        self.assertEqual(pem, test_pem)

    def test_get_network(self):
        wtest = Wallet("cVvay9F4wkxrC6cLwThUnRHEajQ8FNoDEg1pbsgYjh7xYtkQ9LVZ")
        wmain = Wallet("L5ZbWEFDWhGb2f95Z3tMR6nAxW6iavhXAdsMVTE3EaTxJ9deQtub")
        self.assertEqual(wtest.get_network(), "BSV_Testnet")
        self.assertEqual(wmain.get_network(), "BSV_Mainnet")


if __name__ == "__main__":
    unittest.main()
