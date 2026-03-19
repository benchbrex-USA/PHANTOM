//! Remote state management — project state, task graphs, agent configs.
//!
//! All state is encrypted and can be distributed across P2P mesh + R2 storage.
//! State is versioned to detect conflicts and enable rollback.
//!
//! State categories:
//!   • Project metadata (name, description, created_at)
//!   • Task graph (serialized DAG)
//!   • Agent configurations
//!   • Infrastructure bindings (which resources are provisioned where)
//!   • Audit log (tamper-evident hash chain)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::errors::StorageError;

/// A versioned state entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    /// State key (e.g. "project/metadata", "tasks/graph")
    pub key: String,
    /// Serialized value (JSON bytes)
    pub value: Vec<u8>,
    /// Monotonically increasing version number
    pub version: u64,
    /// Timestamp of last update (epoch seconds)
    pub updated_at: i64,
    /// Hash of the value for integrity checking
    pub value_hash: String,
}

impl StateEntry {
    /// Create a new state entry.
    pub fn new(key: impl Into<String>, value: Vec<u8>) -> Self {
        let hash = sha256_hex(&value);
        Self {
            key: key.into(),
            value,
            version: 1,
            updated_at: epoch_secs(),
            value_hash: hash,
        }
    }

    /// Update the value (increments version).
    pub fn update(&mut self, value: Vec<u8>) {
        self.value_hash = sha256_hex(&value);
        self.value = value;
        self.version += 1;
        self.updated_at = epoch_secs();
    }

    /// Verify the value hash matches.
    pub fn verify_integrity(&self) -> bool {
        sha256_hex(&self.value) == self.value_hash
    }

    /// Parse the value as a typed JSON object.
    pub fn parse<T: serde::de::DeserializeOwned>(&self) -> Result<T, StorageError> {
        serde_json::from_slice(&self.value).map_err(StorageError::from)
    }
}

fn epoch_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(data);
    hex::encode(hash)
}

/// Remote state manager.
///
/// In-memory key-value store with versioning, integrity verification,
/// and serialize/deserialize support for remote persistence.
pub struct RemoteState {
    /// State entries keyed by state key
    entries: HashMap<String, StateEntry>,
}

impl RemoteState {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Set a value (creates or updates).
    pub fn set(&mut self, key: impl Into<String>, value: Vec<u8>) {
        let key = key.into();
        match self.entries.get_mut(&key) {
            Some(entry) => {
                entry.update(value);
                debug!(key = %entry.key, version = entry.version, "state updated");
            }
            None => {
                let entry = StateEntry::new(key.clone(), value);
                debug!(key = %key, "state created");
                self.entries.insert(key, entry);
            }
        }
    }

    /// Set a JSON-serializable value.
    pub fn set_json<T: Serialize>(&mut self, key: impl Into<String>, value: &T) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec(value).map_err(StorageError::from)?;
        self.set(key, bytes);
        Ok(())
    }

    /// Get a raw value.
    pub fn get(&self, key: &str) -> Option<&StateEntry> {
        self.entries.get(key)
    }

    /// Get and parse a JSON value.
    pub fn get_json<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<T, StorageError> {
        let entry = self
            .entries
            .get(key)
            .ok_or_else(|| StorageError::StateKeyNotFound(key.to_string()))?;
        entry.parse()
    }

    /// Get the raw bytes of a value.
    pub fn get_bytes(&self, key: &str) -> Option<&[u8]> {
        self.entries.get(key).map(|e| e.value.as_slice())
    }

    /// Delete a state entry.
    pub fn delete(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// Check if a key exists.
    pub fn exists(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Get the version of a key.
    pub fn version(&self, key: &str) -> Option<u64> {
        self.entries.get(key).map(|e| e.version)
    }

    /// List all keys.
    pub fn keys(&self) -> Vec<&str> {
        self.entries.keys().map(|k| k.as_str()).collect()
    }

    /// List keys with a given prefix.
    pub fn keys_with_prefix(&self, prefix: &str) -> Vec<&str> {
        self.entries
            .keys()
            .filter(|k| k.starts_with(prefix))
            .map(|k| k.as_str())
            .collect()
    }

    /// Verify integrity of all entries.
    pub fn verify_all(&self) -> Vec<String> {
        self.entries
            .values()
            .filter(|e| !e.verify_integrity())
            .map(|e| e.key.clone())
            .collect()
    }

    /// Number of state entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Export all state as JSON bytes.
    pub fn export(&self) -> Result<Vec<u8>, StorageError> {
        let entries: Vec<&StateEntry> = self.entries.values().collect();
        serde_json::to_vec(&entries).map_err(StorageError::from)
    }

    /// Import state from JSON bytes (merges, newer versions win).
    pub fn import(&mut self, data: &[u8]) -> Result<usize, StorageError> {
        let entries: Vec<StateEntry> =
            serde_json::from_slice(data).map_err(StorageError::from)?;
        let mut imported = 0;

        for entry in entries {
            let should_insert = self
                .entries
                .get(&entry.key)
                .map(|existing| entry.version > existing.version)
                .unwrap_or(true);

            if should_insert {
                self.entries.insert(entry.key.clone(), entry);
                imported += 1;
            }
        }

        info!(imported, "imported state entries");
        Ok(imported)
    }

    /// Get a summary of the state store.
    pub fn summary(&self) -> StateSummary {
        let total_size: usize = self.entries.values().map(|e| e.value.len()).sum();
        let corrupted = self.verify_all();

        StateSummary {
            entry_count: self.entries.len(),
            total_size_bytes: total_size,
            corrupted_entries: corrupted.len(),
        }
    }
}

/// Summary of the state store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSummary {
    pub entry_count: usize,
    pub total_size_bytes: usize,
    pub corrupted_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_entry_creation() {
        let entry = StateEntry::new("test/key", b"hello world".to_vec());
        assert_eq!(entry.version, 1);
        assert!(entry.verify_integrity());
    }

    #[test]
    fn test_state_entry_update() {
        let mut entry = StateEntry::new("test/key", b"v1".to_vec());
        assert_eq!(entry.version, 1);

        entry.update(b"v2".to_vec());
        assert_eq!(entry.version, 2);
        assert_eq!(entry.value, b"v2");
        assert!(entry.verify_integrity());
    }

    #[test]
    fn test_state_entry_integrity() {
        let mut entry = StateEntry::new("test/key", b"original".to_vec());
        assert!(entry.verify_integrity());

        // Tamper with the value
        entry.value = b"tampered".to_vec();
        assert!(!entry.verify_integrity());
    }

    #[test]
    fn test_remote_state_set_get() {
        let mut state = RemoteState::new();
        state.set("project/name", b"phantom".to_vec());

        let entry = state.get("project/name").unwrap();
        assert_eq!(entry.value, b"phantom");
        assert_eq!(entry.version, 1);
    }

    #[test]
    fn test_remote_state_json() {
        let mut state = RemoteState::new();

        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Config {
            name: String,
            version: u32,
        }

        let config = Config {
            name: "phantom".into(),
            version: 2,
        };
        state.set_json("project/config", &config).unwrap();

        let loaded: Config = state.get_json("project/config").unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn test_remote_state_versioning() {
        let mut state = RemoteState::new();
        state.set("key", b"v1".to_vec());
        assert_eq!(state.version("key"), Some(1));

        state.set("key", b"v2".to_vec());
        assert_eq!(state.version("key"), Some(2));

        state.set("key", b"v3".to_vec());
        assert_eq!(state.version("key"), Some(3));
    }

    #[test]
    fn test_remote_state_delete() {
        let mut state = RemoteState::new();
        state.set("key", b"val".to_vec());
        assert!(state.exists("key"));

        assert!(state.delete("key"));
        assert!(!state.exists("key"));
    }

    #[test]
    fn test_remote_state_keys_with_prefix() {
        let mut state = RemoteState::new();
        state.set("project/name", b"phantom".to_vec());
        state.set("project/version", b"2".to_vec());
        state.set("tasks/graph", b"{}".to_vec());

        let project_keys = state.keys_with_prefix("project/");
        assert_eq!(project_keys.len(), 2);

        let task_keys = state.keys_with_prefix("tasks/");
        assert_eq!(task_keys.len(), 1);
    }

    #[test]
    fn test_remote_state_export_import() {
        let mut state1 = RemoteState::new();
        state1.set("key1", b"val1".to_vec());
        state1.set("key2", b"val2".to_vec());

        let exported = state1.export().unwrap();

        let mut state2 = RemoteState::new();
        let count = state2.import(&exported).unwrap();
        assert_eq!(count, 2);
        assert!(state2.exists("key1"));
        assert!(state2.exists("key2"));
    }

    #[test]
    fn test_import_version_conflict() {
        let mut state = RemoteState::new();
        state.set("key", b"v1".to_vec());
        state.set("key", b"v2".to_vec()); // version 2

        // Create import data with version 1 — should be skipped
        let mut old_state = RemoteState::new();
        old_state.set("key", b"old".to_vec()); // version 1

        let exported = old_state.export().unwrap();
        let imported = state.import(&exported).unwrap();
        assert_eq!(imported, 0); // Nothing imported (existing is newer)
        assert_eq!(state.get_bytes("key").unwrap(), b"v2"); // Unchanged
    }

    #[test]
    fn test_verify_all() {
        let mut state = RemoteState::new();
        state.set("good", b"ok".to_vec());
        state.set("bad", b"ok".to_vec());

        // Tamper
        state.entries.get_mut("bad").unwrap().value = b"tampered".to_vec();

        let corrupted = state.verify_all();
        assert_eq!(corrupted.len(), 1);
        assert!(corrupted.contains(&"bad".to_string()));
    }

    #[test]
    fn test_state_summary() {
        let mut state = RemoteState::new();
        state.set("a", b"hello".to_vec());
        state.set("b", b"world".to_vec());

        let summary = state.summary();
        assert_eq!(summary.entry_count, 2);
        assert_eq!(summary.total_size_bytes, 10);
        assert_eq!(summary.corrupted_entries, 0);
    }

    #[test]
    fn test_state_not_found() {
        let state = RemoteState::new();
        let result: Result<String, _> = state.get_json("missing");
        assert!(result.is_err());
    }
}
