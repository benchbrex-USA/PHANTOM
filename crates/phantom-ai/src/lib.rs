//! Phantom AI: Anthropic client, agent prompts, context management.
//!
//! 8-agent team: CTO, Architect, Backend, Frontend, DevOps, QA, Security, Monitor.
//! Each agent has a specific model, temperature, knowledge scope, and token budget.
//! CTO Agent orchestrates all specialist agents.

pub mod client;
pub mod agents;
pub mod prompts;
pub mod context;
pub mod errors;

pub use errors::AiError;
pub use agents::{AgentRole, AgentConfig, ALL_ROLES};
pub use client::{AnthropicClient, CompletionRequest, CompletionResponse, Message, TokenUsage, CostEstimate};
pub use context::{ContextManager, ContextUsage, KnowledgeChunk};
pub use prompts::{agent_system_prompt, task_prompt};
