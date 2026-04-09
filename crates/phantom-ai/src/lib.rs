//! Phantom AI: Multi-provider LLM orchestration for autonomous AI engineering.
//!
//! 8-agent team: CTO, Architect, Backend, Frontend, DevOps, QA, Security, Monitor.
//! Each agent has a specific model, temperature, knowledge scope, and token budget.
//!
//! Supports: Anthropic Claude, Ollama (local/free), OpenRouter (free tier), any OpenAI-compatible API.
//! Smart routing sends each agent to the best available provider with automatic fallback.

pub mod agents;
pub mod backend;
pub mod claude_md;
pub mod client;
pub mod config;
pub mod context;
pub mod errors;
pub mod file_parser;
pub mod ollama;
pub mod orchestrator;
pub mod prompts;
pub mod provider;
pub mod router;
pub mod tools;

pub use agents::{AgentConfig, AgentRole, ALL_ROLES};
pub use backend::AiBackend;
pub use claude_md::{
    cleanup_all as cleanup_claude_mds, generate_team_claude_mds, ClaudeMdError, ClaudeMdGenerator,
    GeneratedClaudeMd, TemplateVars,
};
pub use client::{
    AnthropicClient, CompletionRequest, CompletionResponse, CostEstimate, Message, StreamCallback,
    StreamEvent, StreamResult, TokenUsage,
};
pub use context::{ContextManager, ContextUsage, KnowledgeChunk};
pub use errors::AiError;
pub use file_parser::{parse_file_output, ParsedFile};
pub use orchestrator::{
    AgentOrchestrator, AgentOutput, DelegationRequest, DelegationResult, OrchestratorConfig,
    OrchestratorHandle, OrchestratorUsage, PipelineBridge, PipelineTaskResult, TaskRequest,
};
pub use prompts::{agent_system_prompt, task_prompt};
pub use provider::{
    AnthropicProvider, LlmProvider, ModelInfo, OllamaProvider, OpenAiCompatibleProvider,
    OpenRouterProvider, TokenCallback, UnifiedRequest, UnifiedResponse, UnifiedRole, UnifiedUsage,
};
pub use config::{PhantomAiConfig, ProvidersConfig, RoutingConfig, PerformanceConfig, CostConfig};
pub use router::{ModelRouter, RouterConfig, RouterMetrics, ProviderStatus};
pub use tools::{
    execute_tool, execute_tool_calls, parse_tool_calls, ToolCall, ToolDefinition, ToolRegistry,
    ToolResult,
};
