//! Phantom AI: Anthropic client, agent prompts, context management.
//!
//! 8-agent team: CTO, Architect, Backend, Frontend, DevOps, QA, Security, Monitor.
//! Each agent has a specific model, temperature, knowledge scope, and token budget.
//! CTO Agent orchestrates all specialist agents.

pub mod agents;
pub mod client;
pub mod context;
pub mod errors;
pub mod prompts;

pub use agents::{AgentConfig, AgentRole, ALL_ROLES};
pub use client::{
    AnthropicClient, CompletionRequest, CompletionResponse, CostEstimate, Message, TokenUsage,
};
pub use context::{ContextManager, ContextUsage, KnowledgeChunk};
pub use errors::AiError;
pub use prompts::{agent_system_prompt, task_prompt};
