//! libp2p Swarm construction and composite network behaviour.
//!
//! Builds the `PhantomBehaviour` (Kademlia + mDNS + Identify) and wires it
//! into a `Swarm` running on the Tokio executor with QUIC transport.

use std::time::Duration;

use libp2p::identity::Keypair;
use libp2p::kad::store::MemoryStore;
use libp2p::{Multiaddr, PeerId, Swarm, SwarmBuilder};
use tracing::info;

use crate::config::MeshConfig;
use crate::errors::NetError;

/// Composite libp2p behaviour for the Phantom mesh.
#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct PhantomBehaviour {
    /// Kademlia DHT for wide-area peer discovery and routing.
    pub kademlia: libp2p::kad::Behaviour<MemoryStore>,
    /// mDNS for zero-config LAN peer discovery.
    pub mdns: libp2p::mdns::tokio::Behaviour,
    /// Identify protocol for exchanging peer metadata.
    pub identify: libp2p::identify::Behaviour,
}

/// Build and configure the libp2p Swarm for Phantom's P2P mesh.
///
/// Uses QUIC transport (which bundles Noise encryption in libp2p 0.54).
/// Configures Kademlia, mDNS, and Identify sub-behaviours based on `MeshConfig`.
pub fn build_swarm(
    config: &MeshConfig,
    keypair: &Keypair,
) -> Result<Swarm<PhantomBehaviour>, NetError> {
    let peer_id = keypair.public().to_peer_id();
    info!(%peer_id, "building libp2p swarm");

    let idle_timeout = config.idle_timeout_secs;

    let swarm = SwarmBuilder::with_existing_identity(keypair.clone())
        .with_tokio()
        .with_quic()
        .with_behaviour(|key: &Keypair| {
            let local_peer_id = key.public().to_peer_id();

            // Kademlia DHT
            let store = MemoryStore::new(local_peer_id);
            let kademlia = libp2p::kad::Behaviour::new(local_peer_id, store);

            // mDNS
            let mdns_config = libp2p::mdns::Config::default();
            let mdns = libp2p::mdns::tokio::Behaviour::new(mdns_config, local_peer_id)?;

            // Identify
            let identify_config =
                libp2p::identify::Config::new("/phantom/1.0.0".into(), key.public());
            let identify = libp2p::identify::Behaviour::new(identify_config);

            Ok(PhantomBehaviour {
                kademlia,
                mdns,
                identify,
            })
        })
        .map_err(|e| NetError::Transport(format!("failed to build behaviour: {e}")))?
        .with_swarm_config(|c: libp2p::swarm::Config| {
            c.with_idle_connection_timeout(Duration::from_secs(idle_timeout))
        })
        .build();

    Ok(swarm)
}

/// Commands sent from `MeshNetwork` methods to the background event loop task.
#[derive(Debug)]
pub enum SwarmCommand {
    /// Dial a remote peer at the given multiaddr.
    Dial(Multiaddr),
    /// Start listening on the given multiaddr.
    Listen(Multiaddr),
    /// Add a peer address to Kademlia routing table.
    AddKademliaAddress(PeerId, Multiaddr),
    /// Perform a Kademlia bootstrap.
    KademliaBootstrap,
    /// Shut down the event loop.
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_swarm_default_config() {
        // SwarmBuilder requires a tokio runtime for the QUIC transport.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let keypair = Keypair::generate_ed25519();
            let config = MeshConfig::default();
            let swarm = build_swarm(&config, &keypair);
            assert!(
                swarm.is_ok(),
                "swarm build should succeed: {:?}",
                swarm.err()
            );
            let swarm = swarm.unwrap();
            assert_eq!(*swarm.local_peer_id(), keypair.public().to_peer_id());
        });
    }

    #[test]
    fn test_build_swarm_custom_timeout() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let keypair = Keypair::generate_ed25519();
            let mut config = MeshConfig::default();
            config.idle_timeout_secs = 120;
            let swarm = build_swarm(&config, &keypair);
            assert!(swarm.is_ok());
        });
    }

    #[test]
    fn test_phantom_behaviour_event_type_exists() {
        // Verify the derive macro generates the PhantomBehaviourEvent enum.
        // This is a compile-time check — if it compiles, the type exists.
        fn _assert_event_type(_e: PhantomBehaviourEvent) {}
    }
}
