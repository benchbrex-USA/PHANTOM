//! Ollama API client — local LLM backend for Phantom agents.
//!
//! Mirrors the `AnthropicClient` API shape so the orchestrator can use either
//! backend transparently via `AiBackend`.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::agents::AgentRole;
use crate::client::{
    backoff_delay, is_retryable, CompletionRequest, CompletionResponse, ContentBlock, TokenUsage,
    UsageInfo,
};
use crate::errors::AiError;

/// Default Ollama API base URL.
const DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// Default model for code generation.
const DEFAULT_MODEL: &str = "deepseek-coder-v2:16b";

/// Ollama API client for local LLM inference.
pub struct OllamaClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
    max_retries: u32,
    token_usage: HashMap<String, TokenUsage>,
}

/// Ollama chat request body.
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

/// Ollama chat response.
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaResponseMessage,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
    #[serde(default)]
    eval_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

impl OllamaClient {
    /// Create a new Ollama client with default settings.
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(600)) // Local models can be slow
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            max_retries: 3,
            token_usage: HashMap::new(),
        }
    }

    /// Set the base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the model name.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Check if the Ollama server is reachable.
    pub async fn is_available(&self) -> bool {
        self.http
            .get(format!("{}/api/tags", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Send a completion request with retry logic.
    pub async fn complete(
        &mut self,
        request: &CompletionRequest,
        agent_id: &str,
    ) -> Result<CompletionResponse, AiError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay = backoff_delay(attempt);
                debug!(attempt, delay_ms = delay.as_millis(), "retrying after backoff");
                tokio::time::sleep(delay).await;
            }

            match self.send_request(request).await {
                Ok(response) => {
                    let usage = self.token_usage.entry(agent_id.to_string()).or_default();
                    usage.record(response.usage.input_tokens, response.usage.output_tokens);

                    debug!(
                        agent = agent_id,
                        input_tokens = response.usage.input_tokens,
                        output_tokens = response.usage.output_tokens,
                        "ollama completion successful"
                    );
                    return Ok(response);
                }
                Err(e) => {
                    if is_retryable(&e) && attempt < self.max_retries {
                        warn!(attempt, error = %e, "retryable error, will retry");
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AiError::RequestFailed("max retries exceeded".into())))
    }

    /// Send a single request to Ollama (no retry).
    async fn send_request(
        &self,
        request: &CompletionRequest,
    ) -> Result<CompletionResponse, AiError> {
        // Build Ollama message array
        let mut messages = Vec::new();

        // System message first
        if let Some(ref system) = request.system {
            messages.push(OllamaMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Conversation messages
        for msg in &request.messages {
            messages.push(OllamaMessage {
                role: match msg.role {
                    crate::client::MessageRole::User => "user".to_string(),
                    crate::client::MessageRole::Assistant => "assistant".to_string(),
                },
                content: msg.content.clone(),
            });
        }

        let ollama_request = OllamaChatRequest {
            model: self.model.clone(), // Always use our configured model, ignore request.model
            messages,
            stream: false,
            options: Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: Some(request.max_tokens),
            }),
        };

        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&ollama_request)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::RequestFailed(format!(
                "Ollama HTTP {}: {}",
                status, body
            )));
        }

        let ollama_response: OllamaChatResponse = resp.json().await.map_err(|e| {
            AiError::InvalidResponse(format!("failed to parse Ollama response: {}", e))
        })?;

        // Map to CompletionResponse
        let input_tokens = ollama_response.prompt_eval_count.unwrap_or(0);
        let output_tokens = ollama_response.eval_count.unwrap_or(0);

        Ok(CompletionResponse {
            id: format!("ollama-{}", uuid::Uuid::new_v4()),
            msg_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![ContentBlock {
                block_type: "text".to_string(),
                text: Some(ollama_response.message.content),
                id: None,
                name: None,
                input: None,
            }],
            model: ollama_response.model,
            stop_reason: Some("end_turn".to_string()),
            usage: UsageInfo {
                input_tokens,
                output_tokens,
            },
        })
    }

    /// Simple single-turn completion (convenience method).
    pub async fn ask(
        &mut self,
        role: AgentRole,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, AiError> {
        let request = CompletionRequest {
            model: self.model.clone(),
            messages: vec![crate::client::Message::user(user_message)],
            system: Some(system_prompt.to_string()),
            max_tokens: role.max_tokens(),
            temperature: Some(role.temperature()),
            stop_sequences: None,
            tools: None,
        };

        let response = self.complete(&request, role.id()).await?;

        response
            .content
            .into_iter()
            .filter_map(|block| block.text)
            .next()
            .ok_or_else(|| AiError::InvalidResponse("no text content in response".into()))
    }

    /// Get token usage for a specific agent.
    pub fn get_usage(&self, agent_id: &str) -> Option<&TokenUsage> {
        self.token_usage.get(agent_id)
    }

    /// Get token usage for all agents.
    pub fn all_usage(&self) -> &HashMap<String, TokenUsage> {
        &self.token_usage
    }

    /// Total tokens used across all agents.
    pub fn total_tokens_used(&self) -> u64 {
        self.token_usage.values().map(|u| u.total_tokens).sum()
    }

    /// Reset usage counters.
    pub fn reset_usage(&mut self) {
        self.token_usage.clear();
    }
}
