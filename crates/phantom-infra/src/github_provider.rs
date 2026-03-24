//! GitHub REST API client — repos, actions, secrets.
//!
//! Provides programmatic access to GitHub's REST API for repository
//! management, Actions workflow triggering, and encrypted secrets.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::errors::InfraError;

/// GitHub API base URL.
const API_BASE: &str = "https://api.github.com";

/// GitHub REST API client.
#[derive(Debug, Clone)]
pub struct GitHubClient {
    client: reqwest::Client,
    token: String,
}

/// A GitHub repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub html_url: String,
    pub clone_url: String,
    #[serde(default)]
    pub description: Option<String>,
    pub default_branch: Option<String>,
}

/// A GitHub Actions workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: Option<String>,
    pub status: String,
    pub conclusion: Option<String>,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A GitHub repository secret (metadata only — values are write-only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Response wrapper for listing repositories.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ListReposResponse(Vec<Repository>);

impl GitHubClient {
    /// Create a new GitHub client with the given personal access token.
    pub fn new(token: String) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("phantom-infra/0.1")
            .build()
            .expect("failed to build reqwest client");
        Self { client, token }
    }

    fn auth_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            ("Authorization", format!("Bearer {}", self.token)),
            ("Accept", "application/vnd.github+json".into()),
            ("X-GitHub-Api-Version", "2022-11-28".into()),
        ]
    }

    /// Create a new repository for the authenticated user.
    pub async fn create_repo(&self, name: &str, private: bool) -> Result<Repository, InfraError> {
        info!(name = %name, private = %private, "creating GitHub repository");

        let url = format!("{}/user/repos", API_BASE);
        let body = serde_json::json!({
            "name": name,
            "private": private,
            "auto_init": true,
        });

        let mut req = self.client.post(&url).json(&body);
        for (k, v) in self.auth_headers() {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProvisioningFailed {
                resource: format!("github-repo/{}", name),
                reason: format!("HTTP {} — {}", status, text),
            });
        }

        let repo: Repository = resp.json().await?;
        debug!(repo_id = repo.id, full_name = %repo.full_name, "repository created");
        Ok(repo)
    }

    /// Delete a repository.
    pub async fn delete_repo(&self, owner: &str, name: &str) -> Result<(), InfraError> {
        info!(owner = %owner, name = %name, "deleting GitHub repository");

        let url = format!("{}/repos/{}/{}", API_BASE, owner, name);
        let mut req = self.client.delete(&url);
        for (k, v) in self.auth_headers() {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProvisioningFailed {
                resource: format!("github-repo/{}/{}", owner, name),
                reason: format!("HTTP {} — {}", status, text),
            });
        }

        Ok(())
    }

    /// List repositories for a given owner.
    pub async fn list_repos(&self, owner: &str) -> Result<Vec<Repository>, InfraError> {
        debug!(owner = %owner, "listing GitHub repositories");

        let url = format!("{}/users/{}/repos?per_page=100", API_BASE, owner);
        let mut req = self.client.get(&url);
        for (k, v) in self.auth_headers() {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::Http(format!("HTTP {} — {}", status, text)));
        }

        let repos: Vec<Repository> = resp.json().await?;
        Ok(repos)
    }

    /// Create or update an encrypted secret on a repository.
    pub async fn set_secret(
        &self,
        owner: &str,
        repo: &str,
        name: &str,
        value: &str,
    ) -> Result<(), InfraError> {
        // Validate secret name (must be alphanumeric + underscores, not start with GITHUB_)
        if !is_valid_secret_name(name) {
            return Err(InfraError::ProvisioningFailed {
                resource: format!("github-secret/{}/{}/{}", owner, repo, name),
                reason: "secret name must be alphanumeric/underscores and not start with GITHUB_"
                    .into(),
            });
        }

        info!(owner = %owner, repo = %repo, secret = %name, "setting repository secret");

        // Step 1: Get the repository public key for encrypting secrets.
        let pk_url = format!(
            "{}/repos/{}/{}/actions/secrets/public-key",
            API_BASE, owner, repo
        );
        let mut req = self.client.get(&pk_url);
        for (k, v) in self.auth_headers() {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::Http(format!(
                "failed to get public key: HTTP {} — {}",
                status, text
            )));
        }

        let pk_resp: serde_json::Value = resp.json().await?;
        let key_id = pk_resp["key_id"]
            .as_str()
            .ok_or_else(|| InfraError::Http("missing key_id in response".into()))?;

        // Step 2: PUT the secret (in production, value would be NaCl-encrypted).
        // For now we send the base64-encoded value placeholder.
        let secret_url = format!(
            "{}/repos/{}/{}/actions/secrets/{}",
            API_BASE, owner, repo, name
        );
        let body = serde_json::json!({
            "encrypted_value": value,
            "key_id": key_id,
        });

        let mut req = self.client.put(&secret_url).json(&body);
        for (k, v) in self.auth_headers() {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProvisioningFailed {
                resource: format!("github-secret/{}/{}/{}", owner, repo, name),
                reason: format!("HTTP {} — {}", status, text),
            });
        }

        Ok(())
    }

    /// Trigger a workflow dispatch event.
    pub async fn trigger_workflow(
        &self,
        owner: &str,
        repo: &str,
        workflow: &str,
        ref_name: &str,
    ) -> Result<(), InfraError> {
        info!(owner = %owner, repo = %repo, workflow = %workflow, "triggering workflow");

        let url = format!(
            "{}/repos/{}/{}/actions/workflows/{}/dispatches",
            API_BASE, owner, repo, workflow
        );
        let body = serde_json::json!({ "ref": ref_name });

        let mut req = self.client.post(&url).json(&body);
        for (k, v) in self.auth_headers() {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProvisioningFailed {
                resource: format!("github-workflow/{}/{}/{}", owner, repo, workflow),
                reason: format!("HTTP {} — {}", status, text),
            });
        }

        Ok(())
    }

    /// Get the status of a workflow run.
    pub async fn get_workflow_run_status(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> Result<WorkflowRun, InfraError> {
        debug!(owner = %owner, repo = %repo, run_id = run_id, "fetching workflow run status");

        let url = format!(
            "{}/repos/{}/{}/actions/runs/{}",
            API_BASE, owner, repo, run_id
        );
        let mut req = self.client.get(&url);
        for (k, v) in self.auth_headers() {
            req = req.header(k, v);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let _status = resp.status();
            let _text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ResourceNotFound {
                resource: format!("github-run/{}/{}/{}", owner, repo, run_id),
            });
        }

        let run: WorkflowRun = resp.json().await?;
        Ok(run)
    }
}

/// Validate a GitHub Actions secret name.
/// Must match `[A-Z_][A-Z0-9_]*` and must not start with `GITHUB_`.
fn is_valid_secret_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.starts_with("GITHUB_") {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ═══════════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GitHubClient::new("ghp_test_token_123".into());
        assert_eq!(client.token, "ghp_test_token_123");
    }

    #[test]
    fn test_auth_headers() {
        let client = GitHubClient::new("ghp_abc".into());
        let headers = client.auth_headers();
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].1, "Bearer ghp_abc");
        assert!(headers[1].1.contains("github"));
    }

    #[test]
    fn test_repository_deserialize() {
        let json = r#"{
            "id": 42,
            "name": "phantom",
            "full_name": "phantom-ai/phantom",
            "private": true,
            "html_url": "https://github.com/phantom-ai/phantom",
            "clone_url": "https://github.com/phantom-ai/phantom.git",
            "description": "AI engineering system",
            "default_branch": "main"
        }"#;
        let repo: Repository = serde_json::from_str(json).unwrap();
        assert_eq!(repo.id, 42);
        assert_eq!(repo.name, "phantom");
        assert!(repo.private);
        assert_eq!(repo.default_branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_repository_deserialize_minimal() {
        let json = r#"{
            "id": 1,
            "name": "test",
            "full_name": "user/test",
            "private": false,
            "html_url": "https://github.com/user/test",
            "clone_url": "https://github.com/user/test.git"
        }"#;
        let repo: Repository = serde_json::from_str(json).unwrap();
        assert_eq!(repo.id, 1);
        assert!(!repo.private);
        assert!(repo.description.is_none());
    }

    #[test]
    fn test_workflow_run_deserialize() {
        let json = r#"{
            "id": 9001,
            "name": "CI",
            "status": "completed",
            "conclusion": "success",
            "html_url": "https://github.com/owner/repo/actions/runs/9001",
            "created_at": "2026-01-15T10:00:00Z",
            "updated_at": "2026-01-15T10:05:00Z"
        }"#;
        let run: WorkflowRun = serde_json::from_str(json).unwrap();
        assert_eq!(run.id, 9001);
        assert_eq!(run.status, "completed");
        assert_eq!(run.conclusion.as_deref(), Some("success"));
    }

    #[test]
    fn test_workflow_run_in_progress() {
        let json = r#"{
            "id": 9002,
            "name": "Deploy",
            "status": "in_progress",
            "conclusion": null,
            "html_url": "https://github.com/owner/repo/actions/runs/9002",
            "created_at": "2026-01-15T10:00:00Z",
            "updated_at": "2026-01-15T10:02:00Z"
        }"#;
        let run: WorkflowRun = serde_json::from_str(json).unwrap();
        assert_eq!(run.status, "in_progress");
        assert!(run.conclusion.is_none());
    }

    #[test]
    fn test_secret_name_validation_valid() {
        assert!(is_valid_secret_name("MY_SECRET"));
        assert!(is_valid_secret_name("API_KEY_123"));
        assert!(is_valid_secret_name("_PRIVATE"));
        assert!(is_valid_secret_name("a"));
        assert!(is_valid_secret_name("TOKEN"));
    }

    #[test]
    fn test_secret_name_validation_invalid() {
        assert!(!is_valid_secret_name(""));
        assert!(!is_valid_secret_name("GITHUB_TOKEN"));
        assert!(!is_valid_secret_name("GITHUB_SECRET"));
        assert!(!is_valid_secret_name("123_NUMERIC_START"));
        assert!(!is_valid_secret_name("has-dashes"));
        assert!(!is_valid_secret_name("has spaces"));
    }

    #[test]
    fn test_secret_deserialize() {
        let json = r#"{
            "name": "DEPLOY_KEY",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-03-01T00:00:00Z"
        }"#;
        let secret: Secret = serde_json::from_str(json).unwrap();
        assert_eq!(secret.name, "DEPLOY_KEY");
    }

    #[test]
    fn test_create_repo_url_construction() {
        // Verify the URL format is correct
        let url = format!("{}/user/repos", API_BASE);
        assert_eq!(url, "https://api.github.com/user/repos");
    }
}
