use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid license key: {0}")]
    InvalidLicense(String),

    #[error("license expired")]
    LicenseExpired,

    #[error("machine fingerprint mismatch")]
    FingerprintMismatch,

    #[error("invalid master key passphrase")]
    InvalidPassphrase,

    #[error("Ed25519 signature verification failed")]
    SignatureVerificationFailed,

    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("random number generation failed")]
    RngFailed,
}
