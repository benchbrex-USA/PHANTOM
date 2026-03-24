//! HTTP relay fallback for peers behind NAT.
//!
//! When direct QUIC connections fail (symmetric NAT, corporate firewall),
//! peers fall back to relaying messages through an HTTP endpoint.
//! The relay is a simple WebSocket-over-HTTP bridge that forwards
//! WireMessage payloads between peers who cannot connect directly.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::errors::NetError;
use crate::protocol::WireMessage;

/// Configuration for the HTTP relay fallback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    /// Relay server URL (e.g. "https://relay.phantom.dev")
    pub relay_url: String,
    /// Room/channel ID to join (derived from project bind token)
    pub room_id: String,
    /// How often to poll for messages (if not using WebSocket)
    pub poll_interval_secs: u64,
    /// Maximum message size through relay
    pub max_message_size: usize,
    /// Connection timeout
    pub connect_timeout_secs: u64,
    /// Authentication token for the relay server
    pub auth_token: Option<String>,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            relay_url: String::new(),
            room_id: String::new(),
            poll_interval_secs: 2,
            max_message_size: 1024 * 1024, // 1MB
            connect_timeout_secs: 10,
            auth_token: None,
        }
    }
}

/// State of the relay connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayState {
    Disconnected,
    Connecting,
    Connected,
    Degraded,
    Failed,
}

/// A relayed peer — someone we communicate with through the relay.
#[derive(Debug, Clone)]
pub struct RelayedPeer {
    pub peer_id: String,
    pub last_seen: Instant,
    pub messages_relayed: u64,
    pub bytes_relayed: u64,
}

/// HTTP relay client for NAT traversal fallback.
///
/// Uses HTTP long-polling to exchange WireMessages when direct P2P fails.
/// Messages are POST'd to the relay server and polled by the recipient.
pub struct RelayClient {
    config: RelayConfig,
    http: reqwest::Client,
    state: Arc<RwLock<RelayState>>,
    local_peer_id: String,
    /// Messages received from relay, forwarded to mesh
    inbound_tx: mpsc::Sender<WireMessage>,
    /// Peers we're communicating with through the relay
    relayed_peers: Arc<RwLock<HashMap<String, RelayedPeer>>>,
    /// Sequence number for ordering
    sequence: Arc<std::sync::atomic::AtomicU64>,
}

/// Message envelope for the relay HTTP API.
#[derive(Debug, Serialize, Deserialize)]
struct RelayEnvelope {
    /// Sender peer ID
    sender: String,
    /// Room ID
    room: String,
    /// Sequence number
    seq: u64,
    /// Serialized WireMessage
    payload: Vec<u8>,
    /// Timestamp (epoch millis)
    timestamp: u64,
}

/// Response from polling the relay.
#[derive(Debug, Deserialize)]
struct PollResponse {
    messages: Vec<RelayEnvelope>,
    /// Server-assigned cursor for next poll
    cursor: Option<String>,
}

/// Response from posting to the relay.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PostResponse {
    accepted: bool,
    #[serde(default)]
    error: Option<String>,
}

impl RelayClient {
    /// Create a new relay client.
    pub fn new(
        config: RelayConfig,
        local_peer_id: String,
        inbound_tx: mpsc::Sender<WireMessage>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.connect_timeout_secs))
            .build()
            .expect("failed to build HTTP client");

        Self {
            config,
            http,
            state: Arc::new(RwLock::new(RelayState::Disconnected)),
            local_peer_id,
            inbound_tx,
            relayed_peers: Arc::new(RwLock::new(HashMap::new())),
            sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get current relay state.
    pub async fn state(&self) -> RelayState {
        *self.state.read().await
    }

    /// Check if relay is connected.
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == RelayState::Connected
    }

    /// Join the relay room and start polling for messages.
    pub async fn connect(&self) -> Result<(), NetError> {
        if self.config.relay_url.is_empty() {
            return Err(NetError::ConnectionFailed(
                "relay URL not configured".into(),
            ));
        }

        *self.state.write().await = RelayState::Connecting;

        // Register with the relay server
        let register_url = format!(
            "{}/rooms/{}/join",
            self.config.relay_url, self.config.room_id
        );
        let mut req = self.http.post(&register_url).json(&serde_json::json!({
            "peer_id": self.local_peer_id,
            "protocol_version": crate::protocol::PROTOCOL_VERSION,
        }));

        if let Some(token) = &self.config.auth_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| NetError::ConnectionFailed(format!("relay registration failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            *self.state.write().await = RelayState::Failed;
            return Err(NetError::ConnectionFailed(format!(
                "relay returned {}: {}",
                status, body
            )));
        }

        *self.state.write().await = RelayState::Connected;
        info!(
            room = %self.config.room_id,
            relay = %self.config.relay_url,
            "connected to relay"
        );
        Ok(())
    }

    /// Send a message through the relay.
    pub async fn send(&self, message: &WireMessage) -> Result<(), NetError> {
        if *self.state.read().await != RelayState::Connected {
            return Err(NetError::ConnectionFailed("relay not connected".into()));
        }

        let payload = message
            .encode()
            .map_err(|e| NetError::Serialization(format!("failed to encode message: {}", e)))?;

        if payload.len() > self.config.max_message_size {
            return Err(NetError::MessageTooLarge {
                size: payload.len(),
                max: self.config.max_message_size,
            });
        }

        let seq = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let envelope = RelayEnvelope {
            sender: self.local_peer_id.clone(),
            room: self.config.room_id.clone(),
            seq,
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        let post_url = format!(
            "{}/rooms/{}/messages",
            self.config.relay_url, self.config.room_id
        );

        let mut req = self.http.post(&post_url).json(&envelope);
        if let Some(token) = &self.config.auth_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| NetError::ConnectionFailed(format!("relay send failed: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(NetError::ConnectionFailed(format!(
                "relay rejected message: {}",
                body
            )));
        }

        // Track relayed peer stats
        {
            let mut peers = self.relayed_peers.write().await;
            let entry = peers
                .entry(message.sender.clone())
                .or_insert_with(|| RelayedPeer {
                    peer_id: message.sender.clone(),
                    last_seen: Instant::now(),
                    messages_relayed: 0,
                    bytes_relayed: 0,
                });
            entry.messages_relayed += 1;
            entry.bytes_relayed += message.payload.len() as u64;
        }

        debug!(
            room = %self.config.room_id,
            seq,
            "message sent via relay"
        );
        Ok(())
    }

    /// Poll for new messages from the relay.
    pub async fn poll(
        &self,
        cursor: Option<&str>,
    ) -> Result<(Vec<WireMessage>, Option<String>), NetError> {
        if *self.state.read().await != RelayState::Connected {
            return Err(NetError::ConnectionFailed("relay not connected".into()));
        }

        let mut poll_url = format!(
            "{}/rooms/{}/messages?peer_id={}",
            self.config.relay_url, self.config.room_id, self.local_peer_id
        );
        if let Some(c) = cursor {
            poll_url.push_str(&format!("&cursor={}", c));
        }

        let mut req = self.http.get(&poll_url);
        if let Some(token) = &self.config.auth_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| NetError::ConnectionFailed(format!("relay poll failed: {}", e)))?;

        if !resp.status().is_success() {
            // Mark as degraded on poll failure
            *self.state.write().await = RelayState::Degraded;
            let body = resp.text().await.unwrap_or_default();
            return Err(NetError::ConnectionFailed(format!(
                "relay poll error: {}",
                body
            )));
        }

        let poll_resp: PollResponse = resp.json().await.map_err(|e| {
            NetError::Serialization(format!("failed to parse poll response: {}", e))
        })?;

        let mut messages = Vec::new();
        for envelope in poll_resp.messages {
            // Skip our own messages
            if envelope.sender == self.local_peer_id {
                continue;
            }

            match WireMessage::decode(&envelope.payload) {
                Ok(msg) => {
                    // Forward to mesh
                    let _ = self.inbound_tx.send(msg.clone()).await;
                    messages.push(msg);

                    // Track peer
                    let mut peers = self.relayed_peers.write().await;
                    let entry =
                        peers
                            .entry(envelope.sender.clone())
                            .or_insert_with(|| RelayedPeer {
                                peer_id: envelope.sender.clone(),
                                last_seen: Instant::now(),
                                messages_relayed: 0,
                                bytes_relayed: 0,
                            });
                    entry.last_seen = Instant::now();
                    entry.messages_relayed += 1;
                    entry.bytes_relayed += envelope.payload.len() as u64;
                }
                Err(e) => {
                    warn!(
                        sender = %envelope.sender,
                        error = %e,
                        "failed to decode relayed message"
                    );
                }
            }
        }

        Ok((messages, poll_resp.cursor))
    }

    /// Start the polling loop in a background task.
    /// Returns a handle to stop the loop.
    pub fn start_polling(&self) -> mpsc::Sender<()> {
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        let state = self.state.clone();
        let relay_url = self.config.relay_url.clone();
        let room_id = self.config.room_id.clone();
        let peer_id = self.local_peer_id.clone();
        let poll_interval = Duration::from_secs(self.config.poll_interval_secs);
        let http = self.http.clone();
        let auth_token = self.config.auth_token.clone();
        let inbound_tx = self.inbound_tx.clone();
        let relayed_peers = self.relayed_peers.clone();

        tokio::spawn(async move {
            let mut cursor: Option<String> = None;

            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        info!("relay polling stopped");
                        break;
                    }
                    _ = tokio::time::sleep(poll_interval) => {
                        if *state.read().await != RelayState::Connected {
                            continue;
                        }

                        let mut poll_url = format!(
                            "{}/rooms/{}/messages?peer_id={}",
                            relay_url, room_id, peer_id
                        );
                        if let Some(ref c) = cursor {
                            poll_url.push_str(&format!("&cursor={}", c));
                        }

                        let mut req = http.get(&poll_url);
                        if let Some(ref token) = auth_token {
                            req = req.header("Authorization", format!("Bearer {}", token));
                        }

                        match req.send().await {
                            Ok(resp) if resp.status().is_success() => {
                                if let Ok(poll_resp) = resp.json::<PollResponse>().await {
                                    cursor = poll_resp.cursor;
                                    for envelope in poll_resp.messages {
                                        if envelope.sender == peer_id {
                                            continue;
                                        }
                                        if let Ok(msg) = WireMessage::decode(&envelope.payload) {
                                            let _ = inbound_tx.send(msg).await;

                                            let mut peers = relayed_peers.write().await;
                                            let entry = peers
                                                .entry(envelope.sender.clone())
                                                .or_insert_with(|| RelayedPeer {
                                                    peer_id: envelope.sender.clone(),
                                                    last_seen: Instant::now(),
                                                    messages_relayed: 0,
                                                    bytes_relayed: 0,
                                                });
                                            entry.last_seen = Instant::now();
                                            entry.messages_relayed += 1;
                                        }
                                    }
                                }
                            }
                            Ok(resp) => {
                                warn!(status = %resp.status(), "relay poll returned error");
                                *state.write().await = RelayState::Degraded;
                            }
                            Err(e) => {
                                warn!(error = %e, "relay poll failed");
                                *state.write().await = RelayState::Degraded;
                            }
                        }
                    }
                }
            }
        });

        stop_tx
    }

    /// Disconnect from the relay.
    pub async fn disconnect(&self) -> Result<(), NetError> {
        let leave_url = format!(
            "{}/rooms/{}/leave",
            self.config.relay_url, self.config.room_id
        );

        let mut req = self.http.post(&leave_url).json(&serde_json::json!({
            "peer_id": self.local_peer_id,
        }));
        if let Some(token) = &self.config.auth_token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        // Best-effort leave — don't fail if server is unreachable
        let _ = req.send().await;

        *self.state.write().await = RelayState::Disconnected;
        info!(room = %self.config.room_id, "disconnected from relay");
        Ok(())
    }

    /// Get stats about relayed peers.
    pub async fn relayed_peer_count(&self) -> usize {
        self.relayed_peers.read().await.len()
    }

    /// Get relay diagnostics.
    pub async fn diagnostics(&self) -> RelayDiagnostics {
        let peers = self.relayed_peers.read().await;
        let total_messages: u64 = peers.values().map(|p| p.messages_relayed).sum();
        let total_bytes: u64 = peers.values().map(|p| p.bytes_relayed).sum();

        RelayDiagnostics {
            state: *self.state.read().await,
            relay_url: self.config.relay_url.clone(),
            room_id: self.config.room_id.clone(),
            relayed_peers: peers.len(),
            total_messages_relayed: total_messages,
            total_bytes_relayed: total_bytes,
        }
    }
}

/// Relay connection diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayDiagnostics {
    pub state: RelayState,
    pub relay_url: String,
    pub room_id: String,
    pub relayed_peers: usize,
    pub total_messages_relayed: u64,
    pub total_bytes_relayed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> RelayConfig {
        RelayConfig {
            relay_url: "https://relay.example.com".into(),
            room_id: "test-room-123".into(),
            ..Default::default()
        }
    }

    #[test]
    fn test_relay_config_default() {
        let config = RelayConfig::default();
        assert!(config.relay_url.is_empty());
        assert_eq!(config.poll_interval_secs, 2);
        assert_eq!(config.max_message_size, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_relay_client_creation() {
        let (tx, _rx) = mpsc::channel(16);
        let client = RelayClient::new(test_config(), "peer-123".into(), tx);
        assert_eq!(client.state().await, RelayState::Disconnected);
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_relay_send_not_connected() {
        let (tx, _rx) = mpsc::channel(16);
        let client = RelayClient::new(test_config(), "peer-123".into(), tx);
        let msg = WireMessage::new(crate::protocol::MessageKind::Heartbeat, "peer-123", vec![]);
        let result = client.send(&msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_relay_connect_empty_url() {
        let (tx, _rx) = mpsc::channel(16);
        let config = RelayConfig::default(); // empty URL
        let client = RelayClient::new(config, "peer-123".into(), tx);
        let result = client.connect().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_relay_diagnostics() {
        let (tx, _rx) = mpsc::channel(16);
        let client = RelayClient::new(test_config(), "peer-123".into(), tx);
        let diag = client.diagnostics().await;
        assert_eq!(diag.state, RelayState::Disconnected);
        assert_eq!(diag.relayed_peers, 0);
        assert_eq!(diag.relay_url, "https://relay.example.com");
    }

    #[test]
    fn test_relay_envelope_serde() {
        let envelope = RelayEnvelope {
            sender: "peer-1".into(),
            room: "room-1".into(),
            seq: 42,
            payload: b"test payload".to_vec(),
            timestamp: 1234567890,
        };
        let json = serde_json::to_vec(&envelope).unwrap();
        let decoded: RelayEnvelope = serde_json::from_slice(&json).unwrap();
        assert_eq!(decoded.sender, "peer-1");
        assert_eq!(decoded.seq, 42);
    }

    #[test]
    fn test_relay_state_serde() {
        let state = RelayState::Connected;
        let json = serde_json::to_string(&state).unwrap();
        let decoded: RelayState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, RelayState::Connected);
    }
}
