use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage connection failed: {0}")]
    ConnectionFailed(String),

    #[error("blob not found: {key}")]
    NotFound { key: String },

    #[error("upload failed: {0}")]
    UploadFailed(String),

    #[error("download failed: {0}")]
    DownloadFailed(String),

    #[error("encryption error: {0}")]
    Encryption(String),

    #[error("decryption error: {0}")]
    Decryption(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("vault entry not found: {service}/{key_name}")]
    VaultEntryNotFound { service: String, key_name: String },

    #[error("state key not found: {0}")]
    StateKeyNotFound(String),

    #[error("bucket not configured: {0}")]
    BucketNotConfigured(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        StorageError::Serialization(e.to_string())
    }
}
