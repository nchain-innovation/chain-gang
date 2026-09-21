//! WhatsOnChain blockchain interface (Rust implementation).
//!
//! A parallel, independent pure-Python client lives in
//! `python/src/tx_engine/interface/woc.py` and `woc_interface.py`. The
//! duplication is deliberate: it lets the Python package talk to WhatsOnChain
//! without depending on this crate's `interface` feature. When changing
//! endpoint paths or the network -> `main`/`test`/`stn` mapping, update both
//! implementations so they stay in sync.

use async_trait::async_trait;
use reqwest::StatusCode;

use crate::util::Serializable;
use serde::{Deserialize, Serialize};

use crate::{
    interface::blockchain_interface::{
        check_status, Balance, BlockchainInterface, Utxo, UtxoEntry, UNCONFIRMED_HEIGHT,
    },
    messages::{BlockHeader, Tx},
    network::Network,
    util::ChainGangError,
};

/// Structure for json serialisation for broadcast_tx
#[derive(Debug, Serialize)]
struct BroadcastTxType {
    pub txhex: String,
}

/// Blockchain interface backed by the WhatsOnChain API.
#[derive(Debug, Clone)]
pub struct WocInterface {
    network_type: Network,
}

impl Default for WocInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl WocInterface {
    /// Create a new `WocInterface` defaulting to the BSV testnet.
    pub fn new() -> Self {
        WocInterface {
            network_type: Network::BSV_Testnet,
        }
    }

    /// Return the current network as a string
    /// The path segment WhatsOnChain uses for the configured network.
    ///
    /// Returns an error rather than panicking for a network the service does
    /// not serve: `set_network` accepts any [`Network`] and cannot reject one,
    /// so the mismatch is only detectable here, at the first call.
    fn get_network_str(&self) -> Result<&'static str, ChainGangError> {
        match self.network_type {
            Network::BSV_Mainnet => Ok("main"),
            Network::BSV_Testnet => Ok("test"),
            Network::BSV_STN => Ok("stn"),
            // WhatsOnChain serves no other chain. Listed rather than caught, so
            // a new network has to choose.
            Network::BSV_Regtest
            | Network::BTC_Mainnet
            | Network::BTC_Testnet
            | Network::BCH_Mainnet
            | Network::BCH_Testnet => Err(ChainGangError::BadArgument(format!(
                "WhatsOnChain does not serve {}",
                self.network_type
            ))),
        }
    }
}

/// HTTP statuses worth retrying rather than failing on, matching the policy the
/// parallel Python client already applies in `woc.get_response`.
const TRANSIENT_STATUS: [u16; 4] = [429, 502, 503, 504];

/// Attempts per page before giving up, as in the Python client.
const MAX_PAGE_RETRIES: usize = 5;

/// GETs `url`, retrying transient failures with a linear backoff.
///
/// Paging a large address turns one request into many, and WhatsOnChain rate
/// limits: fetching the ~21 pages of a busy address is enough to draw a 429.
/// Without this, the move to a paginated endpoint would trade silent
/// truncation for an outright failure on exactly the addresses it is meant to
/// fix.
async fn get_text_with_retry(url: &str) -> Result<String, ChainGangError> {
    for attempt in 0..MAX_PAGE_RETRIES {
        let response = reqwest::get(url).await?;
        let status = response.status().as_u16();
        if TRANSIENT_STATUS.contains(&status) && attempt + 1 < MAX_PAGE_RETRIES {
            log::warn!(
                "WoC HTTP {} for {}, retrying ({}/{})",
                status,
                url,
                attempt + 1,
                MAX_PAGE_RETRIES
            );
            tokio::time::sleep(std::time::Duration::from_secs(1 + attempt as u64)).await;
            continue;
        }
        let response = check_status(response, url)?;
        return match response.text().await {
            Ok(txt) => Ok(txt),
            Err(x) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                x
            ))),
        };
    }
    Err(ChainGangError::ResponseError(format!(
        "gave up after {MAX_PAGE_RETRIES} attempts for {url}"
    )))
}

/// Upper bound on pages fetched by [`WocInterface::get_utxo`].
///
/// Each page is up to 1000 entries, so this allows 1M UTXOs for one address.
/// It exists only so that a server that kept handing back a next-page token
/// could not spin here forever; reaching it is an error rather than a silent
/// truncation, which is the whole point of moving off `/unspent`.
const MAX_UTXO_PAGES: usize = 1000;

/// One entry of an `/unspent/all` response.
///
/// Separate from [`UtxoEntry`] because the wire format carries two fields the
/// public type does not: `status`, which is how a caller is meant to tell a
/// mempool UTXO from a confirmed one, and `isSpentInMempoolTx`, which is
/// ignored here but is the documented way to filter outputs already spent by
/// an unconfirmed transaction.
#[derive(Debug, Deserialize)]
struct UnspentAllEntry {
    height: i32,
    tx_pos: u32,
    tx_hash: String,
    value: i64,
    /// `"confirmed"` or `"unconfirmed"`. Defaulted rather than required so a
    /// response that omits it still parses, falling back to the height test.
    #[serde(default)]
    status: String,
}

/// An `/unspent/all` response page.
#[derive(Debug, Deserialize)]
struct UnspentAllPage {
    #[serde(default)]
    result: Vec<UnspentAllEntry>,
    /// WhatsOnChain reports per-request failures in the body rather than by
    /// HTTP status, so an empty `result` alone does not mean "no UTXOs".
    #[serde(default)]
    error: String,
    /// Present and non-empty while further pages remain.
    #[serde(default, rename = "nextPageToken")]
    next_page_token: String,
}

impl From<UnspentAllEntry> for UtxoEntry {
    /// Translates WhatsOnChain's unconfirmed marker to this crate's.
    ///
    /// [`UtxoEntry::height`] defines a negative height as meaning unconfirmed.
    /// WhatsOnChain signals it two ways: `status: "unconfirmed"` on
    /// `/unspent/all`, and `height: 0`. Both are honoured, the status first,
    /// so the value is translated on the way in rather than left for every
    /// caller to special-case. A height of 0 cannot mean the genesis block
    /// here: the genesis coinbase is unspendable, so it never appears in an
    /// unspent set.
    fn from(entry: UnspentAllEntry) -> Self {
        let unconfirmed = entry.status.eq_ignore_ascii_case("unconfirmed") || entry.height == 0;
        UtxoEntry {
            height: if unconfirmed {
                UNCONFIRMED_HEIGHT
            } else {
                entry.height
            },
            tx_pos: entry.tx_pos,
            tx_hash: entry.tx_hash,
            value: entry.value,
        }
    }
}

#[async_trait]
impl BlockchainInterface for WocInterface {
    fn set_network(&mut self, network: &Network) {
        self.network_type = *network;
    }

    // Return Ok(()) if connection is good
    async fn status(&self) -> Result<(), ChainGangError> {
        log::debug!("status");

        let network = self.get_network_str()?;
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/woc");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        match response.text().await {
            Ok(txt) if txt == "Whats On Chain" => Ok(()),
            Ok(txt) => Err(ChainGangError::ResponseError(format!(
                "Unexpected txt = {}",
                txt
            ))),
            Err(err) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                err
            ))),
        }
    }

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainGangError> {
        log::debug!("get_balance");

        let network = self.get_network_str()?;
        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}/balance");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        let txt = match response.text().await {
            Ok(txt) => txt,
            Err(x) => {
                log::debug!("address = {}", address);
                return Err(ChainGangError::ResponseError(format!(
                    "response.text() = {}",
                    x
                )));
            }
        };
        let data: Balance = match serde_json::from_str(&txt) {
            Ok(data) => data,
            Err(x) => {
                log::debug!("address = {}", address);
                log::warn!("txt = {}", txt);
                return Err(ChainGangError::JSONParseError(format!(
                    "json parse error = {}",
                    x
                )));
            }
        };
        Ok(data)
    }

    /// Get UXTO associated with address
    ///
    /// Reads `/unspent/all`, which covers both confirmed and unconfirmed
    /// outputs and paginates at 1000 entries. Every page is followed, so the
    /// returned set is complete; the superseded `/unspent` stopped at 1000
    /// with no way to ask for the rest, so a busy address came back silently
    /// truncated.
    async fn get_utxo(&self, address: &str) -> Result<Utxo, ChainGangError> {
        log::debug!("get_utxo");
        let network = self.get_network_str()?;

        let base =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}/unspent/all");
        let mut utxo: Utxo = Vec::new();
        let mut token = String::new();

        for page in 0..MAX_UTXO_PAGES {
            // The token is opaque and server-supplied, so it goes through Url
            // rather than being interpolated raw
            let url = if token.is_empty() {
                base.clone()
            } else {
                url::Url::parse_with_params(&base, &[("token", token.as_str())])
                    .map_err(|e| {
                        ChainGangError::BadArgument(format!("could not build page url: {e}"))
                    })?
                    .to_string()
            };
            let txt = get_text_with_retry(&url).await?;
            let data: UnspentAllPage = match serde_json::from_str(&txt) {
                Ok(data) => data,
                Err(x) => {
                    log::warn!("txt = {}", txt);
                    return Err(ChainGangError::JSONParseError(format!(
                        "json parse error = {}",
                        x
                    )));
                }
            };
            if !data.error.is_empty() {
                // Reported in the body with a 200, so it would otherwise read
                // as an empty UTXO set
                return Err(ChainGangError::ResponseError(format!(
                    "WhatsOnChain error = {}",
                    data.error
                )));
            }
            utxo.extend(data.result.into_iter().map(UtxoEntry::from));

            if data.next_page_token.is_empty() {
                return Ok(utxo);
            }
            if data.next_page_token == token {
                // Would otherwise refetch the same page until the page cap
                return Err(ChainGangError::ResponseError(
                    "WhatsOnChain repeated the same next-page token".to_string(),
                ));
            }
            token = data.next_page_token;
            log::debug!("get_utxo page {} done, {} entries so far", page, utxo.len());
        }

        // Failing beats returning a truncated set that looks complete
        Err(ChainGangError::ResponseError(format!(
            "address has more than {MAX_UTXO_PAGES} pages of UTXOs"
        )))
    }

    /// Broadcast Tx
    ///
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String, ChainGangError> {
        log::debug!("broadcast_tx");
        let network = self.get_network_str()?;
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/tx/raw");
        log::debug!("url = {}", url);
        let data_for_broadcast = BroadcastTxType {
            txhex: tx.as_hexstr(),
        };
        //let data = serde_json::to_string(&data_for_broadcast).unwrap();
        let client = reqwest::Client::new();
        let response = client.post(&url).json(&data_for_broadcast).send().await?;
        let status = response.status();
        // Assume a response of 200 means broadcast tx success
        match status {
            StatusCode::OK => {
                let res = response.text().await?;
                let hash = res.trim();
                let txid = hash.trim_matches('"');
                Ok(txid.to_string())
            }
            _ => {
                log::debug!("url = {}", url);
                Err(ChainGangError::ResponseError(format!(
                    "response.status() = {}",
                    status
                )))
            }
        }
    }

    async fn get_tx(&self, txid: &str) -> Result<Tx, ChainGangError> {
        log::debug!("get_tx");

        let network = self.get_network_str()?;
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/tx/{txid}/hex");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        match response.text().await {
            Ok(txt) => {
                let bytes = hex::decode(txt)?;
                let mut byte_slice = &bytes[..];
                let tx: Tx = Tx::read(&mut byte_slice)?;
                Ok(tx)
            }
            Err(x) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                x
            ))),
        }
    }

    async fn get_latest_block_header(&self) -> Result<BlockHeader, ChainGangError> {
        log::debug!("get_latest_block_header");
        let network = self.get_network_str()?;
        let url =
            format!("https://api.whatsonchain.com/v1/bsv/{network}/block/headers/latest?count=1");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        match response.text().await {
            Ok(txt) => {
                let bytes = hex::decode(txt)?;
                let mut byte_slice = &bytes[..];
                let blockheader: BlockHeader = BlockHeader::read(&mut byte_slice)?;
                Ok(blockheader)
            }
            Err(x) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                x
            ))),
        }
    }

    async fn get_block_headers(&self) -> Result<String, ChainGangError> {
        log::debug!("get_block_headers");
        let network = self.get_network_str()?;
        let url = format!("https://api.whatsonchain.com/v1/bsv/{network}/block/headers");
        let response = reqwest::get(&url).await?;
        let response = check_status(response, &url)?;
        match response.text().await {
            Ok(headers) => Ok(headers),
            Err(x) => Err(ChainGangError::ResponseError(format!(
                "response.text() = {}",
                x
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::blockchain_interface::UtxoEntry;

    /// A trimmed `/unspent/all` page, in the shape observed against mainnet:
    /// the entries carry `status` and `isSpentInMempoolTx`, and the array is
    /// nested under `result` alongside `error` and `nextPageToken`.
    fn woc_response() -> &'static str {
        r#"{
            "address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
            "script": "8b01df4e",
            "result": [
                {"height": 964754, "tx_pos": 1, "tx_hash": "aa", "value": 1,
                 "isSpentInMempoolTx": false, "status": "confirmed"},
                {"height": 0, "tx_pos": 1, "tx_hash": "fa7f15f8", "value": 2954693,
                 "isSpentInMempoolTx": false, "status": "unconfirmed"}
            ],
            "error": "",
            "nextPageToken": ""
        }"#
    }

    fn entries(json: &str) -> Utxo {
        let page: UnspentAllPage = serde_json::from_str(json).unwrap();
        page.result.into_iter().map(UtxoEntry::from).collect()
    }

    fn woc_on(network: Network) -> WocInterface {
        let mut woc = WocInterface::new();
        woc.set_network(&network);
        woc
    }

    #[test]
    fn served_networks_map_to_their_path_segment() {
        assert_eq!(
            woc_on(Network::BSV_Mainnet).get_network_str().unwrap(),
            "main"
        );
        assert_eq!(
            woc_on(Network::BSV_Testnet).get_network_str().unwrap(),
            "test"
        );
        assert_eq!(woc_on(Network::BSV_STN).get_network_str().unwrap(), "stn");
    }

    #[test]
    fn unserved_networks_return_an_error_rather_than_panicking() {
        // set_network takes any Network and cannot reject one, so a mismatch is
        // a configuration mistake that used to bring the process down here
        for network in [
            Network::BSV_Regtest,
            Network::BTC_Mainnet,
            Network::BTC_Testnet,
            Network::BCH_Mainnet,
            Network::BCH_Testnet,
        ] {
            let err = woc_on(network)
                .get_network_str()
                .expect_err("WhatsOnChain does not serve this network");
            assert!(
                err.to_string().contains(&network.to_string()),
                "the error should name the network, got: {err}"
            );
        }
    }

    #[test]
    fn unconfirmed_entries_become_the_unconfirmed_sentinel() {
        let utxo = entries(woc_response());

        assert_eq!(utxo[1].height, UNCONFIRMED_HEIGHT);
        assert!(utxo[1].height < 0, "the balance filters test for negative");
        // and the confirmed entry is untouched
        assert_eq!(utxo[0].height, 964754);
        assert_eq!(utxo[1].value, 2954693, "only the height is rewritten");
        assert_eq!(utxo[1].tx_hash, "fa7f15f8");
    }

    #[test]
    fn status_marks_an_unconfirmed_entry_even_at_a_real_height() {
        // status is the documented signal; height is only the fallback
        let utxo = entries(
            r#"{"result": [
                {"height": 964754, "tx_pos": 0, "tx_hash": "aa", "value": 1,
                 "status": "unconfirmed"}
            ]}"#,
        );
        assert_eq!(utxo[0].height, UNCONFIRMED_HEIGHT);
    }

    #[test]
    fn height_zero_still_marks_unconfirmed_when_status_is_absent() {
        // A response without status must not regress to reporting height 0
        let utxo = entries(
            r#"{"result": [
                {"height": 0, "tx_pos": 0, "tx_hash": "aa", "value": 1}
            ]}"#,
        );
        assert_eq!(utxo[0].height, UNCONFIRMED_HEIGHT);
    }

    #[test]
    fn a_confirmed_entry_keeps_its_height() {
        let utxo = entries(
            r#"{"result": [
                {"height": 964754, "tx_pos": 0, "tx_hash": "aa", "value": 1,
                 "status": "confirmed"}
            ]}"#,
        );
        assert_eq!(utxo[0].height, 964754);
    }

    #[test]
    fn an_empty_utxo_set_is_fine() {
        let page: UnspentAllPage = serde_json::from_str(
            r#"{"address": "a", "script": "b", "result": [], "error": "", "nextPageToken": ""}"#,
        )
        .unwrap();
        assert!(page.result.is_empty());
        assert!(page.next_page_token.is_empty(), "so paging stops");
    }

    #[test]
    fn a_page_token_is_read_so_paging_continues() {
        let page: UnspentAllPage =
            serde_json::from_str(r#"{"result": [], "nextPageToken": "000000002f01"}"#).unwrap();
        assert_eq!(page.next_page_token, "000000002f01");
    }

    #[test]
    fn a_body_level_error_is_not_mistaken_for_an_empty_set() {
        // WhatsOnChain reports these with HTTP 200, so the body is the only signal
        let page: UnspentAllPage =
            serde_json::from_str(r#"{"result": [], "error": "invalid address"}"#).unwrap();
        assert_eq!(page.error, "invalid address");
        assert!(page.result.is_empty());
    }
}
