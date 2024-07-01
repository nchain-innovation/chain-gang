import unittest
import sys
sys.path.append("..")

from tx_engine.interface.interface_factory import interface_factory
from tx_engine.interface.woc_interface import WoCInterface
from tx_engine.interface.mock_interface import MockInterface


class InterfaceTest(unittest.TestCase):
    def test_interface_factory_woc(self):
        config = {
            "interface_type": "woc",
            "network_type": "testnet",
        }
        interface = interface_factory.set_config(config)
        self.assertTrue(isinstance(interface, WoCInterface))

    def test_interface_factory_mock(self):
        config = {
            "interface_type": "mock",
            "network_type": "testnet",
        }
        interface = interface_factory.set_config(config)
        self.assertTrue(isinstance(interface, MockInterface))


if __name__ == "__main__":
    unittest.main()
