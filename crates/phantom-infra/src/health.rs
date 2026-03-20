//! Health check system for all infrastructure components.
//!
//! Periodic health checks across all provisioned resources.
//! Feeds into the self-healing pipeline when issues are detected.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::providers::Provider;

/// Health status of a single resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Resource is healthy and responding
    Healthy,
    /// Resource is responding but with degraded performance
    Degraded,
    /// Resource is not responding
    Unhealthy,
    /// Resource has not been checked yet
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Result of a health check on a single resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Resource identifier
    pub resource_id: String,
    /// Provider
    pub provider: Provider,
    /// Health status
    pub status: HealthStatus,
    /// Response latency in milliseconds
    pub latency_ms: Option<u64>,
    /// Human-readable message
    pub message: Option<String>,
    /// Timestamp of this check (epoch millis)
    pub checked_at_epoch_ms: u64,
}

impl HealthCheckResult {
    pub fn healthy(resource_id: impl Into<String>, provider: Provider, latency_ms: u64) -> Self {
        Self {
            resource_id: resource_id.into(),
            provider,
            status: HealthStatus::Healthy,
            latency_ms: Some(latency_ms),
            message: None,
            checked_at_epoch_ms: epoch_ms(),
        }
    }

    pub fn unhealthy(
        resource_id: impl Into<String>,
        provider: Provider,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            resource_id: resource_id.into(),
            provider,
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            message: Some(reason.into()),
            checked_at_epoch_ms: epoch_ms(),
        }
    }
}

fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// A resource being monitored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredResource {
    /// Unique resource identifier
    pub id: String,
    /// Provider hosting this resource
    pub provider: Provider,
    /// Resource type description
    pub resource_type: String,
    /// URL or endpoint to check
    pub endpoint: Option<String>,
    /// Health check interval in seconds
    pub check_interval_secs: u64,
}

/// Infrastructure health checker.
///
/// Tracks monitored resources and their latest health status.
pub struct HealthChecker {
    /// Resources being monitored
    resources: Vec<MonitoredResource>,
    /// Latest health check results keyed by resource ID
    latest_results: HashMap<String, HealthCheckResult>,
    /// Consecutive failure counts keyed by resource ID
    failure_counts: HashMap<String, u32>,
    /// Threshold for consecutive failures before alerting
    failure_threshold: u32,
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
            latest_results: HashMap::new(),
            failure_counts: HashMap::new(),
            failure_threshold: 3,
        }
    }

    /// Set the consecutive failure threshold.
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Register a resource to monitor.
    pub fn add_resource(&mut self, resource: MonitoredResource) {
        info!(id = %resource.id, provider = %resource.provider, "monitoring resource");
        self.resources.push(resource);
    }

    /// Remove a resource from monitoring.
    pub fn remove_resource(&mut self, resource_id: &str) {
        self.resources.retain(|r| r.id != resource_id);
        self.latest_results.remove(resource_id);
        self.failure_counts.remove(resource_id);
    }

    /// Record a health check result.
    pub fn record_result(&mut self, result: HealthCheckResult) {
        let id = result.resource_id.clone();

        match result.status {
            HealthStatus::Healthy => {
                self.failure_counts.remove(&id);
                debug!(resource = %id, "health check passed");
            }
            HealthStatus::Unhealthy | HealthStatus::Degraded => {
                let count = self.failure_counts.entry(id.clone()).or_insert(0);
                *count += 1;
                warn!(
                    resource = %id,
                    consecutive_failures = *count,
                    status = %result.status,
                    "health check failed"
                );
            }
            HealthStatus::Unknown => {}
        }

        self.latest_results.insert(id, result);
    }

    /// Get the latest result for a resource.
    pub fn get_result(&self, resource_id: &str) -> Option<&HealthCheckResult> {
        self.latest_results.get(resource_id)
    }

    /// Get all resources that are currently unhealthy.
    pub fn unhealthy_resources(&self) -> Vec<&HealthCheckResult> {
        self.latest_results
            .values()
            .filter(|r| r.status == HealthStatus::Unhealthy)
            .collect()
    }

    /// Get resources that have exceeded the failure threshold.
    pub fn critical_resources(&self) -> Vec<(&str, u32)> {
        self.failure_counts
            .iter()
            .filter(|(_, count)| **count >= self.failure_threshold)
            .map(|(id, count)| (id.as_str(), *count))
            .collect()
    }

    /// Check if all monitored resources are healthy.
    pub fn all_healthy(&self) -> bool {
        self.latest_results
            .values()
            .all(|r| r.status == HealthStatus::Healthy)
    }

    /// Get a summary of health across all resources.
    pub fn summary(&self) -> HealthSummary {
        let total = self.resources.len();
        let checked = self.latest_results.len();
        let healthy = self
            .latest_results
            .values()
            .filter(|r| r.status == HealthStatus::Healthy)
            .count();
        let degraded = self
            .latest_results
            .values()
            .filter(|r| r.status == HealthStatus::Degraded)
            .count();
        let unhealthy = self
            .latest_results
            .values()
            .filter(|r| r.status == HealthStatus::Unhealthy)
            .count();

        HealthSummary {
            total_monitored: total,
            checked,
            healthy,
            degraded,
            unhealthy,
            critical: self.critical_resources().len(),
        }
    }

    /// Get all monitored resources.
    pub fn resources(&self) -> &[MonitoredResource] {
        &self.resources
    }

    /// Number of monitored resources.
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }
}

/// Summary of infrastructure health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_monitored: usize,
    pub checked: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub unhealthy: usize,
    /// Resources exceeding failure threshold
    pub critical: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_resource(id: &str, provider: Provider) -> MonitoredResource {
        MonitoredResource {
            id: id.into(),
            provider,
            resource_type: "compute".into(),
            endpoint: Some("https://example.com".into()),
            check_interval_secs: 60,
        }
    }

    #[test]
    fn test_health_checker_add_resource() {
        let mut checker = HealthChecker::new();
        checker.add_resource(test_resource("vm-1", Provider::OracleCloud));
        assert_eq!(checker.resource_count(), 1);
    }

    #[test]
    fn test_health_checker_record_healthy() {
        let mut checker = HealthChecker::new();
        checker.add_resource(test_resource("vm-1", Provider::OracleCloud));

        let result = HealthCheckResult::healthy("vm-1", Provider::OracleCloud, 45);
        checker.record_result(result);

        assert!(checker.all_healthy());
        let r = checker.get_result("vm-1").unwrap();
        assert_eq!(r.status, HealthStatus::Healthy);
        assert_eq!(r.latency_ms, Some(45));
    }

    #[test]
    fn test_health_checker_record_unhealthy() {
        let mut checker = HealthChecker::new();
        checker.add_resource(test_resource("vm-1", Provider::OracleCloud));

        let result = HealthCheckResult::unhealthy("vm-1", Provider::OracleCloud, "timeout");
        checker.record_result(result);

        assert!(!checker.all_healthy());
        let unhealthy = checker.unhealthy_resources();
        assert_eq!(unhealthy.len(), 1);
    }

    #[test]
    fn test_health_checker_failure_threshold() {
        let mut checker = HealthChecker::new().with_failure_threshold(3);
        checker.add_resource(test_resource("vm-1", Provider::OracleCloud));

        // 2 failures — not yet critical
        for _ in 0..2 {
            checker.record_result(HealthCheckResult::unhealthy(
                "vm-1",
                Provider::OracleCloud,
                "timeout",
            ));
        }
        assert!(checker.critical_resources().is_empty());

        // 3rd failure — now critical
        checker.record_result(HealthCheckResult::unhealthy(
            "vm-1",
            Provider::OracleCloud,
            "timeout",
        ));
        assert_eq!(checker.critical_resources().len(), 1);
    }

    #[test]
    fn test_health_checker_recovery() {
        let mut checker = HealthChecker::new().with_failure_threshold(2);
        checker.add_resource(test_resource("vm-1", Provider::OracleCloud));

        // Fail twice
        for _ in 0..2 {
            checker.record_result(HealthCheckResult::unhealthy(
                "vm-1",
                Provider::OracleCloud,
                "timeout",
            ));
        }
        assert_eq!(checker.critical_resources().len(), 1);

        // Recover
        checker.record_result(HealthCheckResult::healthy(
            "vm-1",
            Provider::OracleCloud,
            30,
        ));
        assert!(checker.critical_resources().is_empty());
        assert!(checker.all_healthy());
    }

    #[test]
    fn test_health_summary() {
        let mut checker = HealthChecker::new();
        checker.add_resource(test_resource("vm-1", Provider::OracleCloud));
        checker.add_resource(test_resource("db-1", Provider::Supabase));

        checker.record_result(HealthCheckResult::healthy(
            "vm-1",
            Provider::OracleCloud,
            20,
        ));
        checker.record_result(HealthCheckResult::unhealthy(
            "db-1",
            Provider::Supabase,
            "connection refused",
        ));

        let summary = checker.summary();
        assert_eq!(summary.total_monitored, 2);
        assert_eq!(summary.healthy, 1);
        assert_eq!(summary.unhealthy, 1);
    }

    #[test]
    fn test_remove_resource() {
        let mut checker = HealthChecker::new();
        checker.add_resource(test_resource("vm-1", Provider::OracleCloud));
        checker.record_result(HealthCheckResult::healthy(
            "vm-1",
            Provider::OracleCloud,
            20,
        ));
        assert_eq!(checker.resource_count(), 1);

        checker.remove_resource("vm-1");
        assert_eq!(checker.resource_count(), 0);
        assert!(checker.get_result("vm-1").is_none());
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
    }
}
