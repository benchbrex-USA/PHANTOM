//! Core infrastructure traits.
//! All cloud providers implement CloudProvider.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::errors::InfraError;

/// Health status of a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub reachable: bool,
    pub latency_ms: u64,
    pub details: HashMap<String, String>,
}

/// Free tier limits for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeTierLimits {
    pub limits: HashMap<String, String>,
}

/// Information about a provisioned project/resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub region: String,
    pub status: String,
    pub connection_details: HashMap<String, String>,
}

/// Configuration for creating a project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub region: Option<String>,
    pub tier: Option<String>,
    pub framework: Option<String>,
    pub extra: HashMap<String, String>,
}

/// Trait that all cloud providers implement.
#[async_trait]
pub trait CloudProvider: Send + Sync {
    /// Provider name.
    fn name(&self) -> &str;

    /// Authenticate with the provider.
    async fn authenticate(&self) -> Result<(), InfraError>;

    /// Create a project/resource.
    async fn create_project(
        &self,
        name: &str,
        config: &ProjectConfig,
    ) -> Result<ProjectInfo, InfraError>;

    /// Check provider health.
    async fn health_check(&self) -> Result<HealthStatus, InfraError>;

    /// Destroy a project/resource.
    async fn destroy_project(&self, id: &str) -> Result<(), InfraError>;

    /// Get free tier limits.
    fn free_tier_limits(&self) -> FreeTierLimits;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_config_default() {
        let cfg = ProjectConfig::default();
        assert!(cfg.region.is_none());
        assert!(cfg.extra.is_empty());
    }

    #[test]
    fn test_health_status_serde() {
        let hs = HealthStatus {
            reachable: true,
            latency_ms: 42,
            details: HashMap::from([("version".into(), "1.0".into())]),
        };
        let json = serde_json::to_string(&hs).unwrap();
        let restored: HealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.latency_ms, 42);
        assert!(restored.reachable);
    }

    #[test]
    fn test_free_tier_limits() {
        let limits = FreeTierLimits {
            limits: HashMap::from([
                ("compute".into(), "2 VMs".into()),
                ("storage".into(), "200GB".into()),
            ]),
        };
        assert_eq!(limits.limits.len(), 2);
    }
}
