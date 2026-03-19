//! Autonomous account creation pipeline — Architecture Framework §5.
//!
//! Tracks authentication status per provider and drives signup flows:
//!   • CLI-based login (GitHub, Fly.io, Vercel, etc.)
//!   • OAuth browser flows via Playwright stubs
//!   • API token management via env vars
//!   • Credential rotation (30-day default)
//!   • CAPTCHA detection + TUI pause for manual intervention
//!   • Health checks wired into the self-healing engine
//!   • Zero-footprint credential deletion on `master destroy`

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use crate::providers::{Provider, ALL_PROVIDERS};

// ── Auth Types ─────────────────────────────────────────────────────────────

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
        Provider::OracleCloud => Some(
            "oci iam user get --user-id $(oci iam user list --query 'data[0].id' --raw-output)",
        ),
        Provider::GoogleCloud => {
            Some("gcloud auth list --filter=status:ACTIVE --format='value(account)'")
        }
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

// ── OAuth Flow Definitions ─────────────────────────────────────────────────

/// OAuth configuration for a provider's signup/login flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlowConfig {
    /// Provider this flow is for
    pub provider: Provider,
    /// OAuth authorize URL
    pub authorize_url: String,
    /// Token exchange URL
    pub token_url: String,
    /// Required scopes
    pub scopes: Vec<String>,
    /// Whether this flow opens a browser (vs. device code)
    pub browser_flow: bool,
    /// Redirect URI for the callback
    pub redirect_uri: String,
}

/// Get the OAuth flow configuration for providers that support it.
pub fn oauth_flow_config(provider: Provider) -> Option<OAuthFlowConfig> {
    match provider {
        Provider::GitHub => Some(OAuthFlowConfig {
            provider,
            authorize_url: "https://github.com/login/oauth/authorize".into(),
            token_url: "https://github.com/login/oauth/access_token".into(),
            scopes: vec![
                "repo".into(),
                "read:org".into(),
                "workflow".into(),
                "admin:public_key".into(),
            ],
            browser_flow: true,
            redirect_uri: "http://127.0.0.1:19876/callback".into(),
        }),
        Provider::Cloudflare | Provider::CloudflareR2 => Some(OAuthFlowConfig {
            provider,
            authorize_url: "https://dash.cloudflare.com/oauth2/authorize".into(),
            token_url: "https://dash.cloudflare.com/oauth2/token".into(),
            scopes: vec![
                "account:read".into(),
                "workers:write".into(),
                "r2:write".into(),
                "dns:edit".into(),
            ],
            browser_flow: true,
            redirect_uri: "http://127.0.0.1:19876/callback".into(),
        }),
        _ => Option::None,
    }
}

// ── Credential Lifecycle ───────────────────────────────────────────────────

/// Default credential rotation interval (30 days).
pub const DEFAULT_ROTATION_DAYS: u64 = 30;

/// Credential record with lifecycle tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRecord {
    /// Provider this credential is for
    pub provider: Provider,
    /// When the credential was created (epoch seconds)
    pub created_at: u64,
    /// When the credential was last rotated (epoch seconds)
    pub last_rotated_at: u64,
    /// Rotation interval in seconds
    pub rotation_interval_secs: u64,
    /// Whether this credential has been revoked
    pub revoked: bool,
    /// Credential type (token, oauth, ssh-key, etc.)
    pub credential_type: CredentialType,
    /// Keychain service name (for macOS Keychain storage)
    pub keychain_service: Option<String>,
}

/// Type of credential stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    OAuthToken,
    ApiToken,
    SshKey,
    ServiceAccount,
    CliSession,
}

impl fmt::Display for CredentialType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OAuthToken => write!(f, "oauth_token"),
            Self::ApiToken => write!(f, "api_token"),
            Self::SshKey => write!(f, "ssh_key"),
            Self::ServiceAccount => write!(f, "service_account"),
            Self::CliSession => write!(f, "cli_session"),
        }
    }
}

impl CredentialRecord {
    /// Create a new credential record with the default rotation interval.
    pub fn new(provider: Provider, credential_type: CredentialType) -> Self {
        let now = current_epoch_secs();
        Self {
            provider,
            created_at: now,
            last_rotated_at: now,
            rotation_interval_secs: DEFAULT_ROTATION_DAYS * 86400,
            revoked: false,
            credential_type,
            keychain_service: Some(format!("phantom-{}", provider.display_name().to_lowercase().replace(' ', "-"))),
        }
    }

    /// Check if this credential needs rotation.
    pub fn needs_rotation(&self) -> bool {
        if self.revoked {
            return false;
        }
        let now = current_epoch_secs();
        now.saturating_sub(self.last_rotated_at) >= self.rotation_interval_secs
    }

    /// How many seconds until rotation is needed.
    pub fn seconds_until_rotation(&self) -> u64 {
        if self.revoked || self.needs_rotation() {
            return 0;
        }
        let elapsed = current_epoch_secs().saturating_sub(self.last_rotated_at);
        self.rotation_interval_secs.saturating_sub(elapsed)
    }

    /// Mark as rotated right now.
    pub fn mark_rotated(&mut self) {
        self.last_rotated_at = current_epoch_secs();
    }

    /// Mark as revoked.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }

    /// Age in seconds since creation.
    pub fn age_secs(&self) -> u64 {
        current_epoch_secs().saturating_sub(self.created_at)
    }
}

// ── CAPTCHA Detection ──────────────────────────────────────────────────────

/// Known CAPTCHA indicators in HTTP responses and page content.
const CAPTCHA_INDICATORS: &[&str] = &[
    "captcha",
    "recaptcha",
    "hcaptcha",
    "turnstile",
    "challenge-platform",
    "cf-challenge",
    "verify you are human",
    "are you a robot",
    "security check",
    "bot detection",
];

/// Result of CAPTCHA detection scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaDetection {
    /// Whether a CAPTCHA was detected
    pub detected: bool,
    /// Which indicator matched
    pub indicator: Option<String>,
    /// Provider that triggered it
    pub provider: Provider,
    /// Suggested action for the TUI prompt
    pub action: CaptchaAction,
}

/// What to do when a CAPTCHA is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptchaAction {
    /// Pause and display TUI prompt for manual completion
    PauseForManual,
    /// Retry after a delay (some challenges are transient)
    RetryAfterDelay,
    /// Skip this provider and try an alternative
    SkipProvider,
}

impl fmt::Display for CaptchaAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PauseForManual => write!(f, "pause_for_manual"),
            Self::RetryAfterDelay => write!(f, "retry_after_delay"),
            Self::SkipProvider => write!(f, "skip_provider"),
        }
    }
}

/// Scan content for CAPTCHA indicators.
pub fn detect_captcha(content: &str, provider: Provider) -> CaptchaDetection {
    let lower = content.to_lowercase();

    for indicator in CAPTCHA_INDICATORS {
        if lower.contains(indicator) {
            return CaptchaDetection {
                detected: true,
                indicator: Some((*indicator).to_string()),
                provider,
                action: CaptchaAction::PauseForManual,
            };
        }
    }

    CaptchaDetection {
        detected: false,
        indicator: None,
        provider,
        action: CaptchaAction::RetryAfterDelay,
    }
}

// ── Playwright Signup Flows ────────────────────────────────────────────────

/// A step in a browser-based signup flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupStep {
    /// Human-readable description
    pub description: String,
    /// The browser action to perform
    pub action: SignupAction,
    /// Whether to scan for CAPTCHA after this step
    pub check_captcha: bool,
    /// Timeout for this step
    pub timeout_secs: u64,
}

/// Actions the browser automation can take during signup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignupAction {
    /// Navigate to a URL
    Navigate { url: String },
    /// Fill a form field by CSS selector
    Fill { selector: String, value: String },
    /// Click an element by CSS selector
    Click { selector: String },
    /// Wait for a selector to appear
    WaitFor { selector: String },
    /// Wait for navigation to a URL pattern
    WaitForUrl { pattern: String },
    /// Extract text from an element (e.g. to capture a token)
    ExtractToken { selector: String },
    /// Take a screenshot for manual review
    Screenshot { filename: String },
    /// Pause for manual CAPTCHA completion
    PauseForCaptcha { message: String },
}

/// Build the Playwright signup step sequence for a provider.
pub fn signup_steps(provider: Provider) -> Vec<SignupStep> {
    match provider {
        Provider::GitHub => github_signup_steps(),
        Provider::Cloudflare | Provider::CloudflareR2 => cloudflare_signup_steps(),
        _ => Vec::new(),
    }
}

fn github_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to GitHub signup".into(),
            action: SignupAction::Navigate {
                url: "https://github.com/signup".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "#email".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Continue past email step".into(),
            action: SignupAction::Click {
                selector: "[data-continue-to=password-container]".into(),
            },
            check_captcha: true,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "#password".into(),
                value: "{{PASSWORD}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Continue past password step".into(),
            action: SignupAction::Click {
                selector: "[data-continue-to=username-container]".into(),
            },
            check_captcha: true,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter username".into(),
            action: SignupAction::Fill {
                selector: "#login".into(),
                value: "{{USERNAME}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Submit signup form".into(),
            action: SignupAction::Click {
                selector: "[data-continue-to=opt-in-container]".into(),
            },
            check_captcha: true,
            timeout_secs: 30,
        },
        SignupStep {
            description: "CAPTCHA pause — manual intervention may be required".into(),
            action: SignupAction::PauseForCaptcha {
                message: "GitHub may require CAPTCHA verification. Complete it in the browser, then press Enter to continue.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for dashboard".into(),
            action: SignupAction::WaitForUrl {
                pattern: "github.com".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn cloudflare_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Cloudflare signup".into(),
            action: SignupAction::Navigate {
                url: "https://dash.cloudflare.com/sign-up".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter email".into(),
            action: SignupAction::Fill {
                selector: "#email".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "#password".into(),
                value: "{{PASSWORD}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Submit signup".into(),
            action: SignupAction::Click {
                selector: "button[type=submit]".into(),
            },
            check_captcha: true,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Turnstile CAPTCHA pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Cloudflare Turnstile challenge detected. Complete it in the browser, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for dashboard".into(),
            action: SignupAction::WaitForUrl {
                pattern: "dash.cloudflare.com".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

// ── Account Health Checks (Self-Healing Integration) ───────────────────────

/// Account health status for self-healing integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountHealth {
    /// Credential is valid and working
    Healthy,
    /// Credential works but needs rotation soon
    RotationDue,
    /// Credential is expired or revoked
    Expired,
    /// Auth check failed (network error, etc.)
    CheckFailed,
    /// Provider account was suspended or deleted
    Suspended,
    /// No credential present
    Missing,
}

impl fmt::Display for AccountHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::RotationDue => write!(f, "rotation_due"),
            Self::Expired => write!(f, "expired"),
            Self::CheckFailed => write!(f, "check_failed"),
            Self::Suspended => write!(f, "suspended"),
            Self::Missing => write!(f, "missing"),
        }
    }
}

/// Health check result for a single account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountHealthCheck {
    pub provider: Provider,
    pub health: AccountHealth,
    pub message: String,
    /// Self-healing recommendation
    pub healing_action: AccountHealingAction,
}

/// What the self-healing engine should do for an unhealthy account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountHealingAction {
    /// No action needed
    None,
    /// Rotate the credential (Layer 1: retry with new cred)
    RotateCredential,
    /// Re-authenticate via CLI/OAuth (Layer 2: alternative approach)
    Reauthenticate,
    /// Try a fallback provider (Layer 2: alternative)
    FallbackProvider,
    /// Pause and alert the owner (Layer 5)
    PauseAndAlert,
}

impl fmt::Display for AccountHealingAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::RotateCredential => write!(f, "rotate_credential"),
            Self::Reauthenticate => write!(f, "reauthenticate"),
            Self::FallbackProvider => write!(f, "fallback_provider"),
            Self::PauseAndAlert => write!(f, "pause_and_alert"),
        }
    }
}

// ── Account Manager (Extended) ─────────────────────────────────────────────

/// Account creation manager.
///
/// Checks and tracks authentication status across all providers.
/// Manages credential lifecycle: creation, rotation, health checks, deletion.
pub struct AccountManager {
    statuses: Vec<AccountStatus>,
    credentials: HashMap<Provider, CredentialRecord>,
    health_results: HashMap<Provider, AccountHealthCheck>,
}

impl Default for AccountManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountManager {
    pub fn new() -> Self {
        let statuses = ALL_PROVIDERS
            .iter()
            .map(|p| AccountStatus::unchecked(*p))
            .collect();
        Self {
            statuses,
            credentials: HashMap::new(),
            health_results: HashMap::new(),
        }
    }

    // ── Auth Status ────────────────────────────────────────────────────

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
                    let output = std::process::Command::new("sh").arg("-c").arg(cmd).output();

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

    // ── OAuth Flow ─────────────────────────────────────────────────────

    /// Start an OAuth signup flow for a provider.
    ///
    /// Returns the authorization URL the user should visit (or that Playwright
    /// should navigate to). The actual token exchange happens in `complete_oauth`.
    #[instrument(skip(self))]
    pub fn start_oauth_flow(&self, provider: Provider) -> Result<OAuthFlowState, AccountError> {
        let config = oauth_flow_config(provider)
            .ok_or(AccountError::OAuthNotSupported(provider.display_name().into()))?;

        let state_token = format!("phantom-{}-{}", provider.display_name().to_lowercase().replace(' ', "-"), current_epoch_secs());

        let authorize_url = format!(
            "{}?response_type=code&client_id={{{{CLIENT_ID}}}}&redirect_uri={}&scope={}&state={}",
            config.authorize_url,
            config.redirect_uri,
            config.scopes.join("+"),
            state_token
        );

        info!(
            provider = provider.display_name(),
            "starting OAuth flow"
        );

        Ok(OAuthFlowState {
            provider,
            authorize_url,
            token_url: config.token_url,
            redirect_uri: config.redirect_uri,
            state_token,
            completed: false,
        })
    }

    /// Complete an OAuth flow after receiving the authorization code.
    #[instrument(skip(self, _code))]
    pub fn complete_oauth(
        &mut self,
        flow: &OAuthFlowState,
        _code: &str,
    ) -> Result<CredentialRecord, AccountError> {
        if flow.completed {
            return Err(AccountError::FlowAlreadyCompleted);
        }

        // In production, this would exchange the code for a token via HTTP POST.
        // For now, we create a credential record tracking the exchange.
        info!(
            provider = flow.provider.display_name(),
            "completing OAuth token exchange"
        );

        let record = CredentialRecord::new(flow.provider, CredentialType::OAuthToken);
        self.credentials.insert(flow.provider, record.clone());

        // Update auth status
        self.set_authenticated(flow.provider, Some("oauth"));

        Ok(record)
    }

    // ── Credential Lifecycle ───────────────────────────────────────────

    /// Register a credential for a provider.
    pub fn register_credential(
        &mut self,
        provider: Provider,
        credential_type: CredentialType,
    ) -> &CredentialRecord {
        let record = CredentialRecord::new(provider, credential_type);
        self.credentials.insert(provider, record);
        self.credentials.get(&provider).unwrap()
    }

    /// Get a credential record for a provider.
    pub fn get_credential(&self, provider: Provider) -> Option<&CredentialRecord> {
        self.credentials.get(&provider)
    }

    /// Get all credentials that need rotation.
    pub fn credentials_needing_rotation(&self) -> Vec<Provider> {
        self.credentials
            .iter()
            .filter(|(_, r)| r.needs_rotation())
            .map(|(p, _)| *p)
            .collect()
    }

    /// Rotate a credential for a provider.
    ///
    /// Returns the CLI command to execute for rotation, or None if the provider
    /// doesn't support CLI-based rotation.
    #[instrument(skip(self))]
    pub fn rotate_credential(&mut self, provider: Provider) -> Result<RotationResult, AccountError> {
        let record = self.credentials.get_mut(&provider)
            .ok_or(AccountError::NoCredential(provider.display_name().into()))?;

        if record.revoked {
            return Err(AccountError::CredentialRevoked(provider.display_name().into()));
        }

        let command = rotation_command(provider);

        record.mark_rotated();

        info!(
            provider = provider.display_name(),
            credential_type = %record.credential_type,
            "credential rotated"
        );

        Ok(RotationResult {
            provider,
            command,
            rotated_at: record.last_rotated_at,
            next_rotation_at: record.last_rotated_at + record.rotation_interval_secs,
        })
    }

    /// Delete all credentials (zero-footprint compliance for `master destroy`).
    #[instrument(skip(self))]
    pub fn destroy_all_credentials(&mut self) -> Vec<DeletionRecord> {
        let mut deletions = Vec::new();

        for (provider, mut record) in self.credentials.drain() {
            record.revoke();

            let logout_cmd = logout_command(provider);

            deletions.push(DeletionRecord {
                provider,
                credential_type: record.credential_type,
                keychain_service: record.keychain_service.clone(),
                logout_command: logout_cmd.map(String::from),
                deleted_at: current_epoch_secs(),
            });

            info!(
                provider = provider.display_name(),
                "credential destroyed"
            );
        }

        // Reset all auth statuses
        for status in &mut self.statuses {
            status.authenticated = false;
            status.username = None;
            status.message = Some("credentials destroyed".into());
        }

        warn!(count = deletions.len(), "all credentials destroyed (zero-footprint)");

        deletions
    }

    // ── Health Checks ──────────────────────────────────────────────────

    /// Run health check for a single provider's account.
    #[instrument(skip(self))]
    pub fn health_check(&mut self, provider: Provider) -> &AccountHealthCheck {
        let auth_status = self
            .statuses
            .iter()
            .find(|s| s.provider == provider);

        let is_authed = auth_status.map(|s| s.authenticated).unwrap_or(false);
        let credential = self.credentials.get(&provider);

        let (health, message, healing_action) = if !is_authed && credential.is_none() {
            (
                AccountHealth::Missing,
                format!("{}: no credential present", provider.display_name()),
                AccountHealingAction::Reauthenticate,
            )
        } else if let Some(cred) = credential {
            if cred.revoked {
                (
                    AccountHealth::Expired,
                    format!("{}: credential revoked", provider.display_name()),
                    AccountHealingAction::Reauthenticate,
                )
            } else if cred.needs_rotation() {
                (
                    AccountHealth::RotationDue,
                    format!(
                        "{}: rotation overdue by {}s",
                        provider.display_name(),
                        current_epoch_secs().saturating_sub(
                            cred.last_rotated_at + cred.rotation_interval_secs
                        )
                    ),
                    AccountHealingAction::RotateCredential,
                )
            } else if !is_authed {
                (
                    AccountHealth::CheckFailed,
                    format!("{}: credential exists but auth check failed", provider.display_name()),
                    AccountHealingAction::Reauthenticate,
                )
            } else {
                (
                    AccountHealth::Healthy,
                    format!("{}: authenticated and valid", provider.display_name()),
                    AccountHealingAction::None,
                )
            }
        } else if is_authed {
            (
                AccountHealth::Healthy,
                format!("{}: authenticated (no rotation tracking)", provider.display_name()),
                AccountHealingAction::None,
            )
        } else {
            (
                AccountHealth::Missing,
                format!("{}: not authenticated", provider.display_name()),
                AccountHealingAction::Reauthenticate,
            )
        };

        debug!(
            provider = provider.display_name(),
            health = %health,
            action = %healing_action,
            "account health check"
        );

        self.health_results.insert(
            provider,
            AccountHealthCheck {
                provider,
                health,
                message,
                healing_action,
            },
        );

        self.health_results.get(&provider).unwrap()
    }

    /// Run health checks for all providers.
    pub fn health_check_all(&mut self) -> Vec<AccountHealthCheck> {
        let providers: Vec<Provider> = ALL_PROVIDERS.to_vec();
        for p in providers {
            self.health_check(p);
        }
        self.health_results.values().cloned().collect()
    }

    /// Get providers that need healing action.
    pub fn unhealthy_providers(&self) -> Vec<&AccountHealthCheck> {
        self.health_results
            .values()
            .filter(|h| h.healing_action != AccountHealingAction::None)
            .collect()
    }

    /// Map an account health issue to the self-healing layer it should trigger.
    pub fn healing_layer_for(action: AccountHealingAction) -> &'static str {
        match action {
            AccountHealingAction::None => "none",
            AccountHealingAction::RotateCredential => "retry",
            AccountHealingAction::Reauthenticate => "alternative",
            AccountHealingAction::FallbackProvider => "alternative",
            AccountHealingAction::PauseAndAlert => "pause_and_alert",
        }
    }

    // ── Signup Flow Execution ──────────────────────────────────────────

    /// Get the signup steps for a provider (for Playwright execution).
    pub fn signup_flow(&self, provider: Provider) -> Vec<SignupStep> {
        signup_steps(provider)
    }

    /// Check if a provider has a browser-based signup flow.
    pub fn has_signup_flow(&self, provider: Provider) -> bool {
        !signup_steps(provider).is_empty()
    }

    // ── Helpers ────────────────────────────────────────────────────────

    fn set_authenticated(&mut self, provider: Provider, username: Option<&str>) {
        if let Some(status) = self.statuses.iter_mut().find(|s| s.provider == provider) {
            status.authenticated = true;
            status.username = username.map(String::from);
            status.message = Some("authenticated".into());
        }
    }

    /// Get all credential records.
    pub fn all_credentials(&self) -> &HashMap<Provider, CredentialRecord> {
        &self.credentials
    }

    /// Total number of registered credentials.
    pub fn credential_count(&self) -> usize {
        self.credentials.len()
    }
}

// ── OAuth Flow State ───────────────────────────────────────────────────────

/// In-progress OAuth flow state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlowState {
    pub provider: Provider,
    pub authorize_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub state_token: String,
    pub completed: bool,
}

// ── Rotation & Deletion Results ────────────────────────────────────────────

/// Result of a credential rotation operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationResult {
    pub provider: Provider,
    /// CLI command that was/should be run (None if API-only rotation)
    pub command: Option<&'static str>,
    pub rotated_at: u64,
    pub next_rotation_at: u64,
}

/// Record of a deleted credential (for audit trail).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionRecord {
    pub provider: Provider,
    pub credential_type: CredentialType,
    pub keychain_service: Option<String>,
    pub logout_command: Option<String>,
    pub deleted_at: u64,
}

// ── Provider-specific Commands ─────────────────────────────────────────────

/// The CLI command to rotate credentials for a provider.
fn rotation_command(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::GitHub => Some("gh auth refresh"),
        Provider::FlyIo => Some("flyctl auth login"),
        Provider::Vercel => Some("vercel login"),
        Provider::Railway => Some("railway login"),
        Provider::Supabase => Some("supabase login"),
        Provider::GoogleCloud => Some("gcloud auth login"),
        Provider::AwsFreeTier => Some("aws configure"),
        _ => None,
    }
}

/// The CLI command to log out from a provider.
fn logout_command(provider: Provider) -> Option<&'static str> {
    match provider {
        Provider::GitHub => Some("gh auth logout"),
        Provider::FlyIo => Some("flyctl auth logout"),
        Provider::Vercel => Some("vercel logout"),
        Provider::Railway => Some("railway logout"),
        Provider::Supabase => Some("supabase logout"),
        Provider::GoogleCloud => Some("gcloud auth revoke"),
        Provider::AwsFreeTier => Some("rm -f ~/.aws/credentials"),
        _ => None,
    }
}

// ── Errors ─────────────────────────────────────────────────────────────────

/// Errors from account operations.
#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("OAuth not supported for {0}")]
    OAuthNotSupported(String),

    #[error("OAuth flow already completed")]
    FlowAlreadyCompleted,

    #[error("no credential registered for {0}")]
    NoCredential(String),

    #[error("credential revoked for {0}")]
    CredentialRevoked(String),

    #[error("CAPTCHA detected for {0}: manual intervention required")]
    CaptchaDetected(String),

    #[error("signup flow failed at step {step}: {reason}")]
    SignupFailed { step: usize, reason: String },
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn current_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

// ── Tests ──────────────────────────────────────────────────────────────────

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
        assert_eq!(
            env_var_for(Provider::Cloudflare),
            Some("CLOUDFLARE_API_TOKEN")
        );
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

    // ── OAuth flow tests ───────────────────────────────────────────────

    #[test]
    fn test_oauth_flow_config_github() {
        let config = oauth_flow_config(Provider::GitHub).unwrap();
        assert_eq!(config.provider, Provider::GitHub);
        assert!(config.authorize_url.contains("github.com"));
        assert!(config.scopes.contains(&"repo".to_string()));
        assert!(config.browser_flow);
    }

    #[test]
    fn test_oauth_flow_config_cloudflare() {
        let config = oauth_flow_config(Provider::Cloudflare).unwrap();
        assert!(config.authorize_url.contains("cloudflare.com"));
        assert!(config.scopes.contains(&"workers:write".to_string()));
    }

    #[test]
    fn test_oauth_flow_config_unsupported() {
        assert!(oauth_flow_config(Provider::FlyIo).is_none());
        assert!(oauth_flow_config(Provider::Render).is_none());
    }

    #[test]
    fn test_start_oauth_flow() {
        let mgr = AccountManager::new();
        let flow = mgr.start_oauth_flow(Provider::GitHub).unwrap();
        assert_eq!(flow.provider, Provider::GitHub);
        assert!(flow.authorize_url.contains("github.com"));
        assert!(!flow.completed);
    }

    #[test]
    fn test_start_oauth_unsupported() {
        let mgr = AccountManager::new();
        assert!(mgr.start_oauth_flow(Provider::FlyIo).is_err());
    }

    #[test]
    fn test_complete_oauth() {
        let mut mgr = AccountManager::new();
        let flow = mgr.start_oauth_flow(Provider::GitHub).unwrap();
        let record = mgr.complete_oauth(&flow, "test-auth-code").unwrap();

        assert_eq!(record.provider, Provider::GitHub);
        assert_eq!(record.credential_type, CredentialType::OAuthToken);
        assert!(!record.revoked);
        assert!(mgr.get_status(Provider::GitHub).unwrap().authenticated);
    }

    // ── Credential lifecycle tests ─────────────────────────────────────

    #[test]
    fn test_credential_record_creation() {
        let record = CredentialRecord::new(Provider::GitHub, CredentialType::OAuthToken);
        assert_eq!(record.provider, Provider::GitHub);
        assert!(!record.revoked);
        assert!(!record.needs_rotation()); // Just created, shouldn't need rotation yet
        assert!(record.seconds_until_rotation() > 0);
    }

    #[test]
    fn test_credential_rotation_check() {
        let mut record = CredentialRecord::new(Provider::GitHub, CredentialType::OAuthToken);
        // Set last_rotated_at to 31 days ago
        record.last_rotated_at = current_epoch_secs() - (31 * 86400);
        assert!(record.needs_rotation());
        assert_eq!(record.seconds_until_rotation(), 0);
    }

    #[test]
    fn test_credential_mark_rotated() {
        let mut record = CredentialRecord::new(Provider::GitHub, CredentialType::OAuthToken);
        record.last_rotated_at = current_epoch_secs() - (31 * 86400);
        assert!(record.needs_rotation());

        record.mark_rotated();
        assert!(!record.needs_rotation());
    }

    #[test]
    fn test_credential_revoke() {
        let mut record = CredentialRecord::new(Provider::GitHub, CredentialType::ApiToken);
        record.revoke();
        assert!(record.revoked);
        assert!(!record.needs_rotation()); // Revoked creds don't need rotation
    }

    #[test]
    fn test_register_and_get_credential() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::CliSession);

        let cred = mgr.get_credential(Provider::GitHub);
        assert!(cred.is_some());
        assert_eq!(cred.unwrap().credential_type, CredentialType::CliSession);
    }

    #[test]
    fn test_credentials_needing_rotation() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::OAuthToken);
        mgr.register_credential(Provider::Cloudflare, CredentialType::ApiToken);

        // Make GitHub credential stale
        mgr.credentials.get_mut(&Provider::GitHub).unwrap().last_rotated_at =
            current_epoch_secs() - (31 * 86400);

        let needing = mgr.credentials_needing_rotation();
        assert_eq!(needing.len(), 1);
        assert_eq!(needing[0], Provider::GitHub);
    }

    #[test]
    fn test_rotate_credential() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::CliSession);
        mgr.credentials.get_mut(&Provider::GitHub).unwrap().last_rotated_at =
            current_epoch_secs() - (31 * 86400);

        let result = mgr.rotate_credential(Provider::GitHub).unwrap();
        assert_eq!(result.provider, Provider::GitHub);
        assert_eq!(result.command, Some("gh auth refresh"));
        assert!(!mgr.get_credential(Provider::GitHub).unwrap().needs_rotation());
    }

    #[test]
    fn test_rotate_nonexistent_credential() {
        let mut mgr = AccountManager::new();
        assert!(mgr.rotate_credential(Provider::GitHub).is_err());
    }

    #[test]
    fn test_rotate_revoked_credential() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::OAuthToken);
        mgr.credentials.get_mut(&Provider::GitHub).unwrap().revoke();

        assert!(mgr.rotate_credential(Provider::GitHub).is_err());
    }

    #[test]
    fn test_destroy_all_credentials() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::CliSession);
        mgr.register_credential(Provider::Cloudflare, CredentialType::ApiToken);

        let deletions = mgr.destroy_all_credentials();
        assert_eq!(deletions.len(), 2);
        assert_eq!(mgr.credential_count(), 0);

        // All should show as not authenticated
        for status in &mgr.statuses {
            assert!(!status.authenticated);
        }
    }

    #[test]
    fn test_deletion_record_has_logout_command() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::CliSession);

        let deletions = mgr.destroy_all_credentials();
        let github_del = deletions.iter().find(|d| d.provider == Provider::GitHub).unwrap();
        assert_eq!(github_del.logout_command.as_deref(), Some("gh auth logout"));
        assert!(github_del.keychain_service.is_some());
    }

    // ── CAPTCHA detection tests ────────────────────────────────────────

    #[test]
    fn test_captcha_detection_found() {
        let content = "<html><div class='g-recaptcha' data-sitekey='xxx'></div></html>";
        let result = detect_captcha(content, Provider::GitHub);
        assert!(result.detected);
        // "captcha" matches first in the indicator list (substring of "recaptcha")
        assert!(result.indicator.as_deref().unwrap().contains("captcha"));
        assert_eq!(result.action, CaptchaAction::PauseForManual);
    }

    #[test]
    fn test_captcha_detection_turnstile() {
        let content = "Checking if the site connection is secure. Turnstile challenge.";
        let result = detect_captcha(content, Provider::Cloudflare);
        assert!(result.detected);
        assert_eq!(result.indicator.as_deref(), Some("turnstile"));
    }

    #[test]
    fn test_captcha_detection_none() {
        let content = "<html><body>Welcome to GitHub! Please sign in.</body></html>";
        let result = detect_captcha(content, Provider::GitHub);
        assert!(!result.detected);
        assert!(result.indicator.is_none());
    }

    #[test]
    fn test_captcha_detection_case_insensitive() {
        let content = "Please complete the CAPTCHA below to continue.";
        let result = detect_captcha(content, Provider::GitHub);
        assert!(result.detected);
    }

    #[test]
    fn test_captcha_action_display() {
        assert_eq!(CaptchaAction::PauseForManual.to_string(), "pause_for_manual");
        assert_eq!(CaptchaAction::RetryAfterDelay.to_string(), "retry_after_delay");
        assert_eq!(CaptchaAction::SkipProvider.to_string(), "skip_provider");
    }

    // ── Signup flow tests ──────────────────────────────────────────────

    #[test]
    fn test_github_signup_steps() {
        let steps = signup_steps(Provider::GitHub);
        assert!(!steps.is_empty());
        // First step should navigate to signup
        assert!(matches!(&steps[0].action, SignupAction::Navigate { url } if url.contains("github.com/signup")));
        // Should contain a CAPTCHA pause step
        assert!(steps.iter().any(|s| matches!(&s.action, SignupAction::PauseForCaptcha { .. })));
    }

    #[test]
    fn test_cloudflare_signup_steps() {
        let steps = signup_steps(Provider::Cloudflare);
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| matches!(&s.action, SignupAction::Navigate { url } if url.contains("cloudflare.com"))));
    }

    #[test]
    fn test_unsupported_signup_flow() {
        assert!(signup_steps(Provider::Render).is_empty());
        assert!(signup_steps(Provider::Neon).is_empty());
    }

    #[test]
    fn test_has_signup_flow() {
        let mgr = AccountManager::new();
        assert!(mgr.has_signup_flow(Provider::GitHub));
        assert!(mgr.has_signup_flow(Provider::Cloudflare));
        assert!(!mgr.has_signup_flow(Provider::Render));
    }

    #[test]
    fn test_signup_step_serde() {
        let step = SignupStep {
            description: "Navigate".into(),
            action: SignupAction::Navigate { url: "https://example.com".into() },
            check_captcha: true,
            timeout_secs: 30,
        };
        let json = serde_json::to_string(&step).unwrap();
        let decoded: SignupStep = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.description, "Navigate");
        assert!(decoded.check_captcha);
    }

    // ── Health check tests ─────────────────────────────────────────────

    #[test]
    fn test_health_check_missing() {
        let mut mgr = AccountManager::new();
        let check = mgr.health_check(Provider::GitHub);
        assert_eq!(check.health, AccountHealth::Missing);
        assert_eq!(check.healing_action, AccountHealingAction::Reauthenticate);
    }

    #[test]
    fn test_health_check_healthy() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::CliSession);
        // Manually set as authenticated
        mgr.set_authenticated(Provider::GitHub, Some("user"));

        let check = mgr.health_check(Provider::GitHub);
        assert_eq!(check.health, AccountHealth::Healthy);
        assert_eq!(check.healing_action, AccountHealingAction::None);
    }

    #[test]
    fn test_health_check_rotation_due() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::CliSession);
        mgr.set_authenticated(Provider::GitHub, Some("user"));
        mgr.credentials.get_mut(&Provider::GitHub).unwrap().last_rotated_at =
            current_epoch_secs() - (31 * 86400);

        let check = mgr.health_check(Provider::GitHub);
        assert_eq!(check.health, AccountHealth::RotationDue);
        assert_eq!(check.healing_action, AccountHealingAction::RotateCredential);
    }

    #[test]
    fn test_health_check_expired() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::OAuthToken);
        mgr.credentials.get_mut(&Provider::GitHub).unwrap().revoke();

        let check = mgr.health_check(Provider::GitHub);
        assert_eq!(check.health, AccountHealth::Expired);
        assert_eq!(check.healing_action, AccountHealingAction::Reauthenticate);
    }

    #[test]
    fn test_health_check_all() {
        let mut mgr = AccountManager::new();
        let results = mgr.health_check_all();
        assert_eq!(results.len(), ALL_PROVIDERS.len());
    }

    #[test]
    fn test_unhealthy_providers() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::CliSession);
        mgr.set_authenticated(Provider::GitHub, Some("user"));
        mgr.health_check(Provider::GitHub);
        mgr.health_check(Provider::Cloudflare);

        let unhealthy = mgr.unhealthy_providers();
        // GitHub is healthy, Cloudflare is missing credential
        assert!(unhealthy.iter().any(|h| h.provider == Provider::Cloudflare));
        assert!(!unhealthy.iter().any(|h| h.provider == Provider::GitHub));
    }

    #[test]
    fn test_healing_layer_mapping() {
        assert_eq!(AccountManager::healing_layer_for(AccountHealingAction::None), "none");
        assert_eq!(AccountManager::healing_layer_for(AccountHealingAction::RotateCredential), "retry");
        assert_eq!(AccountManager::healing_layer_for(AccountHealingAction::Reauthenticate), "alternative");
        assert_eq!(AccountManager::healing_layer_for(AccountHealingAction::PauseAndAlert), "pause_and_alert");
    }

    // ── Misc ───────────────────────────────────────────────────────────

    #[test]
    fn test_credential_type_display() {
        assert_eq!(CredentialType::OAuthToken.to_string(), "oauth_token");
        assert_eq!(CredentialType::ApiToken.to_string(), "api_token");
        assert_eq!(CredentialType::SshKey.to_string(), "ssh_key");
    }

    #[test]
    fn test_account_health_display() {
        assert_eq!(AccountHealth::Healthy.to_string(), "healthy");
        assert_eq!(AccountHealth::RotationDue.to_string(), "rotation_due");
        assert_eq!(AccountHealth::Expired.to_string(), "expired");
    }

    #[test]
    fn test_rotation_commands() {
        assert_eq!(rotation_command(Provider::GitHub), Some("gh auth refresh"));
        assert_eq!(rotation_command(Provider::GoogleCloud), Some("gcloud auth login"));
        assert!(rotation_command(Provider::Neon).is_none());
    }

    #[test]
    fn test_logout_commands() {
        assert_eq!(logout_command(Provider::GitHub), Some("gh auth logout"));
        assert_eq!(logout_command(Provider::Vercel), Some("vercel logout"));
        assert!(logout_command(Provider::Neon).is_none());
    }

    #[test]
    fn test_credential_keychain_service() {
        let record = CredentialRecord::new(Provider::GitHub, CredentialType::OAuthToken);
        assert_eq!(record.keychain_service.as_deref(), Some("phantom-github"));

        let record2 = CredentialRecord::new(Provider::OracleCloud, CredentialType::ServiceAccount);
        assert_eq!(record2.keychain_service.as_deref(), Some("phantom-oracle-cloud"));
    }

    #[test]
    fn test_credential_age() {
        let record = CredentialRecord::new(Provider::GitHub, CredentialType::OAuthToken);
        // Just created, age should be ~0
        assert!(record.age_secs() < 2);
    }

    #[test]
    fn test_oauth_flow_state_serde() {
        let state = OAuthFlowState {
            provider: Provider::GitHub,
            authorize_url: "https://github.com/login/oauth/authorize".into(),
            token_url: "https://github.com/login/oauth/access_token".into(),
            redirect_uri: "http://127.0.0.1:19876/callback".into(),
            state_token: "phantom-github-12345".into(),
            completed: false,
        };
        let json = serde_json::to_string(&state).unwrap();
        let decoded: OAuthFlowState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.provider, Provider::GitHub);
        assert!(!decoded.completed);
    }

    #[test]
    fn test_default_rotation_days() {
        assert_eq!(DEFAULT_ROTATION_DAYS, 30);
        let record = CredentialRecord::new(Provider::GitHub, CredentialType::ApiToken);
        assert_eq!(record.rotation_interval_secs, 30 * 86400);
    }
}
