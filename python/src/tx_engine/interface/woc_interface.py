import logging
import functools
from . import woc
from .blockchain_interface import BlockchainInterface
from typing import Dict, Optional, List, Any

LOGGER = logging.getLogger(__name__)


class WoCInterface(BlockchainInterface):
    """This is the Whats on chain interface to the BSV network"""

    def __init__(self):
        self.rawtx_cache = {}
        self.network_type = None
        pass

    def set_config(self, config):
        if config["network_type"] == "testnet":
            self.network_type = "test"
        elif config["network_type"] == "mainnet":
            self.network_type = "main"
        else:
            LOGGER.warning("No address type specified Setting the network type to test")
            self.network_type = "test"

    def is_testnet(self) -> bool:
        assert self.network_type is not None
        if self.network_type == "test":
            return True
        else:
            return False

    def get_addr_history(self, address):
        """Return the transaction history with this address"""
        return woc.get_history(address, testnet=self.is_testnet())

    def get_utxo(self, address):
        """Return the utxo associated with this address"""
        return woc.get_unspent_transactions(address, testnet=self.is_testnet())

    def get_balance(self, address):
        return woc.get_balance(address, testnet=self.is_testnet())

    def _get_chain_info(self):
        return woc.get_chain_info(testnet=self.is_testnet())

    def get_block_count(self):
        """Return the height of the chain"""
        return self._get_chain_info()["blocks"]

    def get_chain_height(self):
        """Return the height of the chain"""
        chain_info = self._get_chain_info()
        return chain_info["blocks"]

    def get_best_block_hash(self):
        chain_info = self._get_chain_info()
        return chain_info["bestblockhash"]

    def get_merkle_proof(self, block_hash: str, tx_id: str) -> str:
        return woc.get_merkle_proof(tx_id, testnet=self.is_testnet())

    def get_transaction(self, txid: str):
        """Return the transaction associated with this txid"""
        return woc.get_transaction(txid, testnet=self.is_testnet())

    @functools.lru_cache
    def get_raw_transaction(self, txid: str) -> Optional[str]:
        """Return the transaction associated with this txid.
        Use cached copy if available.
        """
        return woc.get_raw_transaction(txid, testnet=self.is_testnet())

    def broadcast_tx(self, transaction: str):
        """broadcast this tx to the network"""
        return woc.broadcast_tx(transaction, testnet=self.is_testnet())

    def get_tx_out(self, txid: str, txindex: int) -> Dict:
        raise ValueError("get_tx_out call not available via the WoC rest api")

    def get_block(self, blockhash: str) -> Dict:
        return woc.get_block_by_hash(blockhash, testnet=self.is_testnet())

    def get_block_header(self, blockhash: str) -> Dict:
        ''' Returns te block_header for a given block hash
            NB -> It's more than the block header.
        '''
        return woc.get_block_header(blockhash, testnet=self.is_testnet())

    def verifyscript(self, scripts: list, stopOnFirstInvalid: bool = True, totalTimeout: int = 100) -> List[Any]:
        raise NotImplementedError("verifyscript not implemented for the WoC interface")
