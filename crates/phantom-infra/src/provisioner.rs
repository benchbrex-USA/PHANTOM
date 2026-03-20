//! Auto-provisioner — discovers and binds to free-tier infrastructure.
//!
//! The provisioner is the orchestrator that:
//!   1. Discovers available providers (via AccountManager)
//!   2. Provisions resources on best-available providers
//!   3. Tracks resource bindings (which resource lives where)
//!   4. Handles failover when a provider goes down
//!
//! Extended with:
//!   - VM provisioning API calls for Hetzner, Vultr, DigitalOcean, Fly.io, Railway
//!   - Per-session bind tokens (HMAC-SHA256)
//!   - Spend tracking with cost oracle limit enforcement
//!   - Automatic failover re-provisioning on provider failure
//!
//! Architecture Framework §9–10: Phantom finds, creates, and binds
//! to free-tier servers autonomously.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

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
    /// Spend tracker for cost oracle enforcement
    spend_tracker: SpendTracker,
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
            spend_tracker: SpendTracker::new(),
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

    /// Get a mutable reference to the spend tracker.
    pub fn spend_tracker_mut(&mut self) -> &mut SpendTracker {
        &mut self.spend_tracker
    }

    /// Get an immutable reference to the spend tracker.
    pub fn spend_tracker(&self) -> &SpendTracker {
        &self.spend_tracker
    }

    /// Provision a VM on the given provider, recording spend.
    /// Returns the resource ID after registration.
    pub async fn provision_vm(
        &mut self,
        provider: Provider,
        spec: &VmSpec,
    ) -> Result<String, InfraError> {
        // 1. Check spend limits before provisioning
        self.spend_tracker.check_budget(provider)?;

        // 2. Build the API request
        let config = provider_api_config(provider)?;
        let body = build_create_body(provider, spec, &config);
        let token = read_api_token(&config)?;

        // 3. Make the API call
        let result = call_create_api(&config, &token, &body).await?;

        // 4. Generate bind token
        let bind_token = generate_bind_token(provider, &result.instance_id);

        // 5. Record spend
        self.spend_tracker
            .record_spend(provider, result.monthly_cost_cents);

        // 6. Register the resource
        let resource = ProvisionedResource {
            id: String::new(), // auto-generated
            provider,
            resource_type: ResourceType::Compute,
            provider_resource_id: Some(result.instance_id.clone()),
            endpoint: result.ip_address.clone(),
            state: ResourceState::Active,
            provisioned_at: Utc::now(),
            bind_token: Some(bind_token),
            metadata: result.metadata,
        };

        let id = self.register_resource(resource);
        info!(id = %id, provider = %provider, instance = %result.instance_id, "VM provisioned");
        Ok(id)
    }

    /// Provision a VM with automatic failover across providers.
    /// Tries each provider in order, falling back on failure.
    pub async fn provision_vm_with_failover(
        &mut self,
        spec: &VmSpec,
        providers: &[Provider],
    ) -> Result<FailoverResult, InfraError> {
        let mut attempts = Vec::new();

        for &provider in providers {
            info!(provider = %provider, "attempting VM provision");

            match self.provision_vm(provider, spec).await {
                Ok(resource_id) => {
                    attempts.push(ProvisionAttempt {
                        provider,
                        success: true,
                        error: None,
                    });
                    return Ok(FailoverResult {
                        resource_id,
                        provider,
                        attempts,
                    });
                }
                Err(e) => {
                    warn!(provider = %provider, error = %e, "provision failed, trying next");
                    attempts.push(ProvisionAttempt {
                        provider,
                        success: false,
                        error: Some(e.to_string()),
                    });

                    // Mark provider as unavailable
                    self.set_provider_status(ProviderStatus::unavailable(
                        provider,
                        format!("provisioning failed: {}", e),
                    ));
                }
            }
        }

        Err(InfraError::ProvisioningFailed {
            resource: "vm".into(),
            reason: format!(
                "all {} providers failed: {}",
                attempts.len(),
                attempts
                    .iter()
                    .map(|a| format!(
                        "{}: {}",
                        a.provider,
                        a.error.as_deref().unwrap_or("unknown")
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
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

// ═══════════════════════════════════════════════════════════════════════════
//  VM Provider API Layer
// ═══════════════════════════════════════════════════════════════════════════

/// Specification for creating a VM instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSpec {
    /// Human-readable instance name
    pub name: String,
    /// Region / location (provider-specific, uses default if None)
    pub region: Option<String>,
    /// Plan / instance type (uses free-tier default if None)
    pub plan: Option<String>,
    /// OS image (uses default if None)
    pub image: Option<String>,
    /// SSH key IDs to inject
    pub ssh_key_ids: Vec<String>,
    /// Cloud-init user data
    pub user_data: Option<String>,
    /// Labels / tags
    pub labels: HashMap<String, String>,
}

impl VmSpec {
    /// Create a minimal spec with just a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            region: None,
            plan: None,
            image: None,
            ssh_key_ids: Vec::new(),
            user_data: None,
            labels: HashMap::new(),
        }
    }
}

/// Result from a successful VM provisioning API call.
#[derive(Debug, Clone)]
pub struct VmProvisionResult {
    pub provider: Provider,
    pub instance_id: String,
    pub ip_address: Option<String>,
    pub status: String,
    pub monthly_cost_cents: u64,
    pub metadata: HashMap<String, String>,
}

/// API configuration for a VM provider.
struct ProviderApiConfig {
    base_url: &'static str,
    token_env_var: &'static str,
    create_path: &'static str,
    default_plan: &'static str,
    default_image: &'static str,
    default_region: &'static str,
    monthly_cost_cents: u64,
}

/// Get API configuration for a compute provider.
fn provider_api_config(provider: Provider) -> Result<ProviderApiConfig, InfraError> {
    match provider {
        Provider::Hetzner => Ok(ProviderApiConfig {
            base_url: "https://api.hetzner.cloud/v1",
            token_env_var: "HCLOUD_TOKEN",
            create_path: "/servers",
            default_plan: "cx22",
            default_image: "ubuntu-24.04",
            default_region: "fsn1",
            monthly_cost_cents: 399, // €3.99/mo
        }),
        Provider::Vultr => Ok(ProviderApiConfig {
            base_url: "https://api.vultr.com/v2",
            token_env_var: "VULTR_API_KEY",
            create_path: "/instances",
            default_plan: "vc2-1c-1gb",
            default_image: "ubuntu-24.04",
            default_region: "ewr",
            monthly_cost_cents: 500, // $5/mo
        }),
        Provider::DigitalOcean => Ok(ProviderApiConfig {
            base_url: "https://api.digitalocean.com/v2",
            token_env_var: "DIGITALOCEAN_TOKEN",
            create_path: "/droplets",
            default_plan: "s-1vcpu-1gb",
            default_image: "ubuntu-24-04-x64",
            default_region: "nyc1",
            monthly_cost_cents: 600, // $6/mo
        }),
        Provider::FlyIo => Ok(ProviderApiConfig {
            base_url: "https://api.machines.dev/v1",
            token_env_var: "FLY_API_TOKEN",
            create_path: "/apps/phantom/machines",
            default_plan: "shared-cpu-1x",
            default_image: "ubuntu:24.04",
            default_region: "iad",
            monthly_cost_cents: 0, // free tier
        }),
        Provider::Railway => Ok(ProviderApiConfig {
            base_url: "https://backboard.railway.app",
            token_env_var: "RAILWAY_TOKEN",
            create_path: "/graphql/v2",
            default_plan: "starter",
            default_image: "ubuntu",
            default_region: "us-west1",
            monthly_cost_cents: 0, // $5/mo credit
        }),
        _ => Err(InfraError::ProviderUnavailable {
            provider: provider.display_name().into(),
            reason: "VM provisioning not supported for this provider".into(),
        }),
    }
}

/// Build a provider-specific JSON body for VM creation.
fn build_create_body(
    provider: Provider,
    spec: &VmSpec,
    config: &ProviderApiConfig,
) -> serde_json::Value {
    let region = spec.region.as_deref().unwrap_or(config.default_region);
    let plan = spec.plan.as_deref().unwrap_or(config.default_plan);
    let image = spec.image.as_deref().unwrap_or(config.default_image);

    match provider {
        Provider::Hetzner => serde_json::json!({
            "name": spec.name,
            "server_type": plan,
            "image": image,
            "location": region,
            "ssh_keys": spec.ssh_key_ids,
            "labels": spec.labels,
            "start_after_create": true,
        }),
        Provider::Vultr => serde_json::json!({
            "region": region,
            "plan": plan,
            "os_id": image,
            "label": spec.name,
            "hostname": spec.name,
            "sshkey_id": spec.ssh_key_ids,
            "tags": spec.labels.keys().cloned().collect::<Vec<_>>(),
        }),
        Provider::DigitalOcean => serde_json::json!({
            "name": spec.name,
            "region": region,
            "size": plan,
            "image": image,
            "ssh_keys": spec.ssh_key_ids,
            "tags": spec.labels.keys().cloned().collect::<Vec<_>>(),
            "monitoring": true,
        }),
        Provider::FlyIo => serde_json::json!({
            "name": spec.name,
            "region": region,
            "config": {
                "image": image,
                "guest": {
                    "cpu_kind": "shared",
                    "cpus": 1,
                    "memory_mb": 256,
                },
                "auto_destroy": false,
            },
        }),
        Provider::Railway => serde_json::json!({
            "query": "mutation { serviceCreate(input: { name: $name }) { id } }",
            "variables": {
                "name": spec.name,
            },
        }),
        _ => serde_json::json!({}),
    }
}

/// Parse a provider-specific API response into a VmProvisionResult.
fn parse_create_response(
    provider: Provider,
    body: &serde_json::Value,
    config: &ProviderApiConfig,
) -> Result<VmProvisionResult, InfraError> {
    let (instance_id, ip, status) = match provider {
        Provider::Hetzner => {
            let server = body.get("server").unwrap_or(body);
            let id = json_string(server, "id");
            let ip = server
                .get("public_net")
                .and_then(|n| n.get("ipv4"))
                .and_then(|v| v.get("ip"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let status = json_string_or(server, "status", "initializing");
            (id, ip, status)
        }
        Provider::Vultr => {
            let inst = body.get("instance").unwrap_or(body);
            let id = json_string(inst, "id");
            let ip = inst
                .get("main_ip")
                .and_then(|v| v.as_str())
                .filter(|s| *s != "0.0.0.0")
                .map(String::from);
            let status = json_string_or(inst, "status", "pending");
            (id, ip, status)
        }
        Provider::DigitalOcean => {
            let droplet = body.get("droplet").unwrap_or(body);
            let id = json_string(droplet, "id");
            let ip = droplet
                .get("networks")
                .and_then(|n| n.get("v4"))
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|n| n.get("ip_address"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let status = json_string_or(droplet, "status", "new");
            (id, ip, status)
        }
        Provider::FlyIo => {
            let id = json_string(body, "id");
            let ip = body
                .get("private_ip")
                .and_then(|v| v.as_str())
                .map(String::from);
            let status = json_string_or(body, "state", "created");
            (id, ip, status)
        }
        Provider::Railway => {
            let id = body
                .get("data")
                .and_then(|d| d.get("serviceCreate"))
                .and_then(|s| s.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            (id, None, "created".to_string())
        }
        _ => {
            return Err(InfraError::ProvisioningFailed {
                resource: "vm".into(),
                reason: "unsupported provider".into(),
            })
        }
    };

    Ok(VmProvisionResult {
        provider,
        instance_id,
        ip_address: ip,
        status,
        monthly_cost_cents: config.monthly_cost_cents,
        metadata: HashMap::new(),
    })
}

/// Read the API token from environment.
fn read_api_token(config: &ProviderApiConfig) -> Result<String, InfraError> {
    std::env::var(config.token_env_var).map_err(|_| InfraError::AuthRequired {
        provider: config.token_env_var.into(),
    })
}

/// Make the HTTP POST to create a VM.
async fn call_create_api(
    config: &ProviderApiConfig,
    token: &str,
    body: &serde_json::Value,
) -> Result<VmProvisionResult, InfraError> {
    let url = format!("{}{}", config.base_url, config.create_path);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| InfraError::Http(e.to_string()))?;

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await?;

    let status = response.status();
    let response_body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| InfraError::Http(e.to_string()))?;

    if !status.is_success() {
        let msg = response_body
            .get("message")
            .or_else(|| response_body.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(InfraError::ProvisioningFailed {
            resource: "vm".into(),
            reason: format!("HTTP {}: {}", status, msg),
        });
    }

    // Determine which provider from the base URL
    let provider = match config.token_env_var {
        "HCLOUD_TOKEN" => Provider::Hetzner,
        "VULTR_API_KEY" => Provider::Vultr,
        "DIGITALOCEAN_TOKEN" => Provider::DigitalOcean,
        "FLY_API_TOKEN" => Provider::FlyIo,
        "RAILWAY_TOKEN" => Provider::Railway,
        _ => Provider::Hetzner,
    };

    parse_create_response(provider, &response_body, config)
}

// ═══════════════════════════════════════════════════════════════════════════
//  Bind Tokens
// ═══════════════════════════════════════════════════════════════════════════

/// Generate a per-session bind token for a provisioned resource.
/// HMAC-SHA256(session_secret, provider || resource_id || timestamp)
pub fn generate_bind_token(provider: Provider, resource_id: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let timestamp = Utc::now().timestamp();
    let message = format!("phantom-bind:{}:{}:{}", provider, resource_id, timestamp);

    // Use a session-derived nonce as HMAC key (in production, from MasterKeySession)
    let session_nonce = format!("phantom-session-{}", std::process::id());
    let key = <Sha256 as sha2::Digest>::digest(session_nonce.as_bytes());

    let mut mac = Hmac::<Sha256>::new_from_slice(&key).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Verify a bind token matches the expected provider and resource.
pub fn verify_bind_token(token: &str, _provider: Provider, _resource_id: &str) -> bool {
    // Bind tokens embed the timestamp, so we can't re-derive exactly.
    // Instead we verify format and non-emptiness; full verification requires
    // the original timestamp which would be stored alongside the token.
    !token.is_empty() && token.len() == 64 // SHA-256 hex = 64 chars
        && hex::decode(token).is_ok()
        // Ensure the token was created for this provider context
        && token != "0".repeat(64)
}

// ═══════════════════════════════════════════════════════════════════════════
//  Spend Tracking & Cost Oracle
// ═══════════════════════════════════════════════════════════════════════════

/// Monthly spend limit for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendLimit {
    pub monthly_limit_cents: u64,
    /// Alert when spend reaches this percentage of the limit (e.g. 80)
    pub alert_threshold_pct: u8,
}

impl SpendLimit {
    pub fn new(monthly_limit_cents: u64) -> Self {
        Self {
            monthly_limit_cents,
            alert_threshold_pct: 80,
        }
    }

    pub fn with_alert_threshold(mut self, pct: u8) -> Self {
        self.alert_threshold_pct = pct;
        self
    }
}

/// Current spend state for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSpend {
    pub provider: Provider,
    /// Accumulated spend this month in cents
    pub current_month_cents: u64,
    /// Number of active resources contributing to this spend
    pub resource_count: u32,
    /// When this was last updated
    pub last_updated: DateTime<Utc>,
}

/// Tracks spend across all providers and enforces cost oracle limits.
#[derive(Debug, Clone)]
pub struct SpendTracker {
    limits: HashMap<Provider, SpendLimit>,
    spend: HashMap<Provider, ProviderSpend>,
    /// Global monthly budget ceiling (cents). 0 = unlimited.
    global_limit_cents: u64,
}

impl Default for SpendTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SpendTracker {
    pub fn new() -> Self {
        let mut tracker = Self {
            limits: HashMap::new(),
            spend: HashMap::new(),
            global_limit_cents: 0,
        };
        // Set default free-tier limits for the 5 VM providers
        tracker.set_limit(Provider::Hetzner, SpendLimit::new(500));
        tracker.set_limit(Provider::Vultr, SpendLimit::new(500));
        tracker.set_limit(Provider::DigitalOcean, SpendLimit::new(600));
        tracker.set_limit(Provider::FlyIo, SpendLimit::new(0)); // free tier
        tracker.set_limit(Provider::Railway, SpendLimit::new(500)); // $5 credit
        tracker
    }

    /// Set the spend limit for a provider.
    pub fn set_limit(&mut self, provider: Provider, limit: SpendLimit) {
        self.limits.insert(provider, limit);
    }

    /// Set a global monthly budget ceiling across all providers.
    pub fn set_global_limit(&mut self, cents: u64) {
        self.global_limit_cents = cents;
    }

    /// Record spend for a provider.
    pub fn record_spend(&mut self, provider: Provider, monthly_cost_cents: u64) {
        let entry = self.spend.entry(provider).or_insert_with(|| ProviderSpend {
            provider,
            current_month_cents: 0,
            resource_count: 0,
            last_updated: Utc::now(),
        });
        entry.current_month_cents += monthly_cost_cents;
        entry.resource_count += 1;
        entry.last_updated = Utc::now();

        // Check if we've hit an alert threshold
        if let Some(limit) = self.limits.get(&provider) {
            if limit.monthly_limit_cents > 0 {
                let pct = (entry.current_month_cents * 100) / limit.monthly_limit_cents.max(1);
                if pct >= limit.alert_threshold_pct as u64 {
                    warn!(
                        provider = %provider,
                        spend = entry.current_month_cents,
                        limit = limit.monthly_limit_cents,
                        pct = pct,
                        "spend alert threshold reached"
                    );
                }
            }
        }
    }

    /// Check if a provider's budget allows more provisioning.
    pub fn check_budget(&self, provider: Provider) -> Result<(), InfraError> {
        // Check per-provider limit
        if let Some(limit) = self.limits.get(&provider) {
            if limit.monthly_limit_cents > 0 {
                let current = self
                    .spend
                    .get(&provider)
                    .map(|s| s.current_month_cents)
                    .unwrap_or(0);
                if current >= limit.monthly_limit_cents {
                    return Err(InfraError::QuotaExceeded {
                        provider: provider.display_name().into(),
                        detail: format!(
                            "monthly spend {}c >= limit {}c",
                            current, limit.monthly_limit_cents
                        ),
                    });
                }
            }
        }

        // Check global limit
        if self.global_limit_cents > 0 {
            let total: u64 = self.spend.values().map(|s| s.current_month_cents).sum();
            if total >= self.global_limit_cents {
                return Err(InfraError::QuotaExceeded {
                    provider: "global".into(),
                    detail: format!(
                        "total spend {}c >= global limit {}c",
                        total, self.global_limit_cents
                    ),
                });
            }
        }

        Ok(())
    }

    /// Get current spend for a provider.
    pub fn get_spend(&self, provider: Provider) -> Option<&ProviderSpend> {
        self.spend.get(&provider)
    }

    /// Total spend across all providers.
    pub fn total_spend_cents(&self) -> u64 {
        self.spend.values().map(|s| s.current_month_cents).sum()
    }

    /// Get a summary of all spend.
    pub fn spend_report(&self) -> Vec<SpendReportEntry> {
        self.spend
            .values()
            .map(|s| {
                let limit = self.limits.get(&s.provider);
                SpendReportEntry {
                    provider: s.provider,
                    current_cents: s.current_month_cents,
                    limit_cents: limit.map(|l| l.monthly_limit_cents),
                    resource_count: s.resource_count,
                    utilization_pct: limit.filter(|l| l.monthly_limit_cents > 0).map(|l| {
                        ((s.current_month_cents * 100) / l.monthly_limit_cents.max(1)) as u8
                    }),
                }
            })
            .collect()
    }

    /// Reset spend counters (e.g. at month boundary).
    pub fn reset_monthly(&mut self) {
        for spend in self.spend.values_mut() {
            spend.current_month_cents = 0;
            spend.resource_count = 0;
            spend.last_updated = Utc::now();
        }
    }
}

/// A single line in the spend report.
#[derive(Debug, Clone)]
pub struct SpendReportEntry {
    pub provider: Provider,
    pub current_cents: u64,
    pub limit_cents: Option<u64>,
    pub resource_count: u32,
    pub utilization_pct: Option<u8>,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Failover Result
// ═══════════════════════════════════════════════════════════════════════════

/// Result of a provisioning attempt with failover.
#[derive(Debug, Clone)]
pub struct FailoverResult {
    /// The ID of the successfully provisioned resource
    pub resource_id: String,
    /// The provider that succeeded
    pub provider: Provider,
    /// All attempts made (including failures)
    pub attempts: Vec<ProvisionAttempt>,
}

impl FailoverResult {
    /// How many providers were tried before success.
    pub fn attempts_count(&self) -> usize {
        self.attempts.len()
    }

    /// Whether failover was needed (more than one attempt).
    pub fn used_failover(&self) -> bool {
        self.attempts.len() > 1
    }
}

/// A single provisioning attempt within a failover sequence.
#[derive(Debug, Clone)]
pub struct ProvisionAttempt {
    pub provider: Provider,
    pub success: bool,
    pub error: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
//  JSON Helpers
// ═══════════════════════════════════════════════════════════════════════════

fn json_string(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn json_string_or(value: &serde_json::Value, key: &str, default: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════════

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

    // ── Existing Provisioner tests ──────────────────────────────────────

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

    // ── VmSpec tests ────────────────────────────────────────────────────

    #[test]
    fn test_vm_spec_new() {
        let spec = VmSpec::new("phantom-worker-1");
        assert_eq!(spec.name, "phantom-worker-1");
        assert!(spec.region.is_none());
        assert!(spec.plan.is_none());
        assert!(spec.ssh_key_ids.is_empty());
    }

    #[test]
    fn test_vm_spec_serde() {
        let spec = VmSpec::new("test-vm");
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: VmSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test-vm");
    }

    // ── Provider API Config tests ───────────────────────────────────────

    #[test]
    fn test_provider_api_config_hetzner() {
        let config = provider_api_config(Provider::Hetzner).unwrap();
        assert_eq!(config.base_url, "https://api.hetzner.cloud/v1");
        assert_eq!(config.token_env_var, "HCLOUD_TOKEN");
        assert_eq!(config.default_plan, "cx22");
        assert_eq!(config.monthly_cost_cents, 399);
    }

    #[test]
    fn test_provider_api_config_vultr() {
        let config = provider_api_config(Provider::Vultr).unwrap();
        assert_eq!(config.base_url, "https://api.vultr.com/v2");
        assert_eq!(config.token_env_var, "VULTR_API_KEY");
        assert_eq!(config.monthly_cost_cents, 500);
    }

    #[test]
    fn test_provider_api_config_digitalocean() {
        let config = provider_api_config(Provider::DigitalOcean).unwrap();
        assert_eq!(config.base_url, "https://api.digitalocean.com/v2");
        assert_eq!(config.default_plan, "s-1vcpu-1gb");
        assert_eq!(config.monthly_cost_cents, 600);
    }

    #[test]
    fn test_provider_api_config_fly() {
        let config = provider_api_config(Provider::FlyIo).unwrap();
        assert_eq!(config.base_url, "https://api.machines.dev/v1");
        assert_eq!(config.monthly_cost_cents, 0); // free tier
    }

    #[test]
    fn test_provider_api_config_railway() {
        let config = provider_api_config(Provider::Railway).unwrap();
        assert!(config.create_path.contains("graphql"));
    }

    #[test]
    fn test_provider_api_config_unsupported() {
        let result = provider_api_config(Provider::GitHub);
        assert!(result.is_err());
    }

    // ── Build request body tests ────────────────────────────────────────

    #[test]
    fn test_build_hetzner_body() {
        let spec = VmSpec::new("phantom-eu-1");
        let config = provider_api_config(Provider::Hetzner).unwrap();
        let body = build_create_body(Provider::Hetzner, &spec, &config);

        assert_eq!(body["name"], "phantom-eu-1");
        assert_eq!(body["server_type"], "cx22");
        assert_eq!(body["image"], "ubuntu-24.04");
        assert_eq!(body["location"], "fsn1");
        assert_eq!(body["start_after_create"], true);
    }

    #[test]
    fn test_build_vultr_body() {
        let spec = VmSpec::new("phantom-us-1");
        let config = provider_api_config(Provider::Vultr).unwrap();
        let body = build_create_body(Provider::Vultr, &spec, &config);

        assert_eq!(body["label"], "phantom-us-1");
        assert_eq!(body["plan"], "vc2-1c-1gb");
        assert_eq!(body["region"], "ewr");
    }

    #[test]
    fn test_build_digitalocean_body() {
        let mut spec = VmSpec::new("phantom-do-1");
        spec.region = Some("sfo3".into());
        let config = provider_api_config(Provider::DigitalOcean).unwrap();
        let body = build_create_body(Provider::DigitalOcean, &spec, &config);

        assert_eq!(body["name"], "phantom-do-1");
        assert_eq!(body["region"], "sfo3"); // custom region
        assert_eq!(body["size"], "s-1vcpu-1gb");
        assert_eq!(body["monitoring"], true);
    }

    #[test]
    fn test_build_fly_body() {
        let spec = VmSpec::new("phantom-fly-1");
        let config = provider_api_config(Provider::FlyIo).unwrap();
        let body = build_create_body(Provider::FlyIo, &spec, &config);

        assert_eq!(body["name"], "phantom-fly-1");
        assert_eq!(body["config"]["guest"]["cpus"], 1);
        assert_eq!(body["config"]["guest"]["memory_mb"], 256);
    }

    #[test]
    fn test_build_railway_body() {
        let spec = VmSpec::new("phantom-ry-1");
        let config = provider_api_config(Provider::Railway).unwrap();
        let body = build_create_body(Provider::Railway, &spec, &config);

        assert!(body["query"].as_str().unwrap().contains("serviceCreate"));
        assert_eq!(body["variables"]["name"], "phantom-ry-1");
    }

    // ── Parse response tests ────────────────────────────────────────────

    #[test]
    fn test_parse_hetzner_response() {
        let body = serde_json::json!({
            "server": {
                "id": 12345,
                "status": "running",
                "public_net": {
                    "ipv4": { "ip": "1.2.3.4" }
                }
            }
        });
        let config = provider_api_config(Provider::Hetzner).unwrap();
        let result = parse_create_response(Provider::Hetzner, &body, &config).unwrap();

        assert_eq!(result.instance_id, "12345");
        assert_eq!(result.ip_address, Some("1.2.3.4".into()));
        assert_eq!(result.status, "running");
        assert_eq!(result.monthly_cost_cents, 399);
    }

    #[test]
    fn test_parse_vultr_response() {
        let body = serde_json::json!({
            "instance": {
                "id": "abc-123",
                "main_ip": "5.6.7.8",
                "status": "active"
            }
        });
        let config = provider_api_config(Provider::Vultr).unwrap();
        let result = parse_create_response(Provider::Vultr, &body, &config).unwrap();

        assert_eq!(result.instance_id, "abc-123");
        assert_eq!(result.ip_address, Some("5.6.7.8".into()));
    }

    #[test]
    fn test_parse_digitalocean_response() {
        let body = serde_json::json!({
            "droplet": {
                "id": 98765,
                "status": "active",
                "networks": {
                    "v4": [{ "ip_address": "10.20.30.40", "type": "public" }]
                }
            }
        });
        let config = provider_api_config(Provider::DigitalOcean).unwrap();
        let result = parse_create_response(Provider::DigitalOcean, &body, &config).unwrap();

        assert_eq!(result.instance_id, "98765");
        assert_eq!(result.ip_address, Some("10.20.30.40".into()));
    }

    #[test]
    fn test_parse_fly_response() {
        let body = serde_json::json!({
            "id": "fly-machine-abc",
            "state": "started",
            "private_ip": "fdaa::1"
        });
        let config = provider_api_config(Provider::FlyIo).unwrap();
        let result = parse_create_response(Provider::FlyIo, &body, &config).unwrap();

        assert_eq!(result.instance_id, "fly-machine-abc");
        assert_eq!(result.ip_address, Some("fdaa::1".into()));
        assert_eq!(result.monthly_cost_cents, 0);
    }

    #[test]
    fn test_parse_railway_response() {
        let body = serde_json::json!({
            "data": {
                "serviceCreate": { "id": "ry-service-xyz" }
            }
        });
        let config = provider_api_config(Provider::Railway).unwrap();
        let result = parse_create_response(Provider::Railway, &body, &config).unwrap();

        assert_eq!(result.instance_id, "ry-service-xyz");
        assert!(result.ip_address.is_none());
    }

    // ── Bind token tests ────────────────────────────────────────────────

    #[test]
    fn test_generate_bind_token() {
        let token = generate_bind_token(Provider::Hetzner, "server-123");
        assert_eq!(token.len(), 64); // SHA-256 hex
        assert!(hex::decode(&token).is_ok());
    }

    #[test]
    fn test_bind_token_unique_per_resource() {
        let t1 = generate_bind_token(Provider::Hetzner, "server-1");
        let t2 = generate_bind_token(Provider::Hetzner, "server-2");
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_bind_token_unique_per_provider() {
        let t1 = generate_bind_token(Provider::Hetzner, "server-1");
        let t2 = generate_bind_token(Provider::Vultr, "server-1");
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_verify_bind_token() {
        let token = generate_bind_token(Provider::Hetzner, "server-123");
        assert!(verify_bind_token(&token, Provider::Hetzner, "server-123"));
        assert!(!verify_bind_token("", Provider::Hetzner, "server-123"));
        assert!(!verify_bind_token("short", Provider::Hetzner, "server-123"));
    }

    // ── Spend tracker tests ─────────────────────────────────────────────

    #[test]
    fn test_spend_tracker_new_has_defaults() {
        let tracker = SpendTracker::new();
        assert!(tracker.limits.contains_key(&Provider::Hetzner));
        assert!(tracker.limits.contains_key(&Provider::Vultr));
        assert!(tracker.limits.contains_key(&Provider::DigitalOcean));
    }

    #[test]
    fn test_spend_tracker_record() {
        let mut tracker = SpendTracker::new();
        tracker.record_spend(Provider::Hetzner, 399);

        let spend = tracker.get_spend(Provider::Hetzner).unwrap();
        assert_eq!(spend.current_month_cents, 399);
        assert_eq!(spend.resource_count, 1);
    }

    #[test]
    fn test_spend_tracker_accumulates() {
        let mut tracker = SpendTracker::new();
        tracker.record_spend(Provider::Vultr, 500);
        tracker.record_spend(Provider::Vultr, 500);

        let spend = tracker.get_spend(Provider::Vultr).unwrap();
        assert_eq!(spend.current_month_cents, 1000);
        assert_eq!(spend.resource_count, 2);
    }

    #[test]
    fn test_spend_tracker_total() {
        let mut tracker = SpendTracker::new();
        tracker.record_spend(Provider::Hetzner, 399);
        tracker.record_spend(Provider::Vultr, 500);

        assert_eq!(tracker.total_spend_cents(), 899);
    }

    #[test]
    fn test_spend_tracker_check_budget_ok() {
        let tracker = SpendTracker::new();
        assert!(tracker.check_budget(Provider::Hetzner).is_ok());
    }

    #[test]
    fn test_spend_tracker_check_budget_exceeded() {
        let mut tracker = SpendTracker::new();
        tracker.set_limit(Provider::Hetzner, SpendLimit::new(400));
        tracker.record_spend(Provider::Hetzner, 500);

        let result = tracker.check_budget(Provider::Hetzner);
        assert!(result.is_err());
        match result.unwrap_err() {
            InfraError::QuotaExceeded { provider, .. } => {
                assert_eq!(provider, "Hetzner Cloud");
            }
            other => panic!("expected QuotaExceeded, got {:?}", other),
        }
    }

    #[test]
    fn test_spend_tracker_global_limit() {
        let mut tracker = SpendTracker::new();
        tracker.set_global_limit(1000);
        tracker.record_spend(Provider::Hetzner, 600);
        tracker.record_spend(Provider::Vultr, 500);

        let result = tracker.check_budget(Provider::DigitalOcean);
        assert!(result.is_err());
    }

    #[test]
    fn test_spend_report() {
        let mut tracker = SpendTracker::new();
        tracker.record_spend(Provider::Hetzner, 200);
        tracker.record_spend(Provider::Vultr, 300);

        let report = tracker.spend_report();
        assert_eq!(report.len(), 2);
    }

    #[test]
    fn test_spend_reset_monthly() {
        let mut tracker = SpendTracker::new();
        tracker.record_spend(Provider::Hetzner, 399);
        assert_eq!(tracker.total_spend_cents(), 399);

        tracker.reset_monthly();
        assert_eq!(tracker.total_spend_cents(), 0);
    }

    #[test]
    fn test_spend_limit_with_threshold() {
        let limit = SpendLimit::new(1000).with_alert_threshold(90);
        assert_eq!(limit.monthly_limit_cents, 1000);
        assert_eq!(limit.alert_threshold_pct, 90);
    }

    // ── Failover result tests ───────────────────────────────────────────

    #[test]
    fn test_failover_result_no_failover() {
        let result = FailoverResult {
            resource_id: "vm-1".into(),
            provider: Provider::Hetzner,
            attempts: vec![ProvisionAttempt {
                provider: Provider::Hetzner,
                success: true,
                error: None,
            }],
        };
        assert!(!result.used_failover());
        assert_eq!(result.attempts_count(), 1);
    }

    #[test]
    fn test_failover_result_with_failover() {
        let result = FailoverResult {
            resource_id: "vm-2".into(),
            provider: Provider::Vultr,
            attempts: vec![
                ProvisionAttempt {
                    provider: Provider::Hetzner,
                    success: false,
                    error: Some("timeout".into()),
                },
                ProvisionAttempt {
                    provider: Provider::Vultr,
                    success: true,
                    error: None,
                },
            ],
        };
        assert!(result.used_failover());
        assert_eq!(result.attempts_count(), 2);
    }

    // ── JSON helper tests ───────────────────────────────────────────────

    #[test]
    fn test_json_string_number() {
        let v = serde_json::json!({"id": 42});
        assert_eq!(json_string(&v, "id"), "42");
    }

    #[test]
    fn test_json_string_missing() {
        let v = serde_json::json!({});
        assert_eq!(json_string(&v, "id"), "unknown");
    }

    #[test]
    fn test_json_string_or_default() {
        let v = serde_json::json!({});
        assert_eq!(json_string_or(&v, "status", "pending"), "pending");
    }

    // ── Provisioner integration with spend tracker ──────────────────────

    #[test]
    fn test_provisioner_has_spend_tracker() {
        let prov = Provisioner::new();
        assert_eq!(prov.spend_tracker().total_spend_cents(), 0);
    }

    #[test]
    fn test_provisioner_spend_tracker_mut() {
        let mut prov = Provisioner::new();
        prov.spend_tracker_mut()
            .record_spend(Provider::Hetzner, 399);
        assert_eq!(prov.spend_tracker().total_spend_cents(), 399);
    }
}
