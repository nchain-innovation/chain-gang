use hex;
use k256;
use thiserror::Error;

#[cfg(feature = "interface")]
use reqwest;

use url;

/// Errors used throughout the chain-gang library
#[derive(Error, Debug)]
pub enum ChainGangError {
    // Conversion from other Errors
    // --------------------------------------------
    /// An underlying I/O operation failed
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    /// An ECDSA operation in the `k256` crate failed
    #[error("K256 ecdsa Error: {0}")]
    K256EcdsaError(#[from] k256::ecdsa::Error),

    /// An elliptic-curve operation in the `k256` crate failed
    #[error("K256 elliptic_curve Error: {0}")]
    K256EcError(#[from] k256::elliptic_curve::Error),

    /// Base58 encoding or decoding failed
    #[error("Base58 Error: {0}")]
    Base58Error(String),

    /// Parsing an integer from a string failed
    #[error("ParseInt Error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    /// Decoding a hex string failed
    #[error("Hex Error: {0}")]
    HexError(#[from] hex::FromHexError),

    /// Interpreting bytes as a UTF-8 string failed
    #[error("Utf8 Error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// An HTTP request made via `reqwest` failed
    #[cfg(feature = "interface")]
    #[error("Reqwest Error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    /// Serializing or deserializing JSON with `serde_json` failed
    #[error("Serde JSON Parse error")]
    SerdeJSONParseError(#[from] serde_json::Error),

    /// Parsing a URL failed
    #[error("URL Parse error")]
    URLParseError(#[from] url::ParseError),

    // Chain Gang Errors
    // --------------------------------------------
    /// Evaluating a Bitcoin script produced an error
    #[error("Error evaluating the script `{0}`")]
    ScriptError(String),

    /// The object or execution state is not valid
    #[error("The state is not valid `{0}`")]
    IllegalState(String),

    /// A provided argument is not valid
    #[error("A provided argument is not valid `{0}`")]
    BadArgument(String),

    /// Provided data is not valid or malformed
    #[error("A provided data is not valid `{0}`")]
    BadData(String),

    /// The operation exceeded its time limit
    #[error("The operation timed out")]
    Timeout,

    /// The operation is not valid on this object
    #[error("The operation is not valid on this object")]
    InvalidOperation(String),

    /// A received response was invalid
    #[error("Invalid reponse")]
    ResponseError(String),

    /// Parsing JSON failed
    #[error("JSON Parse error")]
    JSONParseError(String),
}

#[cfg(feature = "python")]
use pyo3::exceptions::PyValueError;

#[cfg(feature = "python")]
use pyo3::prelude::*;

// Convert ChainGangError to a Python Error
#[cfg(feature = "python")]
impl From<ChainGangError> for PyErr {
    fn from(err: ChainGangError) -> PyErr {
        PyValueError::new_err(err.to_string())
    }
}
