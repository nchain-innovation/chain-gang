use base58::FromBase58Error;
use hex::FromHexError;
use k256;
//use k256:elliptic_curve::Error;
use std;
use std::io;
use std::string::FromUtf8Error;

/// Standard error type used in the library
#[derive(Debug)]
pub enum Error {
    /// An argument provided is invalid
    BadArgument(String),
    /// The data given is not valid
    BadData(String),
    /// Base58 string could not be decoded
    FromBase58Error(FromBase58Error),
    /// Hex string could not be decoded
    FromHexError(FromHexError),
    /// UTF8 parsing error
    FromUtf8Error(FromUtf8Error),
    /// The state is not valid
    IllegalState(String),
    /// The operation is not valid on this object
    InvalidOperation(String),
    /// Standard library IO error
    IOError(io::Error),
    /// Error parsing an integer
    ParseIntError(std::num::ParseIntError),
    /// Error evaluating the script
    ScriptError(String),
    /// Error in the Secp256k1 library
    K256Error(k256::ecdsa::Error),
    K256ECError(k256::elliptic_curve::Error),
    // Secp256k1Error(secp256k1::Error),
    /// The operation timed out
    Timeout,
    StringError(String),
    /// The data or functionality is not supported by this library
    Unsupported(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::BadArgument(s) => f.write_str(&format!("Bad argument: {}", s)),
            Error::BadData(s) => f.write_str(&format!("Bad data: {}", s)),
            Error::FromBase58Error(e) => f.write_str(&format!("Base58 decoding error: {:?}", e)),
            Error::FromHexError(e) => f.write_str(&format!("Hex decoding error: {}", e)),
            Error::FromUtf8Error(e) => f.write_str(&format!("Utf8 parsing error: {}", e)),
            Error::IllegalState(s) => f.write_str(&format!("Illegal state: {}", s)),
            Error::InvalidOperation(s) => f.write_str(&format!("Invalid operation: {}", s)),
            Error::IOError(e) => f.write_str(&format!("IO error: {}", e)),
            Error::ParseIntError(e) => f.write_str(&format!("ParseIntError: {}", e)),
            Error::ScriptError(s) => f.write_str(&format!("Script error: {}", s)),
            Error::K256Error(e) => f.write_str(&format!("K256 error: {}", e)),
            Error::K256ECError(e) => f.write_str(&format!("K256EC error: {}", e)),
            Error::Timeout => f.write_str("Timeout"),
            Error::Unsupported(s) => f.write_str(&format!("Unsuppored: {}", s)),
            Error::StringError(s) => f.write_str(&format!("StringError: {}", s)),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::BadArgument(_) => "Bad argument",
            Error::BadData(_) => "Bad data",
            Error::FromBase58Error(_) => "Base58 decoding error",
            Error::FromHexError(_) => "Hex decoding error",
            Error::FromUtf8Error(_) => "Utf8 parsing error",
            Error::IllegalState(_) => "Illegal state",
            Error::InvalidOperation(_) => "Invalid operation",
            Error::IOError(_) => "IO error",
            Error::ParseIntError(_) => "Parse int error",
            Error::ScriptError(_) => "Script error",
            Error::K256Error(_) => "K256 error",
            Error::K256ECError(_) => "K256EC error",
            Error::Timeout => "Timeout",
            Error::StringError(_) => "StringError",
            Error::Unsupported(_) => "Unsupported",
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            Error::FromHexError(e) => Some(e),
            Error::FromUtf8Error(e) => Some(e),
            Error::IOError(e) => Some(e),
            Error::ParseIntError(e) => Some(e),
            // Error::Secp256k1Error(e) => Some(e),
            _ => None,
        }
    }
}

impl From<FromBase58Error> for Error {
    fn from(e: FromBase58Error) -> Self {
        Error::FromBase58Error(e)
    }
}

impl From<FromHexError> for Error {
    fn from(e: FromHexError) -> Self {
        Error::FromHexError(e)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Self {
        Error::FromUtf8Error(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Self {
        Error::ParseIntError(e)
    }
}

impl From<k256::ecdsa::Error> for Error {
    fn from(e: k256::ecdsa::Error) -> Self {
        Error::K256Error(e)
    }
}

impl From<k256::elliptic_curve::Error> for Error {
    fn from(e: k256::elliptic_curve::Error) -> Self {
        Error::K256ECError(e)
    }
}

/// Standard Result used in the library
pub type Result<T> = std::result::Result<T, Error>;

// the trait `From<&str>` is not implemented for `util::result::Error`,
// which is required by `std::result::Result<std::string::String, util::result::Error>:
// FromResidual<std::result::Result<Infallible, &str>>`
// this can't be annotated with `?` because it has type `Result<_, &str>`

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::StringError(e.to_string())
    }
}
