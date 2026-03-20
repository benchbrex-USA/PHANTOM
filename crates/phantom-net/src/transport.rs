//! QUIC transport layer with Noise protocol security.
//!
//! Wraps libp2p QUIC transport + Noise XX handshake configuration.
//! Ed25519 identity keypair is generated fresh per session (ephemeral node identity).

use libp2p::identity::Keypair;
use tracing::info;

use crate::config::MeshConfig;

/// QUIC transport configuration and identity management.
pub struct QuicTransport {
    /// Node identity keypair (Ed25519)
    keypair: Keypair,
    /// Mesh configuration
    config: MeshConfig,
}

impl QuicTransport {
    /// Create a new transport with a fresh Ed25519 identity.
    pub fn new(config: MeshConfig) -> Self {
        let keypair = Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        info!(%peer_id, "generated ephemeral node identity");
        Self { keypair, config }
    }

    /// Create a transport with an existing keypair.
    pub fn with_keypair(keypair: Keypair, config: MeshConfig) -> Self {
        let peer_id = keypair.public().to_peer_id();
        info!(%peer_id, "using provided node identity");
        Self { keypair, config }
    }

    /// Get a reference to the node keypair.
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Get the local PeerId.
    pub fn peer_id(&self) -> libp2p::PeerId {
        self.keypair.public().to_peer_id()
    }

    /// Get the mesh config.
    pub fn config(&self) -> &MeshConfig {
        &self.config
    }

    /// Build the multiaddr this node will listen on.
    pub fn listen_multiaddr(&self) -> libp2p::Multiaddr {
        format!(
            "/ip4/{}/udp/{}/quic-v1",
            self.config.listen_addr, self.config.listen_port
        )
        .parse()
        .expect("valid multiaddr")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_creation() {
        let config = MeshConfig::default();
        let transport = QuicTransport::new(config);
        // Should have a valid peer ID
        let peer_id = transport.peer_id();
        assert!(!peer_id.to_string().is_empty());
    }

    #[test]
    fn test_transport_with_keypair() {
        let kp = Keypair::generate_ed25519();
        let expected_id = kp.public().to_peer_id();
        let config = MeshConfig::default();
        let transport = QuicTransport::with_keypair(kp, config);
        assert_eq!(transport.peer_id(), expected_id);
    }

    #[test]
    fn test_listen_multiaddr() {
        let config = MeshConfig::default();
        let transport = QuicTransport::new(config);
        let addr = transport.listen_multiaddr();
        let addr_str = addr.to_string();
        assert!(addr_str.contains("/udp/"));
        assert!(addr_str.contains("/quic-v1"));
    }
}
