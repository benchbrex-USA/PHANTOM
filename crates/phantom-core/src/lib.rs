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
pub mod beyond_human;
pub mod errors;
pub mod framework_ingestion;
pub mod job_queue;
pub mod macos;
pub mod message_bus;
pub mod pipeline;
pub mod self_healer;
pub mod task_graph;
pub mod zero_footprint;

pub use agent_manager::{AgentHandle, AgentManager, AgentState};
pub use audit::{AuditEntry, AuditLog};
pub use beyond_human::{
    AmbientDaemon, AmbientSnapshot, BeyondError, CostOracle, CostReport, GitConfig, GitOpResult,
    LearningCategory, PredictiveScanner, ProjectLearning, ProjectMemory, ScanReport, SelfScheduler,
    SelfUpdater, SmartGit, VoiceConfig, VoiceNotifier,
};
pub use errors::CoreError;
pub use framework_ingestion::{
    ComponentDag, ComponentExtractor, ExecutionPlan, ExtractedArchitecture, IngestionPipeline,
    MarkdownParser, ParsedFramework, PlanGenerator,
};
pub use macos::{
    BrowserAutomation, ClipboardBridge, KeychainManager, LaunchctlManager, OsascriptBridge,
    ScreenCapture,
};
pub use message_bus::{Message, MessageBus, MessageKind};
pub use pipeline::{BuildPhase, BuildPipeline};
pub use self_healer::{HealingLayer, HealingResult, SelfHealer};
pub use task_graph::{Task, TaskGraph, TaskStatus};
pub use zero_footprint::{
    CleanupResult, DiskPolicy, RuntimeSession, SecureBuffer, SecureString, SessionGuard,
    SessionSecrets, StartupValidator, ValidationReport, Violation, ZeroFootprintError,
};
