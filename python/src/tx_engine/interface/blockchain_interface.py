from abc import ABC, abstractmethod
from typing import Dict, Optional, MutableMapping, Any, List

ConfigType = MutableMapping[str, Any]


class BlockchainInterface(ABC):
    """ This is a BlockchainInterface abstract base class
        This will need to be extended with the used methods
    """

    @abstractmethod
    def set_config(self, config: ConfigType):
        pass

    @abstractmethod
    def get_utxo(self, address: str):
        pass

    @abstractmethod
    def get_block_count(self) -> int:
        pass

    @abstractmethod
    def get_raw_transaction(self, txid: str) -> Optional[str]:
        pass

    @abstractmethod
    def get_transaction(self, txid: str) -> Dict:
        pass

    @abstractmethod
    def broadcast_tx(self, tx: str):
        pass

    @abstractmethod
    def is_testnet(self) -> bool:
        pass

    @abstractmethod
    def get_balance(self, address) -> int:
        pass

    """ abstract method definition for get_best_block_hash
    """
    @abstractmethod
    def get_best_block_hash(self) -> str:
        pass

    """ abstract method definition to define the get_tx_out call to an RPC SV node
    """
    @abstractmethod
    def get_tx_out(self, txid: str, txindex: int) -> Dict:
        pass

    """ abstract method definition to define an interface for getblock
    """
    @abstractmethod
    def get_block(self, blockhash: str) -> Dict:
        pass

    """ absract method definition for merkle proof retrieval
    """
    @abstractmethod
    def get_merkle_proof(self, block_hash: str, tx_id: str) -> str:
        pass

    ''' abstract method definition for getting block headers from WoC
    '''
    @abstractmethod
    def get_block_header(self, block_hash: str) -> Dict:
        pass

    ''' abstract method definition for executing verify script.
        This call is not available from WoC
    '''
    @abstractmethod
    def verifyscript(self, scripts: list, stopOnFirstInvalid: bool = True, totalTimeout: int = 100) -> List[Any]:
        pass
