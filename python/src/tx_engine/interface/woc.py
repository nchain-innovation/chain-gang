import logging
import requests
import time
from typing import Optional, Dict
""" What's on Chain interface
"""


LOGGER = logging.getLogger(__name__)


def get_url(testnet: bool = True) -> str:
    if testnet:
        return "https://api.whatsonchain.com/v1/bsv/test"
    else:
        return "https://api.whatsonchain.com/v1/bsv/main"


def get_response(url: str):
    """This is the guts of all the WoC requests.
    Retries until successful
    """
    data = None
    while True:
        try:
            response = requests.get(url)
        except ConnectionError as e:
            # Retry on connection error
            LOGGER.warning(f"WoC ConnectionError {e}")
            time.sleep(1)
        else:
            break

    if response.status_code == 200:
        data = response.json()
        LOGGER.debug(f"data = {data}")
    else:
        LOGGER.debug(f"response = {response}")
    return data


def get_unspent_transactions(address: str, testnet: bool = True):
    """Return the unspent transations associated with this address"""
    return get_response(f"{get_url(testnet)}/address/{address}/unspent")


def get_last_unspent(address: str, testnet: bool = True):
    """Return the unspent transations associated with this address"""
    tx_hash = None
    tx_pos = 0
    value = 0
    url = "{}/address/{}/unspent".format(get_url(testnet), address)
    response = requests.get(url)
    if response.status_code == 200:
        data = response.json()
        LOGGER.debug(f"data = {data}")
        tx_hash = data[-1]["tx_hash"]
        tx_pos = data[-1]["tx_pos"]
        value = data[-1]["value"]
    else:
        LOGGER.info(f"response = {response}")
    return (tx_hash, tx_pos, value)


def get_transaction(tx_id: str, testnet: bool = True):
    """Return the transaction associated with this txid"""
    return get_response(f"{get_url(testnet)}/tx/hash/{tx_id}")


def get_raw_transaction(tx_id: str, testnet: bool = True) -> Optional[str]:
    """Return the raw transaction associated with this txid"""
    data = None
    response = requests.get(f"{get_url(testnet)}/tx/{tx_id}/hex")
    if response.status_code == 200:
        data = response.text
        LOGGER.debug(f"data = {data}")
    else:
        LOGGER.debug(f"response = {response}")
    return data


def get_address(address: str, testnet: bool = True):
    """Return the data associated with this address"""
    return get_response(f"{get_url(testnet)}/address/{address}/info")


def get_history(address: str, testnet: bool = True):
    """Return the transaction history associated with this address"""
    return get_response(f"{get_url(testnet)}/address/{address}/history")


def get_balance(address: str, testnet: bool = True):
    """Return the balance associated with this address"""
    return get_response(f"{get_url(testnet)}/address/{address}/balance")


def get_chain_info(testnet: bool = True):
    """This endpoint retrieves information about the state of the chain for the selected network."""
    return get_response(f"{get_url(testnet)}/chain/info")


def get_merkle_proof(tx_id: str, testnet: bool = True):
    """ This endpoint retrieves the merkle tree info for a given confirmed tx
    """
    return get_response(f"{get_url(testnet)}/tx/{tx_id}/proof")


def broadcast_tx(transaction: str, testnet: bool = True):
    """ Broadcast the transaction, return the txid or error message.
    """
    data = '{"txhex":"' + transaction + '"}'
    url = f"{get_url(testnet)}/tx/raw"
    return requests.post(url, data=data)


def get_block_by_hash(block_hash: str, testnet: bool = True):
    """ Get a block by hash
    """
    return get_response(f"{get_url(testnet)}/block/hash/{block_hash}")


def get_block_header(block_hash: str, testnet: bool = True) -> Dict:
    """ Get a blockheader by hash
    """
    return get_response(f"{get_url(testnet)}/block/hash/{block_hash}/header")
