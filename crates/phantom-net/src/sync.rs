//! CRDT state synchronization using Automerge.
//!
//! Conflict-free replication across the P2P mesh.
//! Each node maintains an Automerge document that is synced via the
//! Automerge sync protocol (generate_sync_message / receive_sync_message).
//!
//! What syncs: project state, task graph, infra bindings, audit log, health metrics.
//! What never syncs: master key, session keys, raw credentials.

use std::collections::HashMap;

use automerge::{sync::SyncDoc, transaction::Transactable, AutoCommit, ReadDoc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::errors::NetError;

/// CRDT-based state synchronization manager.
///
/// Wraps an Automerge document and per-peer sync states.
pub struct CrdtSync {
    /// The local Automerge document.
    doc: AutoCommit,
    /// Per-peer sync state for incremental sync.
    sync_states: HashMap<String, automerge::sync::State>,
    /// Number of completed sync rounds.
    sync_rounds: u64,
}

impl Default for CrdtSync {
    fn default() -> Self {
        Self::new()
    }
}

impl CrdtSync {
    /// Create a new CRDT sync manager with an empty document.
    pub fn new() -> Self {
        Self {
            doc: AutoCommit::new(),
            sync_states: HashMap::new(),
            sync_rounds: 0,
        }
    }

    /// Create from an existing Automerge document (e.g. loaded from storage).
    pub fn from_doc(doc: AutoCommit) -> Self {
        Self {
            doc,
            sync_states: HashMap::new(),
            sync_rounds: 0,
        }
    }

    /// Get a reference to the underlying Automerge document.
    pub fn doc(&self) -> &AutoCommit {
        &self.doc
    }

    /// Get a mutable reference to the document for local changes.
    pub fn doc_mut(&mut self) -> &mut AutoCommit {
        &mut self.doc
    }

    // ── Local state mutations ──────────────────────────────────────────

    /// Put a string value at the given key in the document root.
    pub fn put_str(&mut self, key: &str, value: &str) -> Result<(), NetError> {
        self.doc
            .put(automerge::ROOT, key, value)
            .map_err(|e| NetError::SyncFailed(e.to_string()))?;
        Ok(())
    }

    /// Put a u64 value at the given key in the document root.
    pub fn put_u64(&mut self, key: &str, value: u64) -> Result<(), NetError> {
        self.doc
            .put(automerge::ROOT, key, value as i64)
            .map_err(|e| NetError::SyncFailed(e.to_string()))?;
        Ok(())
    }

    /// Put a boolean value at the given key.
    pub fn put_bool(&mut self, key: &str, value: bool) -> Result<(), NetError> {
        self.doc
            .put(automerge::ROOT, key, value)
            .map_err(|e| NetError::SyncFailed(e.to_string()))?;
        Ok(())
    }

    /// Get a string value from the document root.
    pub fn get_str(&self, key: &str) -> Option<String> {
        self.doc
            .get(automerge::ROOT, key)
            .ok()
            .flatten()
            .and_then(|(val, _)| val.into_string().ok())
    }

    /// Get an i64 value from the document root.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.doc
            .get(automerge::ROOT, key)
            .ok()
            .flatten()
            .and_then(|(val, _)| val.to_i64())
    }

    /// Get a boolean value from the document root.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.doc
            .get(automerge::ROOT, key)
            .ok()
            .flatten()
            .and_then(|(val, _)| val.to_bool())
    }

    // ── Sync protocol ──────────────────────────────────────────────────

    /// Generate a sync message for the given peer.
    /// Returns `None` if the peer is already up to date.
    pub fn generate_sync_message(&mut self, peer_id: &str) -> Option<Vec<u8>> {
        let state = self.sync_states.entry(peer_id.to_string()).or_default();

        self.doc
            .sync()
            .generate_sync_message(state)
            .map(|msg: automerge::sync::Message| msg.encode())
    }

    /// Receive a sync message from a peer, merging their changes.
    pub fn receive_sync_message(&mut self, peer_id: &str, message: &[u8]) -> Result<(), NetError> {
        let state = self.sync_states.entry(peer_id.to_string()).or_default();

        let msg = automerge::sync::Message::decode(message)
            .map_err(|e| NetError::SyncFailed(format!("invalid sync message: {e}")))?;

        self.doc
            .sync()
            .receive_sync_message(state, msg)
            .map_err(|e| NetError::SyncFailed(format!("merge failed: {e}")))?;

        self.sync_rounds += 1;
        debug!(peer_id, rounds = self.sync_rounds, "sync message received");
        Ok(())
    }

    /// Export the full document state as bytes (for snapshot transfer).
    pub fn save(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    /// Load a document from saved bytes.
    pub fn load(bytes: &[u8]) -> Result<Self, NetError> {
        let doc = AutoCommit::load(bytes)
            .map_err(|e| NetError::SyncFailed(format!("load failed: {e}")))?;
        Ok(Self::from_doc(doc))
    }

    /// Merge another document's changes into ours.
    pub fn merge(&mut self, other: &mut AutoCommit) -> Result<(), NetError> {
        self.doc
            .merge(other)
            .map_err(|e| NetError::SyncFailed(format!("merge failed: {e}")))?;
        info!("merged remote document");
        Ok(())
    }

    /// Reset sync state for a peer (e.g. after reconnection).
    pub fn reset_peer_sync(&mut self, peer_id: &str) {
        self.sync_states.remove(peer_id);
        debug!(peer_id, "reset sync state");
    }

    /// Number of peers we have sync state for.
    pub fn tracked_peer_count(&self) -> usize {
        self.sync_states.len()
    }

    /// Total sync rounds completed.
    pub fn total_sync_rounds(&self) -> u64 {
        self.sync_rounds
    }

    /// Get the number of keys at the document root.
    pub fn root_key_count(&self) -> usize {
        self.doc.keys(automerge::ROOT).count()
    }
}

/// Summary of the local CRDT state for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub root_keys: usize,
    pub tracked_peers: usize,
    pub total_sync_rounds: u64,
    pub doc_size_bytes: usize,
}

impl CrdtSync {
    /// Get sync status for diagnostics.
    pub fn status(&mut self) -> SyncStatus {
        let saved = self.save();
        SyncStatus {
            root_keys: self.root_key_count(),
            tracked_peers: self.tracked_peer_count(),
            total_sync_rounds: self.sync_rounds,
            doc_size_bytes: saved.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crdt_put_get_str() {
        let mut sync = CrdtSync::new();
        sync.put_str("project_name", "phantom").unwrap();
        assert_eq!(sync.get_str("project_name"), Some("phantom".to_string()));
    }

    #[test]
    fn test_crdt_put_get_u64() {
        let mut sync = CrdtSync::new();
        sync.put_u64("task_count", 42).unwrap();
        assert_eq!(sync.get_i64("task_count"), Some(42));
    }

    #[test]
    fn test_crdt_put_get_bool() {
        let mut sync = CrdtSync::new();
        sync.put_bool("is_active", true).unwrap();
        assert_eq!(sync.get_bool("is_active"), Some(true));
    }

    #[test]
    fn test_crdt_save_load_roundtrip() {
        let mut sync = CrdtSync::new();
        sync.put_str("key", "value").unwrap();
        sync.put_u64("count", 7).unwrap();

        let saved = sync.save();
        let loaded = CrdtSync::load(&saved).unwrap();
        assert_eq!(loaded.get_str("key"), Some("value".to_string()));
        assert_eq!(loaded.get_i64("count"), Some(7));
    }

    #[test]
    fn test_crdt_merge_two_docs() {
        let mut sync_a = CrdtSync::new();
        sync_a.put_str("from_a", "hello").unwrap();

        let mut sync_b = CrdtSync::new();
        sync_b.put_str("from_b", "world").unwrap();

        // Merge B into A
        sync_a.merge(sync_b.doc_mut()).unwrap();

        assert_eq!(sync_a.get_str("from_a"), Some("hello".to_string()));
        assert_eq!(sync_a.get_str("from_b"), Some("world".to_string()));
    }

    #[test]
    fn test_crdt_sync_protocol() {
        let mut node_a = CrdtSync::new();
        node_a.put_str("data", "from_a").unwrap();

        let mut node_b = CrdtSync::new();

        // Automerge sync requires multiple round-trips.
        // Keep exchanging messages until both sides are in sync.
        for _ in 0..10 {
            if let Some(msg) = node_a.generate_sync_message("node-b") {
                node_b.receive_sync_message("node-a", &msg).unwrap();
            }
            if let Some(msg) = node_b.generate_sync_message("node-a") {
                node_a.receive_sync_message("node-b", &msg).unwrap();
            }
            // Check if sync is complete
            if node_b.get_str("data").is_some() {
                break;
            }
        }

        assert_eq!(node_b.get_str("data"), Some("from_a".to_string()));
        assert!(node_b.total_sync_rounds() >= 1);
    }

    #[test]
    fn test_crdt_reset_peer_sync() {
        let mut sync = CrdtSync::new();
        // Generate a message to create sync state
        let _ = sync.generate_sync_message("peer-1");
        assert_eq!(sync.tracked_peer_count(), 1);

        sync.reset_peer_sync("peer-1");
        assert_eq!(sync.tracked_peer_count(), 0);
    }

    #[test]
    fn test_crdt_status() {
        let mut sync = CrdtSync::new();
        sync.put_str("key", "val").unwrap();
        let status = sync.status();
        assert_eq!(status.root_keys, 1);
        assert!(status.doc_size_bytes > 0);
    }
}
