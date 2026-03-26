//! Unified AI backend — routes requests to Anthropic API or local Ollama.

use std::collections::HashMap;

use crate::agents::AgentRole;
use crate::client::{AnthropicClient, CompletionRequest, CompletionResponse, TokenUsage};
use crate::errors::AiError;
use crate::ollama::OllamaClient;

/// Unified AI backend that can use either Anthropic's API or a local Ollama instance.
pub enum AiBackend {
    Anthropic(AnthropicClient),
    Ollama(OllamaClient),
}

impl AiBackend {
    /// Auto-detect the best available backend.
    ///
    /// Tries `ANTHROPIC_API_KEY` first, falls back to Ollama.
    pub fn auto_detect() -> Result<Self, AiError> {
        match AnthropicClient::from_env() {
            Ok(client) => Ok(Self::Anthropic(client)),
            Err(_) => Ok(Self::Ollama(OllamaClient::new())),
        }
    }

    /// Create an Anthropic backend with the given API key.
    pub fn anthropic(api_key: impl Into<String>) -> Self {
        Self::Anthropic(AnthropicClient::new(api_key))
    }

    /// Create an Ollama backend with default settings.
    pub fn ollama() -> Self {
        Self::Ollama(OllamaClient::new())
    }

    /// Human-readable backend name.
    pub fn backend_name(&self) -> &str {
        match self {
            Self::Anthropic(_) => "Anthropic API",
            Self::Ollama(_) => "Ollama (local)",
        }
    }

    /// Send a completion request.
    pub async fn complete(
        &mut self,
        request: &CompletionRequest,
        agent_id: &str,
    ) -> Result<CompletionResponse, AiError> {
        match self {
            Self::Anthropic(c) => c.complete(request, agent_id).await,
            Self::Ollama(c) => c.complete(request, agent_id).await,
        }
    }

    /// Simple single-turn completion.
    pub async fn ask(
        &mut self,
        role: AgentRole,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, AiError> {
        match self {
            Self::Anthropic(c) => c.ask(role, system_prompt, user_message).await,
            Self::Ollama(c) => c.ask(role, system_prompt, user_message).await,
        }
    }

    /// Get token usage for all agents (returns owned HashMap since we delegate).
    pub fn all_usage(&self) -> HashMap<String, TokenUsage> {
        match self {
            Self::Anthropic(c) => c.all_usage().clone(),
            Self::Ollama(c) => c.all_usage().clone(),
        }
    }

    /// Total tokens used across all agents.
    pub fn total_tokens_used(&self) -> u64 {
        match self {
            Self::Anthropic(c) => c.total_tokens_used(),
            Self::Ollama(c) => c.total_tokens_used(),
        }
    }

    /// Reset usage counters.
    pub fn reset_usage(&mut self) {
        match self {
            Self::Anthropic(c) => c.reset_usage(),
            Self::Ollama(c) => c.reset_usage(),
        }
    }
}
