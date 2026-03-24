//! Phantom Cryptographic Primitives
//!
//! Core Laws enforced:
//! - Law 1: No installation without valid license key (Ed25519 verification)
//! - Law 2: No ownership without master key (Argon2id key derivation)
//! - Law 3: Zero local disk footprint (AES-256-GCM encryption for remote storage)

pub mod aes256gcm;
pub mod argon2id;
pub mod ed25519;
pub mod encryption;
pub mod errors;
pub mod fingerprint;
pub mod hkdf_keys;
pub mod license;
pub mod master_key;
pub mod session;

pub use encryption::{decrypt, decrypt_from_json, encrypt, encrypt_to_json, EncryptedBlob};
pub use errors::CryptoError;
pub use license::{check_capabilities, check_expiry, verify_license, LicenseKey, LicensePayload};
pub use master_key::{
    DestructionPayload, MasterKeySession, MnemonicBackup, RemoteKillPayload, TotpConfig,
};
pub use session::{permissions_for_role, AgentKey, AgentPermissions, SessionKey};
