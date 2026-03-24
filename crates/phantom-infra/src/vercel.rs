//! Vercel REST API v6 client.
//!
//! Provides programmatic access to the Vercel API for managing projects,
//! deployments, domains, and environment variables.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::errors::InfraError;

/// Vercel API base URL.
const API_BASE: &str = "https://api.vercel.com";

// ---------------------------------------------------------------------------
// Domain models
// ---------------------------------------------------------------------------

/// A Vercel project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VercelProject {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub framework: Option<String>,
    #[serde(default, rename = "createdAt")]
    pub created_at: Option<u64>,
}

/// A Vercel deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VercelDeployment {
    /// Deployment unique ID.
    #[serde(alias = "uid")]
    pub id: String,
    /// Deployment URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Deployment state (e.g. "READY", "BUILDING", "ERROR").
    #[serde(default)]
    pub state: Option<String>,
    /// Timestamp of creation (epoch millis).
    #[serde(default, rename = "createdAt")]
    pub created_at: Option<u64>,
    /// Ready state (e.g. "READY", "QUEUED", "BUILDING").
    #[serde(default, rename = "readyState")]
    pub ready_state: Option<String>,
}

/// A Vercel domain attached to a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VercelDomain {
    /// Domain name.
    pub name: String,
    /// Whether the domain is configured.
    #[serde(default)]
    pub configured: bool,
    /// Whether the domain is verified.
    #[serde(default)]
    pub verified: bool,
}

/// An environment variable on a Vercel project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    /// Variable key.
    pub key: String,
    /// Variable value (may be empty/redacted in list responses).
    #[serde(default)]
    pub value: Option<String>,
    /// Target environments (e.g. ["production", "preview", "development"]).
    #[serde(default)]
    pub target: Vec<String>,
}

// ---------------------------------------------------------------------------
// Response wrappers
// ---------------------------------------------------------------------------

/// Paginated list response for projects.
#[derive(Debug, Deserialize)]
struct ProjectListResponse {
    projects: Vec<VercelProject>,
}

/// Paginated list response for deployments.
#[derive(Debug, Deserialize)]
struct DeploymentListResponse {
    deployments: Vec<VercelDeployment>,
}

/// Paginated list response for domains.
#[derive(Debug, Deserialize)]
struct DomainListResponse {
    domains: Vec<VercelDomain>,
}

/// Paginated list response for environment variables.
#[derive(Debug, Deserialize)]
struct EnvListResponse {
    envs: Vec<EnvVar>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Vercel REST API client.
#[derive(Debug)]
pub struct VercelClient {
    http: Client,
    token: String,
    team_id: Option<String>,
}

impl VercelClient {
    /// Create a new Vercel client with the given token (personal account).
    pub fn new(token: String) -> Self {
        Self {
            http: Client::new(),
            token,
            team_id: None,
        }
    }

    /// Create a new Vercel client scoped to a team.
    pub fn with_team(token: String, team_id: String) -> Self {
        Self {
            http: Client::new(),
            token,
            team_id: Some(team_id),
        }
    }

    /// Create a new Vercel client from environment variables.
    ///
    /// Reads `VERCEL_TOKEN` (required) and `VERCEL_TEAM_ID` (optional).
    pub fn from_env() -> Result<Self, InfraError> {
        let token = std::env::var("VERCEL_TOKEN")
            .map_err(|_| InfraError::ProviderError("VERCEL_TOKEN not set".into()))?;
        if token.is_empty() {
            return Err(InfraError::AuthRequired {
                provider: "vercel".into(),
            });
        }
        let team_id = std::env::var("VERCEL_TEAM_ID")
            .ok()
            .filter(|s| !s.is_empty());
        Ok(Self {
            http: Client::new(),
            token,
            team_id,
        })
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// Build a URL with optional teamId query parameter.
    fn url(&self, path: &str) -> String {
        let base = format!("{}{}", API_BASE, path);
        match &self.team_id {
            Some(tid) => {
                if base.contains('?') {
                    format!("{}&teamId={}", base, tid)
                } else {
                    format!("{}?teamId={}", base, tid)
                }
            }
            None => base,
        }
    }

    /// Handle a non-success HTTP response.
    async fn handle_error(resp: reqwest::Response, context: &str) -> InfraError {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        warn!(status = %status, context = %context, "vercel API error");
        InfraError::ProviderError(format!("vercel {}: HTTP {} — {}", context, status, body))
    }

    // -----------------------------------------------------------------------
    // Projects
    // -----------------------------------------------------------------------

    /// List all Vercel projects.
    pub async fn list_projects(&self) -> Result<Vec<VercelProject>, InfraError> {
        let url = self.url("/v9/projects");
        debug!(url = %url, "listing vercel projects");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "list_projects").await);
        }

        let wrapper: ProjectListResponse = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("vercel parse error: {}", e)))?;

        debug!(count = wrapper.projects.len(), "vercel projects listed");
        Ok(wrapper.projects)
    }

    /// Create a new Vercel project.
    pub async fn create_project(
        &self,
        name: &str,
        framework: Option<&str>,
    ) -> Result<VercelProject, InfraError> {
        let url = self.url("/v10/projects");
        info!(name = %name, framework = ?framework, "creating vercel project");

        let mut body = serde_json::json!({
            "name": name,
        });
        if let Some(fw) = framework {
            body["framework"] = serde_json::Value::String(fw.to_string());
        }

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "create_project").await);
        }

        let project: VercelProject = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("vercel parse error: {}", e)))?;

        info!(project_id = %project.id, "vercel project created");
        Ok(project)
    }

    /// Get a Vercel project by name or ID.
    pub async fn get_project(&self, name_or_id: &str) -> Result<VercelProject, InfraError> {
        let url = self.url(&format!("/v9/projects/{}", name_or_id));
        debug!(name_or_id = %name_or_id, "getting vercel project");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(InfraError::ResourceNotFound {
                resource: format!("vercel/project/{}", name_or_id),
            });
        }

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "get_project").await);
        }

        let project: VercelProject = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("vercel parse error: {}", e)))?;

        Ok(project)
    }

    /// Delete a Vercel project by ID.
    pub async fn delete_project(&self, project_id: &str) -> Result<(), InfraError> {
        let url = self.url(&format!("/v9/projects/{}", project_id));
        info!(project_id = %project_id, "deleting vercel project");

        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "delete_project").await);
        }

        info!(project_id = %project_id, "vercel project deleted");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Deployments
    // -----------------------------------------------------------------------

    /// List deployments for a project.
    pub async fn list_deployments(
        &self,
        project_id: &str,
    ) -> Result<Vec<VercelDeployment>, InfraError> {
        let url = self.url(&format!("/v6/deployments?projectId={}", project_id));
        debug!(project_id = %project_id, "listing vercel deployments");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "list_deployments").await);
        }

        let wrapper: DeploymentListResponse = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("vercel parse error: {}", e)))?;

        debug!(
            count = wrapper.deployments.len(),
            "vercel deployments listed"
        );
        Ok(wrapper.deployments)
    }

    /// Get a single deployment by ID.
    pub async fn get_deployment(
        &self,
        deployment_id: &str,
    ) -> Result<VercelDeployment, InfraError> {
        let url = self.url(&format!("/v13/deployments/{}", deployment_id));
        debug!(deployment_id = %deployment_id, "getting vercel deployment");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(InfraError::ResourceNotFound {
                resource: format!("vercel/deployment/{}", deployment_id),
            });
        }

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "get_deployment").await);
        }

        let deployment: VercelDeployment = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("vercel parse error: {}", e)))?;

        Ok(deployment)
    }

    // -----------------------------------------------------------------------
    // Domains
    // -----------------------------------------------------------------------

    /// Add a domain to a project.
    pub async fn add_domain(
        &self,
        project_id: &str,
        domain: &str,
    ) -> Result<VercelDomain, InfraError> {
        let url = self.url(&format!("/v10/projects/{}/domains", project_id));
        info!(project_id = %project_id, domain = %domain, "adding domain to vercel project");

        let body = serde_json::json!({
            "name": domain,
        });

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "add_domain").await);
        }

        let domain_resp: VercelDomain = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("vercel parse error: {}", e)))?;

        Ok(domain_resp)
    }

    /// List domains attached to a project.
    pub async fn list_domains(&self, project_id: &str) -> Result<Vec<VercelDomain>, InfraError> {
        let url = self.url(&format!("/v9/projects/{}/domains", project_id));
        debug!(project_id = %project_id, "listing vercel project domains");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "list_domains").await);
        }

        let wrapper: DomainListResponse = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("vercel parse error: {}", e)))?;

        Ok(wrapper.domains)
    }

    // -----------------------------------------------------------------------
    // Environment variables
    // -----------------------------------------------------------------------

    /// Set an environment variable on a project.
    pub async fn set_env(
        &self,
        project_id: &str,
        key: &str,
        value: &str,
        targets: Vec<String>,
    ) -> Result<(), InfraError> {
        let url = self.url(&format!("/v10/projects/{}/env", project_id));
        info!(
            project_id = %project_id,
            key = %key,
            targets = ?targets,
            "setting vercel env var"
        );

        let body = serde_json::json!({
            "key": key,
            "value": value,
            "target": targets,
            "type": "encrypted",
        });

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "set_env").await);
        }

        Ok(())
    }

    /// List environment variables for a project.
    pub async fn list_envs(&self, project_id: &str) -> Result<Vec<EnvVar>, InfraError> {
        let url = self.url(&format!("/v9/projects/{}/env", project_id));
        debug!(project_id = %project_id, "listing vercel env vars");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "list_envs").await);
        }

        let wrapper: EnvListResponse = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("vercel parse error: {}", e)))?;

        Ok(wrapper.envs)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Client construction ------------------------------------------------

    #[test]
    fn test_client_new() {
        let client = VercelClient::new("test-token".into());
        assert_eq!(client.token, "test-token");
        assert!(client.team_id.is_none());
    }

    #[test]
    fn test_client_with_team() {
        let client = VercelClient::with_team("tok".into(), "team_abc".into());
        assert_eq!(client.token, "tok");
        assert_eq!(client.team_id.as_deref(), Some("team_abc"));
    }

    #[test]
    fn test_auth_header() {
        let client = VercelClient::new("my-vercel-token".into());
        assert_eq!(client.auth_header(), "Bearer my-vercel-token");
    }

    // -- URL construction ---------------------------------------------------

    #[test]
    fn test_url_no_team() {
        let client = VercelClient::new("tok".into());
        assert_eq!(
            client.url("/v9/projects"),
            "https://api.vercel.com/v9/projects"
        );
    }

    #[test]
    fn test_url_with_team() {
        let client = VercelClient::with_team("tok".into(), "team_123".into());
        assert_eq!(
            client.url("/v9/projects"),
            "https://api.vercel.com/v9/projects?teamId=team_123"
        );
    }

    #[test]
    fn test_url_with_team_existing_query() {
        let client = VercelClient::with_team("tok".into(), "team_x".into());
        assert_eq!(
            client.url("/v6/deployments?projectId=prj_abc"),
            "https://api.vercel.com/v6/deployments?projectId=prj_abc&teamId=team_x"
        );
    }

    // -- Type deserialization -----------------------------------------------

    #[test]
    fn test_project_deserialize() {
        let json = r#"{
            "id": "prj_abc123",
            "name": "my-nextjs-app",
            "framework": "nextjs",
            "createdAt": 1711234567890
        }"#;
        let project: VercelProject = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "prj_abc123");
        assert_eq!(project.name, "my-nextjs-app");
        assert_eq!(project.framework.as_deref(), Some("nextjs"));
        assert_eq!(project.created_at, Some(1711234567890));
    }

    #[test]
    fn test_project_minimal_deserialize() {
        let json = r#"{
            "id": "prj_min",
            "name": "minimal-project"
        }"#;
        let project: VercelProject = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "prj_min");
        assert!(project.framework.is_none());
        assert!(project.created_at.is_none());
    }

    #[test]
    fn test_deployment_deserialize() {
        let json = r#"{
            "uid": "dpl_abc123",
            "url": "my-app-abc123.vercel.app",
            "state": "READY",
            "createdAt": 1711234567890,
            "readyState": "READY"
        }"#;
        let deployment: VercelDeployment = serde_json::from_str(json).unwrap();
        assert_eq!(deployment.id, "dpl_abc123");
        assert_eq!(deployment.url.as_deref(), Some("my-app-abc123.vercel.app"));
        assert_eq!(deployment.state.as_deref(), Some("READY"));
        assert_eq!(deployment.ready_state.as_deref(), Some("READY"));
    }

    #[test]
    fn test_deployment_with_id_field() {
        let json = r#"{
            "id": "dpl_xyz789",
            "url": "my-site.vercel.app",
            "state": "BUILDING"
        }"#;
        let deployment: VercelDeployment = serde_json::from_str(json).unwrap();
        assert_eq!(deployment.id, "dpl_xyz789");
        assert_eq!(deployment.state.as_deref(), Some("BUILDING"));
    }

    #[test]
    fn test_domain_deserialize() {
        let json = r#"{
            "name": "example.com",
            "configured": true,
            "verified": false
        }"#;
        let domain: VercelDomain = serde_json::from_str(json).unwrap();
        assert_eq!(domain.name, "example.com");
        assert!(domain.configured);
        assert!(!domain.verified);
    }

    #[test]
    fn test_env_var_deserialize() {
        let json = r#"{
            "key": "DATABASE_URL",
            "value": "postgres://...",
            "target": ["production", "preview"]
        }"#;
        let env: EnvVar = serde_json::from_str(json).unwrap();
        assert_eq!(env.key, "DATABASE_URL");
        assert_eq!(env.value.as_deref(), Some("postgres://..."));
        assert_eq!(env.target, vec!["production", "preview"]);
    }

    #[test]
    fn test_env_var_no_value() {
        let json = r#"{
            "key": "SECRET_KEY",
            "target": ["production"]
        }"#;
        let env: EnvVar = serde_json::from_str(json).unwrap();
        assert_eq!(env.key, "SECRET_KEY");
        assert!(env.value.is_none());
    }

    #[test]
    fn test_project_serde_roundtrip() {
        let project = VercelProject {
            id: "prj_roundtrip".into(),
            name: "test-app".into(),
            framework: Some("nextjs".into()),
            created_at: Some(1711234567890),
        };
        let json = serde_json::to_string(&project).unwrap();
        let decoded: VercelProject = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, project.id);
        assert_eq!(decoded.name, project.name);
        assert_eq!(decoded.framework, project.framework);
    }

    #[test]
    fn test_from_env_missing_token() {
        std::env::remove_var("VERCEL_TOKEN");
        let result = VercelClient::from_env();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("VERCEL_TOKEN"), "got: {msg}");
    }

    #[test]
    fn test_domain_defaults() {
        let json = r#"{"name": "bare.com"}"#;
        let domain: VercelDomain = serde_json::from_str(json).unwrap();
        assert_eq!(domain.name, "bare.com");
        assert!(!domain.configured);
        assert!(!domain.verified);
    }
}
