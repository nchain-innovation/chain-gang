""" This contains the base class for all blockchain interfaces
"""

from abc import ABC, abstractmethod
from typing import Dict, Optional, MutableMapping, Any, List

ConfigType = MutableMapping[str, Any]


class BlockchainInterface(ABC):
    """ This is a BlockchainInterface abstract base class
        This will need to be extended with the used methods
    """

    @abstractmethod
    def set_config(self, config: ConfigType):
        """ Configures the interface based on the provided config
        """

    @abstractmethod
    def get_utxo(self, address: str):
        """ Given the address returns the associated UTXO
        """

    @abstractmethod
    def get_block_count(self) -> int:
        """ Returns the current block height
        """

    @abstractmethod
    def get_raw_transaction(self, txid: str) -> Optional[str]:
        """ Given the txid return the transaction
        """

    @abstractmethod
    def get_transaction(self, txid: str) -> Dict:
        """ Given the txid return the transaction as a dictionary
        """

    @abstractmethod
    def broadcast_tx(self, tx: str):
        """ Broadcast a transaction
        """

    @abstractmethod
    def is_testnet(self) -> bool:
        """ Return true if the interface is operating on testnet
        """

    @abstractmethod
    def get_balance(self, address) -> int:
        """ Return the balance associated with the provided address
        """

    @abstractmethod
    def get_best_block_hash(self) -> str:
        """ Return the current best block hash
        """

    @abstractmethod
    def get_tx_out(self, txid: str, txindex: int) -> Dict:
        """ abstract method definition to define the get_tx_out call to an RPC SV node
        """

    @abstractmethod
    def get_block(self, blockhash: str) -> Dict:
        """ Given the blockhash return the block as a dictionary
        """

    @abstractmethod
    def get_merkle_proof(self, block_hash: str, tx_id: str) -> str:
        """ Given the blockhash and tx_id return the merkle proof
        """

    @abstractmethod
    def get_block_header(self, block_hash: str) -> Dict:
        """ Given the block hash return the block header
        """

    @abstractmethod
    def verifyscript(self, scripts: list, stop_on_first_invalid: bool = True, timeout: int = 100) -> List[Any]:
        """ Given an script and context, verify the script
            This call is only available from local RPC interface
        """
