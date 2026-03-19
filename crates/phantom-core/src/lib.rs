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

pub mod agent_manager;
pub mod audit;
pub mod errors;
pub mod job_queue;
pub mod message_bus;
pub mod pipeline;
pub mod self_healer;
pub mod task_graph;

pub use agent_manager::{AgentHandle, AgentManager, AgentState};
pub use audit::{AuditEntry, AuditLog};
pub use errors::CoreError;
pub use message_bus::{Message, MessageBus, MessageKind};
pub use pipeline::{BuildPhase, BuildPipeline};
pub use self_healer::{HealingLayer, HealingResult, SelfHealer};
pub use task_graph::{Task, TaskGraph, TaskStatus};
