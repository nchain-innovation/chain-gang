#!/usr/bin/python3
from io import BytesIO


from typing import Dict, List, Any
from .blockchain_interface import BlockchainInterface
from tx_engine import Tx


def tx_hex_to_txid(tx_hex: str) -> str:
    """ Given tx as hex str return the txid
    """
    tx = Tx.parse(BytesIO(bytes.fromhex(tx_hex)))
    return tx.id()


class MockInterface(BlockchainInterface):
    """ Mock BSV Client for use in unit tests
    """
    def __init__(self):
        # This is a txid to transaction mapping set up before test
        self.transactions = {}
        # These are the transactions broadcast during the test
        self.broadcast = {}
        # This is a UTXO address to transaction mapping set up before test
        self.utxo = {}
        self.block_count = 0
        self.balance = 0

    def set_config(self, _config):
        pass

    # Mock client supporting methods
    # tx
    def set_transactions(self, txs):
        """ Set dictionary of txid: tx that can be obtained by
            get_raw_transaction() during the test.
        """
        self.transactions = txs

    # Broadcast tx
    def clear_broadcast_txs(self):
        """ Clear the list of broadcast transactions ready for a test.
        """
        self.broadcast.clear()

    def get_broadcast_txs(self):
        """ Return a list of transactions that have been transmitted
            by broadcast_tx() during the test.
        """
        return self.broadcast

    # UTXO
    def clear_utxo(self):
        """ Clear the UTXO set ready for a test.
        """
        self.utxo.clear()

    def set_utxo(self, utxo):
        """ Set the UTXO ser ready for a test.
        """
        self.utxo = utxo

    # Normal client methods
    def get_raw_transaction(self, txid: str) -> str:
        try:
            return self.transactions[txid]
        except KeyError:
            return self.broadcast[txid]

    def broadcast_tx(self, tx: str):
        txid = tx_hex_to_txid(tx)
        self.broadcast[txid] = tx
        return txid

    def get_utxo(self, address: str) -> List:
        try:
            return self.utxo[address]
        except KeyError:
            return []

    def get_block_count(self) -> int:
        return self.block_count

    def get_tx_out(self, txid: str, txindex: int) -> Dict:
        raise NotImplementedError('get_tx_out not implemented for the mock client api')

    def get_best_block_hash(self) -> str:
        raise NotImplementedError('get_best_block_hash no implemeted for the mock client api')

    def get_block(self, blockhash: str):
        raise NotImplementedError('get_block not implemented for the mock client api')

    def get_transaction(self, txid: str) -> Dict:
        raise NotImplementedError('get_transaction not implemented for the mock client api')

    # balance and isTestnet
    def get_balance(self, address: str) -> int:
        return self.balance

    def is_testnet(self) -> bool:
        return False

    def get_block_header(self, block_hash: str) -> Dict:
        raise NotImplementedError('get_block_header not implemented for the mock client api')

    def get_merkle_proof(self, block_hash: str, tx_id: str) -> str:
        raise NotImplementedError('get_merkle_proof not implemented for the mock client api')

    def verifyscript(self, scripts: list, stopOnFirstInvalid: bool = True, totalTimeout: int = 100) -> List[Any]:
        raise NotImplementedError("verifyscript not implemented for the WoC interface")
