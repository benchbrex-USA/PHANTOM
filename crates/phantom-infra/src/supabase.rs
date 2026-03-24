//! Supabase REST API client.
//!
//! Provides programmatic access to the Supabase Management API for creating
//! and managing projects, running SQL queries, and retrieving API keys.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::errors::InfraError;

/// Supabase Management API base URL.
const API_BASE: &str = "https://api.supabase.com/v1";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the Supabase client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseConfig {
    /// Supabase Management API access token.
    pub access_token: String,
}

impl SupabaseConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), InfraError> {
        if self.access_token.is_empty() {
            return Err(InfraError::AuthRequired {
                provider: "supabase".into(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Domain models
// ---------------------------------------------------------------------------

/// Database connection details within a Supabase project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database host.
    pub host: String,
    /// Database port.
    #[serde(default = "default_db_port")]
    pub port: u16,
    /// Database user.
    #[serde(default)]
    pub user: Option<String>,
    /// Database password.
    #[serde(default)]
    pub password: Option<String>,
    /// Database name.
    #[serde(default)]
    pub name: Option<String>,
}

fn default_db_port() -> u16 {
    5432
}

/// A Supabase project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseProject {
    pub id: String,
    pub name: String,
    pub organization_id: String,
    pub region: String,
    #[serde(default)]
    pub database: Option<DatabaseConfig>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// A database migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    /// Migration version / identifier.
    pub id: String,
    /// Human-readable migration name.
    pub name: String,
    /// SQL content of the migration.
    pub sql: String,
}

/// A Supabase API key (anon, service_role, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Key name (e.g. "anon", "service_role").
    pub name: String,
    /// The actual API key value.
    pub api_key: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Supabase Management API client.
#[derive(Debug)]
pub struct SupabaseClient {
    http: Client,
    access_token: String,
}

impl SupabaseClient {
    /// Create a new Supabase client with the given access token.
    pub fn new(access_token: String) -> Self {
        Self {
            http: Client::new(),
            access_token,
        }
    }

    /// Create a new Supabase client from environment variables.
    ///
    /// Reads `SUPABASE_ACCESS_TOKEN`.
    pub fn from_env() -> Result<Self, InfraError> {
        let access_token = std::env::var("SUPABASE_ACCESS_TOKEN")
            .map_err(|_| InfraError::ProviderError("SUPABASE_ACCESS_TOKEN not set".into()))?;
        if access_token.is_empty() {
            return Err(InfraError::AuthRequired {
                provider: "supabase".into(),
            });
        }
        Ok(Self::new(access_token))
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.access_token)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", API_BASE, path)
    }

    /// Handle a non-success HTTP response by reading the body and returning
    /// an appropriate `InfraError`.
    async fn handle_error(resp: reqwest::Response, context: &str) -> InfraError {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        warn!(status = %status, context = %context, "supabase API error");
        InfraError::ProviderError(format!("supabase {}: HTTP {} — {}", context, status, body))
    }

    // -----------------------------------------------------------------------
    // Project management
    // -----------------------------------------------------------------------

    /// List all Supabase projects.
    pub async fn list_projects(&self) -> Result<Vec<SupabaseProject>, InfraError> {
        let url = self.url("/projects");
        debug!(url = %url, "listing supabase projects");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "list_projects").await);
        }

        let projects: Vec<SupabaseProject> = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("supabase parse error: {}", e)))?;

        debug!(count = projects.len(), "supabase projects listed");
        Ok(projects)
    }

    /// Create a new Supabase project.
    pub async fn create_project(
        &self,
        name: &str,
        org_id: &str,
        region: &str,
        db_password: &str,
    ) -> Result<SupabaseProject, InfraError> {
        let url = self.url("/projects");
        info!(name = %name, org_id = %org_id, region = %region, "creating supabase project");

        let body = serde_json::json!({
            "name": name,
            "organization_id": org_id,
            "region": region,
            "plan": "free",
            "db_pass": db_password,
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
            return Err(Self::handle_error(resp, "create_project").await);
        }

        let project: SupabaseProject = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("supabase parse error: {}", e)))?;

        info!(project_id = %project.id, "supabase project created");
        Ok(project)
    }

    /// Get a Supabase project by ID.
    pub async fn get_project(&self, project_id: &str) -> Result<SupabaseProject, InfraError> {
        let url = self.url(&format!("/projects/{}", project_id));
        debug!(project_id = %project_id, "getting supabase project");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(InfraError::ResourceNotFound {
                resource: format!("supabase/project/{}", project_id),
            });
        }

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "get_project").await);
        }

        let project: SupabaseProject = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("supabase parse error: {}", e)))?;

        Ok(project)
    }

    /// Delete a Supabase project.
    pub async fn delete_project(&self, project_id: &str) -> Result<(), InfraError> {
        let url = self.url(&format!("/projects/{}", project_id));
        info!(project_id = %project_id, "deleting supabase project");

        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "delete_project").await);
        }

        info!(project_id = %project_id, "supabase project deleted");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Database
    // -----------------------------------------------------------------------

    /// Run a SQL query against a project's database via the Management API.
    pub async fn run_sql(
        &self,
        project_id: &str,
        sql: &str,
    ) -> Result<serde_json::Value, InfraError> {
        let url = self.url(&format!("/projects/{}/database/query", project_id));
        debug!(project_id = %project_id, "running SQL on supabase project");

        let body = serde_json::json!({
            "query": sql,
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
            return Err(Self::handle_error(resp, "run_sql").await);
        }

        let result: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("supabase parse error: {}", e)))?;

        Ok(result)
    }

    /// Check the health of a project's database.
    pub async fn health_check(&self, project_id: &str) -> Result<bool, InfraError> {
        let url = self.url(&format!("/projects/{}/health", project_id));
        debug!(project_id = %project_id, "health check for supabase project");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(true)
        } else {
            debug!(
                project_id = %project_id,
                status = %resp.status(),
                "supabase health check returned non-success"
            );
            Ok(false)
        }
    }

    // -----------------------------------------------------------------------
    // API keys
    // -----------------------------------------------------------------------

    /// Get the API keys for a project (anon, service_role, etc.).
    pub async fn get_api_keys(&self, project_id: &str) -> Result<Vec<ApiKey>, InfraError> {
        let url = self.url(&format!("/projects/{}/api-keys", project_id));
        debug!(project_id = %project_id, "getting supabase API keys");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::handle_error(resp, "get_api_keys").await);
        }

        let keys: Vec<ApiKey> = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("supabase parse error: {}", e)))?;

        debug!(count = keys.len(), "supabase API keys retrieved");
        Ok(keys)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Config validation --------------------------------------------------

    #[test]
    fn test_config_validate_success() {
        let cfg = SupabaseConfig {
            access_token: "sbp_abc123xyz".into(),
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_token() {
        let cfg = SupabaseConfig {
            access_token: "".into(),
        };
        let err = cfg.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("authentication required"), "got: {msg}");
    }

    // -- Client construction ------------------------------------------------

    #[test]
    fn test_client_new() {
        let client = SupabaseClient::new("test-token".into());
        assert_eq!(client.access_token, "test-token");
    }

    #[test]
    fn test_auth_header() {
        let client = SupabaseClient::new("my-secret-token".into());
        assert_eq!(client.auth_header(), "Bearer my-secret-token");
    }

    #[test]
    fn test_url_construction() {
        let client = SupabaseClient::new("tok".into());
        assert_eq!(
            client.url("/projects"),
            "https://api.supabase.com/v1/projects"
        );
        assert_eq!(
            client.url("/projects/abc123/api-keys"),
            "https://api.supabase.com/v1/projects/abc123/api-keys"
        );
    }

    // -- Type deserialization -----------------------------------------------

    #[test]
    fn test_project_deserialize() {
        let json = r#"{
            "id": "proj-abc-123",
            "name": "phantom-app",
            "organization_id": "org-xyz",
            "region": "us-east-1",
            "status": "ACTIVE_HEALTHY",
            "created_at": "2026-03-01T10:00:00Z"
        }"#;
        let project: SupabaseProject = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "proj-abc-123");
        assert_eq!(project.name, "phantom-app");
        assert_eq!(project.organization_id, "org-xyz");
        assert_eq!(project.region, "us-east-1");
        assert_eq!(project.status.as_deref(), Some("ACTIVE_HEALTHY"));
        assert!(project.database.is_none());
    }

    #[test]
    fn test_project_with_database_deserialize() {
        let json = r#"{
            "id": "proj-db-456",
            "name": "db-project",
            "organization_id": "org-abc",
            "region": "eu-west-1",
            "database": {
                "host": "db.supabase.co",
                "port": 5432,
                "user": "postgres",
                "password": "secret123",
                "name": "postgres"
            },
            "status": "ACTIVE_HEALTHY",
            "created_at": "2026-02-15T08:30:00Z"
        }"#;
        let project: SupabaseProject = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "proj-db-456");
        let db = project.database.unwrap();
        assert_eq!(db.host, "db.supabase.co");
        assert_eq!(db.port, 5432);
        assert_eq!(db.user.as_deref(), Some("postgres"));
        assert_eq!(db.password.as_deref(), Some("secret123"));
        assert_eq!(db.name.as_deref(), Some("postgres"));
    }

    #[test]
    fn test_database_config_default_port() {
        let json = r#"{
            "host": "db.example.com"
        }"#;
        let db: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(db.port, 5432);
        assert!(db.user.is_none());
    }

    #[test]
    fn test_api_key_deserialize() {
        let json = r#"{
            "name": "anon",
            "api_key": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test"
        }"#;
        let key: ApiKey = serde_json::from_str(json).unwrap();
        assert_eq!(key.name, "anon");
        assert!(key.api_key.starts_with("eyJ"));
    }

    #[test]
    fn test_api_key_list_deserialize() {
        let json = r#"[
            {"name": "anon", "api_key": "anon-key-123"},
            {"name": "service_role", "api_key": "service-key-456"}
        ]"#;
        let keys: Vec<ApiKey> = serde_json::from_str(json).unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].name, "anon");
        assert_eq!(keys[1].name, "service_role");
    }

    #[test]
    fn test_migration_serde_roundtrip() {
        let migration = Migration {
            id: "20260301000000".into(),
            name: "create_users_table".into(),
            sql: "CREATE TABLE users (id uuid PRIMARY KEY, email text NOT NULL);".into(),
        };
        let json = serde_json::to_string(&migration).unwrap();
        let decoded: Migration = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "20260301000000");
        assert_eq!(decoded.name, "create_users_table");
        assert!(decoded.sql.contains("CREATE TABLE"));
    }

    #[test]
    fn test_project_serde_roundtrip() {
        let project = SupabaseProject {
            id: "proj-test".into(),
            name: "roundtrip".into(),
            organization_id: "org-1".into(),
            region: "us-east-1".into(),
            database: None,
            status: Some("ACTIVE_HEALTHY".into()),
            created_at: Some("2026-03-24T00:00:00Z".into()),
        };
        let json = serde_json::to_string(&project).unwrap();
        let decoded: SupabaseProject = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, project.id);
        assert_eq!(decoded.name, project.name);
        assert_eq!(decoded.region, project.region);
    }

    #[test]
    fn test_project_minimal_deserialize() {
        // Only required fields present
        let json = r#"{
            "id": "min-proj",
            "name": "minimal",
            "organization_id": "org-min",
            "region": "ap-southeast-1"
        }"#;
        let project: SupabaseProject = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, "min-proj");
        assert!(project.status.is_none());
        assert!(project.created_at.is_none());
        assert!(project.database.is_none());
    }

    #[test]
    fn test_from_env_missing_token() {
        // Temporarily ensure the env var is not set
        std::env::remove_var("SUPABASE_ACCESS_TOKEN");
        let result = SupabaseClient::from_env();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("SUPABASE_ACCESS_TOKEN"), "got: {msg}");
    }
}
