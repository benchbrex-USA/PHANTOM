//! Neon Serverless Postgres API client.
//!
//! Provides programmatic access to Neon's management API for creating
//! projects, branches, and obtaining connection URIs for serverless
//! Postgres databases.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::errors::InfraError;

/// Neon API base URL.
const API_BASE: &str = "https://console.neon.tech/api/v2";

/// Supported Neon regions.
const VALID_REGIONS: &[&str] = &[
    "aws-us-east-1",
    "aws-us-east-2",
    "aws-us-west-2",
    "aws-eu-central-1",
    "aws-ap-southeast-1",
    "aws-ap-southeast-2",
];

/// Neon Serverless Postgres API client.
#[derive(Debug, Clone)]
pub struct NeonClient {
    client: reqwest::Client,
    api_key: String,
}

/// A Neon project (top-level container for databases and branches).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeonProject {
    pub id: String,
    pub name: String,
    pub region_id: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub pg_version: Option<u32>,
    #[serde(default)]
    pub store_passwords: Option<bool>,
}

/// A Neon branch (Git-like branch of a database).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeonBranch {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub current_state: Option<String>,
}

/// A Neon endpoint (compute instance attached to a branch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeonEndpoint {
    pub id: String,
    pub project_id: String,
    pub branch_id: String,
    pub host: String,
    #[serde(default)]
    pub region_id: Option<String>,
    #[serde(default)]
    pub current_state: Option<String>,
}

/// Wrapper for the create-project response envelope.
#[derive(Debug, Deserialize)]
struct CreateProjectResponse {
    project: NeonProject,
}

/// Wrapper for the list-projects response envelope.
#[derive(Debug, Deserialize)]
struct ListProjectsResponse {
    projects: Vec<NeonProject>,
}

/// Wrapper for the create-branch response envelope.
#[derive(Debug, Deserialize)]
struct CreateBranchResponse {
    branch: NeonBranch,
}

/// Wrapper for connection URI response.
#[derive(Debug, Deserialize)]
struct ConnectionUriResponse {
    uri: String,
}

impl NeonClient {
    /// Create a new Neon client with the given API key.
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .build()
            .expect("failed to build reqwest client");
        Self { client, api_key }
    }

    fn auth_header(&self) -> (&'static str, String) {
        ("Authorization", format!("Bearer {}", self.api_key))
    }

    /// Create a new Neon project.
    pub async fn create_project(
        &self,
        name: &str,
        region: &str,
    ) -> Result<NeonProject, InfraError> {
        if !is_valid_region(region) {
            return Err(InfraError::ProvisioningFailed {
                resource: format!("neon-project/{}", name),
                reason: format!(
                    "invalid region '{}', must be one of: {}",
                    region,
                    VALID_REGIONS.join(", ")
                ),
            });
        }

        info!(name = %name, region = %region, "creating Neon project");

        let url = format!("{}/projects", API_BASE);
        let body = serde_json::json!({
            "project": {
                "name": name,
                "region_id": region,
            }
        });

        let (hdr_k, hdr_v) = self.auth_header();
        let resp = self
            .client
            .post(&url)
            .header(hdr_k, hdr_v)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProvisioningFailed {
                resource: format!("neon-project/{}", name),
                reason: format!("HTTP {} — {}", status, text),
            });
        }

        let wrapper: CreateProjectResponse = resp.json().await?;
        debug!(project_id = %wrapper.project.id, "Neon project created");
        Ok(wrapper.project)
    }

    /// Delete a Neon project.
    pub async fn delete_project(&self, project_id: &str) -> Result<(), InfraError> {
        info!(project_id = %project_id, "deleting Neon project");

        let url = format!("{}/projects/{}", API_BASE, project_id);
        let (hdr_k, hdr_v) = self.auth_header();
        let resp = self.client.delete(&url).header(hdr_k, hdr_v).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProvisioningFailed {
                resource: format!("neon-project/{}", project_id),
                reason: format!("HTTP {} — {}", status, text),
            });
        }

        Ok(())
    }

    /// List all Neon projects.
    pub async fn list_projects(&self) -> Result<Vec<NeonProject>, InfraError> {
        debug!("listing Neon projects");

        let url = format!("{}/projects", API_BASE);
        let (hdr_k, hdr_v) = self.auth_header();
        let resp = self.client.get(&url).header(hdr_k, hdr_v).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::Http(format!("HTTP {} — {}", status, text)));
        }

        let wrapper: ListProjectsResponse = resp.json().await?;
        Ok(wrapper.projects)
    }

    /// Get a connection URI for a project's default branch and database.
    pub async fn get_connection_uri(&self, project_id: &str) -> Result<String, InfraError> {
        debug!(project_id = %project_id, "fetching Neon connection URI");

        let url = format!("{}/projects/{}/connection_uri", API_BASE, project_id);
        let (hdr_k, hdr_v) = self.auth_header();
        let resp = self.client.get(&url).header(hdr_k, hdr_v).send().await?;

        if !resp.status().is_success() {
            let _status = resp.status();
            let _text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ResourceNotFound {
                resource: format!("neon-project/{}/connection_uri", project_id),
            });
        }

        let wrapper: ConnectionUriResponse = resp.json().await?;
        Ok(wrapper.uri)
    }

    /// Create a branch on an existing project.
    pub async fn create_branch(
        &self,
        project_id: &str,
        name: &str,
    ) -> Result<NeonBranch, InfraError> {
        if !is_valid_branch_name(name) {
            return Err(InfraError::ProvisioningFailed {
                resource: format!("neon-branch/{}/{}", project_id, name),
                reason: "branch name must be non-empty, alphanumeric with hyphens/underscores, max 63 chars".into(),
            });
        }

        info!(project_id = %project_id, name = %name, "creating Neon branch");

        let url = format!("{}/projects/{}/branches", API_BASE, project_id);
        let body = serde_json::json!({
            "branch": {
                "name": name,
            }
        });

        let (hdr_k, hdr_v) = self.auth_header();
        let resp = self
            .client
            .post(&url)
            .header(hdr_k, hdr_v)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProvisioningFailed {
                resource: format!("neon-branch/{}/{}", project_id, name),
                reason: format!("HTTP {} — {}", status, text),
            });
        }

        let wrapper: CreateBranchResponse = resp.json().await?;
        debug!(branch_id = %wrapper.branch.id, "branch created");
        Ok(wrapper.branch)
    }
}

/// Check if a region string is valid for Neon.
fn is_valid_region(region: &str) -> bool {
    VALID_REGIONS.contains(&region)
}

/// Validate a branch name (non-empty, max 63 chars, alphanumeric + hyphens/underscores).
fn is_valid_branch_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 63 {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

// ═══════════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = NeonClient::new("neon_api_key_xyz".into());
        assert_eq!(client.api_key, "neon_api_key_xyz");
    }

    #[test]
    fn test_auth_header() {
        let client = NeonClient::new("key123".into());
        let (name, value) = client.auth_header();
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer key123");
    }

    #[test]
    fn test_project_deserialize() {
        let json = r#"{
            "id": "proj-abc-123",
            "name": "phantom-db",
            "region_id": "aws-us-east-1",
            "created_at": "2026-01-15T10:00:00Z",
            "pg_version": 16,
            "store_passwords": true
        }"#;
        let project: NeonProject = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "proj-abc-123");
        assert_eq!(project.name, "phantom-db");
        assert_eq!(project.region_id, "aws-us-east-1");
        assert_eq!(project.pg_version, Some(16));
    }

    #[test]
    fn test_branch_deserialize() {
        let json = r#"{
            "id": "br-xyz-789",
            "project_id": "proj-abc-123",
            "name": "dev-feature",
            "parent_id": "br-main-001",
            "created_at": "2026-01-20T12:00:00Z",
            "current_state": "ready"
        }"#;
        let branch: NeonBranch = serde_json::from_str(json).unwrap();
        assert_eq!(branch.id, "br-xyz-789");
        assert_eq!(branch.name, "dev-feature");
        assert_eq!(branch.parent_id.as_deref(), Some("br-main-001"));
    }

    #[test]
    fn test_connection_uri_format() {
        // Verify Neon connection URIs follow the postgres:// scheme
        let uri = "postgres://user:pass@ep-cool-name-123456.us-east-1.aws.neon.tech/neondb";
        assert!(uri.starts_with("postgres://"));
        assert!(uri.contains("neon.tech"));
    }

    #[test]
    fn test_branch_naming_valid() {
        assert!(is_valid_branch_name("main"));
        assert!(is_valid_branch_name("dev-feature"));
        assert!(is_valid_branch_name("release_v2"));
        assert!(is_valid_branch_name("a"));
        assert!(is_valid_branch_name("branch-with-123"));
    }

    #[test]
    fn test_branch_naming_invalid() {
        assert!(!is_valid_branch_name(""));
        assert!(!is_valid_branch_name("has spaces"));
        assert!(!is_valid_branch_name("has/slash"));
        assert!(!is_valid_branch_name("has.dot"));
        // 64 chars — too long
        let long = "a".repeat(64);
        assert!(!is_valid_branch_name(&long));
    }

    #[test]
    fn test_region_validation() {
        assert!(is_valid_region("aws-us-east-1"));
        assert!(is_valid_region("aws-eu-central-1"));
        assert!(!is_valid_region("us-east-1")); // must have aws- prefix
        assert!(!is_valid_region(""));
    }

    #[test]
    fn test_endpoint_deserialize() {
        let json = r#"{
            "id": "ep-cool-name-123",
            "project_id": "proj-abc",
            "branch_id": "br-main",
            "host": "ep-cool-name-123.us-east-1.aws.neon.tech",
            "region_id": "aws-us-east-1",
            "current_state": "active"
        }"#;
        let endpoint: NeonEndpoint = serde_json::from_str(json).unwrap();
        assert_eq!(endpoint.id, "ep-cool-name-123");
        assert!(endpoint.host.contains("neon.tech"));
    }
}
