from .blockchain_interface import ConfigType
from .mock_interface import MockInterface
from .woc_interface import WoCInterface
from .rpc_interface import RPCInterface


INTERFACE_MAPPING = {
    "mock": MockInterface,
    "woc": WoCInterface,
    "rpc": RPCInterface,
}


class InterfaceFactory:
    """ A class for creating interfaces to the BSV blockchain """

    def set_config(self, config: ConfigType) -> MockInterface | WoCInterface | RPCInterface:
        interface_type = config['interface_type']
        interface = INTERFACE_MAPPING[interface_type]()  # type: ignore[abstract]
        interface.set_config(config)
        return interface  # type: ignore[return-value]


interface_factory = InterfaceFactory()
