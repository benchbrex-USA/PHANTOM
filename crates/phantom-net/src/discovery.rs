//! Peer discovery via Kademlia DHT + mDNS (local network).
//!
//! Two discovery backends run in parallel:
//!   • mDNS — zero-config LAN discovery (same subnet)
//!   • Kademlia DHT — wide-area discovery via bootstrap peers
//!
//! Discovered peers are fed into the PeerTable for connection management.

use std::collections::HashSet;

use tracing::info;

use crate::peer::{PeerState, PeerTable};

/// Tracks discovered (but not yet connected) peers from all discovery sources.
pub struct DiscoveryTracker {
    /// Peers discovered via mDNS
    mdns_peers: HashSet<String>,
    /// Peers discovered via Kademlia
    kademlia_peers: HashSet<String>,
    /// Bootstrap peer IDs (always attempt reconnect)
    bootstrap_peers: HashSet<String>,
}

impl DiscoveryTracker {
    pub fn new() -> Self {
        Self {
            mdns_peers: HashSet::new(),
            kademlia_peers: HashSet::new(),
            bootstrap_peers: HashSet::new(),
        }
    }

    /// Register a bootstrap peer by its peer ID.
    pub fn add_bootstrap(&mut self, peer_id: impl Into<String>) {
        self.bootstrap_peers.insert(peer_id.into());
    }

    /// Record a peer discovered via mDNS.
    pub fn discovered_mdns(&mut self, peer_id: impl Into<String>) {
        let id = peer_id.into();
        if self.mdns_peers.insert(id.clone()) {
            info!(peer_id = %id, "mDNS: new peer discovered");
        }
    }

    /// Record a peer discovered via Kademlia.
    pub fn discovered_kademlia(&mut self, peer_id: impl Into<String>) {
        let id = peer_id.into();
        if self.kademlia_peers.insert(id.clone()) {
            info!(peer_id = %id, "Kademlia: new peer discovered");
        }
    }

    /// Remove a peer from all discovery sets (e.g. on ban).
    pub fn remove(&mut self, peer_id: &str) {
        self.mdns_peers.remove(peer_id);
        self.kademlia_peers.remove(peer_id);
        // Don't remove from bootstrap — we always want to know about them
    }

    /// All unique discovered peer IDs.
    pub fn all_discovered(&self) -> HashSet<&str> {
        self.mdns_peers
            .iter()
            .chain(self.kademlia_peers.iter())
            .map(|s| s.as_str())
            .collect()
    }

    /// Peers we know about but haven't connected to yet.
    pub fn unconnected(&self, table: &PeerTable) -> Vec<String> {
        self.all_discovered()
            .into_iter()
            .filter(|id| {
                table
                    .get(id)
                    .map(|p| p.state != PeerState::Connected && p.state != PeerState::Banned)
                    .unwrap_or(true) // Not in table at all → unconnected
            })
            .map(|s| s.to_string())
            .collect()
    }

    /// Check if a peer was discovered via any source.
    pub fn is_known(&self, peer_id: &str) -> bool {
        self.mdns_peers.contains(peer_id) || self.kademlia_peers.contains(peer_id)
    }

    /// Check if a peer is a bootstrap peer.
    pub fn is_bootstrap(&self, peer_id: &str) -> bool {
        self.bootstrap_peers.contains(peer_id)
    }

    /// Number of unique discovered peers.
    pub fn discovered_count(&self) -> usize {
        self.all_discovered().len()
    }

    /// Peer counts by source.
    pub fn stats(&self) -> DiscoveryStats {
        DiscoveryStats {
            mdns_count: self.mdns_peers.len(),
            kademlia_count: self.kademlia_peers.len(),
            bootstrap_count: self.bootstrap_peers.len(),
            total_unique: self.discovered_count(),
        }
    }
}

/// Discovery statistics.
#[derive(Debug, Clone)]
pub struct DiscoveryStats {
    pub mdns_count: usize,
    pub kademlia_count: usize,
    pub bootstrap_count: usize,
    pub total_unique: usize,
}

/// Parse a multiaddr string into a PeerId (extracts /p2p/<peer_id> component).
pub fn peer_id_from_multiaddr(addr: &str) -> Option<String> {
    // Multiaddr format: /ip4/x.x.x.x/udp/port/quic-v1/p2p/<peer_id>
    addr.split("/p2p/").nth(1).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer::PeerInfo;

    #[test]
    fn test_discovery_tracker_mdns() {
        let mut tracker = DiscoveryTracker::new();
        tracker.discovered_mdns("peer-1");
        tracker.discovered_mdns("peer-2");
        tracker.discovered_mdns("peer-1"); // duplicate

        assert_eq!(tracker.discovered_count(), 2);
        assert!(tracker.is_known("peer-1"));
        assert!(tracker.is_known("peer-2"));
        assert!(!tracker.is_known("peer-3"));
    }

    #[test]
    fn test_discovery_tracker_kademlia() {
        let mut tracker = DiscoveryTracker::new();
        tracker.discovered_kademlia("peer-a");
        tracker.discovered_kademlia("peer-b");

        assert_eq!(tracker.stats().kademlia_count, 2);
        assert_eq!(tracker.stats().mdns_count, 0);
    }

    #[test]
    fn test_discovery_dedup_across_sources() {
        let mut tracker = DiscoveryTracker::new();
        tracker.discovered_mdns("peer-1");
        tracker.discovered_kademlia("peer-1");

        // Same peer found by both sources — should count as 1 unique
        assert_eq!(tracker.discovered_count(), 1);
    }

    #[test]
    fn test_unconnected_peers() {
        let mut tracker = DiscoveryTracker::new();
        tracker.discovered_mdns("peer-1");
        tracker.discovered_mdns("peer-2");
        tracker.discovered_mdns("peer-3");

        let mut table = PeerTable::new(10);
        let mut p1 = PeerInfo::new("peer-1");
        p1.state = PeerState::Connected;
        table.upsert(p1);

        let mut p3 = PeerInfo::new("peer-3");
        p3.state = PeerState::Banned;
        table.upsert(p3);

        let unconnected = tracker.unconnected(&table);
        assert_eq!(unconnected.len(), 1);
        assert!(unconnected.contains(&"peer-2".to_string()));
    }

    #[test]
    fn test_bootstrap_peers() {
        let mut tracker = DiscoveryTracker::new();
        tracker.add_bootstrap("boot-1");
        assert!(tracker.is_bootstrap("boot-1"));
        assert!(!tracker.is_bootstrap("peer-1"));
    }

    #[test]
    fn test_remove_peer() {
        let mut tracker = DiscoveryTracker::new();
        tracker.discovered_mdns("peer-1");
        tracker.discovered_kademlia("peer-1");
        assert!(tracker.is_known("peer-1"));

        tracker.remove("peer-1");
        assert!(!tracker.is_known("peer-1"));
    }

    #[test]
    fn test_peer_id_from_multiaddr() {
        let addr = "/ip4/192.168.1.5/udp/4001/quic-v1/p2p/12D3KooWAbcDef";
        assert_eq!(
            peer_id_from_multiaddr(addr),
            Some("12D3KooWAbcDef".to_string())
        );
        assert_eq!(peer_id_from_multiaddr("/ip4/127.0.0.1/tcp/8080"), None);
    }
}
