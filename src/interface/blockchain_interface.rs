use async_trait::async_trait;

use crate::{
    messages::{BlockHeader, Tx},
    network::Network,
    util::ChainGangError,
};
use serde::Deserialize;

//#[allow(unused_must_use)]

/// Balance returned from WoC
#[derive(Debug, Default, Deserialize, Clone, Copy)]
pub struct Balance {
    /// Confirmed balance in satoshis
    pub confirmed: i64,
    /// Unconfirmed balance in satoshis
    pub unconfirmed: i64,
}

/// Height reported for a UTXO that has not been confirmed in a block.
///
/// Any negative height means unconfirmed; this is the value the interfaces in
/// this crate emit, and the one the parallel Python clients emit too.
pub const UNCONFIRMED_HEIGHT: i32 = -1;

// The balance filters distinguish unconfirmed outputs by testing for a negative
// height, so the sentinel has to stay negative
const _: () = assert!(UNCONFIRMED_HEIGHT < 0);

/// Type to represent UTXO Entry
#[allow(dead_code)]
#[derive(Debug, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct UtxoEntry {
    /// Block height at which the UTXO was confirmed, or a negative height if it
    /// is unconfirmed. See [`UNCONFIRMED_HEIGHT`].
    pub height: i32,
    /// Output index within the transaction
    pub tx_pos: u32,
    /// Hex-encoded transaction ID containing the output
    pub tx_hash: String,
    /// Output value in satoshis
    pub value: i64,
}
/// Type to represent UTXO set
pub type Utxo = Vec<UtxoEntry>;

/// Returns `response` unchanged if its HTTP status is 200, otherwise logs the
/// request URL and returns a [`ChainGangError::ResponseError`].
///
/// Shared by the HTTP-backed blockchain interfaces so the status check lives in
/// one place.
pub(crate) fn check_status(
    response: reqwest::Response,
    url: impl std::fmt::Display,
) -> Result<reqwest::Response, ChainGangError> {
    if response.status() != 200 {
        log::warn!("url = {}", url);
        return Err(ChainGangError::ResponseError(format!(
            "response.status() = {}",
            response.status()
        )));
    }
    Ok(response)
}

/// The most of a refused broadcast's response body kept in its error, in
/// bytes. A node's reason (`258: txn-mempool-conflict`) is a few dozen; the
/// cap is there for a proxy's HTML error page, or a body that never ends.
pub(crate) const MAX_REFUSAL_BODY: usize = 512;

/// The error for a broadcast answered with a status other than 200: the status
/// and then the response body, which is where WhatsOnChain and UaaS put the
/// node's reason for refusing the transaction.
///
/// Reads at most [`MAX_REFUSAL_BODY`] bytes of the body, so a large or endless
/// one costs no more than that. A body that cannot be read leaves the status
/// alone, as it was before the body was kept.
pub(crate) async fn refusal(
    mut response: reqwest::Response,
    url: impl std::fmt::Display,
) -> ChainGangError {
    log::debug!("url = {}", url);
    let status = response.status();
    let mut body = Vec::new();
    while body.len() <= MAX_REFUSAL_BODY {
        match response.chunk().await {
            Ok(Some(chunk)) => body.extend_from_slice(&chunk),
            Ok(None) => break,
            Err(e) => {
                log::debug!("reading the refusal's body failed: {e}");
                break;
            }
        }
    }
    ChainGangError::ResponseError(refusal_message(status, &body))
}

/// `response.status() = 400 Bad Request: 258: txn-mempool-conflict`.
///
/// The status comes first and is written as it always was, so a caller that
/// reads it off the message still finds it. The body follows on one line,
/// its whitespace collapsed, cut to [`MAX_REFUSAL_BODY`] bytes on a character
/// boundary and marked `…` where it was cut. An empty body adds nothing.
pub(crate) fn refusal_message(status: reqwest::StatusCode, body: &[u8]) -> String {
    let truncated = body.len() > MAX_REFUSAL_BODY;
    let text = String::from_utf8_lossy(&body[..body.len().min(MAX_REFUSAL_BODY)]);
    // A cut through a multi-byte character decodes to U+FFFD; drop it
    let text = if truncated {
        text.trim_end_matches('\u{FFFD}')
    } else {
        &text
    };
    let mut text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return format!("response.status() = {status}");
    }
    if truncated {
        text.push('…');
    }
    format!("response.status() = {status}: {text}")
}

/// Trait of the blockchain interface
///
#[async_trait]
pub trait BlockchainInterface: Send + Sync {
    /// Set the network this interface operates on
    fn set_network(&mut self, network: &Network);

    /// Return `Ok(())` if the connection to the blockchain service is good
    async fn status(&self) -> Result<(), ChainGangError>;

    /// Get balance associated with address
    async fn get_balance(&self, address: &str) -> Result<Balance, ChainGangError>;

    /// Get UXTO associated with address
    async fn get_utxo(&self, address: &str) -> Result<Utxo, ChainGangError>;

    /// Broadcast Tx, return the txid
    ///
    /// A transaction the service refuses comes back as
    /// [`ChainGangError::ResponseError`] carrying the node's reason: the HTTP
    /// status and then the response body (`response.status() = 400 Bad
    /// Request: 258: txn-mempool-conflict`) from `woc` and `uaas`, and the
    /// JSON-RPC error (`sendrawtransaction failed with code -26: ..`) from
    /// `rpc`.
    async fn broadcast_tx(&self, tx: &Tx) -> Result<String, ChainGangError>;

    /// Get the transaction identified by `txid`
    async fn get_tx(&self, txid: &str) -> Result<Tx, ChainGangError>;

    /// Get the most recent block header
    async fn get_latest_block_header(&self) -> Result<BlockHeader, ChainGangError>;

    /// Get the block headers
    async fn get_block_headers(&self) -> Result<String, ChainGangError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::UaaSInterface;
    use reqwest::StatusCode;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // --- CS-701: a refused broadcast keeps the node's reason ---

    /// Answers one request with `status` and `body`, after reading the
    /// request in full so the client sees the answer rather than a reset.
    /// `repeat` writes the body that many times, as a server that keeps
    /// sending would.
    async fn answer_once(status: &'static str, body: &'static str, repeat: usize) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = socket.read(&mut buf).await.unwrap();
                request.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&request).to_ascii_lowercase();
                if let Some(end) = text.find("\r\n\r\n") {
                    let length = text
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .map_or(0, |v| v.trim().parse::<usize>().unwrap());
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let head = format!(
                "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len() * repeat
            );
            let _ = socket.write_all(head.as_bytes()).await;
            for _ in 0..repeat {
                if socket.write_all(body.as_bytes()).await.is_err() {
                    break; // the client stopped reading, as it should
                }
            }
        });
        format!("http://{addr}")
    }

    #[test]
    fn the_status_comes_first_and_the_body_follows_it() {
        assert_eq!(
            refusal_message(StatusCode::BAD_REQUEST, b"258: txn-mempool-conflict"),
            "response.status() = 400 Bad Request: 258: txn-mempool-conflict"
        );
    }

    /// Callers that read the status off the message see what they always did.
    #[test]
    fn an_empty_body_leaves_the_message_as_it_was() {
        for body in [&b""[..], b"  \n"] {
            assert_eq!(
                refusal_message(StatusCode::BAD_GATEWAY, body),
                "response.status() = 502 Bad Gateway"
            );
        }
    }

    #[test]
    fn a_body_is_kept_on_one_line() {
        assert_eq!(
            refusal_message(
                StatusCode::INTERNAL_SERVER_ERROR,
                b"<html>\n  <body>upstream\r\n\tfailed</body>\n</html>\n"
            ),
            "response.status() = 500 Internal Server Error: <html> <body>upstream failed</body> </html>"
        );
    }

    #[test]
    fn a_long_body_is_cut_at_the_cap_and_marked() {
        let body = "x".repeat(MAX_REFUSAL_BODY + 100);
        let message = refusal_message(StatusCode::BAD_REQUEST, body.as_bytes());
        let kept = message
            .strip_prefix("response.status() = 400 Bad Request: ")
            .unwrap();
        assert_eq!(kept, format!("{}…", "x".repeat(MAX_REFUSAL_BODY)));
    }

    /// A cut through a multi-byte character drops the part of it that was
    /// kept, rather than leaving a replacement character.
    #[test]
    fn a_cut_never_splits_a_character() {
        let body = format!("{}é tail", "x".repeat(MAX_REFUSAL_BODY - 1));
        let message = refusal_message(StatusCode::BAD_REQUEST, body.as_bytes());
        assert!(
            message.ends_with(&format!("{}…", "x".repeat(MAX_REFUSAL_BODY - 1))),
            "{message}"
        );
    }

    /// WhatsOnChain answers a refusal with the node's reason in the body.
    /// `refusal` is what its `broadcast_tx` returns; its URL is fixed, so the
    /// answer is fetched here directly.
    #[tokio::test]
    async fn a_refusal_carries_the_nodes_reason() {
        let url = answer_once(
            "400 Bad Request",
            "\"unexpected response code 500: 258: txn-mempool-conflict\"",
            1,
        )
        .await;
        let response = reqwest::Client::new().post(&url).send().await.unwrap();
        let ChainGangError::ResponseError(message) = refusal(response, &url).await else {
            panic!("not a ResponseError");
        };
        assert_eq!(
            message,
            "response.status() = 400 Bad Request: \
             \"unexpected response code 500: 258: txn-mempool-conflict\""
        );
    }

    /// Through UaaS's `broadcast_tx` itself.
    #[tokio::test]
    async fn a_uaas_refusal_carries_the_nodes_reason() {
        let url = answer_once("400 Bad Request", "16: missing-inputs", 1).await;
        let uaas = UaaSInterface::new(&url).unwrap();
        let error = uaas.broadcast_tx(&Tx::default()).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            ChainGangError::ResponseError(
                "response.status() = 400 Bad Request: 16: missing-inputs".to_string()
            )
            .to_string()
        );
    }

    /// A body far past the cap is not read to its end: the error comes back
    /// with the first bytes of it.
    #[tokio::test]
    async fn a_huge_body_is_not_read_to_its_end() {
        let chunk = "y".repeat(64 * 1024);
        let chunk: &'static str = Box::leak(chunk.into_boxed_str());
        // 64 MiB announced; only the first chunks are read
        let url = answer_once("400 Bad Request", chunk, 1024).await;
        let uaas = UaaSInterface::new(&url).unwrap();
        let error = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            uaas.broadcast_tx(&Tx::default()),
        )
        .await
        .expect("the body was read to its end")
        .unwrap_err();
        let ChainGangError::ResponseError(message) = error else {
            panic!("not a ResponseError: {error:?}");
        };
        assert!(message.ends_with('…'), "{message}");
        assert!(message.len() < MAX_REFUSAL_BODY + 64, "{}", message.len());
    }
}
