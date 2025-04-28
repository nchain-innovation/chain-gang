""" Test of WhatsOnChain API calls
"""
import unittest
from tx_engine import interface_factory


CONFIG = {
    "interface_type": "woc",
    "network_type": "testnet",
}


class WoCTests(unittest.TestCase):
    """ Tests of WhatsOnChain API calls
    """
    def setUp(self):
        self.woc_interface = interface_factory.set_config(CONFIG)
        self.maxDiff = 4096
        return super().setUp()

    def test_get_block_header(self):
        block_hash = "000000001a06963e6bc2bd798fa848e57856b9239c22feba644ec100dc809fe4"
        result = self.woc_interface.get_block_header(block_hash)
        assert result is not None
        expected_result = {
            'hash': '000000001a06963e6bc2bd798fa848e57856b9239c22feba644ec100dc809fe4',
            # 'confirmations': 8,
            'size': 184,
            'height': 1671921,
            'version': 536870912,
            'versionHex': '20000000',
            'merkleroot': '998b84019194d5dc5a505997dc17554bb398ac026e4d1fd023571a1d76e3fb63',
            'time': 1745842569,
            'mediantime': 1745839474,
            'nonce': 3248024860,
            'bits': '1c5abd74',
            'difficulty': 2.82120287754299,
            'chainwork': '00000000000000000000000000000000000000000000015814b6013f8e293653',
            'previousblockhash': '00000000122ac20c9fcf1d9f5dca32f8466215a2ef87efb86d9efd1cd0010a88',
            'nextblockhash': '000000001a704a33a4a82cf348b1ffe4403e49f1e2368897712745fabfaf0182',
            'nTx': 0,
            'num_tx': 1
        }
        # Check all fields except `confirmations` which will change
        for k, v in expected_result.items():
            self.assertEqual(result[k], v)

    def test_get_block(self):
        block_hash = "000000001a06963e6bc2bd798fa848e57856b9239c22feba644ec100dc809fe4"
        result = self.woc_interface.get_block(block_hash)
        assert result is not None
        expected_result = {
            'hash': '000000001a06963e6bc2bd798fa848e57856b9239c22feba644ec100dc809fe4',
            # 'confirmations': 9,
            'size': 184,
            'height': 1671921,
            'version': 536870912,
            'versionHex': '20000000',
            'merkleroot': '998b84019194d5dc5a505997dc17554bb398ac026e4d1fd023571a1d76e3fb63',
            'txcount': 1,
            'nTx': 0,
            'num_tx': 1,
            'tx': ['998b84019194d5dc5a505997dc17554bb398ac026e4d1fd023571a1d76e3fb63'],
            'time': 1745842569,
            'mediantime': 1745839474,
            'nonce': 3248024860,
            'bits': '1c5abd74',
            'difficulty': 2.82120287754299,
            'chainwork': '00000000000000000000000000000000000000000000015814b6013f8e293653',
            'previousblockhash': '00000000122ac20c9fcf1d9f5dca32f8466215a2ef87efb86d9efd1cd0010a88',
            'nextblockhash': '000000001a704a33a4a82cf348b1ffe4403e49f1e2368897712745fabfaf0182',
            'coinbaseTx': {
                'txid': '998b84019194d5dc5a505997dc17554bb398ac026e4d1fd023571a1d76e3fb63',
                'hash': '998b84019194d5dc5a505997dc17554bb398ac026e4d1fd023571a1d76e3fb63',
                'version': 1,
                'size': 103,
                'locktime': 0,
                'vin': [{
                    'coinbase': '03f182190d546573746e6574204d696e6572',
                    'txid': '',
                    'vout': 0,
                    'scriptSig': {
                        'asm': '',
                        'hex': ''
                    },
                    'sequence': 4294967295
                }],
                'vout': [{
                    'value': 0.390625,
                    'n': 0,
                    'scriptPubKey': {
                        'asm': 'OP_DUP OP_HASH160 0e84c845ae3af3ba20e8da29a4827abe93b639a4 OP_EQUALVERIFY OP_CHECKSIG',
                        'hex': '76a9140e84c845ae3af3ba20e8da29a4827abe93b639a488ac',
                        'reqSigs': 1,
                        'type': 'pubkeyhash',
                        'addresses': ['mgqipciCS56nCYSjB1vTcDGskN82yxfo1G'],
                        'isTruncated': False
                    }
                }],
                'blockhash': '000000001a06963e6bc2bd798fa848e57856b9239c22feba644ec100dc809fe4',
                # 'confirmations': 9,
                'time': 1745842569,
                'blocktime': 1745842569,
                'blockheight': 1671921
            },
            'totalFees': 0,
            'miner': '\x03��\x19\rTestnet Miner',
            'pages': None
        }

        # Check all fields except `confirmations` which will change
        # Note dictionary can have dictionary..
        for k, v in expected_result.items():
            if isinstance(v, dict):
                for k1, v1 in v.items():
                    self.assertEqual(result[k][k1], v1)
            else:
                self.assertEqual(result[k], v)

    def test_get_merkle_proof(self):
        block_hash = ""
        txid = "6106903f0e8e905b749b73d2a7239a22d2f06faf95f66e2ee4db77d875bf7bea"
        result = self.woc_interface.get_merkle_proof(block_hash, txid)
        assert result is not None
        expected_result = [{
            'index': 3,
            'txOrId': '6106903f0e8e905b749b73d2a7239a22d2f06faf95f66e2ee4db77d875bf7bea',
            'target': '0000000011eb7961f5b07c64f130c19eb0e1c61a1273d5774eff54f72a847d14',
            'nodes': ['d947f541793cccf9a43463d21a1318f99144a2a7ee4b41fd36c74dfe87df065a', '8205865d2b22f2a83367dd338498d7bf41c0a7cf3eedcfb0579885cb98a767d1']
        }]
        self.assertEqual(result, expected_result)


if __name__ == "__main__":
    unittest.main()
