//! Fly.io Machines API client for P2P mesh node deployment.
//!
//! Architecture Framework §10: P2P mesh nodes on Fly.io free tier.
//! Free tier: 3 shared-cpu-1x VMs with 256MB RAM.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::errors::InfraError;

/// Fly.io API base URL.
const API_BASE: &str = "https://api.machines.dev/v1";

/// Fly.io configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlyConfig {
    /// API token (from `fly auth token`)
    pub api_token: String,
    /// Organization slug
    pub org_slug: String,
}

impl FlyConfig {
    pub fn validate(&self) -> Result<(), InfraError> {
        if self.api_token.is_empty() {
            return Err(InfraError::ProviderError(
                "Fly.io api_token is empty".into(),
            ));
        }
        if self.org_slug.is_empty() {
            return Err(InfraError::ProviderError("Fly.io org_slug is empty".into()));
        }
        Ok(())
    }
}

/// A Fly.io application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlyApp {
    pub id: String,
    pub name: String,
    pub organization: Option<OrgRef>,
    pub status: String,
}

/// Organization reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgRef {
    pub slug: String,
}

/// A Fly.io Machine (VM).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlyMachine {
    pub id: String,
    pub name: Option<String>,
    pub state: MachineState,
    pub region: String,
    pub instance_id: Option<String>,
    pub config: Option<MachineConfig>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Machine state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MachineState {
    Created,
    Starting,
    Started,
    Stopping,
    Stopped,
    Replacing,
    Destroying,
    Destroyed,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for MachineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Starting => write!(f, "starting"),
            Self::Started => write!(f, "started"),
            Self::Stopping => write!(f, "stopping"),
            Self::Stopped => write!(f, "stopped"),
            Self::Replacing => write!(f, "replacing"),
            Self::Destroying => write!(f, "destroying"),
            Self::Destroyed => write!(f, "destroyed"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Machine configuration for creating new machines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfig {
    /// Docker image
    pub image: String,
    /// Guest VM resources
    #[serde(default)]
    pub guest: GuestConfig,
    /// Environment variables
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    /// Services (port mappings)
    #[serde(default)]
    pub services: Vec<MachineService>,
}

/// Guest VM resource configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestConfig {
    /// CPU kind (shared, performance)
    #[serde(default = "default_cpu_kind")]
    pub cpu_kind: String,
    /// Number of CPUs
    #[serde(default = "default_cpus")]
    pub cpus: u32,
    /// Memory in MB
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u32,
}

fn default_cpu_kind() -> String {
    "shared".into()
}
fn default_cpus() -> u32 {
    1
}
fn default_memory_mb() -> u32 {
    256
}

impl Default for GuestConfig {
    fn default() -> Self {
        Self {
            cpu_kind: default_cpu_kind(),
            cpus: default_cpus(),
            memory_mb: default_memory_mb(),
        }
    }
}

/// Service configuration (port mapping).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineService {
    pub protocol: String,
    pub internal_port: u16,
    pub ports: Vec<ServicePort>,
}

/// A mapped port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicePort {
    pub port: u16,
    pub handlers: Vec<String>,
}

/// Known Fly.io regions for mesh deployment.
pub const MESH_REGIONS: &[&str] = &[
    "iad", // Ashburn, Virginia
    "lax", // Los Angeles
    "sjc", // San Jose
    "ams", // Amsterdam
    "fra", // Frankfurt
    "lhr", // London
    "nrt", // Tokyo
    "sin", // Singapore
    "syd", // Sydney
];

/// Validate an app name (lowercase, alphanumeric + hyphens, 3-30 chars).
pub fn validate_app_name(name: &str) -> Result<(), InfraError> {
    if name.len() < 3 || name.len() > 30 {
        return Err(InfraError::ProviderError(
            "Fly app name must be 3-30 characters".into(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(InfraError::ProviderError(
            "Fly app name must be lowercase alphanumeric with hyphens".into(),
        ));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err(InfraError::ProviderError(
            "Fly app name cannot start or end with a hyphen".into(),
        ));
    }
    Ok(())
}

/// The Fly.io Machines API client.
pub struct FlyClient {
    http: reqwest::Client,
    config: FlyConfig,
}

impl FlyClient {
    /// Create a new Fly.io client.
    pub fn new(config: FlyConfig) -> Result<Self, InfraError> {
        config.validate()?;
        let http = reqwest::Client::new();
        Ok(Self { http, config })
    }

    /// Create from environment variables.
    pub fn from_env() -> Result<Self, InfraError> {
        let config = FlyConfig {
            api_token: std::env::var("FLY_API_TOKEN")
                .map_err(|_| InfraError::ProviderError("FLY_API_TOKEN not set".into()))?,
            org_slug: std::env::var("FLY_ORG").unwrap_or_else(|_| "personal".into()),
        };
        Self::new(config)
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_token)
    }

    /// Create a new Fly app.
    pub async fn create_app(&self, name: &str, region: &str) -> Result<FlyApp, InfraError> {
        validate_app_name(name)?;

        let url = format!("{}/apps", API_BASE);
        let body = serde_json::json!({
            "app_name": name,
            "org_slug": self.config.org_slug,
            "network": format!("{}-net", name),
        });

        debug!("Fly.io creating app: {} in region {}", name, region);

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("Fly.io create app failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "Fly.io create app failed ({}): {}",
                status, body
            )));
        }

        let app: FlyApp = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("Fly.io parse error: {}", e)))?;

        info!("Fly.io: created app {}", app.name);
        Ok(app)
    }

    /// Delete a Fly app.
    pub async fn delete_app(&self, name: &str) -> Result<(), InfraError> {
        let url = format!("{}/apps/{}", API_BASE, name);

        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("Fly.io delete app failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "Fly.io delete app failed ({}): {}",
                status, body
            )));
        }

        info!("Fly.io: deleted app {}", name);
        Ok(())
    }

    /// List machines in an app.
    pub async fn list_machines(&self, app_name: &str) -> Result<Vec<FlyMachine>, InfraError> {
        let url = format!("{}/apps/{}/machines", API_BASE, app_name);

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| {
                InfraError::ProviderError(format!("Fly.io list machines failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "Fly.io list machines failed ({}): {}",
                status, body
            )));
        }

        resp.json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("Fly.io parse error: {}", e)))
    }

    /// Create a new machine in an app.
    pub async fn create_machine(
        &self,
        app_name: &str,
        region: &str,
        config: MachineConfig,
    ) -> Result<FlyMachine, InfraError> {
        let url = format!("{}/apps/{}/machines", API_BASE, app_name);

        let body = serde_json::json!({
            "region": region,
            "config": config,
        });

        debug!("Fly.io creating machine in {} ({})", app_name, region);

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                InfraError::ProviderError(format!("Fly.io create machine failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "Fly.io create machine failed ({}): {}",
                status, body
            )));
        }

        let machine: FlyMachine = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("Fly.io parse error: {}", e)))?;

        info!("Fly.io: created machine {} in {}", machine.id, region);
        Ok(machine)
    }

    /// Stop a machine.
    pub async fn stop_machine(&self, app_name: &str, machine_id: &str) -> Result<(), InfraError> {
        let url = format!(
            "{}/apps/{}/machines/{}/stop",
            API_BASE, app_name, machine_id
        );

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("Fly.io stop failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "Fly.io stop machine failed ({}): {}",
                status, body
            )));
        }

        info!("Fly.io: stopped machine {}", machine_id);
        Ok(())
    }

    /// Destroy (permanently delete) a machine.
    pub async fn destroy_machine(
        &self,
        app_name: &str,
        machine_id: &str,
    ) -> Result<(), InfraError> {
        let url = format!("{}/apps/{}/machines/{}", API_BASE, app_name, machine_id);

        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("Fly.io destroy failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "Fly.io destroy machine failed ({}): {}",
                status, body
            )));
        }

        info!("Fly.io: destroyed machine {}", machine_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> FlyConfig {
        FlyConfig {
            api_token: "fo1_test_token_12345".into(),
            org_slug: "benchbrex".into(),
        }
    }

    #[test]
    fn test_config_validate() {
        assert!(test_config().validate().is_ok());
    }

    #[test]
    fn test_config_empty_token() {
        let mut config = test_config();
        config.api_token = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_empty_org() {
        let mut config = test_config();
        config.org_slug = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_app_name_valid() {
        assert!(validate_app_name("phantom-mesh-1").is_ok());
        assert!(validate_app_name("abc").is_ok());
        assert!(validate_app_name("my-app-123").is_ok());
    }

    #[test]
    fn test_app_name_too_short() {
        assert!(validate_app_name("ab").is_err());
    }

    #[test]
    fn test_app_name_too_long() {
        let long = "a".repeat(31);
        assert!(validate_app_name(&long).is_err());
    }

    #[test]
    fn test_app_name_invalid_chars() {
        assert!(validate_app_name("My-App").is_err());
        assert!(validate_app_name("my_app").is_err());
        assert!(validate_app_name("my app").is_err());
    }

    #[test]
    fn test_app_name_leading_trailing_hyphen() {
        assert!(validate_app_name("-myapp").is_err());
        assert!(validate_app_name("myapp-").is_err());
    }

    #[test]
    fn test_machine_state_serde() {
        let json = r#""started""#;
        let state: MachineState = serde_json::from_str(json).unwrap();
        assert_eq!(state, MachineState::Started);

        let json = r#""something_new""#;
        let state: MachineState = serde_json::from_str(json).unwrap();
        assert_eq!(state, MachineState::Unknown);
    }

    #[test]
    fn test_machine_deser() {
        let json = serde_json::json!({
            "id": "d890dead",
            "name": "phantom-mesh-iad",
            "state": "started",
            "region": "iad",
            "instance_id": "01HTEST",
            "created_at": "2026-03-20T10:00:00Z",
            "updated_at": "2026-03-20T10:05:00Z"
        });
        let machine: FlyMachine = serde_json::from_value(json).unwrap();
        assert_eq!(machine.id, "d890dead");
        assert_eq!(machine.state, MachineState::Started);
        assert_eq!(machine.region, "iad");
    }

    #[test]
    fn test_guest_config_defaults() {
        let guest = GuestConfig::default();
        assert_eq!(guest.cpu_kind, "shared");
        assert_eq!(guest.cpus, 1);
        assert_eq!(guest.memory_mb, 256);
    }

    #[test]
    fn test_machine_config_serde() {
        let config = MachineConfig {
            image: "phantom-mesh:latest".into(),
            guest: GuestConfig::default(),
            env: [("PHANTOM_ROLE".into(), "mesh-node".into())]
                .into_iter()
                .collect(),
            services: vec![MachineService {
                protocol: "tcp".into(),
                internal_port: 9000,
                ports: vec![ServicePort {
                    port: 443,
                    handlers: vec!["tls".into(), "http".into()],
                }],
            }],
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["image"], "phantom-mesh:latest");
        assert_eq!(json["guest"]["cpu_kind"], "shared");
    }

    #[test]
    fn test_mesh_regions() {
        assert!(MESH_REGIONS.contains(&"iad"));
        assert!(MESH_REGIONS.contains(&"fra"));
        assert!(MESH_REGIONS.contains(&"nrt"));
        assert!(MESH_REGIONS.len() >= 9);
    }

    #[test]
    fn test_fly_app_deser() {
        let json = serde_json::json!({
            "id": "phantom-mesh-prod",
            "name": "phantom-mesh-prod",
            "organization": {"slug": "benchbrex"},
            "status": "deployed"
        });
        let app: FlyApp = serde_json::from_value(json).unwrap();
        assert_eq!(app.name, "phantom-mesh-prod");
        assert_eq!(app.organization.unwrap().slug, "benchbrex");
    }
}
