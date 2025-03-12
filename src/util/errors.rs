use thiserror::Error;
use k256;
use hex;

#[cfg(feature = "interface")]
use reqwest;


use url;


// Errors used in the chain-gang library
#[derive(Error, Debug)]
pub enum ChainGangError {
    // Conversion from other Errors
    // --------------------------------------------
    
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("K256 ecdsa Error: {0}")]
    K256EcdsaError(#[from] k256::ecdsa::Error),
    
    #[error("K256 elliptic_curve Error: {0}")]
    K256EcError(#[from] k256::elliptic_curve::Error),

    #[error("Base58 Error: {0}")]
    Base58Error(String),

    #[error("ParseInt Error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Hex Error: {0}")]
    HexError(#[from] hex::FromHexError),

    #[error("Utf8 Error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[cfg(feature = "interface")]
    #[error("Reqwest Error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Serde JSON Parse error")]
    SerdeJSONParseError(#[from] serde_json::Error),

    #[error("URL Parse error")]
    URLParseError(#[from] url::ParseError),

    // Chain Gang Errors
    // --------------------------------------------
    #[error("Error evaluating the script `{0}`")]
    ScriptError(String),

    #[error("The state is not valid `{0}`")]
    IllegalState(String),

    #[error("A provided argument is not valid `{0}`")]
    BadArgument(String),

    #[error("A provided data is not valid `{0}`")]
    BadData(String),

    #[error("The operation timed out")]
    Timeout,

    #[error("The operation is not valid on this object")]
    InvalidOperation(String),
    
    #[error("Invalid reponse")]
    ResponseError(String),

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
