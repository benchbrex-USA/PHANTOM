//! Autonomous account creation pipeline.
//!
//! Tracks authentication status per provider and guides the user through
//! CLI-based login flows. Account creation is driven by the provisioner
//! when a provider is needed but not yet authenticated.

use serde::{Deserialize, Serialize};

use crate::providers::{Provider, ALL_PROVIDERS};

/// Authentication method for a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// CLI-based login (e.g. `gh auth login`, `flyctl auth login`)
    CliLogin,
    /// API token set via environment variable
    ApiToken,
    /// OAuth browser flow
    OAuth,
    /// No auth required for free tier
    None,
}

/// Credential status for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountStatus {
    pub provider: Provider,
    pub auth_method: AuthMethod,
    pub authenticated: bool,
    pub username: Option<String>,
    pub message: Option<String>,
}

impl AccountStatus {
    pub fn unchecked(provider: Provider) -> Self {
        Self {
            provider,
            auth_method: auth_method_for(provider),
            authenticated: false,
            username: None,
            message: None,
        }
    }
}

/// Get the authentication method for a provider.
pub fn auth_method_for(provider: Provider) -> AuthMethod {
    match provider {
        Provider::GitHub => AuthMethod::CliLogin,
        Provider::FlyIo => AuthMethod::CliLogin,
        Provider::Cloudflare | Provider::CloudflareR2 => AuthMethod::ApiToken,
        Provider::Vercel => AuthMethod::CliLogin,
        Provider::Railway => AuthMethod::CliLogin,
        Provider::Supabase => AuthMethod::CliLogin,
        Provider::Netlify => AuthMethod::CliLogin,
        Provider::OracleCloud => AuthMethod::CliLogin,
        Provider::GoogleCloud => AuthMethod::CliLogin,
        Provider::AwsFreeTier => AuthMethod::CliLogin,
        Provider::Neon => AuthMethod::ApiToken,
        Provider::Upstash => AuthMethod::ApiToken,
        Provider::Render => AuthMethod::ApiToken,
    }
}

/// The CLI command to check auth status for a provider.
pub fn auth_check_command(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::GitHub => Some("gh auth status"),
        Provider::FlyIo => Some("flyctl auth whoami"),
        Provider::Vercel => Some("vercel whoami"),
        Provider::Railway => Some("railway whoami"),
        Provider::Supabase => Some("supabase projects list"),
        Provider::OracleCloud => Some("oci iam user get --user-id $(oci iam user list --query 'data[0].id' --raw-output)"),
        Provider::GoogleCloud => Some("gcloud auth list --filter=status:ACTIVE --format='value(account)'"),
        Provider::AwsFreeTier => Some("aws sts get-caller-identity"),
        _ => None,
    }
}

/// The CLI command to initiate login for a provider.
pub fn login_command(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::GitHub => Some("gh auth login"),
        Provider::FlyIo => Some("flyctl auth login"),
        Provider::Vercel => Some("vercel login"),
        Provider::Railway => Some("railway login"),
        Provider::Supabase => Some("supabase login"),
        Provider::OracleCloud => Some("oci setup config"),
        Provider::GoogleCloud => Some("gcloud auth login"),
        Provider::AwsFreeTier => Some("aws configure"),
        _ => None,
    }
}

/// The environment variable name for API-token-based auth.
pub fn env_var_for(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::Cloudflare | Provider::CloudflareR2 => Some("CLOUDFLARE_API_TOKEN"),
        Provider::Neon => Some("NEON_API_KEY"),
        Provider::Upstash => Some("UPSTASH_API_KEY"),
        Provider::Render => Some("RENDER_API_KEY"),
        _ => None,
    }
}

/// Account creation manager.
///
/// Checks and tracks authentication status across all providers.
pub struct AccountManager {
    statuses: Vec<AccountStatus>,
}

impl AccountManager {
    pub fn new() -> Self {
        let statuses = ALL_PROVIDERS
            .iter()
            .map(|p| AccountStatus::unchecked(*p))
            .collect();
        Self { statuses }
    }

    /// Check auth status for a single provider.
    pub fn check_auth(&mut self, provider: Provider) -> &AccountStatus {
        let idx = self
            .statuses
            .iter()
            .position(|s| s.provider == provider)
            .expect("all providers in statuses");

        let status = &mut self.statuses[idx];

        match status.auth_method {
            AuthMethod::CliLogin => {
                if let Some(cmd) = auth_check_command(provider) {
                    let output = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd)
                        .output();

                    match output {
                        Ok(o) if o.status.success() => {
                            let stdout = String::from_utf8_lossy(&o.stdout);
                            status.authenticated = true;
                            status.username = stdout.lines().next().map(|l| l.trim().to_string());
                            status.message = Some("authenticated".into());
                        }
                        _ => {
                            status.authenticated = false;
                            status.message = Some(format!(
                                "run `{}` to authenticate",
                                login_command(provider).unwrap_or("(manual setup)")
                            ));
                        }
                    }
                }
            }
            AuthMethod::ApiToken => {
                if let Some(var) = env_var_for(provider) {
                    status.authenticated = std::env::var(var).is_ok();
                    if status.authenticated {
                        status.message = Some(format!("${} is set", var));
                    } else {
                        status.message = Some(format!("set ${} to authenticate", var));
                    }
                }
            }
            AuthMethod::OAuth | AuthMethod::None => {
                status.message = Some("manual auth required".into());
            }
        }

        &self.statuses[idx]
    }

    /// Check auth for all providers.
    pub fn check_all(&mut self) -> &[AccountStatus] {
        let providers: Vec<Provider> = ALL_PROVIDERS.to_vec();
        for p in providers {
            self.check_auth(p);
        }
        &self.statuses
    }

    /// Get providers that are authenticated.
    pub fn authenticated_providers(&self) -> Vec<Provider> {
        self.statuses
            .iter()
            .filter(|s| s.authenticated)
            .map(|s| s.provider)
            .collect()
    }

    /// Get providers that need authentication.
    pub fn unauthenticated_providers(&self) -> Vec<Provider> {
        self.statuses
            .iter()
            .filter(|s| !s.authenticated)
            .map(|s| s.provider)
            .collect()
    }

    /// Get status for a specific provider.
    pub fn get_status(&self, provider: Provider) -> Option<&AccountStatus> {
        self.statuses.iter().find(|s| s.provider == provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_manager_creation() {
        let mgr = AccountManager::new();
        assert_eq!(mgr.statuses.len(), ALL_PROVIDERS.len());
    }

    #[test]
    fn test_auth_method_mapping() {
        assert_eq!(auth_method_for(Provider::GitHub), AuthMethod::CliLogin);
        assert_eq!(auth_method_for(Provider::Cloudflare), AuthMethod::ApiToken);
        assert_eq!(auth_method_for(Provider::Neon), AuthMethod::ApiToken);
    }

    #[test]
    fn test_env_var_mapping() {
        assert_eq!(env_var_for(Provider::Cloudflare), Some("CLOUDFLARE_API_TOKEN"));
        assert_eq!(env_var_for(Provider::Upstash), Some("UPSTASH_API_KEY"));
        assert_eq!(env_var_for(Provider::GitHub), None);
    }

    #[test]
    fn test_login_command_mapping() {
        assert_eq!(login_command(Provider::GitHub), Some("gh auth login"));
        assert_eq!(login_command(Provider::FlyIo), Some("flyctl auth login"));
    }

    #[test]
    fn test_unchecked_status() {
        let status = AccountStatus::unchecked(Provider::GitHub);
        assert!(!status.authenticated);
        assert_eq!(status.auth_method, AuthMethod::CliLogin);
    }
}
