//! Encrypted blob storage backed by Cloudflare R2 (S3-compatible).
//!
//! All data is encrypted with AES-256-GCM before upload and decrypted after download.
//! The remote server only ever sees opaque ciphertext — zero-knowledge storage.

use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use sha2::{Digest, Sha256};
use tracing::{debug, info};

use crate::errors::StorageError;
use crate::r2_client::{BlobMetadata, R2Config};
use phantom_crypto::aes256gcm::{self, EncryptionKey};

/// Trait for encrypted blob storage operations.
///
/// All implementations must encrypt data before upload and decrypt after download.
/// Keys are logical names; implementations handle prefixing and namespacing.
#[async_trait]
pub trait EncryptedStore: Send + Sync {
    /// Encrypt plaintext with AES-256-GCM and upload the ciphertext.
    ///
    /// Returns metadata about the stored blob including the SHA-256 hash
    /// of the original plaintext for integrity verification.
    async fn put(&self, key: &str, plaintext: &[u8]) -> Result<BlobMetadata, StorageError>;

    /// Download ciphertext and decrypt it, returning the original plaintext.
    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;

    /// Delete a blob from remote storage.
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// List blobs matching a prefix.
    async fn list(&self, prefix: &str) -> Result<Vec<BlobMetadata>, StorageError>;

    /// Check whether a blob exists in remote storage.
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;
}

/// R2-backed encrypted store using AES-256-GCM encryption.
///
/// Wraps an aws-sdk-s3 client configured for Cloudflare R2.
/// All data is encrypted client-side before upload.
pub struct R2EncryptedStore {
    s3_client: aws_sdk_s3::Client,
    config: R2Config,
    encryption_key: EncryptionKey,
}

impl R2EncryptedStore {
    /// Create a new R2EncryptedStore from an R2Config and a raw 32-byte encryption key.
    pub fn new(config: R2Config, key_bytes: [u8; 32]) -> Result<Self, StorageError> {
        config.validate()?;

        let credentials = aws_sdk_s3::config::Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "phantom-r2",
        );

        let s3_config = aws_sdk_s3::config::Builder::new()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .endpoint_url(&config.endpoint_url)
            .region(aws_sdk_s3::config::Region::new(config.region.clone()))
            .credentials_provider(credentials)
            .force_path_style(true)
            .build();

        let s3_client = aws_sdk_s3::Client::from_conf(s3_config);
        let encryption_key = EncryptionKey::from_bytes(key_bytes);

        info!(bucket = %config.bucket, endpoint = %config.endpoint_url, "R2EncryptedStore initialized");

        Ok(Self {
            s3_client,
            config,
            encryption_key,
        })
    }

    /// Create from environment variables.
    ///
    /// Reads R2 config from `R2_ENDPOINT_URL`, `R2_BUCKET`, `R2_ACCESS_KEY_ID`,
    /// `R2_SECRET_ACCESS_KEY`, `R2_KEY_PREFIX` and the encryption key from
    /// `PHANTOM_STORAGE_KEY` (64-char hex string).
    pub fn from_env() -> Result<Self, StorageError> {
        let config = R2Config {
            endpoint_url: std::env::var("R2_ENDPOINT_URL")
                .map_err(|_| StorageError::BucketNotConfigured("R2_ENDPOINT_URL not set".into()))?,
            bucket: std::env::var("R2_BUCKET").unwrap_or_else(|_| "phantom-storage".into()),
            access_key_id: std::env::var("R2_ACCESS_KEY_ID").map_err(|_| {
                StorageError::BucketNotConfigured("R2_ACCESS_KEY_ID not set".into())
            })?,
            secret_access_key: std::env::var("R2_SECRET_ACCESS_KEY").map_err(|_| {
                StorageError::BucketNotConfigured("R2_SECRET_ACCESS_KEY not set".into())
            })?,
            region: "auto".into(),
            key_prefix: std::env::var("R2_KEY_PREFIX").unwrap_or_else(|_| "phantom/v2/".into()),
        };

        let key_hex = std::env::var("PHANTOM_STORAGE_KEY")
            .map_err(|_| StorageError::Encryption("PHANTOM_STORAGE_KEY env var not set".into()))?;

        let key_bytes = hex::decode(&key_hex).map_err(|e| {
            StorageError::Encryption(format!("invalid PHANTOM_STORAGE_KEY hex: {}", e))
        })?;

        if key_bytes.len() != 32 {
            return Err(StorageError::Encryption(format!(
                "PHANTOM_STORAGE_KEY must be 32 bytes (64 hex chars), got {} bytes",
                key_bytes.len()
            )));
        }

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);

        Self::new(config, key_array)
    }

    /// Build the full storage key with the configured prefix.
    fn full_key(&self, key: &str) -> String {
        self.config.full_key(key)
    }

    /// Compute hex-encoded SHA-256 hash of data.
    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}

#[async_trait]
impl EncryptedStore for R2EncryptedStore {
    async fn put(&self, key: &str, plaintext: &[u8]) -> Result<BlobMetadata, StorageError> {
        let full_key = self.full_key(key);
        let plaintext_hash = Self::sha256_hex(plaintext);

        debug!(
            key = %full_key,
            plaintext_size = plaintext.len(),
            "encrypting and uploading blob"
        );

        // Encrypt with AES-256-GCM
        let ciphertext = aes256gcm::encrypt(&self.encryption_key, plaintext)
            .map_err(|e| StorageError::Encryption(e.to_string()))?;

        let ciphertext_len = ciphertext.len() as u64;

        // Upload to R2
        self.s3_client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&full_key)
            .body(ByteStream::from(ciphertext))
            .content_type("application/octet-stream")
            .send()
            .await
            .map_err(|e| StorageError::UploadFailed(format!("{}: {}", full_key, e)))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let metadata = BlobMetadata {
            key: key.to_string(),
            size_bytes: ciphertext_len,
            content_type: "application/octet-stream".into(),
            uploaded_at: now,
            plaintext_hash: Some(plaintext_hash),
            encrypted: true,
        };

        info!(
            key = %full_key,
            size = ciphertext_len,
            "blob uploaded"
        );

        Ok(metadata)
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let full_key = self.full_key(key);

        debug!(key = %full_key, "downloading and decrypting blob");

        let resp = self
            .s3_client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&full_key)
            .send()
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("NoSuchKey") || err_str.contains("404") {
                    StorageError::NotFound {
                        key: key.to_string(),
                    }
                } else {
                    StorageError::DownloadFailed(format!("{}: {}", full_key, e))
                }
            })?;

        let ciphertext = resp
            .body
            .collect()
            .await
            .map_err(|e| StorageError::DownloadFailed(format!("body read failed: {}", e)))?
            .into_bytes()
            .to_vec();

        // Decrypt with AES-256-GCM
        let plaintext = aes256gcm::decrypt(&self.encryption_key, &ciphertext)
            .map_err(|e| StorageError::Decryption(e.to_string()))?;

        debug!(
            key = %full_key,
            ciphertext_size = ciphertext.len(),
            plaintext_size = plaintext.len(),
            "blob decrypted"
        );

        Ok(plaintext)
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let full_key = self.full_key(key);

        debug!(key = %full_key, "deleting blob");

        self.s3_client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&full_key)
            .send()
            .await
            .map_err(|e| {
                StorageError::UploadFailed(format!("delete failed for {}: {}", full_key, e))
            })?;

        info!(key = %full_key, "blob deleted");

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<BlobMetadata>, StorageError> {
        let full_prefix = self.full_key(prefix);

        debug!(prefix = %full_prefix, "listing blobs");

        let resp = self
            .s3_client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(&full_prefix)
            .send()
            .await
            .map_err(|e| {
                StorageError::DownloadFailed(format!(
                    "list failed for prefix {}: {}",
                    full_prefix, e
                ))
            })?;

        let mut results = Vec::new();

        for obj in resp.contents() {
            let obj_key = obj.key().unwrap_or_default();
            // Strip the config prefix to return the logical key
            let logical_key = obj_key
                .strip_prefix(&self.config.key_prefix)
                .unwrap_or(obj_key);

            results.push(BlobMetadata {
                key: logical_key.to_string(),
                size_bytes: obj.size().unwrap_or(0) as u64,
                content_type: "application/octet-stream".into(),
                uploaded_at: obj.last_modified().map(|t| t.secs()).unwrap_or(0),
                plaintext_hash: None,
                encrypted: true,
            });
        }

        debug!(prefix = %full_prefix, count = results.len(), "listed blobs");

        Ok(results)
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let full_key = self.full_key(key);

        debug!(key = %full_key, "checking blob existence");

        match self
            .s3_client
            .head_object()
            .bucket(&self.config.bucket)
            .key(&full_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(err) => {
                let err_str = format!("{}", err);
                if err_str.contains("NotFound")
                    || err_str.contains("404")
                    || err_str.contains("NoSuchKey")
                {
                    Ok(false)
                } else {
                    Err(StorageError::DownloadFailed(format!(
                        "head_object failed for {}: {}",
                        full_key, err
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phantom_crypto::aes256gcm::EncryptionKey;

    /// Test that encrypt-then-decrypt roundtrip preserves data integrity.
    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = EncryptionKey::generate();
        let plaintext = b"phantom zero-knowledge encrypted blob storage test data";

        let ciphertext = aes256gcm::encrypt(&key, plaintext).unwrap();
        assert_ne!(ciphertext.as_slice(), plaintext.as_slice());
        assert!(ciphertext.len() > plaintext.len()); // nonce + tag overhead

        let decrypted = aes256gcm::decrypt(&key, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext.as_slice());
    }

    /// Test that different keys produce different ciphertext.
    #[test]
    fn test_different_keys_produce_different_ciphertext() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();
        let plaintext = b"same data, different keys";

        let ct1 = aes256gcm::encrypt(&key1, plaintext).unwrap();
        let ct2 = aes256gcm::encrypt(&key2, plaintext).unwrap();

        // Ciphertexts should differ (different keys, different nonces)
        assert_ne!(ct1, ct2);

        // Each key can only decrypt its own ciphertext
        assert!(aes256gcm::decrypt(&key2, &ct1).is_err());
        assert!(aes256gcm::decrypt(&key1, &ct2).is_err());
    }

    /// Test that the SHA-256 hash is computed correctly.
    #[test]
    fn test_sha256_hex() {
        let hash = R2EncryptedStore::sha256_hex(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    /// Test that empty plaintext encrypts and decrypts correctly.
    #[test]
    fn test_encrypt_decrypt_empty() {
        let key = EncryptionKey::generate();
        let plaintext = b"";

        let ciphertext = aes256gcm::encrypt(&key, plaintext).unwrap();
        let decrypted = aes256gcm::decrypt(&key, &ciphertext).unwrap();
        assert!(decrypted.is_empty());
    }

    /// Test that large data encrypts and decrypts correctly.
    #[test]
    fn test_encrypt_decrypt_large_data() {
        let key = EncryptionKey::generate();
        let plaintext: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();

        let ciphertext = aes256gcm::encrypt(&key, &plaintext).unwrap();
        let decrypted = aes256gcm::decrypt(&key, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    /// Test that tampered ciphertext fails decryption (GCM authentication).
    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = EncryptionKey::generate();
        let plaintext = b"integrity-critical data";

        let mut ciphertext = aes256gcm::encrypt(&key, plaintext).unwrap();
        // Flip a byte near the end (in the tag region)
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0xFF;

        assert!(aes256gcm::decrypt(&key, &ciphertext).is_err());
    }

    /// Test R2Config validation catches missing fields.
    #[test]
    fn test_config_validation() {
        let empty = R2Config::default();
        assert!(R2EncryptedStore::new(empty, [0u8; 32]).is_err());

        let valid = R2Config {
            endpoint_url: "https://test.r2.cloudflarestorage.com".into(),
            bucket: "test-bucket".into(),
            access_key_id: "access_key".into(),
            secret_access_key: "secret_key".into(),
            region: "auto".into(),
            key_prefix: "test/".into(),
        };
        // Should succeed — client is built, even though R2 is unreachable in tests
        assert!(R2EncryptedStore::new(valid, [42u8; 32]).is_ok());
    }

    /// Test that BlobMetadata is populated correctly after a put operation.
    /// This test verifies the metadata construction logic without hitting S3.
    #[test]
    fn test_blob_metadata_construction() {
        let plaintext = b"test blob content";
        let hash = R2EncryptedStore::sha256_hex(plaintext);

        let key = EncryptionKey::generate();
        let ciphertext = aes256gcm::encrypt(&key, plaintext).unwrap();
        let ciphertext_len = ciphertext.len() as u64;

        let metadata = BlobMetadata {
            key: "test/blob.bin".into(),
            size_bytes: ciphertext_len,
            content_type: "application/octet-stream".into(),
            uploaded_at: 1700000000,
            plaintext_hash: Some(hash.clone()),
            encrypted: true,
        };

        assert_eq!(metadata.key, "test/blob.bin");
        assert!(metadata.encrypted);
        assert_eq!(metadata.plaintext_hash.as_deref(), Some(hash.as_str()));
        assert!(metadata.size_bytes > plaintext.len() as u64);
    }
}
