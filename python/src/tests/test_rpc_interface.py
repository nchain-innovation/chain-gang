""" Test of the RPC interface helpers

These cover the parts that need no node: network mapping, the height
calculation and the satoshi conversion.
"""
import unittest

from tx_engine import interface_factory
from tx_engine.interface.rpc_interface import RPCInterface, UNCONFIRMED_HEIGHT


def _configured(network_type):
    """Return an RPCInterface configured for network_type"""
    interface = RPCInterface()
    interface.set_config({
        "interface_type": "rpc",
        "network_type": network_type,
        "user": "user",
        "password": "password",
        "address": "127.0.0.1:18443",
    })
    return interface


class RPCInterfaceTest(unittest.TestCase):
    """ Test of the RPC interface helpers """

    def test_interface_factory_rpc(self):
        interface = interface_factory.set_config({
            "interface_type": "rpc",
            "network_type": "regtest",
            "user": "user",
            "password": "password",
            "address": "127.0.0.1:18443",
        })
        self.assertTrue(isinstance(interface, RPCInterface))

    def test_is_testnet(self):
        # Regression: the network mapping used to be overwritten with the raw
        # config value, leaving is_testnet() false on testnet
        self.assertTrue(_configured("testnet").is_testnet())
        self.assertTrue(_configured("regtest").is_testnet())
        self.assertFalse(_configured("mainnet").is_testnet())

    def test_unknown_network_defaults_to_test(self):
        self.assertTrue(_configured("nonsense").is_testnet())

    def test_calc_block_height(self):
        interface = _configured("regtest")
        # An output in the tip block has one confirmation
        self.assertEqual(interface._calc_block_height(100, 1), 100)
        self.assertEqual(interface._calc_block_height(100, 2), 99)
        self.assertEqual(interface._calc_block_height(100, 100), 1)
        self.assertEqual(interface._calc_block_height(100, 101), 0)

    def test_calc_block_height_unconfirmed(self):
        # Aligned with the Rust crate: a negative height means unconfirmed
        interface = _configured("regtest")
        self.assertEqual(interface._calc_block_height(100, 0), UNCONFIRMED_HEIGHT)
        self.assertLess(UNCONFIRMED_HEIGHT, 0)

    def test_as_satoshis(self):
        interface = _configured("regtest")
        self.assertEqual(interface._as_satoshis(0), 0)
        self.assertEqual(interface._as_satoshis(1), 100000000)
        self.assertEqual(interface._as_satoshis(0.00000001), 1)

    def test_as_satoshis_rounds_rather_than_truncating(self):
        interface = _configured("regtest")
        # 0.29 * 1e8 is 28999999.999999996 in binary floating point, so
        # truncating loses a satoshi
        self.assertEqual(interface._as_satoshis(0.29), 29000000)
        self.assertEqual(interface._as_satoshis(0.57), 57000000)


if __name__ == "__main__":
    unittest.main()
