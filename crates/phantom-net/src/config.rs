//! Mesh network configuration.

use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Configuration for the P2P mesh network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshConfig {
    /// Listen address for QUIC transport
    pub listen_addr: Ipv4Addr,
    /// Listen port (0 = random)
    pub listen_port: u16,
    /// Enable mDNS local peer discovery
    pub mdns_enabled: bool,
    /// Enable Kademlia DHT for wide-area discovery
    pub kademlia_enabled: bool,
    /// Bootstrap peers (multiaddr format)
    pub bootstrap_peers: Vec<String>,
    /// Maximum number of connected peers
    pub max_peers: usize,
    /// Sync interval in seconds (how often to push CRDT state)
    pub sync_interval_secs: u64,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Connection idle timeout in seconds
    pub idle_timeout_secs: u64,
    /// Enable identify protocol (peer info exchange)
    pub identify_enabled: bool,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            listen_addr: Ipv4Addr::UNSPECIFIED,
            listen_port: 0,
            mdns_enabled: true,
            kademlia_enabled: true,
            bootstrap_peers: Vec::new(),
            max_peers: 50,
            sync_interval_secs: 30,
            max_message_size: 16 * 1024 * 1024, // 16MB
            idle_timeout_secs: 300,
            identify_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MeshConfig::default();
        assert!(config.mdns_enabled);
        assert!(config.kademlia_enabled);
        assert_eq!(config.max_peers, 50);
        assert_eq!(config.sync_interval_secs, 30);
    }
}
