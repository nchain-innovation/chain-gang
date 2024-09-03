from .blockchain_interface import BlockchainInterface

from http.client import CannotSendRequest
import logging
import time
from bitcoinrpc.authproxy import AuthServiceProxy, JSONRPCException
from typing import Dict, List, Any

LOGGER = logging.getLogger(__name__)


class RPC_RETURN_INFO:
    def __init__(self, message):
        self.content = message
        self.status_code = -1


def retry_call(func):
    """ Decorator to retry calls that fail due to connection issues
    """

    def _inner_call(*args, **kwargs):
        while True:
            try:
                return func(*args, **kwargs)
            except (
                ConnectionError,
                ConnectionRefusedError,
                ConnectionAbortedError,
                ConnectionResetError,
                CannotSendRequest,
            ) as e:
                print(e)
                time.sleep(0.25)

    return _inner_call


class RPCInterface(BlockchainInterface):
    """This is an RPC (Regtest, etc) interface to the BSV network"""

    """This client talks to the bitcoin node via rpc
    full list of available commands:
    https://en.bitcoin.it/wiki/Original_Bitcoin_client/API_calls_list
    """

    def __init__(self):
        self.user = None
        self.password = None
        self.address = None
        self.rpc_connection: AuthServiceProxy
        self.network_type = None

    def set_config(self, config):
        """Configure the client based on the provided config"""
        if config["network_type"] == "testnet":
            self.network_type = "test"
        elif config["network_type"] == "mainnet":
            self.network_type = "main"
        else:
            LOGGER.warning("No address type specified Setting the network type to test")
            self.network_type = "test"

        self.user = config["user"]
        self.password = config["password"]
        # Address in the format 127.0.0.1:8080
        self.address = config["address"]
        self.network_type = config["network_type"]

        self.rpc_connection = AuthServiceProxy(
            f"http://{self.user}:{self.password}@{self.address}"
        )

    def is_testnet(self) -> bool:
        assert self.network_type is not None
        if self.network_type == "test":
            return True
        else:
            return False

    def _calc_block_height(self, block_height, confs):
        if confs == 0:
            return confs
        else:
            return block_height - confs - 1

    def _as_satoshis(self, value):
        """Return the bitcoin amount in satoshis"""
        SATOSHIS = 100000000
        return int(value * SATOSHIS)

    def get_unspent(self, address=None):
        """Private function to return unspent"""
        while True:
            try:
                unspent = self.rpc_connection.listunspent(0)
            except (
                ConnectionError,
                ConnectionRefusedError,
                ConnectionAbortedError,
                ConnectionResetError,
                CannotSendRequest,
            ) as e:
                LOGGER.error(f"Connection failed {e}")
                time.sleep(2)
            else:
                if address is None:
                    return unspent
                else:
                    return list(filter(lambda x: x["address"] == address, unspent))

    def _get_chain_info(self) -> Dict:
        return self.rpc_connection.getblockchaininfo()

    def _get_history(self, address, count=50):
        return self.rpc_connection.listtransactions(address, count)

    def get_addr_history(self, address):
        """Return the transaction history with this address"""
        return self._get_history(address)

    def get_utxo(self, address):
        """Return ordered list of UTXOs for this address"""
        unspent_address = self.get_unspent(address)
        block_count = self.get_block_count()

        response = [
            {
                "height": self._calc_block_height(block_count, x["confirmations"]),
                "tx_pos": x["vout"],
                "tx_hash": x["txid"],
                "value": self._as_satoshis(x["amount"]),
            }
            for x in unspent_address
        ]
        response.sort(key=lambda x: x["height"])
        return response

    def get_balance(self, address, confirmations=6):
        """Return the confirmed and unconfirmed balance associated with this address"""
        unspent_address = self.get_unspent(address)

        confirmed = sum(
            [
                x["amount"] if x["confirmations"] >= confirmations else 0
                for x in unspent_address
            ]
        )
        unconfirmed = sum(
            [
                x["amount"] if x["confirmations"] < confirmations else 0
                for x in unspent_address
            ]
        )
        return {
            "confirmed": self._as_satoshis(confirmed),
            "unconfirmed": self._as_satoshis(unconfirmed),
        }

    @retry_call
    def get_block_count(self):
        """Return the block height"""
        return self.rpc_connection.getblockcount()

    @retry_call
    def get_transaction(self, txid: str):
        """ Return the transaction associated with this txid
            Note that the returned format is different from that returned by WOC
        """
        return self.rpc_connection.gettransaction(txid)

    # extra
    @retry_call
    def get_raw_transaction(self, txid: str) -> str:
        return self.rpc_connection.getrawtransaction(txid)

    """ Returns a dictionary describing the unspent tx out point
    """
    @retry_call
    def get_tx_out(self, txid: str, txindex: int) -> Dict:
        return self.rpc_connection.gettxout(txid, txindex)

    """ Returns the best block hash
    """
    @retry_call
    def get_best_block_hash(self) -> str:
        chain_info = self._get_chain_info()
        return chain_info["bestblockhash"]

    """ returns the merkle proof for a tx
    """
    @retry_call
    def get_merkle_proof(self, block_hash: str, tx_id: str) -> str:
        return self.rpc_connection.gettxoutproof([tx_id], block_hash)

    def broadcast_tx(self, hexstring: str):
        for _ in range(5):
            try:
                message = self.rpc_connection.sendrawtransaction(hexstring)
                api_return = RPC_RETURN_INFO(message)
                api_return.status_code = 200
                return api_return
            except JSONRPCException as err:
                api_return = RPC_RETURN_INFO(err.message)
                api_return.status_code = err.code
                return api_return
            except BrokenPipeError as err:
                print(f"BrokenPipeError: {err}")
                time.sleep(0.25)
            except (
                ConnectionError,
                ConnectionRefusedError,
                ConnectionAbortedError,
                ConnectionResetError,
                CannotSendRequest
            ) as err:
                print(f"ConnectionError: {err}")
                time.sleep(0.25)

    def get_block_hash(self, index: int) -> str:
        """Given a block index return the block hash"""
        return self.rpc_connection.getblockhash(index)

    def get_block(self, hash: str) -> Dict:
        """Given a block hash return the block"""
        return self.rpc_connection.getblock(hash)

    # Comands used in regtest container
    @retry_call
    def get_info(self):
        return self.rpc_connection.getinfo()

    @retry_call
    def get_mining_info(self):
        return self.rpc_connection.getmininginfo()

    @retry_call
    def get_wallet_info(self):
        return self.rpc_connection.getwalletinfo()

    def get_new_address(self):
        return self.rpc_connection.getnewaddress()

    def generate_to_address(self, addr: str, amount=101):
        """Generate blocks and pay them to this account"""
        return self.rpc_connection.generatetoaddress(amount, addr)

    def send_to_address(self, addr: str, amount=5):
        return self.rpc_connection.sendtoaddress(addr, amount)

    def list_accounts(self):
        # listaccounts will be removed from api
        return self.rpc_connection.listaddressgroupings()
        # alternative - return self.rpc_connection.listreceivedbyaccount()

    def import_address(self, address: str):
        return self.rpc_connection.importaddress(address)

    @retry_call
    def get_raw_mempool(self):
        return self.rpc_connection.getrawmempool()

    def generate_blocks(self, n=1):
        return self.rpc_connection.generate(n)

    @retry_call
    def get_block_header(self, block_hash: str):
        return self.rpc_connection.getblockheader(block_hash)

    @retry_call
    def verifyscript(self, scripts: List[Any], stopOnFirstInvalid: bool = True, totalTimeout: int = 100) -> List[Any]:
        return self.rpc_connection.verifyscript(scripts, stopOnFirstInvalid, totalTimeout)
