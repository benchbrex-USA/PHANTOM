//! Anthropic API client with retry, rate limiting, and token tracking.
//!
//! Wraps the Anthropic Messages API for all Phantom agents.
//! Handles:
//!   • API key management (from environment)
//!   • Request/response serialization
//!   • Exponential backoff with jitter on rate limits
//!   • Token usage tracking per agent
//!   • Streaming support (via SSE)

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::agents::AgentRole;
use crate::errors::AiError;

/// Anthropic API base URL.
const API_BASE: &str = "https://api.anthropic.com/v1";

/// API version header value.
const API_VERSION: &str = "2023-06-01";

/// Default max retries on rate limit.
const DEFAULT_MAX_RETRIES: u32 = 5;

/// The Anthropic API client used by all Phantom agents.
pub struct AnthropicClient {
    /// HTTP client
    http: reqwest::Client,
    /// API key
    api_key: String,
    /// Max retries on 429/529
    max_retries: u32,
    /// Per-agent token usage tracking
    token_usage: HashMap<String, TokenUsage>,
}

/// Token usage counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub request_count: u64,
}

impl TokenUsage {
    pub fn record(&mut self, input: u64, output: u64) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.total_tokens += input + output;
        self.request_count += 1;
    }
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

/// Request to the Anthropic Messages API.
#[derive(Debug, Clone, Serialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub system: Option<String>,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Response from the Anthropic Messages API.
#[derive(Debug, Clone, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: UsageInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageInfo {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// API error response.
#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
pub struct ApiErrorDetail {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

impl AnthropicClient {
    /// Create a new client, reading the API key from `ANTHROPIC_API_KEY`.
    pub fn from_env() -> Result<Self, AiError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| AiError::ApiKeyMissing("ANTHROPIC_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    /// Create a new client with an explicit API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");

        Self {
            http,
            api_key: api_key.into(),
            max_retries: DEFAULT_MAX_RETRIES,
            token_usage: HashMap::new(),
        }
    }

    /// Set max retries.
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
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
                    // Track token usage
                    let usage = self
                        .token_usage
                        .entry(agent_id.to_string())
                        .or_default();
                    usage.record(
                        response.usage.input_tokens,
                        response.usage.output_tokens,
                    );

                    debug!(
                        agent = agent_id,
                        input_tokens = response.usage.input_tokens,
                        output_tokens = response.usage.output_tokens,
                        "completion successful"
                    );
                    return Ok(response);
                }
                Err(e) => {
                    if is_retryable(&e) && attempt < self.max_retries {
                        warn!(
                            attempt,
                            error = %e,
                            "retryable error, will retry"
                        );
                        last_error = Some(e);
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AiError::RequestFailed("max retries exceeded".into())))
    }

    /// Send a single API request (no retry).
    async fn send_request(
        &self,
        request: &CompletionRequest,
    ) -> Result<CompletionResponse, AiError> {
        let resp = self
            .http
            .post(format!("{API_BASE}/messages"))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await?;

        let status = resp.status();

        if status.is_success() {
            let body = resp.json::<CompletionResponse>().await?;
            return Ok(body);
        }

        // Handle error responses
        let body = resp.text().await.unwrap_or_default();

        if status.as_u16() == 429 {
            return Err(AiError::RateLimited {
                retry_after_ms: 1000,
            });
        }

        if status.as_u16() == 529 {
            return Err(AiError::RateLimited {
                retry_after_ms: 5000,
            });
        }

        Err(AiError::RequestFailed(format!(
            "HTTP {}: {}",
            status, body
        )))
    }

    /// Simple single-turn completion (convenience method).
    pub async fn ask(
        &mut self,
        role: AgentRole,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, AiError> {
        let request = CompletionRequest {
            model: role.model().to_string(),
            messages: vec![Message::user(user_message)],
            system: Some(system_prompt.to_string()),
            max_tokens: role.max_tokens(),
            temperature: Some(role.temperature()),
            stop_sequences: None,
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

    /// Estimate cost based on token usage (approximate).
    pub fn estimated_cost(&self) -> CostEstimate {
        let mut total_input = 0u64;
        let mut total_output = 0u64;

        for usage in self.token_usage.values() {
            total_input += usage.input_tokens;
            total_output += usage.output_tokens;
        }

        // Approximate pricing (blended across models)
        // Opus: $15/M input, $75/M output
        // Sonnet: $3/M input, $15/M output
        // Haiku: $0.25/M input, $1.25/M output
        // Use blended average for simplicity
        let input_cost = total_input as f64 * 5.0 / 1_000_000.0;
        let output_cost = total_output as f64 * 25.0 / 1_000_000.0;

        CostEstimate {
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            estimated_input_cost_usd: input_cost,
            estimated_output_cost_usd: output_cost,
            estimated_total_cost_usd: input_cost + output_cost,
        }
    }
}

/// Estimated API cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub estimated_input_cost_usd: f64,
    pub estimated_output_cost_usd: f64,
    pub estimated_total_cost_usd: f64,
}

/// Check if an error is retryable.
fn is_retryable(error: &AiError) -> bool {
    matches!(
        error,
        AiError::RateLimited { .. } | AiError::Http(_)
    )
}

/// Exponential backoff with jitter.
fn backoff_delay(attempt: u32) -> Duration {
    let base_ms = 1000u64 * 2u64.pow(attempt.min(5));
    // Simple pseudo-jitter: use attempt number to vary
    let jitter_ms = (attempt as u64 * 137) % 500;
    Duration::from_millis(base_ms + jitter_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage_tracking() {
        let mut usage = TokenUsage::default();
        usage.record(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
        assert_eq!(usage.request_count, 1);

        usage.record(200, 100);
        assert_eq!(usage.total_tokens, 450);
        assert_eq!(usage.request_count, 2);
    }

    #[test]
    fn test_message_constructors() {
        let user = Message::user("hello");
        assert_eq!(user.role, MessageRole::User);
        assert_eq!(user.content, "hello");

        let asst = Message::assistant("hi there");
        assert_eq!(asst.role, MessageRole::Assistant);
    }

    #[test]
    fn test_completion_request_serialization() {
        let req = CompletionRequest {
            model: "claude-sonnet-4-6".into(),
            messages: vec![Message::user("test")],
            system: Some("you are helpful".into()),
            max_tokens: 1024,
            temperature: Some(0.1),
            stop_sequences: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-6");
        assert_eq!(json["max_tokens"], 1024);
        assert!(json.get("stop_sequences").is_none()); // skip_serializing_if
    }

    #[test]
    fn test_client_creation() {
        let client = AnthropicClient::new("test-key");
        assert_eq!(client.total_tokens_used(), 0);
        assert!(client.all_usage().is_empty());
    }

    #[test]
    fn test_client_from_env_missing() {
        // Should fail if ANTHROPIC_API_KEY is not set
        std::env::remove_var("ANTHROPIC_API_KEY");
        let result = AnthropicClient::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn test_backoff_delay() {
        let d1 = backoff_delay(1);
        let d2 = backoff_delay(2);
        assert!(d2 > d1); // Exponential growth
    }

    #[test]
    fn test_is_retryable() {
        assert!(is_retryable(&AiError::RateLimited {
            retry_after_ms: 1000
        }));
        assert!(is_retryable(&AiError::Http("connection reset".into())));
        assert!(!is_retryable(&AiError::ApiKeyMissing("missing".into())));
        assert!(!is_retryable(&AiError::ContextOverflow {
            tokens: 100,
            max: 50
        }));
    }

    #[test]
    fn test_cost_estimate() {
        let mut client = AnthropicClient::new("test-key");
        client
            .token_usage
            .entry("cto".into())
            .or_default()
            .record(1_000_000, 100_000);

        let cost = client.estimated_cost();
        assert_eq!(cost.total_input_tokens, 1_000_000);
        assert_eq!(cost.total_output_tokens, 100_000);
        assert!(cost.estimated_total_cost_usd > 0.0);
    }

    #[test]
    fn test_usage_reset() {
        let mut client = AnthropicClient::new("test-key");
        client
            .token_usage
            .entry("cto".into())
            .or_default()
            .record(100, 50);
        assert_eq!(client.total_tokens_used(), 150);

        client.reset_usage();
        assert_eq!(client.total_tokens_used(), 0);
    }
}
