//! Phantom wire protocol — message types exchanged between peers.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Protocol version string.
pub const PROTOCOL_VERSION: &str = "/phantom/sync/1.0.0";

/// Gossipsub topic for CRDT state sync.
pub const SYNC_TOPIC: &str = "phantom-crdt-sync";

/// Gossipsub topic for heartbeat / presence.
pub const HEARTBEAT_TOPIC: &str = "phantom-heartbeat";

/// Envelope for all wire messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    /// Message type tag
    pub kind: MessageKind,
    /// Sender peer ID
    pub sender: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Payload bytes (serialised inner message)
    pub payload: Vec<u8>,
    /// Optional encryption nonce (if payload is encrypted)
    pub nonce: Option<Vec<u8>>,
}

impl WireMessage {
    pub fn new(kind: MessageKind, sender: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            kind,
            sender: sender.into(),
            timestamp: Utc::now(),
            payload,
            nonce: None,
        }
    }

    /// Serialise to bytes for the wire.
    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialise from wire bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Discriminator for wire message types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    /// Full CRDT state snapshot
    CrdtState,
    /// Incremental CRDT sync message (Automerge sync protocol)
    CrdtSyncRequest,
    /// Response to a sync request
    CrdtSyncResponse,
    /// Heartbeat / presence announcement
    Heartbeat,
    /// Peer requesting to join the mesh
    JoinRequest,
    /// Acknowledgement of join
    JoinAck,
    /// Graceful leave announcement
    Leave,
    /// Bind token exchange (cryptographic ownership proof)
    BindTokenExchange,
}

/// Heartbeat payload — lightweight presence announcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    /// Peer's current CRDT document heads (for sync negotiation)
    pub doc_heads: Vec<String>,
    /// Number of connected peers this node sees
    pub peer_count: usize,
    /// Peer's uptime in seconds
    pub uptime_secs: u64,
    /// Peer's current pipeline phase (if any)
    pub current_phase: Option<String>,
}

/// Join request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinPayload {
    /// Agent version string
    pub agent_version: String,
    /// Protocol version
    pub protocol_version: String,
    /// Bind token for server ownership proof
    pub bind_token: Option<String>,
    /// Capabilities offered by this peer
    pub capabilities: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_message_roundtrip() {
        let msg = WireMessage::new(
            MessageKind::Heartbeat,
            "peer-123",
            b"hello".to_vec(),
        );
        let encoded = msg.encode().unwrap();
        let decoded = WireMessage::decode(&encoded).unwrap();
        assert_eq!(decoded.kind, MessageKind::Heartbeat);
        assert_eq!(decoded.sender, "peer-123");
        assert_eq!(decoded.payload, b"hello");
    }

    #[test]
    fn test_heartbeat_payload() {
        let hb = HeartbeatPayload {
            doc_heads: vec!["abc123".into()],
            peer_count: 3,
            uptime_secs: 120,
            current_phase: Some("code_generation".into()),
        };
        let bytes = serde_json::to_vec(&hb).unwrap();
        let decoded: HeartbeatPayload = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.peer_count, 3);
        assert_eq!(decoded.uptime_secs, 120);
    }

    #[test]
    fn test_join_payload() {
        let jp = JoinPayload {
            agent_version: "phantom/0.1.0".into(),
            protocol_version: PROTOCOL_VERSION.into(),
            bind_token: Some("tok_abc".into()),
            capabilities: vec!["sync".into(), "relay".into()],
        };
        let bytes = serde_json::to_vec(&jp).unwrap();
        let decoded: JoinPayload = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.protocol_version, PROTOCOL_VERSION);
        assert_eq!(decoded.capabilities.len(), 2);
    }

    #[test]
    fn test_message_kind_serde() {
        let kind = MessageKind::CrdtSyncRequest;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"crdt_sync_request\"");
        let decoded: MessageKind = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, MessageKind::CrdtSyncRequest);
    }
}
