//! WhatsOnChain blockchain interface (Rust implementation).
//!
//! A parallel, independent pure-Python client lives in
//! `python/src/tx_engine/interface/woc.py` and `woc_interface.py`. The
//! duplication is deliberate: it lets the Python package talk to WhatsOnChain
//! without depending on this crate's `interface` feature. When changing
//! endpoint paths or the network -> `main`/`test`/`stn` mapping, update both
//! implementations so they stay in sync.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_lock::Mutex;
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

/// Spaces out the HTTP requests an interface issues.
///
/// Slots are reserved rather than taken: a caller claims the next free moment,
/// releases the lock, and only then waits for it. Holding the lock across the
/// wait would serialise callers behind whoever got there first, so two
/// concurrent requests would be spaced by the sum of their waits rather than
/// by one interval.
#[derive(Debug, Clone)]
struct RateLimit {
    /// Minimum gap between two requests.
    interval: Duration,
    /// The moment the next request may go out, shared by every clone so that
    /// concurrent callers queue behind one another.
    next: Arc<Mutex<Instant>>,
}

impl RateLimit {
    fn new(requests_per_second: u32) -> Self {
        // A limit of zero would mean an infinite interval, which is a stopped
        // interface rather than a rate limit. Treated as one per second, the
        // slowest rate that still makes progress.
        let per_second = requests_per_second.max(1);
        RateLimit {
            interval: Duration::from_secs(1) / per_second,
            next: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Wait until this request's slot comes round.
    async fn acquire(&self) {
        let slot = {
            let mut next = self.next.lock().await;
            // Not `*next` alone: after an idle spell the stored moment is in
            // the past, and a burst would then be issued all at once.
            let slot = (*next).max(Instant::now());
            *next = slot + self.interval;
            slot
        };
        let now = Instant::now();
        if slot > now {
            tokio::time::sleep(slot - now).await;
        }
    }
}

/// Blockchain interface backed by the WhatsOnChain API.
#[derive(Debug, Clone)]
pub struct WocInterface {
    network_type: Network,
    /// Applied to every HTTP request below, when the caller asked for one.
    rate_limit: Option<RateLimit>,
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
            rate_limit: None,
        }
    }

    /// Bound the HTTP requests this interface issues to `requests_per_second`.
    ///
    /// This has to be set here rather than by the caller, because one
    /// interface call is not one request: [`Self::get_balance`] makes two, and
    /// [`Self::get_utxo`] one per 1000 UTXOs -- 21 for a busy mainnet address.
    /// A caller that spaces its own calls to this type cannot see the paging
    /// and so cannot bound the request rate; this is the only place the whole
    /// burst is visible.
    ///
    /// Off by default, which is the behaviour of every release before this
    /// one. WhatsOnChain documents up to 3 requests per second as free.
    ///
    /// The limit is per `WocInterface`. Clones share it, so cloning to use the
    /// interface from several tasks keeps one budget; separately constructed
    /// interfaces each get their own.
    pub fn set_max_requests_per_second(&mut self, requests_per_second: u32) {
        self.rate_limit = Some(RateLimit::new(requests_per_second));
    }

    /// [`Self::set_max_requests_per_second`], for a value being built.
    pub fn with_max_requests_per_second(mut self, requests_per_second: u32) -> Self {
        self.set_max_requests_per_second(requests_per_second);
        self
    }

    /// Wait for this request's slot, if a limit is set.
    async fn slot(&self) {
        if let Some(rate_limit) = &self.rate_limit {
            rate_limit.acquire().await;
        }
    }

    /// GET `url`, paced.
    ///
    /// Every request this interface makes goes through here or through
    /// [`Self::slot`], so that adding an endpoint cannot quietly escape the
    /// limit.
    async fn get(&self, url: &str) -> Result<reqwest::Response, ChainGangError> {
        self.slot().await;
        Ok(reqwest::get(url).await?)
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
async fn get_text_with_retry(woc: &WocInterface, url: &str) -> Result<String, ChainGangError> {
    for attempt in 0..MAX_PAGE_RETRIES {
        // Inside the loop: a retry is another request, and the point of the
        // limit is the requests, not the calls.
        let response = woc.get(url).await?;
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

/// A `/confirmed/balance` response.
///
/// The figure is the same one the superseded combined `/balance` reported: it
/// aggregates the address's associated scripts, so for an address with both a
/// P2PKH and a P2PK script it counts both.
#[derive(Debug, Deserialize)]
struct ConfirmedBalance {
    #[serde(default)]
    confirmed: i64,
    #[serde(default)]
    error: String,
}

/// An `/unconfirmed/balance` response.
#[derive(Debug, Deserialize)]
struct UnconfirmedBalance {
    #[serde(default)]
    unconfirmed: i64,
    #[serde(default)]
    error: String,
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
/// mempool UTXO from a confirmed one, and `isSpentInMempoolTx`, which marks an
/// output an unconfirmed transaction has already spent and is filtered on
/// before an entry ever becomes a [`UtxoEntry`].
#[derive(Debug, Deserialize)]
struct UnspentAllEntry {
    /// Absent on a mempool entry: WhatsOnChain omits the field rather than
    /// sending 0, so this is an Option and `None` means unconfirmed. Requiring
    /// it failed the whole page with `missing field \`height\`` the moment an
    /// address had one unconfirmed output (CS-462).
    #[serde(default)]
    height: Option<i32>,
    tx_pos: u32,
    tx_hash: String,
    value: i64,
    /// `"confirmed"` or `"unconfirmed"`. Defaulted rather than required so a
    /// response that omits it still parses, falling back to the height test.
    #[serde(default)]
    status: String,
    /// Whether a mempool transaction has already spent this output.
    ///
    /// `/unspent/all` goes on listing a confirmed output after something in
    /// the mempool spends it, and says so only here. Until CS-465 this was
    /// ignored, so `get_utxo` returned outputs already spent as if they were
    /// not -- and a caller building on them made a transaction the network
    /// can only treat as a double spend. Defaulted to `false`, so a response
    /// that omits it is taken at its word.
    #[serde(default, rename = "isSpentInMempoolTx")]
    is_spent_in_mempool_tx: bool,
}

/// The entries of an `/unspent/all` page that are genuinely unspent.
///
/// An output a mempool transaction has already spent is dropped, which makes
/// this interface agree with a node's own `listunspent`: both report what can
/// still be spent, not what was unspent at the last block.
fn unspent_entries(result: Vec<UnspentAllEntry>) -> impl Iterator<Item = UtxoEntry> {
    result
        .into_iter()
        .filter(|entry| !entry.is_spent_in_mempool_tx)
        .map(UtxoEntry::from)
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
    /// WhatsOnChain signals it three ways: by omitting `height` altogether on
    /// a mempool entry, by `status: "unconfirmed"` on `/unspent/all`, and by
    /// `height: 0`. All are honoured, so the value is translated on the way in
    /// rather than left for every caller to special-case. A height of 0 cannot
    /// mean the genesis block here: the genesis coinbase is unspendable, so it
    /// never appears in an unspent set.
    fn from(entry: UnspentAllEntry) -> Self {
        // Any of the three signals means unconfirmed: an absent height, an
        // explicit status, or the height 0 older responses used
        let unconfirmed = entry.height.is_none()
            || entry.status.eq_ignore_ascii_case("unconfirmed")
            || entry.height == Some(0);
        UtxoEntry {
            height: match entry.height {
                Some(h) if !unconfirmed => h,
                _ => UNCONFIRMED_HEIGHT,
            },
            tx_pos: entry.tx_pos,
            tx_hash: entry.tx_hash,
            value: entry.value,
        }
    }
}

impl WocInterface {
    /// GETs `url` and deserialises the body, with the shared retry policy.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, ChainGangError> {
        let txt = get_text_with_retry(self, url).await?;
        serde_json::from_str(&txt).map_err(|x| {
            log::warn!("txt = {}", txt);
            ChainGangError::JSONParseError(format!("json parse error = {}", x))
        })
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
        let response = self.get(&url).await?;
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
    /// Get balance associated with address
    ///
    /// Reads `/confirmed/balance` and `/unconfirmed/balance`, the endpoints
    /// that replaced the combined `/balance`. That costs two requests where
    /// there was one, so the two halves are read a moment apart; an address
    /// being spent to between them could report a confirmed figure from just
    /// before a block and an unconfirmed one from just after. The combined
    /// endpoint is undocumented, which is the trade being made.
    ///
    /// [`Balance`] is unchanged, and both figures were verified to match what
    /// the combined endpoint reported.
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainGangError> {
        log::debug!("get_balance");

        let network = self.get_network_str()?;
        let base = format!("https://api.whatsonchain.com/v1/bsv/{network}/address/{address}");

        let confirmed: ConfirmedBalance =
            self.get_json(&format!("{base}/confirmed/balance")).await?;
        if !confirmed.error.is_empty() {
            log::debug!("address = {}", address);
            return Err(ChainGangError::ResponseError(format!(
                "WhatsOnChain error = {}",
                confirmed.error
            )));
        }

        let unconfirmed: UnconfirmedBalance = self
            .get_json(&format!("{base}/unconfirmed/balance"))
            .await?;
        if !unconfirmed.error.is_empty() {
            log::debug!("address = {}", address);
            return Err(ChainGangError::ResponseError(format!(
                "WhatsOnChain error = {}",
                unconfirmed.error
            )));
        }

        Ok(Balance {
            confirmed: confirmed.confirmed,
            unconfirmed: unconfirmed.unconfirmed,
        })
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
            let txt = get_text_with_retry(self, &url).await?;
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
            utxo.extend(unspent_entries(data.result));

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
        // Paced like the GETs: a broadcast is a request to the same host
        // against the same budget.
        self.slot().await;
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
        let response = self.get(&url).await?;
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
        let response = self.get(&url).await?;
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
        let response = self.get(&url).await?;
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

    // --- CS-457: pacing the requests, not the calls ---

    /// The limit is off unless asked for, which is how every release before
    /// this one behaved.
    #[test]
    fn an_interface_is_unlimited_unless_a_limit_is_set() {
        assert!(WocInterface::new().rate_limit.is_none());
        assert!(WocInterface::default().rate_limit.is_none());
        assert!(WocInterface::new()
            .with_max_requests_per_second(3)
            .rate_limit
            .is_some());
    }

    /// Three per second is one every 333ms, which is what WhatsOnChain's
    /// documented free tier allows.
    #[test]
    fn the_interval_is_the_reciprocal_of_the_rate() {
        assert_eq!(
            RateLimit::new(3).interval,
            Duration::from_secs(1) / 3,
            "3/s is one every 333ms"
        );
        assert_eq!(RateLimit::new(1).interval, Duration::from_secs(1));
    }

    /// Zero would be an infinite interval -- a stopped interface rather than a
    /// slow one.
    #[test]
    fn a_rate_of_zero_is_treated_as_one_per_second() {
        assert_eq!(RateLimit::new(0).interval, Duration::from_secs(1));
    }

    /// The point of the whole change: successive requests are spaced, so a
    /// paginated `get_utxo` cannot issue its pages all at once.
    #[tokio::test]
    async fn requests_are_spaced_by_the_interval() {
        // 20/s is 50ms apart, short enough to keep the test quick and long
        // enough that scheduling noise cannot account for the total.
        let limit = RateLimit::new(20);
        let started = Instant::now();
        for _ in 0..4 {
            limit.acquire().await;
        }
        // The first goes immediately, so four requests cost three intervals.
        assert!(
            started.elapsed() >= Duration::from_millis(150),
            "four requests at 20/s took {:?}, which is less than three intervals",
            started.elapsed()
        );
    }

    /// An idle spell does not bank credit for a burst.
    ///
    /// Without clamping the stored moment to the present, a limiter left alone
    /// for a while holds a next-slot far in the past, and the next several
    /// requests all find their slot already due -- so the burst this exists to
    /// prevent goes out in one go.
    #[tokio::test]
    async fn an_idle_spell_does_not_earn_a_free_burst() {
        let limit = RateLimit::new(20);
        limit.acquire().await;
        tokio::time::sleep(Duration::from_millis(250)).await; // five intervals

        let started = Instant::now();
        for _ in 0..4 {
            limit.acquire().await;
        }
        assert!(
            started.elapsed() >= Duration::from_millis(150),
            "the idle spell bought a burst: four requests took {:?}",
            started.elapsed()
        );
    }

    /// Clones share one budget, so using the interface from several tasks does
    /// not multiply the rate.
    #[tokio::test]
    async fn clones_share_one_budget() {
        let limit = RateLimit::new(20);
        let other = limit.clone();
        let started = Instant::now();
        limit.acquire().await;
        other.acquire().await;
        limit.acquire().await;
        other.acquire().await;
        assert!(
            started.elapsed() >= Duration::from_millis(150),
            "clones paced independently: {:?}",
            started.elapsed()
        );
    }

    /// A trimmed `/unspent/all` page, in the shape the live API actually
    /// sends: the array is nested under `result` alongside `error` and
    /// `nextPageToken`, entries carry `status` and `isSpentInMempoolTx`, and
    /// the mempool entry has **no `height` field at all** plus an extra `hex`.
    /// This fixture previously carried `"height": 0` there, which the API does
    /// not send, and that is how CS-462 escaped.
    fn woc_response() -> &'static str {
        r#"{
            "address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
            "script": "8b01df4e",
            "result": [
                {"height": 964754, "tx_pos": 1, "tx_hash": "aa", "value": 1,
                 "isSpentInMempoolTx": false, "status": "confirmed"},
                {"tx_pos": 1, "tx_hash": "fa7f15f8", "value": 2954693,
                 "isSpentInMempoolTx": false, "hex": "76a914", "status": "unconfirmed"}
            ],
            "error": "",
            "nextPageToken": ""
        }"#
    }

    /// A page as `get_utxo` reads it, filter included.
    fn entries(json: &str) -> Utxo {
        let page: UnspentAllPage = serde_json::from_str(json).unwrap();
        unspent_entries(page.result).collect()
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

    /// CS-462: WhatsOnChain omits `height` entirely on a mempool entry. Every
    /// fixture here used `"height": 0` for unconfirmed, which the live API does
    /// not send, so a required `height` parsed in tests and failed in the field
    /// with `missing field \`height\``. Payload copied from the report.
    #[test]
    fn a_mempool_entry_without_height_parses() {
        let utxo = entries(
            r#"{"result": [
                {"tx_pos": 0, "tx_hash": "73c933af", "value": 44,
                 "isSpentInMempoolTx": false, "hex": "76a914", "status": "unconfirmed"}
            ]}"#,
        );
        assert_eq!(utxo.len(), 1, "the entry must parse at all");
        assert_eq!(
            utxo[0].height, UNCONFIRMED_HEIGHT,
            "absent height is unconfirmed"
        );
        assert_eq!(utxo[0].value, 44);
        assert_eq!(utxo[0].tx_hash, "73c933af");
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
    fn the_two_balance_halves_combine_into_one_balance() {
        // Shapes as observed against mainnet: each carries its own figure,
        // plus address/script/error, and confirmed adds associatedScripts
        let c: ConfirmedBalance = serde_json::from_str(
            r#"{"address": "1A1z", "script": "8b01df4e", "confirmed": 7297358945,
                "error": "", "associatedScripts": [{"script": "740485f3", "type": "pubkey"}]}"#,
        )
        .unwrap();
        let u: UnconfirmedBalance = serde_json::from_str(
            r#"{"address": "1A1z", "script": "8b01df4e", "unconfirmed": 42, "error": ""}"#,
        )
        .unwrap();

        let balance = Balance {
            confirmed: c.confirmed,
            unconfirmed: u.unconfirmed,
        };
        assert_eq!(balance.confirmed, 7297358945);
        assert_eq!(balance.unconfirmed, 42);
    }

    #[test]
    fn a_balance_body_error_is_not_mistaken_for_a_zero_balance() {
        // Reported with HTTP 200, so the body is the only signal
        let c: ConfirmedBalance =
            serde_json::from_str(r#"{"confirmed": 0, "error": "invalid address"}"#).unwrap();
        assert_eq!(c.error, "invalid address");
        assert_eq!(c.confirmed, 0, "which is why the error must be checked");

        let u: UnconfirmedBalance =
            serde_json::from_str(r#"{"unconfirmed": 0, "error": "invalid address"}"#).unwrap();
        assert_eq!(u.error, "invalid address");
    }

    #[test]
    fn a_negative_unconfirmed_balance_survives_the_round_trip() {
        // Spending a confirmed UTXO from the mempool makes this negative
        let u: UnconfirmedBalance =
            serde_json::from_str(r#"{"unconfirmed": -5000, "error": ""}"#).unwrap();
        assert_eq!(u.unconfirmed, -5000);
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

    /// CS-465. An output a mempool transaction has already spent is still
    /// listed by `/unspent/all`, flagged `isSpentInMempoolTx`. It is not
    /// unspent, and handing it to a caller as if it were invites a double
    /// spend -- so it is dropped, and the outputs around it are not.
    #[test]
    fn an_output_already_spent_in_the_mempool_is_not_unspent() {
        let utxo = entries(
            r#"{"result": [
                {"height": 964754, "tx_pos": 0, "tx_hash": "spent", "value": 2000,
                 "isSpentInMempoolTx": true, "status": "confirmed"},
                {"height": 964754, "tx_pos": 1, "tx_hash": "kept", "value": 9904,
                 "isSpentInMempoolTx": false, "status": "confirmed"},
                {"tx_pos": 0, "tx_hash": "change", "value": 1904,
                 "isSpentInMempoolTx": false, "status": "unconfirmed"}
            ]}"#,
        );
        let hashes: Vec<&str> = utxo.iter().map(|entry| entry.tx_hash.as_str()).collect();
        assert_eq!(
            hashes,
            ["kept", "change"],
            "the spent output is gone, the rest are not"
        );
    }

    /// Unconfirmed change a later mempool transaction has already spent is
    /// dropped too: a chain of unconfirmed transactions leaves only its tip.
    #[test]
    fn a_chain_of_unconfirmed_spends_leaves_only_its_tip() {
        let utxo = entries(
            r#"{"result": [
                {"tx_pos": 0, "tx_hash": "first", "value": 1968,
                 "isSpentInMempoolTx": true, "status": "unconfirmed"},
                {"tx_pos": 0, "tx_hash": "second", "value": 1936,
                 "isSpentInMempoolTx": true, "status": "unconfirmed"},
                {"tx_pos": 0, "tx_hash": "tip", "value": 1904,
                 "isSpentInMempoolTx": false, "status": "unconfirmed"}
            ]}"#,
        );
        assert_eq!(utxo.len(), 1);
        assert_eq!(utxo[0].tx_hash, "tip");
    }

    /// A response that does not carry the flag is taken at its word, as it
    /// was before: absent means not spent.
    #[test]
    fn an_absent_flag_means_not_spent() {
        let utxo = entries(
            r#"{"result": [
                {"height": 964754, "tx_pos": 0, "tx_hash": "aa", "value": 1, "status": "confirmed"}
            ]}"#,
        );
        assert_eq!(utxo.len(), 1);
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
