//! In-memory encrypted store for testing and development.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use phantom_crypto::encryption;

use crate::errors::StorageError;
use crate::traits::EncryptedStore;

/// In-memory implementation of EncryptedStore.
/// Data is encrypted with AES-256-GCM using the key path as AAD.
pub struct InMemoryStore {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    encryption_key: [u8; 32],
}

impl InMemoryStore {
    /// Create a new in-memory store with a given encryption key.
    pub fn new(encryption_key: [u8; 32]) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            encryption_key,
        }
    }

    /// Create with a random key (for testing).
    pub fn new_random() -> Self {
        let mut key = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut key);
        Self::new(key)
    }

    /// Number of stored entries.
    pub async fn len(&self) -> usize {
        self.data.read().await.len()
    }

    /// Whether the store is empty.
    pub async fn is_empty(&self) -> bool {
        self.data.read().await.is_empty()
    }

    /// Clear all entries.
    pub async fn clear(&self) {
        self.data.write().await.clear();
    }
}

#[async_trait]
impl EncryptedStore for InMemoryStore {
    async fn put(&self, key: &str, plaintext: &[u8]) -> Result<(), StorageError> {
        let blob = encryption::encrypt(plaintext, &self.encryption_key, key.as_bytes())
            .map_err(|e| StorageError::Encryption(e.to_string()))?;
        let serialized =
            serde_json::to_vec(&blob).map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.data.write().await.insert(key.to_string(), serialized);
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let data = self.data.read().await;
        let serialized = data.get(key).ok_or_else(|| StorageError::NotFound {
            key: key.to_string(),
        })?;
        let blob: phantom_crypto::EncryptedBlob = serde_json::from_slice(serialized)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        encryption::decrypt(&blob, &self.encryption_key, key.as_bytes())
            .map_err(|e| StorageError::Decryption(e.to_string()))
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.data.write().await.remove(key);
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let data = self.data.read().await;
        let keys: Vec<String> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        Ok(self.data.read().await.contains_key(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put_get_roundtrip() {
        let store = InMemoryStore::new_random();
        store.put("test/key", b"hello world").await.unwrap();
        let result = store.get("test/key").await.unwrap();
        assert_eq!(result, b"hello world");
    }

    #[tokio::test]
    async fn test_not_found() {
        let store = InMemoryStore::new_random();
        let result = store.get("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete() {
        let store = InMemoryStore::new_random();
        store.put("key1", b"value1").await.unwrap();
        assert!(store.exists("key1").await.unwrap());
        store.delete("key1").await.unwrap();
        assert!(!store.exists("key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_prefix() {
        let store = InMemoryStore::new_random();
        store.put("vault/github/token", b"t1").await.unwrap();
        store.put("vault/github/secret", b"s1").await.unwrap();
        store.put("vault/vercel/token", b"t2").await.unwrap();
        store.put("state/config", b"c1").await.unwrap();

        let mut github = store.list("vault/github/").await.unwrap();
        github.sort();
        assert_eq!(github.len(), 2);

        let vault = store.list("vault/").await.unwrap();
        assert_eq!(vault.len(), 3);
    }

    #[tokio::test]
    async fn test_overwrite() {
        let store = InMemoryStore::new_random();
        store.put("key", b"original").await.unwrap();
        store.put("key", b"updated").await.unwrap();
        let result = store.get("key").await.unwrap();
        assert_eq!(result, b"updated");
    }

    #[tokio::test]
    async fn test_aad_prevents_swap() {
        // Store data under one key, then try to read under a different key path.
        // Since AAD = key path, this should fail.
        let store = InMemoryStore::new_random();
        store.put("vault/github/token", b"secret").await.unwrap();

        // Manually move the encrypted blob to a different key
        let data = store.data.read().await;
        let blob = data.get("vault/github/token").unwrap().clone();
        drop(data);

        store
            .data
            .write()
            .await
            .insert("vault/evil/token".to_string(), blob);

        // Trying to decrypt with the wrong AAD should fail
        let result = store.get("vault/evil/token").await;
        assert!(
            result.is_err(),
            "AAD mismatch should cause decryption failure"
        );
    }

    #[tokio::test]
    async fn test_empty_value() {
        let store = InMemoryStore::new_random();
        store.put("empty", b"").await.unwrap();
        let result = store.get("empty").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_large_value() {
        let store = InMemoryStore::new_random();
        let data = vec![0xABu8; 1_000_000];
        store.put("large", &data).await.unwrap();
        let result = store.get("large").await.unwrap();
        assert_eq!(result, data);
    }
}
