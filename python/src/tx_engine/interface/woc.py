""" What's On Chain (WoC) low-level request helpers (pure Python).

A parallel Rust implementation of the WoC client lives in
`src/interface/woc_interface.rs`. The two are deliberately independent so the
Python package can reach WhatsOnChain without the Rust `interface` feature.
Keep the endpoint paths and the network -> main/test/stn mapping in sync across
both when either changes.
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


def get_response(url: str, max_retries: int = 5, params: Optional[Dict] = None):
    """This is the guts of all the WoC requests.

    Retries on connection errors and transient HTTP responses (429/502/503/504).
    `params` are appended as a query string, encoded by requests.
    """
    transient_status = {429, 502, 503, 504}
    for attempt in range(max_retries):
        try:
            response = requests.get(url, timeout=30, params=params)
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


#: Upper bound on pages fetched for one address. Each page is up to 1000
#: entries, so this allows 1M UTXOs. It exists only so that a server that kept
#: handing back a next-page token could not spin forever; reaching it raises
#: rather than truncating, which is the whole point of moving off /unspent.
MAX_UTXO_PAGES = 1000


def get_unspent_transactions(address: str, testnet: bool = True):
    """Return the unspent transactions associated with this address

    Reads /unspent/all, which covers both confirmed and unconfirmed outputs and
    paginates at 1000 entries. Every page is followed, so the returned list is
    complete; the superseded /unspent stopped at 1000 with no way to ask for
    the rest, so a busy address came back silently truncated.

    Returns a list of entries, as before. None if a request fails.
    """
    base = f"{get_url(testnet)}/address/{address}/unspent/all"
    entries: list = []
    token = ""

    for _ in range(MAX_UTXO_PAGES):
        # The token is opaque and server-supplied, so it is passed as a
        # parameter rather than interpolated into the path
        page = get_response(base, params={"token": token} if token else None)
        if page is None:
            return None
        error = page.get("error")
        if error:
            # Reported in the body with a 200, so it would otherwise read as an
            # empty UTXO set
            LOGGER.warning(f"WhatsOnChain error = {error}")
            return None
        entries.extend(page.get("result") or [])

        next_token = page.get("nextPageToken") or ""
        if not next_token or next_token == token:
            return entries
        token = next_token

    raise ValueError(f"address has more than {MAX_UTXO_PAGES} pages of UTXOs")


def get_last_unspent(address: str, testnet: bool = True):
    """Return the last unspent transaction associated with this address"""
    data = get_unspent_transactions(address, testnet=testnet)
    if not data:
        return (None, 0, 0)
    LOGGER.debug(f"data = {data}")
    return (data[-1]["tx_hash"], data[-1]["tx_pos"], data[-1]["value"])


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
    """Return the balance associated with this address

    Reads /confirmed/balance and /unconfirmed/balance, the endpoints that
    replaced the combined /balance. That costs two requests where there was
    one, so the two halves are read a moment apart; an address being spent to
    between them could report a confirmed figure from just before a block and
    an unconfirmed one from just after. The combined endpoint is undocumented,
    which is the trade being made.

    Returns {"confirmed": int, "unconfirmed": int} as before, or None if
    either request fails.
    """
    base = f"{get_url(testnet)}/address/{address}"

    confirmed = get_response(f"{base}/confirmed/balance")
    if confirmed is None or confirmed.get("error"):
        if confirmed is not None:
            LOGGER.warning(f"WhatsOnChain error = {confirmed['error']}")
        return None

    unconfirmed = get_response(f"{base}/unconfirmed/balance")
    if unconfirmed is None or unconfirmed.get("error"):
        if unconfirmed is not None:
            LOGGER.warning(f"WhatsOnChain error = {unconfirmed['error']}")
        return None

    return {
        "confirmed": confirmed.get("confirmed", 0),
        "unconfirmed": unconfirmed.get("unconfirmed", 0),
    }


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
