//! Wallet and key management

mod extended_key;
mod hd_wallet;
mod hd_watch_wallet;
mod mnemonic;

pub mod base58_checksum;
#[allow(clippy::module_inception)]
pub mod wallet;

pub use self::extended_key::{
    derive_extended_key, master_extended_key_from_seed, ExtendedKey, ExtendedKeyType,
    BIP32_MASTER_SEED_KEY, HARDENED_KEY, INVALID_CHILD_KEY_MSG, MIN_BIP32_SEED_LENGTH,
    MAINNET_PRIVATE_EXTENDED_KEY, MAINNET_PUBLIC_EXTENDED_KEY, TESTNET_PRIVATE_EXTENDED_KEY,
    TESTNET_PUBLIC_EXTENDED_KEY,
};
pub use self::hd_wallet::{bip32_path, bip44_path, BSV_COIN_TYPE, HdWallet};
pub use self::hd_watch_wallet::{
    scan_address_indices, watch_bip32_path, watch_bip44_path, DEFAULT_GAP_LIMIT, HdWatchWallet,
};
pub use self::mnemonic::{
    load_wordlist, mnemonic_decode, mnemonic_encode, mnemonic_parse, mnemonic_to_seed,
    mnemonic_to_seed_validated, Wordlist, BIP39_PBKDF2_ITERATIONS, BIP39_SALT_PREFIX,
};

pub use self::wallet::{
    create_sighash, create_sighash_checksig_index, public_key_to_address, Wallet, MAIN_PRIVATE_KEY,
    TEST_PRIVATE_KEY,
};
