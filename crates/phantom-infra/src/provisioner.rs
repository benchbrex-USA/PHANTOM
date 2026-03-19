//! Auto-provisioner — discovers and binds to free-tier infrastructure.
//!
//! The provisioner is the orchestrator that:
//!   1. Discovers available providers (via AccountManager)
//!   2. Provisions resources on best-available providers
//!   3. Tracks resource bindings (which resource lives where)
//!   4. Handles failover when a provider goes down
//!
//! Architecture Framework §9: Phantom finds, creates, and binds
//! to free-tier servers autonomously.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::errors::InfraError;
use crate::providers::{
    providers_for_resource, Provider, ProviderState, ProviderStatus, ResourceType,
};

/// A provisioned resource — something Phantom is using.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionedResource {
    /// Unique resource ID (e.g. "oracle-vm-1", "supabase-db-prod")
    pub id: String,
    /// Provider hosting this resource
    pub provider: Provider,
    /// Resource type
    pub resource_type: ResourceType,
    /// Provider-specific resource identifier (e.g. instance ID, project ID)
    pub provider_resource_id: Option<String>,
    /// Endpoint / connection string
    pub endpoint: Option<String>,
    /// Current state
    pub state: ResourceState,
    /// When this resource was provisioned
    pub provisioned_at: DateTime<Utc>,
    /// Bind token (cryptographic proof of ownership)
    pub bind_token: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// State of a provisioned resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceState {
    /// Resource is being provisioned
    Provisioning,
    /// Resource is active and in use
    Active,
    /// Resource is being migrated to another provider
    Migrating,
    /// Resource has been decommissioned
    Decommissioned,
    /// Resource is in an error state
    Error,
}

impl std::fmt::Display for ResourceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provisioning => write!(f, "provisioning"),
            Self::Active => write!(f, "active"),
            Self::Migrating => write!(f, "migrating"),
            Self::Decommissioned => write!(f, "decommissioned"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// A provisioning request — what the system needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionRequest {
    /// Desired resource type
    pub resource_type: ResourceType,
    /// Human-readable purpose (e.g. "primary database", "CI pipeline")
    pub purpose: String,
    /// Preferred provider (if any)
    pub preferred_provider: Option<Provider>,
    /// Additional requirements
    pub requirements: HashMap<String, String>,
}

/// Infrastructure provisioner.
///
/// Manages the lifecycle of provisioned resources across providers.
pub struct Provisioner {
    /// Provisioned resources keyed by ID
    resources: HashMap<String, ProvisionedResource>,
    /// Provider availability status
    provider_status: HashMap<Provider, ProviderStatus>,
    /// Resource ID counter
    next_id: u64,
}

impl Default for Provisioner {
    fn default() -> Self {
        Self::new()
    }
}

impl Provisioner {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            provider_status: HashMap::new(),
            next_id: 1,
        }
    }

    /// Update the status of a provider.
    pub fn set_provider_status(&mut self, status: ProviderStatus) {
        self.provider_status.insert(status.provider, status);
    }

    /// Get the status of a provider.
    pub fn get_provider_status(&self, provider: Provider) -> Option<&ProviderStatus> {
        self.provider_status.get(&provider)
    }

    /// Find the best available provider for a resource type.
    pub fn best_provider_for(&self, resource_type: ResourceType) -> Option<Provider> {
        providers_for_resource(resource_type).into_iter().find(|p| {
            self.provider_status
                .get(p)
                .map(|s| s.state == ProviderState::Available)
                .unwrap_or(false)
        })
    }

    /// Plan provisioning: determine which provider to use for a request.
    pub fn plan(&self, request: &ProvisionRequest) -> Result<Provider, InfraError> {
        // Try preferred provider first
        if let Some(preferred) = request.preferred_provider {
            if let Some(status) = self.provider_status.get(&preferred) {
                if status.state == ProviderState::Available {
                    return Ok(preferred);
                }
            }
        }

        // Fall back to best available
        self.best_provider_for(request.resource_type)
            .ok_or_else(|| InfraError::ProviderUnavailable {
                provider: "any".into(),
                reason: format!("no available provider for {}", request.resource_type),
            })
    }

    /// Register a provisioned resource.
    pub fn register_resource(&mut self, mut resource: ProvisionedResource) -> String {
        if resource.id.is_empty() {
            resource.id = format!(
                "{}-{}-{}",
                resource
                    .provider
                    .display_name()
                    .to_lowercase()
                    .replace(' ', "-"),
                resource.resource_type,
                self.next_id
            );
            self.next_id += 1;
        }
        let id = resource.id.clone();
        info!(
            id = %id,
            provider = %resource.provider,
            resource_type = %resource.resource_type,
            "registered resource"
        );
        self.resources.insert(id.clone(), resource);
        id
    }

    /// Get a provisioned resource by ID.
    pub fn get_resource(&self, id: &str) -> Option<&ProvisionedResource> {
        self.resources.get(id)
    }

    /// Get a mutable reference to a resource.
    pub fn get_resource_mut(&mut self, id: &str) -> Option<&mut ProvisionedResource> {
        self.resources.get_mut(id)
    }

    /// Update resource state.
    pub fn set_resource_state(&mut self, id: &str, state: ResourceState) -> Result<(), InfraError> {
        let resource = self
            .resources
            .get_mut(id)
            .ok_or_else(|| InfraError::ResourceNotFound {
                resource: id.to_string(),
            })?;
        info!(id = %id, old = %resource.state, new = %state, "resource state change");
        resource.state = state;
        Ok(())
    }

    /// Decommission a resource.
    pub fn decommission(&mut self, id: &str) -> Result<(), InfraError> {
        self.set_resource_state(id, ResourceState::Decommissioned)
    }

    /// Get all active resources.
    pub fn active_resources(&self) -> Vec<&ProvisionedResource> {
        self.resources
            .values()
            .filter(|r| r.state == ResourceState::Active)
            .collect()
    }

    /// Get active resources by type.
    pub fn resources_by_type(&self, resource_type: ResourceType) -> Vec<&ProvisionedResource> {
        self.resources
            .values()
            .filter(|r| r.resource_type == resource_type && r.state == ResourceState::Active)
            .collect()
    }

    /// Get active resources by provider.
    pub fn resources_by_provider(&self, provider: Provider) -> Vec<&ProvisionedResource> {
        self.resources
            .values()
            .filter(|r| r.provider == provider && r.state == ResourceState::Active)
            .collect()
    }

    /// Total number of resources (all states).
    pub fn total_resources(&self) -> usize {
        self.resources.len()
    }

    /// Number of active resources.
    pub fn active_count(&self) -> usize {
        self.active_resources().len()
    }

    /// Get a summary of provisioned infrastructure.
    pub fn summary(&self) -> ProvisionerSummary {
        let total = self.resources.len();
        let active = self.active_count();
        let by_provider: HashMap<String, usize> = self
            .resources
            .values()
            .filter(|r| r.state == ResourceState::Active)
            .fold(HashMap::new(), |mut acc, r| {
                *acc.entry(r.provider.display_name().to_string())
                    .or_insert(0) += 1;
                acc
            });

        ProvisionerSummary {
            total_resources: total,
            active_resources: active,
            providers_in_use: by_provider.len(),
            resources_by_provider: by_provider,
        }
    }
}

/// Summary of provisioned infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionerSummary {
    pub total_resources: usize,
    pub active_resources: usize,
    pub providers_in_use: usize,
    pub resources_by_provider: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_resource(id: &str, provider: Provider, rt: ResourceType) -> ProvisionedResource {
        ProvisionedResource {
            id: id.into(),
            provider,
            resource_type: rt,
            provider_resource_id: None,
            endpoint: None,
            state: ResourceState::Active,
            provisioned_at: Utc::now(),
            bind_token: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_provisioner_register() {
        let mut prov = Provisioner::new();
        let resource = make_resource("vm-1", Provider::OracleCloud, ResourceType::Compute);
        let id = prov.register_resource(resource);
        assert_eq!(id, "vm-1");
        assert_eq!(prov.total_resources(), 1);
    }

    #[test]
    fn test_provisioner_auto_id() {
        let mut prov = Provisioner::new();
        let mut resource = make_resource("", Provider::OracleCloud, ResourceType::Compute);
        resource.id = String::new();
        let id = prov.register_resource(resource);
        assert!(!id.is_empty());
        assert!(id.contains("oracle"));
    }

    #[test]
    fn test_provisioner_state_change() {
        let mut prov = Provisioner::new();
        prov.register_resource(make_resource(
            "vm-1",
            Provider::OracleCloud,
            ResourceType::Compute,
        ));

        prov.set_resource_state("vm-1", ResourceState::Migrating)
            .unwrap();
        assert_eq!(
            prov.get_resource("vm-1").unwrap().state,
            ResourceState::Migrating
        );
    }

    #[test]
    fn test_provisioner_decommission() {
        let mut prov = Provisioner::new();
        prov.register_resource(make_resource(
            "vm-1",
            Provider::OracleCloud,
            ResourceType::Compute,
        ));

        assert_eq!(prov.active_count(), 1);
        prov.decommission("vm-1").unwrap();
        assert_eq!(prov.active_count(), 0);
        assert_eq!(prov.total_resources(), 1); // Still in table
    }

    #[test]
    fn test_provisioner_best_provider() {
        let mut prov = Provisioner::new();
        prov.set_provider_status(ProviderStatus::available(Provider::Supabase));
        prov.set_provider_status(ProviderStatus::available(Provider::Neon));

        let best = prov.best_provider_for(ResourceType::Database);
        assert_eq!(best, Some(Provider::Supabase)); // Lower priority number
    }

    #[test]
    fn test_provisioner_plan_preferred() {
        let mut prov = Provisioner::new();
        prov.set_provider_status(ProviderStatus::available(Provider::Supabase));
        prov.set_provider_status(ProviderStatus::available(Provider::Neon));

        let request = ProvisionRequest {
            resource_type: ResourceType::Database,
            purpose: "backup db".into(),
            preferred_provider: Some(Provider::Neon),
            requirements: HashMap::new(),
        };

        let chosen = prov.plan(&request).unwrap();
        assert_eq!(chosen, Provider::Neon); // Preferred wins
    }

    #[test]
    fn test_provisioner_plan_fallback() {
        let mut prov = Provisioner::new();
        // Supabase unavailable, Neon available
        prov.set_provider_status(ProviderStatus::unavailable(Provider::Supabase, "quota"));
        prov.set_provider_status(ProviderStatus::available(Provider::Neon));

        let request = ProvisionRequest {
            resource_type: ResourceType::Database,
            purpose: "primary db".into(),
            preferred_provider: Some(Provider::Supabase),
            requirements: HashMap::new(),
        };

        let chosen = prov.plan(&request).unwrap();
        assert_eq!(chosen, Provider::Neon); // Falls back
    }

    #[test]
    fn test_provisioner_resources_by_type() {
        let mut prov = Provisioner::new();
        prov.register_resource(make_resource(
            "vm-1",
            Provider::OracleCloud,
            ResourceType::Compute,
        ));
        prov.register_resource(make_resource(
            "db-1",
            Provider::Supabase,
            ResourceType::Database,
        ));
        prov.register_resource(make_resource(
            "vm-2",
            Provider::FlyIo,
            ResourceType::Compute,
        ));

        let compute = prov.resources_by_type(ResourceType::Compute);
        assert_eq!(compute.len(), 2);

        let db = prov.resources_by_type(ResourceType::Database);
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_provisioner_summary() {
        let mut prov = Provisioner::new();
        prov.register_resource(make_resource(
            "vm-1",
            Provider::OracleCloud,
            ResourceType::Compute,
        ));
        prov.register_resource(make_resource(
            "db-1",
            Provider::Supabase,
            ResourceType::Database,
        ));

        let summary = prov.summary();
        assert_eq!(summary.total_resources, 2);
        assert_eq!(summary.active_resources, 2);
        assert_eq!(summary.providers_in_use, 2);
    }

    #[test]
    fn test_resource_state_display() {
        assert_eq!(ResourceState::Active.to_string(), "active");
        assert_eq!(ResourceState::Provisioning.to_string(), "provisioning");
    }
}
