""" Test of WhatsOnChain API calls

By default these run against responses recorded from WhatsOnChain testnet
(fixtures/woc/), with requests.get patched, so the suite needs no network and
a WhatsOnChain outage cannot fail an unrelated change.

To run the same checks against the live API as well:

    CHAIN_GANG_LIVE_TESTS=1 python3 -m unittest test_woc
"""
import json
import os
import unittest
from unittest.mock import MagicMock, patch

from tx_engine import interface_factory
from tx_engine.interface import woc


CONFIG = {
    "interface_type": "woc",
    "network_type": "testnet",
}

LIVE_TESTS = os.environ.get("CHAIN_GANG_LIVE_TESTS") == "1"

FIXTURES = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fixtures", "woc")

BLOCK_HASH = "000000001a06963e6bc2bd798fa848e57856b9239c22feba644ec100dc809fe4"
TXID = "6106903f0e8e905b749b73d2a7239a22d2f06faf95f66e2ee4db77d875bf7bea"

# Recorded with curl from the URL on the left
RECORDED = {
    f"{woc.get_url(testnet=True)}/block/{BLOCK_HASH}/header": "block_header.json",
    f"{woc.get_url(testnet=True)}/block/hash/{BLOCK_HASH}": "block_by_hash.json",
    f"{woc.get_url(testnet=True)}/tx/{TXID}/proof/tsc": "merkle_proof_tsc.json",
}


def recorded_get(url, timeout=None, params=None):
    """Stand in for requests.get, answering from the recorded responses"""
    if url not in RECORDED:
        raise AssertionError(f"no recorded WhatsOnChain response for {url}")
    with open(os.path.join(FIXTURES, RECORDED[url]), encoding="utf-8") as f:
        body = json.load(f)
    response = MagicMock()
    response.status_code = 200
    response.json.return_value = body
    return response


class _Checks:
    """ Holder, so unittest does not collect WoCChecks itself """

    class WoCChecks(unittest.TestCase):
        """ Checks shared by the recorded and live runs """

        def setUp(self):
            self.woc_interface = interface_factory.set_config(CONFIG)
            self.maxDiff = 4096
            return super().setUp()

        def test_get_block_header(self):
            result = self.woc_interface.get_block_header(BLOCK_HASH)
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
            result = self.woc_interface.get_block(BLOCK_HASH)
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
            result = self.woc_interface.get_merkle_proof(block_hash, TXID)
            assert result is not None
            expected_result = [{
                'index': 3,
                'txOrId': '6106903f0e8e905b749b73d2a7239a22d2f06faf95f66e2ee4db77d875bf7bea',
                'target': '0000000011eb7961f5b07c64f130c19eb0e1c61a1273d5774eff54f72a847d14',
                'nodes': ['d947f541793cccf9a43463d21a1318f99144a2a7ee4b41fd36c74dfe87df065a', '8205865d2b22f2a83367dd338498d7bf41c0a7cf3eedcfb0579885cb98a767d1']
            }]
            self.assertEqual(result, expected_result)


class WoCTests(_Checks.WoCChecks):
    """ Tests of WhatsOnChain API calls, against recorded responses
    """
    def setUp(self):
        patcher = patch.object(woc.requests, "get", side_effect=recorded_get)
        self.requests_get = patcher.start()
        self.addCleanup(patcher.stop)
        return super().setUp()

    def test_requests_go_to_the_testnet_endpoints(self):
        # The recorded responses are keyed by URL, so this also pins the paths
        self.woc_interface.get_block_header(BLOCK_HASH)
        self.woc_interface.get_block(BLOCK_HASH)
        self.woc_interface.get_merkle_proof("", TXID)
        urls = [call.args[0] for call in self.requests_get.call_args_list]
        self.assertEqual(urls, list(RECORDED))


@unittest.skipUnless(LIVE_TESTS, "set CHAIN_GANG_LIVE_TESTS=1 to call WhatsOnChain testnet")
class WoCLiveTests(_Checks.WoCChecks):
    """ Tests of WhatsOnChain API calls, against the live testnet API
    """


if __name__ == "__main__":
    unittest.main()
