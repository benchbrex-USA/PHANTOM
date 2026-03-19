//! Phantom AI: Anthropic client, agent prompts, context management.
//!
//! 8-agent team: CTO, Architect, Backend, Frontend, DevOps, QA, Security, Monitor.
//! Each agent has a specific model, temperature, knowledge scope, and token budget.
//! CTO Agent orchestrates all specialist agents.

pub mod agents;
pub mod claude_md;
pub mod client;
pub mod context;
pub mod errors;
pub mod orchestrator;
pub mod prompts;

pub use agents::{AgentConfig, AgentRole, ALL_ROLES};
pub use claude_md::{
    ClaudeMdError, ClaudeMdGenerator, GeneratedClaudeMd, TemplateVars,
    cleanup_all as cleanup_claude_mds, generate_team_claude_mds,
};
pub use client::{
    AnthropicClient, CompletionRequest, CompletionResponse, CostEstimate, Message, TokenUsage,
};
pub use context::{ContextManager, ContextUsage, KnowledgeChunk};
pub use errors::AiError;
pub use orchestrator::{
    AgentOrchestrator, AgentOutput, DelegationRequest, DelegationResult, OrchestratorConfig,
    OrchestratorHandle, OrchestratorUsage, PipelineBridge, PipelineTaskResult, TaskRequest,
};
pub use prompts::{agent_system_prompt, task_prompt};
