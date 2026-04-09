use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("API request failed: {0}")]
    RequestFailed(String),

    #[error("rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("context window exceeded: {tokens} tokens (max: {max})")]
    ContextOverflow { tokens: usize, max: usize },

    #[error("agent timeout: {agent_id}")]
    AgentTimeout { agent_id: String },

    #[error("invalid response: {0}")]
    InvalidResponse(String),

    #[error("token budget exhausted for agent {agent_id}")]
    TokenBudgetExhausted { agent_id: String },

    #[error("API key not set: {0}")]
    ApiKeyMissing(String),

    #[error("model not available: {model}")]
    ModelNotAvailable { model: String },

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("provider unavailable: {provider}")]
    ProviderUnavailable { provider: String },

    #[error("all providers failed for agent {agent_id}: {details}")]
    AllProvidersFailed { agent_id: String, details: String },

    #[error("provider not registered: {0}")]
    ProviderNotRegistered(String),

    #[error("routing failed: {0}")]
    RoutingFailed(String),

    #[error("model pull failed: {model} on {provider}")]
    ModelPullFailed { model: String, provider: String },

    #[error("cache error: {0}")]
    CacheError(String),

    #[error("config error: {0}")]
    ConfigError(String),

    #[error("concurrent limit reached for provider {provider}")]
    ConcurrentLimitReached { provider: String },
}

impl From<reqwest::Error> for AiError {
    fn from(e: reqwest::Error) -> Self {
        AiError::Http(e.to_string())
    }
}

impl From<serde_json::Error> for AiError {
    fn from(e: serde_json::Error) -> Self {
        AiError::Serialization(e.to_string())
    }
}
