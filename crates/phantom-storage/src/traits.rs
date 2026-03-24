//! Core storage traits.
//! All storage backends implement EncryptedStore.
//! AAD for encryption = key path (prevents blob-swapping attacks).

use async_trait::async_trait;

use crate::errors::StorageError;

/// Trait for encrypted key-value stores.
/// All data is encrypted before storage; AAD = the key path.
#[async_trait]
pub trait EncryptedStore: Send + Sync {
    /// Store encrypted data at the given key.
    async fn put(&self, key: &str, plaintext: &[u8]) -> Result<(), StorageError>;

    /// Retrieve and decrypt data at the given key.
    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Delete data at the given key.
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// List keys matching a prefix.
    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError>;

    /// Check if a key exists.
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify EncryptedStore is object-safe
    fn _assert_object_safe(_: &dyn EncryptedStore) {}
}
