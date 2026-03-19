use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("task already exists: {0}")]
    TaskAlreadyExists(String),

    #[error("agent {agent_id} failed: {reason}")]
    AgentFailed { agent_id: String, reason: String },

    #[error("agent not found: {0}")]
    AgentNotFound(String),

    #[error("task dependency cycle detected involving: {0}")]
    DependencyCycle(String),

    #[error("task {task_id} has unresolved dependencies: {deps:?}")]
    UnresolvedDependencies { task_id: String, deps: Vec<String> },

    #[error("self-healing exhausted all {layers} layers for task {task_id}")]
    SelfHealingExhausted { task_id: String, layers: u32 },

    #[error("message bus error: {0}")]
    MessageBus(String),

    #[error("audit log error: {0}")]
    AuditError(String),

    #[error("job queue error: {0}")]
    JobQueue(String),

    #[error("pipeline error in phase {phase}: {reason}")]
    PipelineError { phase: String, reason: String },

    #[error("operation cancelled: {0}")]
    Cancelled(String),

    #[error("emergency halt activated")]
    EmergencyHalt,

    #[error("invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
}
