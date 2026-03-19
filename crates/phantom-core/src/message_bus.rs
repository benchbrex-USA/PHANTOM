//! Inter-agent message bus — typed, async, multi-producer multi-consumer.
//!
//! Agents communicate via the message bus. The CTO Agent uses it to delegate tasks,
//! specialist agents report results, and the Monitor Agent receives health updates.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::CoreError;

/// A message sent between agents via the bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: String,
    /// Sender agent ID (or "system")
    pub from: String,
    /// Recipient agent ID (or "broadcast")
    pub to: String,
    /// Message kind
    pub kind: MessageKind,
    /// Payload (JSON)
    pub payload: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl Message {
    /// Create a new message.
    pub fn new(from: impl Into<String>, to: impl Into<String>, kind: MessageKind, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from: from.into(),
            to: to.into(),
            kind,
            payload,
            timestamp: Utc::now(),
        }
    }

    /// Create a broadcast message (sent to all agents).
    pub fn broadcast(from: impl Into<String>, kind: MessageKind, payload: serde_json::Value) -> Self {
        Self::new(from, "broadcast", kind, payload)
    }
}

/// Types of messages agents can exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    /// CTO assigns a task to a specialist agent
    TaskAssignment,
    /// Agent reports task completion
    TaskCompleted,
    /// Agent reports task failure
    TaskFailed,
    /// Agent asks another agent for help (escalation)
    EscalationRequest,
    /// Response to an escalation
    EscalationResponse,
    /// Knowledge Brain query result
    KnowledgeResult,
    /// Health status update
    HealthUpdate,
    /// System halt command
    Halt,
    /// Progress update from an agent
    ProgressUpdate,
    /// Agent requests input from owner
    OwnerInput,
    /// Owner provides input to an agent
    OwnerResponse,
}

/// The message bus — coordinates all inter-agent communication.
pub struct MessageBus {
    /// Broadcast channel for system-wide messages
    broadcast_tx: broadcast::Sender<Message>,
    /// Direct channels to specific agents
    agent_channels: Arc<RwLock<HashMap<String, mpsc::Sender<Message>>>>,
    /// Channel buffer size
    buffer_size: usize,
}

/// A handle for an agent to receive messages.
pub struct AgentMailbox {
    /// Agent's ID
    pub agent_id: String,
    /// Direct message receiver
    direct_rx: mpsc::Receiver<Message>,
    /// Broadcast receiver
    broadcast_rx: broadcast::Receiver<Message>,
}

impl MessageBus {
    /// Create a new message bus.
    pub fn new(buffer_size: usize) -> Self {
        let (broadcast_tx, _) = broadcast::channel(buffer_size);
        Self {
            broadcast_tx,
            agent_channels: Arc::new(RwLock::new(HashMap::new())),
            buffer_size,
        }
    }

    /// Register an agent and get its mailbox for receiving messages.
    #[instrument(skip(self))]
    pub async fn register_agent(&self, agent_id: &str) -> Result<AgentMailbox, CoreError> {
        let (tx, rx) = mpsc::channel(self.buffer_size);
        let broadcast_rx = self.broadcast_tx.subscribe();

        self.agent_channels
            .write()
            .await
            .insert(agent_id.to_string(), tx);

        debug!(agent_id, "agent registered on message bus");

        Ok(AgentMailbox {
            agent_id: agent_id.to_string(),
            direct_rx: rx,
            broadcast_rx,
        })
    }

    /// Unregister an agent from the bus.
    pub async fn unregister_agent(&self, agent_id: &str) {
        self.agent_channels.write().await.remove(agent_id);
        debug!(agent_id, "agent unregistered from message bus");
    }

    /// Send a direct message to a specific agent.
    #[instrument(skip(self, message), fields(from = %message.from, to = %message.to, kind = ?message.kind))]
    pub async fn send(&self, message: Message) -> Result<(), CoreError> {
        if message.to == "broadcast" {
            return self.broadcast(message).await;
        }

        let channels = self.agent_channels.read().await;
        let tx = channels
            .get(&message.to)
            .ok_or_else(|| CoreError::MessageBus(format!("agent not found: {}", message.to)))?;

        tx.send(message)
            .await
            .map_err(|e| CoreError::MessageBus(format!("send failed: {}", e)))?;

        Ok(())
    }

    /// Broadcast a message to all agents.
    #[instrument(skip(self, message), fields(from = %message.from, kind = ?message.kind))]
    pub async fn broadcast(&self, message: Message) -> Result<(), CoreError> {
        // Ignore error when no subscribers (receiver count = 0 is ok)
        let _ = self.broadcast_tx.send(message);
        Ok(())
    }

    /// Send a halt message to all agents.
    pub async fn halt_all(&self, reason: &str) -> Result<(), CoreError> {
        let msg = Message::broadcast(
            "system",
            MessageKind::Halt,
            serde_json::json!({"reason": reason}),
        );
        self.broadcast(msg).await
    }

    /// Get the number of registered agents.
    pub async fn agent_count(&self) -> usize {
        self.agent_channels.read().await.len()
    }

    /// Check if an agent is registered.
    pub async fn is_registered(&self, agent_id: &str) -> bool {
        self.agent_channels.read().await.contains_key(agent_id)
    }
}

impl AgentMailbox {
    /// Receive the next direct message (blocks until one is available).
    pub async fn recv(&mut self) -> Option<Message> {
        self.direct_rx.recv().await
    }

    /// Try to receive a direct message without blocking.
    pub fn try_recv(&mut self) -> Option<Message> {
        self.direct_rx.try_recv().ok()
    }

    /// Receive the next broadcast message.
    pub async fn recv_broadcast(&mut self) -> Option<Message> {
        self.broadcast_rx.recv().await.ok()
    }

    /// Try to receive a broadcast message without blocking.
    pub fn try_recv_broadcast(&mut self) -> Option<Message> {
        self.broadcast_rx.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_send() {
        let bus = MessageBus::new(16);
        let mut mailbox = bus.register_agent("backend").await.unwrap();

        let msg = Message::new("cto", "backend", MessageKind::TaskAssignment, serde_json::json!({"task": "build API"}));
        bus.send(msg).await.unwrap();

        let received = mailbox.recv().await.unwrap();
        assert_eq!(received.from, "cto");
        assert_eq!(received.kind, MessageKind::TaskAssignment);
    }

    #[tokio::test]
    async fn test_send_to_unknown_agent_fails() {
        let bus = MessageBus::new(16);
        let msg = Message::new("cto", "nonexistent", MessageKind::TaskAssignment, serde_json::Value::Null);
        assert!(bus.send(msg).await.is_err());
    }

    #[tokio::test]
    async fn test_broadcast() {
        let bus = MessageBus::new(16);
        let mut mb1 = bus.register_agent("agent1").await.unwrap();
        let mut mb2 = bus.register_agent("agent2").await.unwrap();

        let msg = Message::broadcast("system", MessageKind::Halt, serde_json::json!({"reason": "test"}));
        bus.broadcast(msg).await.unwrap();

        let r1 = mb1.recv_broadcast().await.unwrap();
        let r2 = mb2.recv_broadcast().await.unwrap();
        assert_eq!(r1.kind, MessageKind::Halt);
        assert_eq!(r2.kind, MessageKind::Halt);
    }

    #[tokio::test]
    async fn test_unregister() {
        let bus = MessageBus::new(16);
        bus.register_agent("agent1").await.unwrap();
        assert_eq!(bus.agent_count().await, 1);

        bus.unregister_agent("agent1").await;
        assert_eq!(bus.agent_count().await, 0);
    }

    #[tokio::test]
    async fn test_halt_all() {
        let bus = MessageBus::new(16);
        let mut mb = bus.register_agent("agent1").await.unwrap();

        bus.halt_all("emergency").await.unwrap();

        let msg = mb.recv_broadcast().await.unwrap();
        assert_eq!(msg.kind, MessageKind::Halt);
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::new("cto", "backend", MessageKind::TaskAssignment, serde_json::json!({"task_id": "123"}));
        assert_eq!(msg.from, "cto");
        assert_eq!(msg.to, "backend");
        assert!(!msg.id.is_empty());
    }
}
