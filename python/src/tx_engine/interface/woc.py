""" What's on Chain interface
"""
import logging
import time
from typing import Optional, Dict
import requests


LOGGER = logging.getLogger(__name__)


def get_url(testnet: bool = True) -> str:
    """ Based on the network return the URL string
    """
    if testnet:
        return "https://api.whatsonchain.com/v1/bsv/test"
    return "https://api.whatsonchain.com/v1/bsv/main"


def get_response(url: str, max_retries: int = 5):
    """This is the guts of all the WoC requests.

    Retries on connection errors and transient HTTP responses (429/502/503/504).
    """
    transient_status = {429, 502, 503, 504}
    for attempt in range(max_retries):
        try:
            response = requests.get(url, timeout=30)
        except (ConnectionError, requests.Timeout) as e:
            LOGGER.warning(f"WoC request error for {url}: {e}")
            if attempt + 1 >= max_retries:
                return None
            time.sleep(1 + attempt)
            continue

        if response.status_code == 200:
            data = response.json()
            LOGGER.debug(f"data = {data}")
            return data

        if response.status_code in transient_status and attempt + 1 < max_retries:
            LOGGER.warning(
                f"WoC HTTP {response.status_code} for {url}, retrying "
                f"({attempt + 1}/{max_retries})"
            )
            time.sleep(1 + attempt)
            continue

        LOGGER.debug(f"response = {response}")
        return None

    return None


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
    return get_response(f"{get_url(testnet)}/tx/{tx_id}/proof/tsc")


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
    return get_response(f"{get_url(testnet)}/block/{block_hash}/header")
