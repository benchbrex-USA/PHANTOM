//! Multi-provider LLM abstraction layer.
//!
//! Defines a unified `LlmProvider` trait and concrete implementations for:
//!   - **AnthropicProvider** — wraps the existing `AnthropicClient`
//!   - **OllamaProvider** — local inference via Ollama's OpenAI-compatible endpoint
//!   - **OpenRouterProvider** — free/paid open-source models via OpenRouter
//!   - **OpenAiCompatibleProvider** — any OpenAI-compatible API (vLLM, llama.cpp, LM Studio, TGI)
//!
//! All providers use connection-pooled `reqwest::Client`, map errors to `AiError`,
//! support both sync and streaming completions, and track token usage in `UnifiedResponse`.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::client::{
    AnthropicClient, CompletionRequest, Message, MessageRole, StreamCallback,
};
use crate::errors::AiError;

// ── Unified types ───────────────────────────────────────────────────────

/// Callback invoked on each streamed token.
pub type TokenCallback = Box<dyn Fn(&str) + Send + Sync>;

/// A message in a provider-agnostic conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMessage {
    pub role: UnifiedRole,
    pub content: String,
}

/// Message role, including system (which some providers handle separately).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnifiedRole {
    System,
    User,
    Assistant,
}

impl fmt::Display for UnifiedRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::System => write!(f, "system"),
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
        }
    }
}

/// Provider-agnostic completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedRequest {
    /// Model identifier (provider-specific, e.g. "claude-sonnet-4-6", "llama3.1", "meta-llama/llama-3.1-8b-instruct:free").
    pub model: String,
    /// Conversation messages. System messages are extracted by providers that need a separate system field.
    pub messages: Vec<UnifiedMessage>,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Sampling temperature (0.0 - 2.0).
    pub temperature: Option<f32>,
    /// Stop sequences.
    pub stop_sequences: Option<Vec<String>>,
}

impl UnifiedRequest {
    /// Create a simple single-turn request.
    pub fn simple(model: impl Into<String>, system: &str, user_message: &str, max_tokens: u32) -> Self {
        let mut messages = Vec::new();
        if !system.is_empty() {
            messages.push(UnifiedMessage {
                role: UnifiedRole::System,
                content: system.to_string(),
            });
        }
        messages.push(UnifiedMessage {
            role: UnifiedRole::User,
            content: user_message.to_string(),
        });
        Self {
            model: model.into(),
            messages,
            max_tokens,
            temperature: None,
            stop_sequences: None,
        }
    }

    /// Extract system messages (concatenated) and non-system messages.
    fn split_system(&self) -> (Option<String>, Vec<&UnifiedMessage>) {
        let system_parts: Vec<&str> = self
            .messages
            .iter()
            .filter(|m| m.role == UnifiedRole::System)
            .map(|m| m.content.as_str())
            .collect();

        let others: Vec<&UnifiedMessage> = self
            .messages
            .iter()
            .filter(|m| m.role != UnifiedRole::System)
            .collect();

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        (system, others)
    }
}

/// Provider-agnostic completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedResponse {
    /// Provider-assigned request/message ID.
    pub id: String,
    /// Model that actually handled the request.
    pub model: String,
    /// Generated text content.
    pub content: String,
    /// Why generation stopped (e.g. "end_turn", "stop", "length").
    pub stop_reason: Option<String>,
    /// Token usage tracking.
    pub usage: UnifiedUsage,
    /// Name of the provider that served this request.
    pub provider: String,
}

/// Token usage counters for a single request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnifiedUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

/// Metadata about a model supported by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier used in API requests.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Context window size in tokens (0 if unknown).
    pub context_window: u64,
    /// Whether the model is free to use.
    pub is_free: bool,
}

// ── LlmProvider trait ───────────────────────────────────────────────────

/// Trait for any LLM provider backend.
///
/// Implementations must be `Send + Sync` for use across async tasks.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &str;

    /// Whether the provider is currently available (key set, server reachable, etc.).
    fn is_available(&self) -> bool;

    /// Send a completion request and return the full response.
    async fn complete(&self, request: &UnifiedRequest) -> Result<UnifiedResponse, AiError>;

    /// Send a streaming completion request, calling `on_token` for each generated token.
    /// Returns the accumulated response when the stream completes.
    async fn stream_complete(
        &self,
        request: &UnifiedRequest,
        on_token: Option<TokenCallback>,
    ) -> Result<UnifiedResponse, AiError>;

    /// Lightweight health check — returns `true` if the provider can serve requests.
    async fn health_check(&self) -> bool;

    /// List models this provider supports.
    fn supported_models(&self) -> Vec<ModelInfo>;
}

// ── AnthropicProvider ───────────────────────────────────────────────────

/// Wraps the existing `AnthropicClient` as an `LlmProvider`.
pub struct AnthropicProvider {
    client: Arc<Mutex<AnthropicClient>>,
    available: bool,
}

impl AnthropicProvider {
    /// Create from environment (`ANTHROPIC_API_KEY`).
    pub fn from_env() -> Result<Self, AiError> {
        let client = AnthropicClient::from_env()?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            available: true,
        })
    }

    /// Create with an explicit API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Arc::new(Mutex::new(AnthropicClient::new(api_key))),
            available: true,
        }
    }

    /// Convert unified messages to Anthropic `CompletionRequest`.
    fn to_completion_request(request: &UnifiedRequest) -> CompletionRequest {
        let (system, others) = request.split_system();
        let messages: Vec<Message> = others
            .into_iter()
            .map(|m| Message {
                role: match m.role {
                    UnifiedRole::User | UnifiedRole::System => MessageRole::User,
                    UnifiedRole::Assistant => MessageRole::Assistant,
                },
                content: m.content.clone(),
            })
            .collect();

        CompletionRequest {
            model: request.model.clone(),
            messages,
            system,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stop_sequences: request.stop_sequences.clone(),
            tools: None,
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    async fn complete(&self, request: &UnifiedRequest) -> Result<UnifiedResponse, AiError> {
        let comp_req = Self::to_completion_request(request);
        let mut client = self.client.lock().await;
        let resp = client.complete(&comp_req, "provider").await?;

        let content = resp
            .content
            .into_iter()
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(UnifiedResponse {
            id: resp.id,
            model: resp.model,
            content,
            stop_reason: resp.stop_reason,
            usage: UnifiedUsage {
                input_tokens: resp.usage.input_tokens,
                output_tokens: resp.usage.output_tokens,
                total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
            },
            provider: "Anthropic".to_string(),
        })
    }

    async fn stream_complete(
        &self,
        request: &UnifiedRequest,
        on_token: Option<TokenCallback>,
    ) -> Result<UnifiedResponse, AiError> {
        let comp_req = Self::to_completion_request(request);

        // Wrap the token callback into the Anthropic StreamCallback shape.
        let stream_cb: Option<StreamCallback> = on_token.map(|cb| {
            let cb: StreamCallback = Box::new(move |event| {
                if let crate::client::StreamEvent::ContentDelta { text, .. } = &event {
                    cb(text);
                }
            });
            cb
        });

        let mut client = self.client.lock().await;
        let result = client
            .stream_complete(&comp_req, "provider", stream_cb)
            .await?;

        Ok(UnifiedResponse {
            id: result.id,
            model: result.model,
            content: result.text,
            stop_reason: result.stop_reason,
            usage: UnifiedUsage {
                input_tokens: result.input_tokens,
                output_tokens: result.output_tokens,
                total_tokens: result.input_tokens + result.output_tokens,
            },
            provider: "Anthropic".to_string(),
        })
    }

    async fn health_check(&self) -> bool {
        // If we have a valid API key, assume healthy (actual check would cost tokens).
        self.available
    }

    fn supported_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "claude-opus-4-6".to_string(),
                name: "Claude Opus 4".to_string(),
                context_window: 200_000,
                is_free: false,
            },
            ModelInfo {
                id: "claude-sonnet-4-6".to_string(),
                name: "Claude Sonnet 4".to_string(),
                context_window: 200_000,
                is_free: false,
            },
            ModelInfo {
                id: "claude-haiku-4-5-20251001".to_string(),
                name: "Claude Haiku 4.5".to_string(),
                context_window: 200_000,
                is_free: false,
            },
        ]
    }
}

// ── OpenAI-compatible request/response types (shared by Ollama, OpenRouter, generic) ─

/// OpenAI chat completion request body.
#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

/// OpenAI chat completion response.
#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    id: Option<String>,
    model: Option<String>,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiResponseMessage>,
    #[allow(dead_code)]
    delta: Option<OpenAiResponseMessage>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

/// Convert unified messages to OpenAI message format.
fn to_openai_messages(request: &UnifiedRequest) -> Vec<OpenAiMessage> {
    request
        .messages
        .iter()
        .map(|m| OpenAiMessage {
            role: m.role.to_string(),
            content: m.content.clone(),
        })
        .collect()
}

/// Build an `OpenAiChatRequest` from a `UnifiedRequest`.
fn build_openai_request(request: &UnifiedRequest, stream: bool) -> OpenAiChatRequest {
    OpenAiChatRequest {
        model: request.model.clone(),
        messages: to_openai_messages(request),
        max_tokens: Some(request.max_tokens),
        temperature: request.temperature,
        stop: request.stop_sequences.clone(),
        stream,
    }
}

/// Parse an OpenAI-format non-streaming response into `UnifiedResponse`.
fn parse_openai_response(
    body: OpenAiChatResponse,
    provider_name: &str,
) -> Result<UnifiedResponse, AiError> {
    let choice = body
        .choices
        .first()
        .ok_or_else(|| AiError::InvalidResponse("no choices in response".into()))?;

    let content = choice
        .message
        .as_ref()
        .and_then(|m| m.content.clone())
        .unwrap_or_default();

    let usage = body.usage.as_ref();

    Ok(UnifiedResponse {
        id: body.id.unwrap_or_default(),
        model: body.model.unwrap_or_default(),
        content,
        stop_reason: choice.finish_reason.clone(),
        usage: UnifiedUsage {
            input_tokens: usage.and_then(|u| u.prompt_tokens).unwrap_or(0),
            output_tokens: usage.and_then(|u| u.completion_tokens).unwrap_or(0),
            total_tokens: usage.and_then(|u| u.total_tokens).unwrap_or(0),
        },
        provider: provider_name.to_string(),
    })
}

/// Parse SSE stream from an OpenAI-compatible endpoint into a `UnifiedResponse`.
/// Calls `on_token` for each content delta.
async fn parse_openai_stream(
    resp: reqwest::Response,
    model: &str,
    provider_name: &str,
    on_token: Option<TokenCallback>,
) -> Result<UnifiedResponse, AiError> {
    let body = resp.text().await?;
    let mut full_content = String::new();
    let mut finish_reason: Option<String> = None;
    let mut resp_id = String::new();
    let mut resp_model = model.to_string();

    for line in body.lines() {
        let data = match line.strip_prefix("data: ") {
            Some(d) if d.trim() != "[DONE]" => d,
            _ => continue,
        };

        let chunk: serde_json::Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(id) = chunk["id"].as_str() {
            resp_id = id.to_string();
        }
        if let Some(m) = chunk["model"].as_str() {
            resp_model = m.to_string();
        }

        if let Some(choices) = chunk["choices"].as_array() {
            for choice in choices {
                if let Some(delta_content) = choice["delta"]["content"].as_str() {
                    full_content.push_str(delta_content);
                    if let Some(ref cb) = on_token {
                        cb(delta_content);
                    }
                }
                if let Some(fr) = choice["finish_reason"].as_str() {
                    finish_reason = Some(fr.to_string());
                }
            }
        }

        // Some providers include usage in the final chunk.
        if let Some(usage) = chunk.get("usage") {
            let input = usage["prompt_tokens"].as_u64().unwrap_or(0);
            let output = usage["completion_tokens"].as_u64().unwrap_or(0);
            return Ok(UnifiedResponse {
                id: resp_id,
                model: resp_model,
                content: full_content,
                stop_reason: finish_reason,
                usage: UnifiedUsage {
                    input_tokens: input,
                    output_tokens: output,
                    total_tokens: input + output,
                },
                provider: provider_name.to_string(),
            });
        }
    }

    // If no usage was provided in the stream, estimate from content length.
    Ok(UnifiedResponse {
        id: resp_id,
        model: resp_model,
        content: full_content,
        stop_reason: finish_reason,
        usage: UnifiedUsage::default(),
        provider: provider_name.to_string(),
    })
}

// ── OllamaProvider ──────────────────────────────────────────────────────

/// Provider for local Ollama inference via the OpenAI-compatible `/v1/chat/completions` endpoint.
///
/// Supports automatic model pulling when a requested model is not found locally.
pub struct OllamaProvider {
    http: reqwest::Client,
    base_url: String,
}

impl OllamaProvider {
    /// Create with the default Ollama URL (`http://localhost:11434`).
    pub fn new() -> Self {
        Self::with_base_url("http://localhost:11434")
    }

    /// Create with a custom base URL.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    /// Pull a model if it is not already available locally.
    /// This is a blocking operation that streams the pull progress.
    pub async fn pull_model(&self, model: &str) -> Result<(), AiError> {
        debug!(model, "pulling model via Ollama");
        let resp = self
            .http
            .post(format!("{}/api/pull", self.base_url))
            .json(&serde_json::json!({ "name": model }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::RequestFailed(format!(
                "Ollama pull failed: {}",
                body
            )));
        }

        // Consume the streaming response (ndjson progress lines).
        let _ = resp.text().await;
        debug!(model, "model pull complete");
        Ok(())
    }

    /// List locally available models from Ollama.
    pub async fn list_local_models(&self) -> Result<Vec<String>, AiError> {
        let resp = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AiError::RequestFailed("failed to list Ollama models".into()));
        }

        let body: serde_json::Value = resp.json().await?;
        let models = body["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }

    /// Send a request, pulling the model automatically on 404.
    async fn send_with_auto_pull(
        &self,
        request: &UnifiedRequest,
        stream: bool,
    ) -> Result<reqwest::Response, AiError> {
        let oai_req = build_openai_request(request, stream);
        let url = format!("{}/v1/chat/completions", self.base_url);

        let resp = self
            .http
            .post(&url)
            .json(&oai_req)
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 404 || status.as_u16() == 400 {
            // Model likely not found — attempt to pull it.
            warn!(model = %request.model, "model not found locally, attempting pull");
            self.pull_model(&request.model).await?;

            // Retry after pull.
            let oai_req = build_openai_request(request, stream);
            let resp = self
                .http
                .post(&url)
                .json(&oai_req)
                .send()
                .await?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(AiError::RequestFailed(format!(
                    "Ollama HTTP {} after pull: {}",
                    status, body
                )));
            }
            return Ok(resp);
        }

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::RequestFailed(format!(
                "Ollama HTTP {}: {}",
                status, body
            )));
        }

        Ok(resp)
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama"
    }

    fn is_available(&self) -> bool {
        // Synchronous check — we assume available. Use `health_check()` for async verification.
        true
    }

    async fn complete(&self, request: &UnifiedRequest) -> Result<UnifiedResponse, AiError> {
        let resp = self.send_with_auto_pull(request, false).await?;
        let body: OpenAiChatResponse = resp.json().await.map_err(|e| {
            AiError::InvalidResponse(format!("failed to parse Ollama response: {}", e))
        })?;
        parse_openai_response(body, "Ollama")
    }

    async fn stream_complete(
        &self,
        request: &UnifiedRequest,
        on_token: Option<TokenCallback>,
    ) -> Result<UnifiedResponse, AiError> {
        let resp = self.send_with_auto_pull(request, true).await?;
        parse_openai_stream(resp, &request.model, "Ollama", on_token).await
    }

    async fn health_check(&self) -> bool {
        self.http
            .get(format!("{}/api/tags", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    fn supported_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "llama3.1".to_string(),
                name: "Llama 3.1 8B".to_string(),
                context_window: 128_000,
                is_free: true,
            },
            ModelInfo {
                id: "codellama".to_string(),
                name: "Code Llama".to_string(),
                context_window: 16_384,
                is_free: true,
            },
            ModelInfo {
                id: "mistral".to_string(),
                name: "Mistral 7B".to_string(),
                context_window: 32_768,
                is_free: true,
            },
            ModelInfo {
                id: "deepseek-coder".to_string(),
                name: "DeepSeek Coder".to_string(),
                context_window: 16_384,
                is_free: true,
            },
            ModelInfo {
                id: "phi3".to_string(),
                name: "Phi-3".to_string(),
                context_window: 128_000,
                is_free: true,
            },
        ]
    }
}

// ── OpenRouterProvider ──────────────────────────────────────────────────

/// Provider for the OpenRouter API, which gives access to many open-source models
/// (including free tiers).
///
/// Reads `OPENROUTER_API_KEY` from the environment.
pub struct OpenRouterProvider {
    http: reqwest::Client,
    api_key: String,
}

impl OpenRouterProvider {
    /// API base URL.
    const BASE_URL: &'static str = "https://openrouter.ai/api/v1";

    /// Create from the `OPENROUTER_API_KEY` environment variable.
    pub fn from_env() -> Result<Self, AiError> {
        let api_key = std::env::var("OPENROUTER_API_KEY").map_err(|_| {
            AiError::ApiKeyMissing("OPENROUTER_API_KEY not set".into())
        })?;
        Ok(Self::new(api_key))
    }

    /// Create with an explicit API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http,
            api_key: api_key.into(),
        }
    }

    /// Send a request to the OpenRouter API.
    async fn send_request(
        &self,
        request: &UnifiedRequest,
        stream: bool,
    ) -> Result<reqwest::Response, AiError> {
        let oai_req = build_openai_request(request, stream);

        let resp = self
            .http
            .post(format!("{}/chat/completions", Self::BASE_URL))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/phantom")
            .header("X-Title", "Phantom AI")
            .json(&oai_req)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(AiError::RateLimited {
                    retry_after_ms: 1000,
                });
            }
            return Err(AiError::RequestFailed(format!(
                "OpenRouter HTTP {}: {}",
                status, body
            )));
        }

        Ok(resp)
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "OpenRouter"
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn complete(&self, request: &UnifiedRequest) -> Result<UnifiedResponse, AiError> {
        let resp = self.send_request(request, false).await?;
        let body: OpenAiChatResponse = resp.json().await.map_err(|e| {
            AiError::InvalidResponse(format!("failed to parse OpenRouter response: {}", e))
        })?;
        parse_openai_response(body, "OpenRouter")
    }

    async fn stream_complete(
        &self,
        request: &UnifiedRequest,
        on_token: Option<TokenCallback>,
    ) -> Result<UnifiedResponse, AiError> {
        let resp = self.send_request(request, true).await?;
        parse_openai_stream(resp, &request.model, "OpenRouter", on_token).await
    }

    async fn health_check(&self) -> bool {
        if self.api_key.is_empty() {
            return false;
        }
        // Lightweight check: hit the models endpoint.
        self.http
            .get(format!("{}/models", Self::BASE_URL))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    fn supported_models(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "meta-llama/llama-3.1-8b-instruct:free".to_string(),
                name: "Llama 3.1 8B Instruct (free)".to_string(),
                context_window: 128_000,
                is_free: true,
            },
            ModelInfo {
                id: "google/gemma-2-9b-it:free".to_string(),
                name: "Gemma 2 9B IT (free)".to_string(),
                context_window: 8_192,
                is_free: true,
            },
            ModelInfo {
                id: "mistralai/mistral-7b-instruct:free".to_string(),
                name: "Mistral 7B Instruct (free)".to_string(),
                context_window: 32_768,
                is_free: true,
            },
            ModelInfo {
                id: "qwen/qwen-2-7b-instruct:free".to_string(),
                name: "Qwen 2 7B Instruct (free)".to_string(),
                context_window: 32_768,
                is_free: true,
            },
        ]
    }
}

// ── OpenAiCompatibleProvider ────────────────────────────────────────────

/// Generic provider for any OpenAI-compatible API endpoint.
///
/// Works with vLLM, llama.cpp server, text-generation-inference, LM Studio,
/// and any other server implementing the OpenAI `/v1/chat/completions` API.
pub struct OpenAiCompatibleProvider {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    provider_name: String,
    models: Vec<ModelInfo>,
}

impl OpenAiCompatibleProvider {
    /// Create a new provider with a custom base URL and optional API key.
    ///
    /// # Arguments
    /// * `name` — Human-readable name for this provider instance (e.g. "vLLM", "LM Studio").
    /// * `base_url` — Base URL of the OpenAI-compatible API (e.g. `http://localhost:8000/v1`).
    /// * `api_key` — Optional API key for authentication.
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .pool_max_idle_per_host(4)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key,
            provider_name: name.into(),
            models: Vec::new(),
        }
    }

    /// Register known models for this provider.
    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        self.models = models;
        self
    }

    /// Send a request to the endpoint.
    async fn send_request(
        &self,
        request: &UnifiedRequest,
        stream: bool,
    ) -> Result<reqwest::Response, AiError> {
        let oai_req = build_openai_request(request, stream);

        let mut builder = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .json(&oai_req);

        if let Some(ref key) = self.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", key));
        }

        let resp = builder.send().await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(AiError::RateLimited {
                    retry_after_ms: 1000,
                });
            }
            return Err(AiError::RequestFailed(format!(
                "{} HTTP {}: {}",
                self.provider_name, status, body
            )));
        }

        Ok(resp)
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.provider_name
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn complete(&self, request: &UnifiedRequest) -> Result<UnifiedResponse, AiError> {
        let resp = self.send_request(request, false).await?;
        let body: OpenAiChatResponse = resp.json().await.map_err(|e| {
            AiError::InvalidResponse(format!(
                "failed to parse {} response: {}",
                self.provider_name, e
            ))
        })?;
        parse_openai_response(body, &self.provider_name)
    }

    async fn stream_complete(
        &self,
        request: &UnifiedRequest,
        on_token: Option<TokenCallback>,
    ) -> Result<UnifiedResponse, AiError> {
        let resp = self.send_request(request, true).await?;
        parse_openai_stream(resp, &request.model, &self.provider_name, on_token).await
    }

    async fn health_check(&self) -> bool {
        // Try the /models endpoint, fall back to a simple GET on base_url.
        let models_url = format!("{}/models", self.base_url);
        let mut builder = self.http.get(&models_url).timeout(Duration::from_secs(5));
        if let Some(ref key) = self.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", key));
        }

        builder
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    fn supported_models(&self) -> Vec<ModelInfo> {
        self.models.clone()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_request_simple() {
        let req = UnifiedRequest::simple("llama3.1", "You are helpful.", "Hello", 1024);
        assert_eq!(req.model, "llama3.1");
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, UnifiedRole::System);
        assert_eq!(req.messages[1].role, UnifiedRole::User);
    }

    #[test]
    fn test_unified_request_split_system() {
        let req = UnifiedRequest {
            model: "test".into(),
            messages: vec![
                UnifiedMessage {
                    role: UnifiedRole::System,
                    content: "Part 1".into(),
                },
                UnifiedMessage {
                    role: UnifiedRole::System,
                    content: "Part 2".into(),
                },
                UnifiedMessage {
                    role: UnifiedRole::User,
                    content: "Hello".into(),
                },
            ],
            max_tokens: 100,
            temperature: None,
            stop_sequences: None,
        };
        let (system, others) = req.split_system();
        assert_eq!(system, Some("Part 1\n\nPart 2".to_string()));
        assert_eq!(others.len(), 1);
        assert_eq!(others[0].role, UnifiedRole::User);
    }

    #[test]
    fn test_unified_request_no_system() {
        let req = UnifiedRequest {
            model: "test".into(),
            messages: vec![UnifiedMessage {
                role: UnifiedRole::User,
                content: "Hello".into(),
            }],
            max_tokens: 100,
            temperature: None,
            stop_sequences: None,
        };
        let (system, others) = req.split_system();
        assert!(system.is_none());
        assert_eq!(others.len(), 1);
    }

    #[test]
    fn test_unified_response_serialization() {
        let resp = UnifiedResponse {
            id: "test-123".into(),
            model: "llama3.1".into(),
            content: "Hello world".into(),
            stop_reason: Some("stop".into()),
            usage: UnifiedUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
            },
            provider: "Ollama".into(),
        };
        let json = serde_json::to_value(&resp).expect("serialization should succeed");
        assert_eq!(json["provider"], "Ollama");
        assert_eq!(json["usage"]["total_tokens"], 15);
    }

    #[test]
    fn test_model_info() {
        let info = ModelInfo {
            id: "llama3.1".into(),
            name: "Llama 3.1".into(),
            context_window: 128_000,
            is_free: true,
        };
        assert!(info.is_free);
        assert_eq!(info.context_window, 128_000);
    }

    #[test]
    fn test_openai_request_building() {
        let req = UnifiedRequest::simple("gpt-4", "system", "hello", 512);
        let oai = build_openai_request(&req, false);
        assert_eq!(oai.model, "gpt-4");
        assert_eq!(oai.messages.len(), 2);
        assert!(!oai.stream);
        assert_eq!(oai.max_tokens, Some(512));
    }

    #[test]
    fn test_parse_openai_response_ok() {
        let body = OpenAiChatResponse {
            id: Some("chatcmpl-123".into()),
            model: Some("llama3.1".into()),
            choices: vec![OpenAiChoice {
                message: Some(OpenAiResponseMessage {
                    content: Some("Hello!".into()),
                }),
                delta: None,
                finish_reason: Some("stop".into()),
            }],
            usage: Some(OpenAiUsage {
                prompt_tokens: Some(10),
                completion_tokens: Some(3),
                total_tokens: Some(13),
            }),
        };
        let resp = parse_openai_response(body, "test").expect("should parse");
        assert_eq!(resp.content, "Hello!");
        assert_eq!(resp.usage.total_tokens, 13);
        assert_eq!(resp.provider, "test");
    }

    #[test]
    fn test_parse_openai_response_empty_choices() {
        let body = OpenAiChatResponse {
            id: None,
            model: None,
            choices: vec![],
            usage: None,
        };
        let result = parse_openai_response(body, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::new("test-key");
        assert_eq!(provider.name(), "Anthropic");
        assert!(provider.is_available());
        assert_eq!(provider.supported_models().len(), 3);
    }

    #[test]
    fn test_anthropic_provider_from_env_missing() {
        std::env::remove_var("ANTHROPIC_API_KEY");
        let result = AnthropicProvider::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_ollama_provider_defaults() {
        let provider = OllamaProvider::new();
        assert_eq!(provider.name(), "Ollama");
        assert!(provider.is_available());
        assert!(!provider.supported_models().is_empty());
    }

    #[test]
    fn test_ollama_provider_custom_url() {
        let provider = OllamaProvider::with_base_url("http://gpu-server:11434");
        assert_eq!(provider.base_url, "http://gpu-server:11434");
    }

    #[test]
    fn test_openrouter_provider_from_env_missing() {
        std::env::remove_var("OPENROUTER_API_KEY");
        let result = OpenRouterProvider::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_openrouter_provider_creation() {
        let provider = OpenRouterProvider::new("test-key");
        assert_eq!(provider.name(), "OpenRouter");
        assert!(provider.is_available());
        let models = provider.supported_models();
        assert_eq!(models.len(), 4);
        assert!(models.iter().all(|m| m.is_free));
    }

    #[test]
    fn test_openai_compatible_provider() {
        let provider = OpenAiCompatibleProvider::new(
            "vLLM",
            "http://localhost:8000/v1",
            Some("sk-test".into()),
        );
        assert_eq!(provider.name(), "vLLM");
        assert!(provider.is_available());
        assert!(provider.supported_models().is_empty());
    }

    #[test]
    fn test_openai_compatible_provider_with_models() {
        let provider = OpenAiCompatibleProvider::new("LM Studio", "http://localhost:1234/v1", None)
            .with_models(vec![ModelInfo {
                id: "local-model".into(),
                name: "Local Model".into(),
                context_window: 4096,
                is_free: true,
            }]);
        assert_eq!(provider.supported_models().len(), 1);
    }

    #[test]
    fn test_openai_compatible_url_trailing_slash() {
        let provider =
            OpenAiCompatibleProvider::new("test", "http://localhost:8000/v1/", None);
        assert_eq!(provider.base_url, "http://localhost:8000/v1");
    }

    #[test]
    fn test_unified_role_display() {
        assert_eq!(UnifiedRole::System.to_string(), "system");
        assert_eq!(UnifiedRole::User.to_string(), "user");
        assert_eq!(UnifiedRole::Assistant.to_string(), "assistant");
    }

    #[test]
    fn test_unified_usage_default() {
        let usage = UnifiedUsage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_to_completion_request_conversion() {
        let req = UnifiedRequest::simple("claude-sonnet-4-6", "Be helpful", "Hi", 2048);
        let comp = AnthropicProvider::to_completion_request(&req);
        assert_eq!(comp.model, "claude-sonnet-4-6");
        assert_eq!(comp.system, Some("Be helpful".to_string()));
        assert_eq!(comp.messages.len(), 1);
        assert_eq!(comp.messages[0].role, MessageRole::User);
        assert_eq!(comp.max_tokens, 2048);
    }
}
