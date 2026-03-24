//! Phantom P2P Mesh: libp2p networking, CRDT state sync, peer discovery.
//!
//! Protocol stack (from Architecture Framework §11):
//!   Transport:   QUIC (UDP, NAT-traversal friendly)
//!   Security:    Noise protocol (XX handshake)
//!   Identity:    Ed25519 peer IDs
//!   Discovery:   Kademlia DHT + mDNS (local)
//!   Sync:        CRDT (Automerge) — conflict-free replication
//!   Encryption:  ChaCha20-Poly1305
//!
//! What syncs:       project state, task graph, infra bindings, audit log, health metrics
//! What never syncs: master key, session keys, raw credentials

pub mod config;
pub mod discovery;
pub mod errors;
pub mod mesh;
pub mod peer;
pub mod protocol;
pub mod relay;
pub mod swarm;
pub mod sync;
pub mod transport;

pub use config::MeshConfig;
pub use discovery::{DiscoveryStats, DiscoveryTracker};
pub use errors::NetError;
pub use mesh::{MeshEvent, MeshNetwork, MeshStatus};
pub use peer::{PeerInfo, PeerState, PeerTable};
pub use protocol::{MessageKind, WireMessage, PROTOCOL_VERSION};
pub use relay::{RelayClient, RelayConfig, RelayDiagnostics, RelayState};
pub use swarm::{build_swarm, PhantomBehaviour};
pub use sync::{CrdtSync, SyncStatus};
pub use transport::QuicTransport;
