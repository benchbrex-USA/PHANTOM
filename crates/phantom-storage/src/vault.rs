//! Credential vault — encrypted storage for API keys, tokens, secrets.
//!
//! Core Law 3: Zero local disk footprint.
//! Credentials are encrypted with AES-256-GCM before leaving memory.
//! Servers store only opaque encrypted blobs — no plaintext ever touches disk.
//!
//! Vault entries are keyed by (service, key_name) and store:
//!   • Encrypted value (AES-256-GCM ciphertext)
//!   • Creation timestamp
//!   • Rotation timestamp (if rotated)
//!   • TTL (optional expiry)

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use phantom_crypto::aes256gcm::{decrypt, encrypt, EncryptionKey};

use crate::errors::StorageError;

/// A stored credential in the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    /// Service name (e.g. "cloudflare", "supabase", "github")
    pub service: String,
    /// Key name (e.g. "api_token", "db_password", "oauth_secret")
    pub key_name: String,
    /// Encrypted value (AES-256-GCM: nonce || ciphertext || tag)
    pub encrypted_value: Vec<u8>,
    /// Creation timestamp (unix epoch seconds)
    pub created_at: i64,
    /// Last rotation timestamp
    pub rotated_at: Option<i64>,
    /// TTL in seconds (None = no expiry)
    pub ttl_secs: Option<u64>,
}

impl VaultEntry {
    /// Check if this entry has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_secs {
            let base = self.rotated_at.unwrap_or(self.created_at);
            let now = epoch_secs();
            now > base + ttl as i64
        } else {
            false
        }
    }

    /// Composite key for this entry.
    pub fn key(&self) -> String {
        format!("{}/{}", self.service, self.key_name)
    }
}

fn epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// The credential vault manager.
///
/// Stores encrypted credentials in memory, with serialize/deserialize
/// support for persisting to remote storage (R2/S3).
pub struct Vault {
    /// Entries keyed by "service/key_name"
    entries: HashMap<String, VaultEntry>,
    /// Encryption key (derived from master key via HKDF)
    encryption_key: EncryptionKey,
}

impl Vault {
    /// Create a new vault with the given encryption key.
    pub fn new(encryption_key: EncryptionKey) -> Self {
        Self {
            entries: HashMap::new(),
            encryption_key,
        }
    }

    /// Store a credential in the vault (encrypts before storing).
    pub fn store(
        &mut self,
        service: impl Into<String>,
        key_name: impl Into<String>,
        plaintext_value: &[u8],
        ttl_secs: Option<u64>,
    ) -> Result<(), StorageError> {
        let service = service.into();
        let key_name = key_name.into();

        let encrypted = encrypt(&self.encryption_key, plaintext_value)
            .map_err(|e| StorageError::Encryption(e.to_string()))?;

        let entry = VaultEntry {
            service: service.clone(),
            key_name: key_name.clone(),
            encrypted_value: encrypted,
            created_at: epoch_secs(),
            rotated_at: None,
            ttl_secs,
        };

        let key = entry.key();
        debug!(key = %key, "stored vault entry");
        self.entries.insert(key, entry);
        Ok(())
    }

    /// Retrieve and decrypt a credential.
    pub fn retrieve(
        &self,
        service: &str,
        key_name: &str,
    ) -> Result<Vec<u8>, StorageError> {
        let key = format!("{}/{}", service, key_name);
        let entry = self
            .entries
            .get(&key)
            .ok_or_else(|| StorageError::VaultEntryNotFound {
                service: service.into(),
                key_name: key_name.into(),
            })?;

        if entry.is_expired() {
            warn!(key = %key, "vault entry expired");
            return Err(StorageError::VaultEntryNotFound {
                service: service.into(),
                key_name: key_name.into(),
            });
        }

        decrypt(&self.encryption_key, &entry.encrypted_value)
            .map_err(|e| StorageError::Decryption(e.to_string()))
    }

    /// Retrieve and decrypt as a UTF-8 string.
    pub fn retrieve_string(
        &self,
        service: &str,
        key_name: &str,
    ) -> Result<String, StorageError> {
        let bytes = self.retrieve(service, key_name)?;
        String::from_utf8(bytes).map_err(|e| StorageError::Decryption(e.to_string()))
    }

    /// Rotate a credential (re-encrypts with new value, updates timestamp).
    pub fn rotate(
        &mut self,
        service: &str,
        key_name: &str,
        new_plaintext: &[u8],
    ) -> Result<(), StorageError> {
        let key = format!("{}/{}", service, key_name);

        let encrypted = encrypt(&self.encryption_key, new_plaintext)
            .map_err(|e| StorageError::Encryption(e.to_string()))?;

        let entry = self
            .entries
            .get_mut(&key)
            .ok_or_else(|| StorageError::VaultEntryNotFound {
                service: service.into(),
                key_name: key_name.into(),
            })?;

        entry.encrypted_value = encrypted;
        entry.rotated_at = Some(epoch_secs());
        info!(key = %key, "rotated vault entry");
        Ok(())
    }

    /// Delete a credential.
    pub fn delete(&mut self, service: &str, key_name: &str) -> bool {
        let key = format!("{}/{}", service, key_name);
        self.entries.remove(&key).is_some()
    }

    /// Check if a credential exists (and is not expired).
    pub fn exists(&self, service: &str, key_name: &str) -> bool {
        let key = format!("{}/{}", service, key_name);
        self.entries
            .get(&key)
            .map(|e| !e.is_expired())
            .unwrap_or(false)
    }

    /// List all entries for a service.
    pub fn list_service(&self, service: &str) -> Vec<&VaultEntry> {
        self.entries
            .values()
            .filter(|e| e.service == service && !e.is_expired())
            .collect()
    }

    /// List all services in the vault.
    pub fn services(&self) -> Vec<String> {
        let mut services: Vec<String> = self
            .entries
            .values()
            .map(|e| e.service.clone())
            .collect();
        services.sort();
        services.dedup();
        services
    }

    /// Number of entries in the vault.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Export vault as serialized bytes (entries are already encrypted).
    pub fn export(&self) -> Result<Vec<u8>, StorageError> {
        let entries: Vec<&VaultEntry> = self.entries.values().collect();
        serde_json::to_vec(&entries).map_err(StorageError::from)
    }

    /// Import vault entries from serialized bytes.
    pub fn import(&mut self, data: &[u8]) -> Result<usize, StorageError> {
        let entries: Vec<VaultEntry> =
            serde_json::from_slice(data).map_err(StorageError::from)?;
        let count = entries.len();
        for entry in entries {
            let key = entry.key();
            self.entries.insert(key, entry);
        }
        info!(count, "imported vault entries");
        Ok(count)
    }

    /// Remove all expired entries.
    pub fn prune_expired(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, e| !e.is_expired());
        let removed = before - self.entries.len();
        if removed > 0 {
            info!(removed, "pruned expired vault entries");
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> EncryptionKey {
        EncryptionKey::from_bytes([0x42u8; 32])
    }

    #[test]
    fn test_store_and_retrieve() {
        let mut vault = Vault::new(test_key());
        vault
            .store("github", "api_token", b"ghp_secret123", None)
            .unwrap();

        let value = vault.retrieve_string("github", "api_token").unwrap();
        assert_eq!(value, "ghp_secret123");
    }

    #[test]
    fn test_store_and_retrieve_bytes() {
        let mut vault = Vault::new(test_key());
        let binary_data = vec![0u8, 1, 2, 255, 254, 253];
        vault
            .store("test", "binary", &binary_data, None)
            .unwrap();

        let retrieved = vault.retrieve("test", "binary").unwrap();
        assert_eq!(retrieved, binary_data);
    }

    #[test]
    fn test_entry_not_found() {
        let vault = Vault::new(test_key());
        let result = vault.retrieve("missing", "key");
        assert!(result.is_err());
    }

    #[test]
    fn test_rotate_credential() {
        let mut vault = Vault::new(test_key());
        vault
            .store("aws", "access_key", b"old_key", None)
            .unwrap();

        vault.rotate("aws", "access_key", b"new_key").unwrap();

        let value = vault.retrieve_string("aws", "access_key").unwrap();
        assert_eq!(value, "new_key");
    }

    #[test]
    fn test_delete_credential() {
        let mut vault = Vault::new(test_key());
        vault.store("svc", "key", b"value", None).unwrap();
        assert!(vault.exists("svc", "key"));

        assert!(vault.delete("svc", "key"));
        assert!(!vault.exists("svc", "key"));
        assert!(!vault.delete("svc", "key")); // Already deleted
    }

    #[test]
    fn test_list_service() {
        let mut vault = Vault::new(test_key());
        vault
            .store("cloudflare", "api_token", b"tok1", None)
            .unwrap();
        vault
            .store("cloudflare", "account_id", b"acc1", None)
            .unwrap();
        vault
            .store("github", "token", b"gh_tok", None)
            .unwrap();

        let cf_entries = vault.list_service("cloudflare");
        assert_eq!(cf_entries.len(), 2);

        let gh_entries = vault.list_service("github");
        assert_eq!(gh_entries.len(), 1);
    }

    #[test]
    fn test_services() {
        let mut vault = Vault::new(test_key());
        vault.store("b_svc", "key", b"val", None).unwrap();
        vault.store("a_svc", "key", b"val", None).unwrap();

        let services = vault.services();
        assert_eq!(services, vec!["a_svc", "b_svc"]);
    }

    #[test]
    fn test_export_import() {
        let mut vault1 = Vault::new(test_key());
        vault1.store("svc1", "key1", b"val1", None).unwrap();
        vault1.store("svc2", "key2", b"val2", None).unwrap();

        let exported = vault1.export().unwrap();

        let mut vault2 = Vault::new(test_key());
        let count = vault2.import(&exported).unwrap();
        assert_eq!(count, 2);

        let val = vault2.retrieve_string("svc1", "key1").unwrap();
        assert_eq!(val, "val1");
    }

    #[test]
    fn test_entry_expiry() {
        let entry = VaultEntry {
            service: "test".into(),
            key_name: "key".into(),
            encrypted_value: vec![],
            created_at: 0, // Unix epoch — very old
            rotated_at: None,
            ttl_secs: Some(1), // 1 second TTL
        };
        assert!(entry.is_expired());

        let entry_no_ttl = VaultEntry {
            ttl_secs: None,
            ..entry
        };
        assert!(!entry_no_ttl.is_expired());
    }

    #[test]
    fn test_vault_len() {
        let mut vault = Vault::new(test_key());
        assert!(vault.is_empty());

        vault.store("svc", "key", b"val", None).unwrap();
        assert_eq!(vault.len(), 1);
        assert!(!vault.is_empty());
    }

    #[test]
    fn test_wrong_key_fails_decrypt() {
        let mut vault = Vault::new(test_key());
        vault.store("svc", "key", b"secret", None).unwrap();

        // Export and reimport with wrong key
        let exported = vault.export().unwrap();
        let wrong_key = EncryptionKey::from_bytes([0xAAu8; 32]);
        let mut vault2 = Vault::new(wrong_key);
        vault2.import(&exported).unwrap();

        let result = vault2.retrieve("svc", "key");
        assert!(result.is_err());
    }
}
