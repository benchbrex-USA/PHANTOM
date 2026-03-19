//! P2P mesh network — manages peer connections, message routing, and CRDT sync.
//!
//! The `MeshNetwork` is the top-level orchestrator that ties together:
//!   • QUIC transport (via libp2p)
//!   • Peer discovery (Kademlia DHT + mDNS)
//!   • CRDT state synchronization (Automerge)
//!   • Peer table management
//!   • Wire protocol message handling

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn};

use crate::config::MeshConfig;
use crate::discovery::DiscoveryTracker;
use crate::errors::NetError;
use crate::peer::{PeerInfo, PeerState, PeerTable};
use crate::protocol::{
    HeartbeatPayload, JoinPayload, MessageKind, WireMessage, PROTOCOL_VERSION,
};
use crate::sync::CrdtSync;
use crate::transport::QuicTransport;

/// Events emitted by the mesh network for the application layer to consume.
#[derive(Debug, Clone)]
pub enum MeshEvent {
    /// A new peer connected
    PeerConnected { peer_id: String },
    /// A peer disconnected
    PeerDisconnected { peer_id: String },
    /// A peer was discovered (but not yet connected)
    PeerDiscovered { peer_id: String },
    /// CRDT state was updated via sync
    StateUpdated { peer_id: String },
    /// A wire message was received that the app should handle
    MessageReceived { message: WireMessage },
    /// The mesh network started listening
    Listening { address: String },
    /// Error occurred
    Error { error: String },
}

/// The P2P mesh network manager.
///
/// Thread-safe: all interior state is behind Arc<RwLock<_>> / Arc<Mutex<_>>.
pub struct MeshNetwork {
    /// QUIC transport + identity
    transport: QuicTransport,
    /// Peer table
    peers: Arc<RwLock<PeerTable>>,
    /// Discovery tracker
    discovery: Arc<RwLock<DiscoveryTracker>>,
    /// CRDT sync manager
    crdt: Arc<Mutex<CrdtSync>>,
    /// Configuration
    config: MeshConfig,
    /// Event sender (for the application layer)
    event_tx: mpsc::Sender<MeshEvent>,
    /// Event receiver (handed to the application)
    event_rx: Option<mpsc::Receiver<MeshEvent>>,
    /// Whether the mesh is currently running
    running: Arc<std::sync::atomic::AtomicBool>,
    /// Start time (for uptime tracking)
    started_at: Option<Instant>,
}

impl MeshNetwork {
    /// Create a new mesh network with the given configuration.
    pub fn new(config: MeshConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(256);
        let transport = QuicTransport::new(config.clone());
        let max_peers = config.max_peers;

        Self {
            transport,
            peers: Arc::new(RwLock::new(PeerTable::new(max_peers))),
            discovery: Arc::new(RwLock::new(DiscoveryTracker::new())),
            crdt: Arc::new(Mutex::new(CrdtSync::new())),
            config,
            event_tx,
            event_rx: Some(event_rx),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            started_at: None,
        }
    }

    /// Create with an existing CRDT document (e.g. restored from storage).
    pub fn with_crdt(config: MeshConfig, crdt: CrdtSync) -> Self {
        let (event_tx, event_rx) = mpsc::channel(256);
        let transport = QuicTransport::new(config.clone());
        let max_peers = config.max_peers;

        Self {
            transport,
            peers: Arc::new(RwLock::new(PeerTable::new(max_peers))),
            discovery: Arc::new(RwLock::new(DiscoveryTracker::new())),
            crdt: Arc::new(Mutex::new(crdt)),
            config,
            event_tx,
            event_rx: Some(event_rx),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            started_at: None,
        }
    }

    /// Take the event receiver (can only be called once).
    pub fn take_event_rx(&mut self) -> Option<mpsc::Receiver<MeshEvent>> {
        self.event_rx.take()
    }

    /// Get the local peer ID.
    pub fn local_peer_id(&self) -> String {
        self.transport.peer_id().to_string()
    }

    /// Check if the mesh is running.
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get uptime in seconds (None if not started).
    pub fn uptime_secs(&self) -> Option<u64> {
        self.started_at.map(|t| t.elapsed().as_secs())
    }

    // ── Peer management ────────────────────────────────────────────────

    /// Register a discovered peer.
    pub async fn on_peer_discovered(&self, peer_id: &str) {
        {
            let mut disc = self.discovery.write().await;
            disc.discovered_mdns(peer_id);
        }

        let mut peers = self.peers.write().await;
        if peers.get(peer_id).is_none() {
            let info = PeerInfo::new(peer_id);
            peers.upsert(info);
        }

        let _ = self
            .event_tx
            .send(MeshEvent::PeerDiscovered {
                peer_id: peer_id.to_string(),
            })
            .await;
    }

    /// Handle a peer connecting.
    pub async fn on_peer_connected(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        match peers.get_mut(peer_id) {
            Some(info) => {
                info.state = PeerState::Connected;
                info.touch();
            }
            None => {
                let mut info = PeerInfo::new(peer_id);
                info.state = PeerState::Connected;
                peers.upsert(info);
            }
        }

        info!(peer_id, "peer connected");
        let _ = self
            .event_tx
            .send(MeshEvent::PeerConnected {
                peer_id: peer_id.to_string(),
            })
            .await;
    }

    /// Handle a peer disconnecting.
    pub async fn on_peer_disconnected(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(info) = peers.get_mut(peer_id) {
            info.state = PeerState::Disconnected;
            info.touch();
        }

        // Reset sync state for this peer
        {
            let mut crdt = self.crdt.lock().await;
            crdt.reset_peer_sync(peer_id);
        }

        info!(peer_id, "peer disconnected");
        let _ = self
            .event_tx
            .send(MeshEvent::PeerDisconnected {
                peer_id: peer_id.to_string(),
            })
            .await;
    }

    /// Ban a misbehaving peer.
    pub async fn ban_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(info) = peers.get_mut(peer_id) {
            info.state = PeerState::Banned;
            info.touch();
        }

        let mut disc = self.discovery.write().await;
        disc.remove(peer_id);

        let mut crdt = self.crdt.lock().await;
        crdt.reset_peer_sync(peer_id);

        warn!(peer_id, "peer banned");
    }

    // ── CRDT sync ──────────────────────────────────────────────────────

    /// Generate a sync message for a specific peer.
    pub async fn generate_sync_for(&self, peer_id: &str) -> Option<Vec<u8>> {
        let mut crdt = self.crdt.lock().await;
        crdt.generate_sync_message(peer_id)
    }

    /// Receive and process a sync message from a peer.
    pub async fn receive_sync_from(
        &self,
        peer_id: &str,
        message: &[u8],
    ) -> Result<(), NetError> {
        {
            let mut crdt = self.crdt.lock().await;
            crdt.receive_sync_message(peer_id, message)?;
        }

        // Update peer stats
        {
            let mut peers = self.peers.write().await;
            if let Some(info) = peers.get_mut(peer_id) {
                info.touch();
            }
        }

        let _ = self
            .event_tx
            .send(MeshEvent::StateUpdated {
                peer_id: peer_id.to_string(),
            })
            .await;

        Ok(())
    }

    /// Put a value in the shared CRDT state.
    pub async fn crdt_put(&self, key: &str, value: &str) -> Result<(), NetError> {
        let mut crdt = self.crdt.lock().await;
        crdt.put_str(key, value)
    }

    /// Get a value from the shared CRDT state.
    pub async fn crdt_get(&self, key: &str) -> Option<String> {
        let crdt = self.crdt.lock().await;
        crdt.get_str(key)
    }

    /// Save the CRDT document to bytes (for persistence).
    pub async fn save_crdt(&self) -> Vec<u8> {
        let mut crdt = self.crdt.lock().await;
        crdt.save()
    }

    // ── Wire protocol ──────────────────────────────────────────────────

    /// Handle an incoming wire message.
    pub async fn handle_message(&self, message: WireMessage) -> Result<(), NetError> {
        match message.kind {
            MessageKind::CrdtSyncRequest | MessageKind::CrdtSyncResponse => {
                self.receive_sync_from(&message.sender, &message.payload)
                    .await?;
            }
            MessageKind::Heartbeat => {
                // Update peer's last-seen timestamp
                let mut peers = self.peers.write().await;
                if let Some(info) = peers.get_mut(&message.sender) {
                    info.touch();
                }
                debug!(peer_id = %message.sender, "heartbeat received");
            }
            MessageKind::JoinRequest => {
                let peers = self.peers.read().await;
                if !peers.can_accept() {
                    warn!(
                        peer_id = %message.sender,
                        "rejecting join: at max peers"
                    );
                    return Ok(());
                }
                drop(peers);
                self.on_peer_connected(&message.sender).await;
            }
            MessageKind::Leave => {
                self.on_peer_disconnected(&message.sender).await;
            }
            MessageKind::BindTokenExchange => {
                if let Ok(token) = String::from_utf8(message.payload.clone()) {
                    let mut peers = self.peers.write().await;
                    if let Some(info) = peers.get_mut(&message.sender) {
                        info.bind_token = Some(token);
                        info.touch();
                    }
                }
            }
            _ => {
                // Forward to application layer
                let _ = self
                    .event_tx
                    .send(MeshEvent::MessageReceived { message })
                    .await;
            }
        }
        Ok(())
    }

    /// Create a heartbeat message from this node.
    pub async fn create_heartbeat(&self) -> WireMessage {
        let peers = self.peers.read().await;

        let payload = HeartbeatPayload {
            doc_heads: Vec::new(), // Simplified — full impl would extract Automerge heads
            peer_count: peers.connected_count(),
            uptime_secs: self.uptime_secs().unwrap_or(0),
            current_phase: None,
        };

        let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
        WireMessage::new(MessageKind::Heartbeat, self.local_peer_id(), payload_bytes)
    }

    /// Create a join request message.
    pub fn create_join_request(&self, bind_token: Option<String>) -> WireMessage {
        let payload = JoinPayload {
            agent_version: format!("phantom/{}", env!("CARGO_PKG_VERSION")),
            protocol_version: PROTOCOL_VERSION.to_string(),
            bind_token,
            capabilities: vec!["sync".into(), "relay".into()],
        };
        let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
        WireMessage::new(MessageKind::JoinRequest, self.local_peer_id(), payload_bytes)
    }

    // ── Diagnostics ────────────────────────────────────────────────────

    /// Get the current network status.
    pub async fn status(&self) -> MeshStatus {
        let peers = self.peers.read().await;
        let disc = self.discovery.read().await;
        let mut crdt = self.crdt.lock().await;
        let sync_status = crdt.status();

        MeshStatus {
            local_peer_id: self.local_peer_id(),
            is_running: self.is_running(),
            uptime_secs: self.uptime_secs(),
            connected_peers: peers.connected_count(),
            total_known_peers: peers.len(),
            discovered_peers: disc.discovered_count(),
            max_peers: self.config.max_peers,
            sync_rounds: sync_status.total_sync_rounds,
            crdt_doc_size_bytes: sync_status.doc_size_bytes,
            crdt_root_keys: sync_status.root_keys,
        }
    }

    /// Get connected peer list.
    pub async fn connected_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.connected().into_iter().cloned().collect()
    }

    /// Get all peers sorted by reliability.
    pub async fn peers_by_reliability(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.by_reliability().into_iter().cloned().collect()
    }

    /// Prune stale peers from the peer table.
    pub async fn prune_stale_peers(&self) -> Vec<String> {
        let max_silence = self.config.idle_timeout_secs as i64 * 2;
        let mut peers = self.peers.write().await;
        let pruned = peers.prune_stale(max_silence);
        if !pruned.is_empty() {
            info!(count = pruned.len(), "pruned stale peers");
        }
        pruned
    }
}

/// Snapshot of the mesh network status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeshStatus {
    pub local_peer_id: String,
    pub is_running: bool,
    pub uptime_secs: Option<u64>,
    pub connected_peers: usize,
    pub total_known_peers: usize,
    pub discovered_peers: usize,
    pub max_peers: usize,
    pub sync_rounds: u64,
    pub crdt_doc_size_bytes: usize,
    pub crdt_root_keys: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_mesh() -> MeshNetwork {
        MeshNetwork::new(MeshConfig::default())
    }

    #[tokio::test]
    async fn test_mesh_creation() {
        let mesh = test_mesh();
        assert!(!mesh.is_running());
        assert!(!mesh.local_peer_id().is_empty());
    }

    #[tokio::test]
    async fn test_peer_lifecycle() {
        let mesh = test_mesh();

        // Discover
        mesh.on_peer_discovered("peer-1").await;
        let status = mesh.status().await;
        assert_eq!(status.total_known_peers, 1);
        assert_eq!(status.connected_peers, 0);

        // Connect
        mesh.on_peer_connected("peer-1").await;
        let status = mesh.status().await;
        assert_eq!(status.connected_peers, 1);

        // Disconnect
        mesh.on_peer_disconnected("peer-1").await;
        let status = mesh.status().await;
        assert_eq!(status.connected_peers, 0);
        assert_eq!(status.total_known_peers, 1); // Still in table
    }

    #[tokio::test]
    async fn test_ban_peer() {
        let mesh = test_mesh();
        mesh.on_peer_connected("bad-peer").await;
        mesh.ban_peer("bad-peer").await;

        let peers = mesh.peers.read().await;
        let info = peers.get("bad-peer").unwrap();
        assert_eq!(info.state, PeerState::Banned);
    }

    #[tokio::test]
    async fn test_crdt_operations() {
        let mesh = test_mesh();
        mesh.crdt_put("project", "phantom").await.unwrap();
        assert_eq!(
            mesh.crdt_get("project").await,
            Some("phantom".to_string())
        );
    }

    #[tokio::test]
    async fn test_crdt_persistence() {
        let mesh = test_mesh();
        mesh.crdt_put("key", "value").await.unwrap();
        let saved = mesh.save_crdt().await;

        // Restore into new mesh
        let crdt = CrdtSync::load(&saved).unwrap();
        let mesh2 = MeshNetwork::with_crdt(MeshConfig::default(), crdt);
        assert_eq!(mesh2.crdt_get("key").await, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_heartbeat_message() {
        let mesh = test_mesh();
        mesh.on_peer_connected("p1").await;
        let hb = mesh.create_heartbeat().await;
        assert_eq!(hb.kind, MessageKind::Heartbeat);
        assert!(!hb.sender.is_empty());

        let payload: HeartbeatPayload = serde_json::from_slice(&hb.payload).unwrap();
        assert_eq!(payload.peer_count, 1);
    }

    #[tokio::test]
    async fn test_join_request() {
        let mesh = test_mesh();
        let msg = mesh.create_join_request(Some("tok_abc".into()));
        assert_eq!(msg.kind, MessageKind::JoinRequest);

        let payload: JoinPayload = serde_json::from_slice(&msg.payload).unwrap();
        assert_eq!(payload.bind_token, Some("tok_abc".into()));
        assert!(payload.protocol_version.contains("phantom"));
    }

    #[tokio::test]
    async fn test_handle_heartbeat() {
        let mesh = test_mesh();
        mesh.on_peer_connected("peer-1").await;

        let msg = WireMessage::new(MessageKind::Heartbeat, "peer-1", vec![]);
        mesh.handle_message(msg).await.unwrap();

        // Peer should still be connected, last_seen updated
        let peers = mesh.peers.read().await;
        let info = peers.get("peer-1").unwrap();
        assert_eq!(info.state, PeerState::Connected);
    }

    #[tokio::test]
    async fn test_handle_leave() {
        let mesh = test_mesh();
        mesh.on_peer_connected("peer-1").await;

        let msg = WireMessage::new(MessageKind::Leave, "peer-1", vec![]);
        mesh.handle_message(msg).await.unwrap();

        let peers = mesh.peers.read().await;
        let info = peers.get("peer-1").unwrap();
        assert_eq!(info.state, PeerState::Disconnected);
    }

    #[tokio::test]
    async fn test_mesh_status() {
        let mesh = test_mesh();
        mesh.on_peer_connected("p1").await;
        mesh.on_peer_connected("p2").await;
        mesh.on_peer_discovered("p3").await;

        let status = mesh.status().await;
        assert_eq!(status.connected_peers, 2);
        assert_eq!(status.total_known_peers, 3);
        assert_eq!(status.max_peers, 50);
    }
}
