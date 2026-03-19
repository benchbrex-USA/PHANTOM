//! Cloudflare R2 (S3-compatible) encrypted blob storage client.
//!
//! Core Law 3: Zero local disk footprint.
//! All data is encrypted with AES-256-GCM before upload.
//! R2 only ever sees opaque ciphertext — zero-knowledge remote storage.
//!
//! Uses the aws-sdk-s3 client with R2-compatible endpoint configuration.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::errors::StorageError;

/// R2/S3 bucket configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Config {
    /// R2 endpoint URL (e.g. https://<account_id>.r2.cloudflarestorage.com)
    pub endpoint_url: String,
    /// Bucket name
    pub bucket: String,
    /// Access key ID
    pub access_key_id: String,
    /// Secret access key
    pub secret_access_key: String,
    /// Region (R2 uses "auto")
    pub region: String,
    /// Key prefix for namespacing (e.g. "phantom/v2/")
    pub key_prefix: String,
}

impl Default for R2Config {
    fn default() -> Self {
        Self {
            endpoint_url: String::new(),
            bucket: "phantom-storage".into(),
            access_key_id: String::new(),
            secret_access_key: String::new(),
            region: "auto".into(),
            key_prefix: "phantom/v2/".into(),
        }
    }
}

impl R2Config {
    /// Build the full key with prefix.
    pub fn full_key(&self, key: &str) -> String {
        format!("{}{}", self.key_prefix, key)
    }

    /// Validate that required fields are set.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.endpoint_url.is_empty() {
            return Err(StorageError::BucketNotConfigured(
                "endpoint_url is empty".into(),
            ));
        }
        if self.bucket.is_empty() {
            return Err(StorageError::BucketNotConfigured(
                "bucket name is empty".into(),
            ));
        }
        if self.access_key_id.is_empty() || self.secret_access_key.is_empty() {
            return Err(StorageError::BucketNotConfigured(
                "credentials not set".into(),
            ));
        }
        Ok(())
    }
}

/// Metadata for a stored blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// Storage key
    pub key: String,
    /// Size in bytes (of encrypted blob)
    pub size_bytes: u64,
    /// Content type
    pub content_type: String,
    /// Upload timestamp (epoch seconds)
    pub uploaded_at: i64,
    /// SHA-256 hash of plaintext (for integrity verification)
    pub plaintext_hash: Option<String>,
    /// Whether the blob is encrypted
    pub encrypted: bool,
}

/// Tracks blobs that have been stored (in-memory index).
/// The actual blobs live in R2; this is the local catalog.
pub struct BlobIndex {
    blobs: std::collections::HashMap<String, BlobMetadata>,
}

impl BlobIndex {
    pub fn new() -> Self {
        Self {
            blobs: std::collections::HashMap::new(),
        }
    }

    /// Register a blob in the index.
    pub fn register(&mut self, metadata: BlobMetadata) {
        debug!(key = %metadata.key, size = metadata.size_bytes, "indexed blob");
        self.blobs.insert(metadata.key.clone(), metadata);
    }

    /// Look up a blob by key.
    pub fn get(&self, key: &str) -> Option<&BlobMetadata> {
        self.blobs.get(key)
    }

    /// Remove a blob from the index.
    pub fn remove(&mut self, key: &str) -> Option<BlobMetadata> {
        self.blobs.remove(key)
    }

    /// Check if a blob exists.
    pub fn exists(&self, key: &str) -> bool {
        self.blobs.contains_key(key)
    }

    /// List all blob keys.
    pub fn keys(&self) -> Vec<&str> {
        self.blobs.keys().map(|k| k.as_str()).collect()
    }

    /// List blobs with a given prefix.
    pub fn list_prefix(&self, prefix: &str) -> Vec<&BlobMetadata> {
        self.blobs
            .values()
            .filter(|b| b.key.starts_with(prefix))
            .collect()
    }

    /// Total size of all indexed blobs.
    pub fn total_size_bytes(&self) -> u64 {
        self.blobs.values().map(|b| b.size_bytes).sum()
    }

    /// Number of indexed blobs.
    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }

    /// Export the index as JSON.
    pub fn export(&self) -> Result<Vec<u8>, StorageError> {
        let entries: Vec<&BlobMetadata> = self.blobs.values().collect();
        serde_json::to_vec(&entries).map_err(StorageError::from)
    }

    /// Import index from JSON.
    pub fn import(&mut self, data: &[u8]) -> Result<usize, StorageError> {
        let entries: Vec<BlobMetadata> =
            serde_json::from_slice(data).map_err(StorageError::from)?;
        let count = entries.len();
        for entry in entries {
            self.blobs.insert(entry.key.clone(), entry);
        }
        info!(count, "imported blob index");
        Ok(count)
    }
}

/// R2 storage client for zero-footprint encrypted blob storage.
///
/// This client manages the local blob index and configuration.
/// Actual S3-compatible API calls go through aws-sdk-s3 at runtime.
pub struct R2Client {
    /// Configuration
    config: R2Config,
    /// Local blob index
    index: BlobIndex,
}

impl R2Client {
    /// Create a new R2 client with the given configuration.
    pub fn new(config: R2Config) -> Self {
        Self {
            config,
            index: BlobIndex::new(),
        }
    }

    /// Create from environment variables.
    pub fn from_env() -> Result<Self, StorageError> {
        let config = R2Config {
            endpoint_url: std::env::var("R2_ENDPOINT_URL")
                .unwrap_or_default(),
            bucket: std::env::var("R2_BUCKET")
                .unwrap_or_else(|_| "phantom-storage".into()),
            access_key_id: std::env::var("R2_ACCESS_KEY_ID")
                .unwrap_or_default(),
            secret_access_key: std::env::var("R2_SECRET_ACCESS_KEY")
                .unwrap_or_default(),
            region: "auto".into(),
            key_prefix: std::env::var("R2_KEY_PREFIX")
                .unwrap_or_else(|_| "phantom/v2/".into()),
        };
        Ok(Self::new(config))
    }

    /// Get the configuration.
    pub fn config(&self) -> &R2Config {
        &self.config
    }

    /// Get a mutable reference to the blob index.
    pub fn index_mut(&mut self) -> &mut BlobIndex {
        &mut self.index
    }

    /// Get the blob index.
    pub fn index(&self) -> &BlobIndex {
        &self.index
    }

    /// Build the full storage key for a blob.
    pub fn storage_key(&self, key: &str) -> String {
        self.config.full_key(key)
    }

    /// Register a blob after upload.
    pub fn register_upload(
        &mut self,
        key: impl Into<String>,
        size_bytes: u64,
        encrypted: bool,
    ) {
        let key = key.into();
        let metadata = BlobMetadata {
            key: key.clone(),
            size_bytes,
            content_type: "application/octet-stream".into(),
            uploaded_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            plaintext_hash: None,
            encrypted,
        };
        self.index.register(metadata);
    }

    /// Check if the client is configured (has credentials).
    pub fn is_configured(&self) -> bool {
        self.config.validate().is_ok()
    }

    /// Get storage usage summary.
    pub fn usage_summary(&self) -> StorageUsage {
        StorageUsage {
            bucket: self.config.bucket.clone(),
            blob_count: self.index.len(),
            total_size_bytes: self.index.total_size_bytes(),
            encrypted_count: self
                .index
                .blobs
                .values()
                .filter(|b| b.encrypted)
                .count(),
        }
    }
}

/// Storage usage summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsage {
    pub bucket: String,
    pub blob_count: usize,
    pub total_size_bytes: u64,
    pub encrypted_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r2_config_default() {
        let config = R2Config::default();
        assert_eq!(config.bucket, "phantom-storage");
        assert_eq!(config.region, "auto");
        assert_eq!(config.key_prefix, "phantom/v2/");
    }

    #[test]
    fn test_r2_config_full_key() {
        let config = R2Config::default();
        assert_eq!(config.full_key("state.json"), "phantom/v2/state.json");
    }

    #[test]
    fn test_r2_config_validate() {
        let config = R2Config::default();
        assert!(config.validate().is_err()); // Missing endpoint

        let valid = R2Config {
            endpoint_url: "https://example.r2.cloudflarestorage.com".into(),
            access_key_id: "key".into(),
            secret_access_key: "secret".into(),
            ..R2Config::default()
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_blob_index_operations() {
        let mut index = BlobIndex::new();
        assert!(index.is_empty());

        index.register(BlobMetadata {
            key: "test/file1.bin".into(),
            size_bytes: 1024,
            content_type: "application/octet-stream".into(),
            uploaded_at: 0,
            plaintext_hash: None,
            encrypted: true,
        });

        assert_eq!(index.len(), 1);
        assert!(index.exists("test/file1.bin"));
        assert!(!index.exists("missing"));

        let meta = index.get("test/file1.bin").unwrap();
        assert_eq!(meta.size_bytes, 1024);
        assert!(meta.encrypted);
    }

    #[test]
    fn test_blob_index_prefix_list() {
        let mut index = BlobIndex::new();
        for i in 0..5 {
            index.register(BlobMetadata {
                key: format!("state/chunk_{}.bin", i),
                size_bytes: 100,
                content_type: "application/octet-stream".into(),
                uploaded_at: 0,
                plaintext_hash: None,
                encrypted: true,
            });
        }
        index.register(BlobMetadata {
            key: "vault/secrets.bin".into(),
            size_bytes: 50,
            content_type: "application/octet-stream".into(),
            uploaded_at: 0,
            plaintext_hash: None,
            encrypted: true,
        });

        let state_blobs = index.list_prefix("state/");
        assert_eq!(state_blobs.len(), 5);

        let vault_blobs = index.list_prefix("vault/");
        assert_eq!(vault_blobs.len(), 1);
    }

    #[test]
    fn test_blob_index_total_size() {
        let mut index = BlobIndex::new();
        for i in 0..3 {
            index.register(BlobMetadata {
                key: format!("file_{}", i),
                size_bytes: 1000,
                content_type: "application/octet-stream".into(),
                uploaded_at: 0,
                plaintext_hash: None,
                encrypted: true,
            });
        }
        assert_eq!(index.total_size_bytes(), 3000);
    }

    #[test]
    fn test_blob_index_export_import() {
        let mut index1 = BlobIndex::new();
        index1.register(BlobMetadata {
            key: "key1".into(),
            size_bytes: 100,
            content_type: "application/octet-stream".into(),
            uploaded_at: 12345,
            plaintext_hash: None,
            encrypted: true,
        });

        let exported = index1.export().unwrap();
        let mut index2 = BlobIndex::new();
        let count = index2.import(&exported).unwrap();
        assert_eq!(count, 1);
        assert!(index2.exists("key1"));
    }

    #[test]
    fn test_blob_index_remove() {
        let mut index = BlobIndex::new();
        index.register(BlobMetadata {
            key: "to_delete".into(),
            size_bytes: 50,
            content_type: "application/octet-stream".into(),
            uploaded_at: 0,
            plaintext_hash: None,
            encrypted: false,
        });

        assert!(index.exists("to_delete"));
        let removed = index.remove("to_delete");
        assert!(removed.is_some());
        assert!(!index.exists("to_delete"));
    }

    #[test]
    fn test_r2_client_creation() {
        let config = R2Config::default();
        let client = R2Client::new(config);
        assert!(!client.is_configured()); // No credentials
        assert_eq!(client.index().len(), 0);
    }

    #[test]
    fn test_r2_client_register_upload() {
        let mut client = R2Client::new(R2Config::default());
        client.register_upload("state.bin", 2048, true);

        assert_eq!(client.index().len(), 1);
        let usage = client.usage_summary();
        assert_eq!(usage.blob_count, 1);
        assert_eq!(usage.total_size_bytes, 2048);
        assert_eq!(usage.encrypted_count, 1);
    }

    #[test]
    fn test_r2_client_storage_key() {
        let client = R2Client::new(R2Config::default());
        assert_eq!(client.storage_key("vault.bin"), "phantom/v2/vault.bin");
    }
}
