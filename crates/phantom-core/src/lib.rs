//! Phantom Core: task graph, agent manager, message bus, self-healing engine.
//!
//! The orchestration heart of Phantom.
//!
//! Architecture Framework → CTO Agent parses → Task Graph (DAG) →
//! Agent Manager spawns specialist agents → Message Bus coordinates →
//! Self-Healer handles failures (5-layer recovery) →
//! Audit Log records every action (tamper-evident, signed).
//!
//! Core Laws enforced here:
//! - Law 7: Self-healing at every layer
//! - Law 8: Master key holder has absolute power (halt, kill)
//! - Law 9: Every action is audited

pub mod task_graph;
pub mod agent_manager;
pub mod message_bus;
pub mod self_healer;
pub mod job_queue;
pub mod audit;
pub mod pipeline;
pub mod errors;

pub use errors::CoreError;
pub use task_graph::{Task, TaskGraph, TaskStatus};
pub use agent_manager::{AgentManager, AgentHandle, AgentState};
pub use message_bus::{MessageBus, Message, MessageKind};
pub use self_healer::{SelfHealer, HealingLayer, HealingResult};
pub use audit::{AuditLog, AuditEntry};
pub use pipeline::{BuildPipeline, BuildPhase};
