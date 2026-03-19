//! Peer information and state tracking.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Information about a connected peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer ID (libp2p PeerId as string)
    pub peer_id: String,
    /// Observed addresses
    pub addresses: Vec<String>,
    /// Current connection state
    pub state: PeerState,
    /// When the peer was first seen
    pub first_seen: DateTime<Utc>,
    /// When we last communicated
    pub last_seen: DateTime<Utc>,
    /// Peer's reported agent version
    pub agent_version: Option<String>,
    /// Peer's reported protocol version
    pub protocol_version: Option<String>,
    /// Number of successful syncs with this peer
    pub sync_count: u64,
    /// Number of failed syncs with this peer
    pub sync_failures: u64,
    /// Average sync latency in milliseconds
    pub avg_sync_latency_ms: f64,
    /// Whether this peer is a bootstrap peer
    pub is_bootstrap: bool,
    /// Server bind token (cryptographic ownership proof)
    pub bind_token: Option<String>,
}

impl PeerInfo {
    pub fn new(peer_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            peer_id: peer_id.into(),
            addresses: Vec::new(),
            state: PeerState::Discovered,
            first_seen: now,
            last_seen: now,
            agent_version: None,
            protocol_version: None,
            sync_count: 0,
            sync_failures: 0,
            avg_sync_latency_ms: 0.0,
            is_bootstrap: false,
            bind_token: None,
        }
    }

    /// Update the last-seen timestamp.
    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }

    /// Record a successful sync.
    pub fn record_sync(&mut self, latency_ms: f64) {
        self.sync_count += 1;
        // Running average
        self.avg_sync_latency_ms =
            (self.avg_sync_latency_ms * (self.sync_count - 1) as f64 + latency_ms)
                / self.sync_count as f64;
        self.touch();
    }

    /// Record a failed sync.
    pub fn record_sync_failure(&mut self) {
        self.sync_failures += 1;
        self.touch();
    }

    /// Reliability score (0.0 - 1.0).
    pub fn reliability(&self) -> f64 {
        let total = self.sync_count + self.sync_failures;
        if total == 0 {
            return 1.0; // Assume reliable until proven otherwise
        }
        self.sync_count as f64 / total as f64
    }

    /// How long since we last heard from this peer.
    pub fn silence_duration_secs(&self) -> i64 {
        (Utc::now() - self.last_seen).num_seconds()
    }
}

/// Connection state of a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerState {
    /// Peer discovered but not yet connected
    Discovered,
    /// Connection in progress
    Connecting,
    /// Connected and authenticated
    Connected,
    /// Connection lost, attempting reconnection
    Reconnecting,
    /// Peer intentionally disconnected
    Disconnected,
    /// Peer banned (authentication failure or misbehavior)
    Banned,
}

impl std::fmt::Display for PeerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discovered => write!(f, "discovered"),
            Self::Connecting => write!(f, "connecting"),
            Self::Connected => write!(f, "connected"),
            Self::Reconnecting => write!(f, "reconnecting"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Banned => write!(f, "banned"),
        }
    }
}

/// Manages the peer table — all known peers and their state.
pub struct PeerTable {
    peers: HashMap<String, PeerInfo>,
    max_peers: usize,
}

impl PeerTable {
    pub fn new(max_peers: usize) -> Self {
        Self {
            peers: HashMap::new(),
            max_peers,
        }
    }

    /// Add or update a peer.
    pub fn upsert(&mut self, info: PeerInfo) {
        self.peers.insert(info.peer_id.clone(), info);
    }

    /// Get a peer by ID.
    pub fn get(&self, peer_id: &str) -> Option<&PeerInfo> {
        self.peers.get(peer_id)
    }

    /// Get a mutable peer by ID.
    pub fn get_mut(&mut self, peer_id: &str) -> Option<&mut PeerInfo> {
        self.peers.get_mut(peer_id)
    }

    /// Remove a peer.
    pub fn remove(&mut self, peer_id: &str) -> Option<PeerInfo> {
        self.peers.remove(peer_id)
    }

    /// Get all connected peers.
    pub fn connected(&self) -> Vec<&PeerInfo> {
        self.peers
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .collect()
    }

    /// Get all peers.
    pub fn all(&self) -> impl Iterator<Item = &PeerInfo> {
        self.peers.values()
    }

    /// Number of connected peers.
    pub fn connected_count(&self) -> usize {
        self.peers
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .count()
    }

    /// Check if we can accept more connections.
    pub fn can_accept(&self) -> bool {
        self.connected_count() < self.max_peers
    }

    /// Get peers sorted by reliability (most reliable first).
    pub fn by_reliability(&self) -> Vec<&PeerInfo> {
        let mut peers: Vec<&PeerInfo> = self.connected().into_iter().collect();
        peers.sort_by(|a, b| {
            b.reliability()
                .partial_cmp(&a.reliability())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        peers
    }

    /// Prune stale peers (disconnected for too long).
    pub fn prune_stale(&mut self, max_silence_secs: i64) -> Vec<String> {
        let stale: Vec<String> = self
            .peers
            .values()
            .filter(|p| {
                p.state == PeerState::Disconnected
                    && p.silence_duration_secs() > max_silence_secs
            })
            .map(|p| p.peer_id.clone())
            .collect();

        for id in &stale {
            self.peers.remove(id);
        }

        stale
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_info_creation() {
        let peer = PeerInfo::new("12D3KooW...");
        assert_eq!(peer.state, PeerState::Discovered);
        assert_eq!(peer.sync_count, 0);
        assert_eq!(peer.reliability(), 1.0);
    }

    #[test]
    fn test_peer_reliability() {
        let mut peer = PeerInfo::new("test-peer");
        peer.record_sync(10.0);
        peer.record_sync(20.0);
        peer.record_sync_failure();

        assert_eq!(peer.sync_count, 2);
        assert_eq!(peer.sync_failures, 1);
        // 2 / (2 + 1) = 0.666...
        assert!((peer.reliability() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_peer_avg_latency() {
        let mut peer = PeerInfo::new("test-peer");
        peer.record_sync(10.0);
        peer.record_sync(20.0);
        peer.record_sync(30.0);

        assert!((peer.avg_sync_latency_ms - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_peer_table_operations() {
        let mut table = PeerTable::new(10);

        let mut p1 = PeerInfo::new("peer-1");
        p1.state = PeerState::Connected;
        table.upsert(p1);

        let mut p2 = PeerInfo::new("peer-2");
        p2.state = PeerState::Connected;
        table.upsert(p2);

        table.upsert(PeerInfo::new("peer-3")); // Discovered, not connected

        assert_eq!(table.len(), 3);
        assert_eq!(table.connected_count(), 2);
        assert!(table.can_accept());
    }

    #[test]
    fn test_peer_table_max_peers() {
        let mut table = PeerTable::new(2);

        let mut p1 = PeerInfo::new("p1");
        p1.state = PeerState::Connected;
        table.upsert(p1);

        let mut p2 = PeerInfo::new("p2");
        p2.state = PeerState::Connected;
        table.upsert(p2);

        assert!(!table.can_accept());
    }

    #[test]
    fn test_peer_table_by_reliability() {
        let mut table = PeerTable::new(10);

        let mut reliable = PeerInfo::new("reliable");
        reliable.state = PeerState::Connected;
        reliable.sync_count = 100;
        reliable.sync_failures = 1;
        table.upsert(reliable);

        let mut unreliable = PeerInfo::new("unreliable");
        unreliable.state = PeerState::Connected;
        unreliable.sync_count = 10;
        unreliable.sync_failures = 10;
        table.upsert(unreliable);

        let sorted = table.by_reliability();
        assert_eq!(sorted[0].peer_id, "reliable");
        assert_eq!(sorted[1].peer_id, "unreliable");
    }

    #[test]
    fn test_remove_peer() {
        let mut table = PeerTable::new(10);
        table.upsert(PeerInfo::new("p1"));
        assert_eq!(table.len(), 1);

        table.remove("p1");
        assert_eq!(table.len(), 0);
    }
}
