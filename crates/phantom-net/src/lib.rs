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

pub mod mesh;
pub mod discovery;
pub mod sync;
pub mod transport;
pub mod protocol;
pub mod peer;
pub mod config;
pub mod errors;

pub use errors::NetError;
pub use mesh::{MeshNetwork, MeshEvent, MeshStatus};
pub use peer::{PeerInfo, PeerState, PeerTable};
pub use sync::{CrdtSync, SyncStatus};
pub use config::MeshConfig;
pub use discovery::{DiscoveryTracker, DiscoveryStats};
pub use transport::QuicTransport;
pub use protocol::{WireMessage, MessageKind, PROTOCOL_VERSION};
