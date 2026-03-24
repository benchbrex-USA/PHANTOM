use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfraError {
    #[error("provider {provider} unavailable: {reason}")]
    ProviderUnavailable { provider: String, reason: String },

    #[error("provisioning failed for {resource}: {reason}")]
    ProvisioningFailed { resource: String, reason: String },

    #[error("dependency installation failed: {dep}")]
    DependencyFailed { dep: String },

    #[error("dependency not found: {dep}")]
    DependencyNotFound { dep: String },

    #[error("account creation failed for {service}: {reason}")]
    AccountCreationFailed { service: String, reason: String },

    #[error("health check failed for {target}: {reason}")]
    HealthCheckFailed { target: String, reason: String },

    #[error("command execution failed: {0}")]
    CommandFailed(String),

    #[error("timeout waiting for {target}")]
    Timeout { target: String },

    #[error("resource not found: {resource}")]
    ResourceNotFound { resource: String },

    #[error("quota exceeded for {provider}: {detail}")]
    QuotaExceeded { provider: String, detail: String },

    #[error("authentication required for {provider}")]
    AuthRequired { provider: String },

    #[error("provider error: {0}")]
    ProviderError(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<reqwest::Error> for InfraError {
    fn from(e: reqwest::Error) -> Self {
        InfraError::Http(e.to_string())
    }
}
