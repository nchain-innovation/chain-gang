//! Wallet and key management

mod extended_key;
mod mnemonic;

pub mod base58_checksum;
#[allow(clippy::module_inception)]
pub mod wallet;

pub use self::extended_key::{
    derive_extended_key, ExtendedKey, ExtendedKeyType, HARDENED_KEY, MAINNET_PRIVATE_EXTENDED_KEY,
    MAINNET_PUBLIC_EXTENDED_KEY, TESTNET_PRIVATE_EXTENDED_KEY, TESTNET_PUBLIC_EXTENDED_KEY,
};
pub use self::mnemonic::{load_wordlist, mnemonic_decode, mnemonic_encode, Wordlist};

pub use self::wallet::{
    create_sighash, public_key_to_address, Wallet, MAIN_PRIVATE_KEY, TEST_PRIVATE_KEY,
};
