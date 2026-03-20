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

// ── Signup Executor Trait ─────────────────────────────────────────────────

/// Trait abstracting browser automation for signup flow execution.
///
/// Implemented by `BrowserAutomation` in phantom-core to avoid circular deps.
/// Each method corresponds to a `SignupAction` variant. Implementations drive
/// the actual browser (osascript JXA, Playwright, etc.).
pub trait SignupExecutor {
    /// Navigate the browser to a URL.
    fn navigate(&mut self, url: &str) -> Result<(), AccountError>;

    /// Fill a form field identified by CSS selector with a value.
    fn fill(&mut self, selector: &str, value: &str) -> Result<(), AccountError>;

    /// Click an element identified by CSS selector.
    fn click(&mut self, selector: &str) -> Result<(), AccountError>;

    /// Wait for an element matching CSS selector to appear.
    fn wait_for(&mut self, selector: &str, timeout: Duration) -> Result<(), AccountError>;

    /// Wait for navigation to a URL matching the pattern.
    fn wait_for_url(&mut self, pattern: &str, timeout: Duration) -> Result<(), AccountError>;

    /// Extract text content from an element (e.g. to capture a generated token).
    fn extract_token(&mut self, selector: &str) -> Result<String, AccountError>;

    /// Take a screenshot and save to the given filename.
    fn screenshot(&mut self, filename: &str) -> Result<(), AccountError>;

    /// Get the current page HTML content (for CAPTCHA scanning).
    fn page_content(&mut self) -> Result<String, AccountError>;

    /// Notify the owner that manual CAPTCHA intervention is needed.
    /// The implementation should emit a message via the message bus / TUI prompt
    /// and block until the user confirms completion.
    fn notify_captcha_pause(&mut self, message: &str) -> Result<(), AccountError>;
}

/// Result of executing a signup flow to completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupFlowResult {
    pub provider: Provider,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub extracted_tokens: Vec<String>,
    pub captcha_pauses: usize,
    pub success: bool,
    pub error: Option<String>,
}

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
        Provider::Hetzner => AuthMethod::ApiToken,
        Provider::Vultr => AuthMethod::ApiToken,
        Provider::DigitalOcean => AuthMethod::CliLogin,
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
        Provider::Hetzner => Some("HCLOUD_TOKEN"),
        Provider::Vultr => Some("VULTR_API_KEY"),
        Provider::DigitalOcean => Some("DIGITALOCEAN_TOKEN"),
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
            keychain_service: Some(format!(
                "phantom-{}",
                provider.display_name().to_lowercase().replace(' ', "-")
            )),
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

/// Build the browser-based signup step sequence for a provider.
pub fn signup_steps(provider: Provider) -> Vec<SignupStep> {
    match provider {
        Provider::GitHub => github_signup_steps(),
        Provider::Cloudflare | Provider::CloudflareR2 => cloudflare_signup_steps(),
        Provider::Vercel => vercel_signup_steps(),
        Provider::Supabase => supabase_signup_steps(),
        Provider::Upstash => upstash_signup_steps(),
        Provider::Neon => neon_signup_steps(),
        Provider::FlyIo => flyio_signup_steps(),
        Provider::Railway => railway_signup_steps(),
        Provider::Render => render_signup_steps(),
        Provider::Netlify => netlify_signup_steps(),
        Provider::DigitalOcean => digitalocean_signup_steps(),
        Provider::Hetzner => hetzner_signup_steps(),
        Provider::Vultr => vultr_signup_steps(),
        Provider::OracleCloud => oracle_cloud_signup_steps(),
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

fn vercel_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Vercel signup".into(),
            action: SignupAction::Navigate {
                url: "https://vercel.com/signup".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Select 'Continue with Email' option".into(),
            action: SignupAction::Click {
                selector: "a[href='/signup?type=email']".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Submit email".into(),
            action: SignupAction::Click {
                selector: "button[type=submit]".into(),
            },
            check_captcha: true,
            timeout_secs: 30,
        },
        SignupStep {
            description: "CAPTCHA pause — manual intervention may be required".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Vercel may require email verification or CAPTCHA. Complete it in the browser, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for dashboard".into(),
            action: SignupAction::WaitForUrl {
                pattern: "vercel.com/new".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn supabase_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Supabase signup".into(),
            action: SignupAction::Navigate {
                url: "https://supabase.com/dashboard/sign-up".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "input[name=password]".into(),
                value: "{{PASSWORD}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Submit signup form".into(),
            action: SignupAction::Click {
                selector: "button[type=submit]".into(),
            },
            check_captcha: true,
            timeout_secs: 30,
        },
        SignupStep {
            description: "CAPTCHA pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Supabase may require hCaptcha verification. Complete it in the browser, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for dashboard".into(),
            action: SignupAction::WaitForUrl {
                pattern: "supabase.com/dashboard".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn upstash_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Upstash signup".into(),
            action: SignupAction::Navigate {
                url: "https://console.upstash.com/signup".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "input[name=password]".into(),
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
            description: "CAPTCHA pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Upstash may require verification. Complete it in the browser, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for console".into(),
            action: SignupAction::WaitForUrl {
                pattern: "console.upstash.com".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn neon_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Neon signup".into(),
            action: SignupAction::Navigate {
                url: "https://console.neon.tech/signup".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "input[name=password]".into(),
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
            description: "CAPTCHA pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Neon may require email verification. Complete it in the browser, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for console".into(),
            action: SignupAction::WaitForUrl {
                pattern: "console.neon.tech".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn flyio_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Fly.io signup".into(),
            action: SignupAction::Navigate {
                url: "https://fly.io/app/sign-up".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter name".into(),
            action: SignupAction::Fill {
                selector: "input[name=name]".into(),
                value: "{{USERNAME}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "input[name=password]".into(),
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
            description: "CAPTCHA pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Fly.io may require CAPTCHA or email verification. Complete it in the browser, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for dashboard".into(),
            action: SignupAction::WaitForUrl {
                pattern: "fly.io/dashboard".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn railway_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Railway signup".into(),
            action: SignupAction::Navigate {
                url: "https://railway.app/register".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Select email signup".into(),
            action: SignupAction::Click {
                selector: "button[data-testid='email-signup']".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Submit email".into(),
            action: SignupAction::Click {
                selector: "button[type=submit]".into(),
            },
            check_captcha: true,
            timeout_secs: 30,
        },
        SignupStep {
            description: "CAPTCHA pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Railway requires email verification. Check your email, click the link, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for dashboard".into(),
            action: SignupAction::WaitForUrl {
                pattern: "railway.app/dashboard".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn render_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Render signup".into(),
            action: SignupAction::Navigate {
                url: "https://dashboard.render.com/register".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter name".into(),
            action: SignupAction::Fill {
                selector: "input[name=name]".into(),
                value: "{{USERNAME}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "input[name=password]".into(),
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
            description: "CAPTCHA pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Render may require email verification. Complete it, then press Enter."
                    .into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for dashboard".into(),
            action: SignupAction::WaitForUrl {
                pattern: "dashboard.render.com".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn netlify_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Netlify signup".into(),
            action: SignupAction::Navigate {
                url: "https://app.netlify.com/signup".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Click 'Sign up with email' link".into(),
            action: SignupAction::Click {
                selector: "a[href='/signup/email']".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "input[name=password]".into(),
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
            description: "CAPTCHA pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Netlify may require email verification. Check your email, click the link, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for dashboard".into(),
            action: SignupAction::WaitForUrl {
                pattern: "app.netlify.com".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn digitalocean_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to DigitalOcean signup".into(),
            action: SignupAction::Navigate {
                url: "https://cloud.digitalocean.com/registrations/new".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "input[name=password]".into(),
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
            description: "CAPTCHA pause — Turnstile challenge likely".into(),
            action: SignupAction::PauseForCaptcha {
                message: "DigitalOcean uses Turnstile CAPTCHA and email verification. Complete both, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for console".into(),
            action: SignupAction::WaitForUrl {
                pattern: "cloud.digitalocean.com".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn hetzner_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Hetzner signup".into(),
            action: SignupAction::Navigate {
                url: "https://accounts.hetzner.com/signUp".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Submit email".into(),
            action: SignupAction::Click {
                selector: "button[type=submit]".into(),
            },
            check_captcha: true,
            timeout_secs: 30,
        },
        SignupStep {
            description: "CAPTCHA pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Hetzner requires email verification and identity confirmation. Complete the process, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for console".into(),
            action: SignupAction::WaitForUrl {
                pattern: "console.hetzner.cloud".into(),
            },
            check_captcha: false,
            timeout_secs: 120,
        },
    ]
}

fn vultr_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Vultr signup".into(),
            action: SignupAction::Navigate {
                url: "https://www.vultr.com/register/".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter name".into(),
            action: SignupAction::Fill {
                selector: "input[name=name]".into(),
                value: "{{USERNAME}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "input[name=password]".into(),
                value: "{{PASSWORD}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Accept terms".into(),
            action: SignupAction::Click {
                selector: "input[name=agree]".into(),
            },
            check_captcha: false,
            timeout_secs: 5,
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
            description: "CAPTCHA pause — reCAPTCHA likely".into(),
            action: SignupAction::PauseForCaptcha {
                message:
                    "Vultr uses reCAPTCHA and email verification. Complete both, then press Enter."
                        .into(),
            },
            check_captcha: false,
            timeout_secs: 300,
        },
        SignupStep {
            description: "Wait for console".into(),
            action: SignupAction::WaitForUrl {
                pattern: "my.vultr.com".into(),
            },
            check_captcha: false,
            timeout_secs: 60,
        },
    ]
}

fn oracle_cloud_signup_steps() -> Vec<SignupStep> {
    vec![
        SignupStep {
            description: "Navigate to Oracle Cloud free tier signup".into(),
            action: SignupAction::Navigate {
                url: "https://signup.cloud.oracle.com/".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Enter country".into(),
            action: SignupAction::Fill {
                selector: "select[name=country]".into(),
                value: "{{COUNTRY}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter first name".into(),
            action: SignupAction::Fill {
                selector: "input[name=firstName]".into(),
                value: "{{FIRST_NAME}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter last name".into(),
            action: SignupAction::Fill {
                selector: "input[name=lastName]".into(),
                value: "{{LAST_NAME}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Enter email address".into(),
            action: SignupAction::Fill {
                selector: "input[name=email]".into(),
                value: "{{EMAIL}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Verify email".into(),
            action: SignupAction::Click {
                selector: "button[data-testid='verify-email']".into(),
            },
            check_captcha: true,
            timeout_secs: 30,
        },
        SignupStep {
            description: "CAPTCHA + email verification pause".into(),
            action: SignupAction::PauseForCaptcha {
                message: "Oracle Cloud requires email verification, CAPTCHA, and possibly phone verification. Complete all steps, then press Enter.".into(),
            },
            check_captcha: false,
            timeout_secs: 600,
        },
        SignupStep {
            description: "Enter password".into(),
            action: SignupAction::Fill {
                selector: "input[name=password]".into(),
                value: "{{PASSWORD}}".into(),
            },
            check_captcha: false,
            timeout_secs: 10,
        },
        SignupStep {
            description: "Submit registration".into(),
            action: SignupAction::Click {
                selector: "button[type=submit]".into(),
            },
            check_captcha: false,
            timeout_secs: 30,
        },
        SignupStep {
            description: "Wait for console".into(),
            action: SignupAction::WaitForUrl {
                pattern: "cloud.oracle.com".into(),
            },
            check_captcha: false,
            timeout_secs: 120,
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
        let config = oauth_flow_config(provider).ok_or(AccountError::OAuthNotSupported(
            provider.display_name().into(),
        ))?;

        let state_token = format!(
            "phantom-{}-{}",
            provider.display_name().to_lowercase().replace(' ', "-"),
            current_epoch_secs()
        );

        let authorize_url = format!(
            "{}?response_type=code&client_id={{{{CLIENT_ID}}}}&redirect_uri={}&scope={}&state={}",
            config.authorize_url,
            config.redirect_uri,
            config.scopes.join("+"),
            state_token
        );

        info!(provider = provider.display_name(), "starting OAuth flow");

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
    pub fn rotate_credential(
        &mut self,
        provider: Provider,
    ) -> Result<RotationResult, AccountError> {
        let record = self
            .credentials
            .get_mut(&provider)
            .ok_or(AccountError::NoCredential(provider.display_name().into()))?;

        if record.revoked {
            return Err(AccountError::CredentialRevoked(
                provider.display_name().into(),
            ));
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

            info!(provider = provider.display_name(), "credential destroyed");
        }

        // Reset all auth statuses
        for status in &mut self.statuses {
            status.authenticated = false;
            status.username = None;
            status.message = Some("credentials destroyed".into());
        }

        warn!(
            count = deletions.len(),
            "all credentials destroyed (zero-footprint)"
        );

        deletions
    }

    // ── Health Checks ──────────────────────────────────────────────────

    /// Run health check for a single provider's account.
    #[instrument(skip(self))]
    pub fn health_check(&mut self, provider: Provider) -> &AccountHealthCheck {
        let auth_status = self.statuses.iter().find(|s| s.provider == provider);

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
                        current_epoch_secs()
                            .saturating_sub(cred.last_rotated_at + cred.rotation_interval_secs)
                    ),
                    AccountHealingAction::RotateCredential,
                )
            } else if !is_authed {
                (
                    AccountHealth::CheckFailed,
                    format!(
                        "{}: credential exists but auth check failed",
                        provider.display_name()
                    ),
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
                format!(
                    "{}: authenticated (no rotation tracking)",
                    provider.display_name()
                ),
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

    /// Get the signup steps for a provider.
    pub fn signup_flow(&self, provider: Provider) -> Vec<SignupStep> {
        signup_steps(provider)
    }

    /// Check if a provider has a browser-based signup flow.
    pub fn has_signup_flow(&self, provider: Provider) -> bool {
        !signup_steps(provider).is_empty()
    }

    /// Execute a full signup flow for a provider via a `SignupExecutor`.
    ///
    /// Converts each `SignupStep`/`SignupAction` into calls on the executor
    /// (which wraps `BrowserAutomation` in phantom-core). Performs CAPTCHA
    /// scanning after steps that request it and emits TUI pause prompts via
    /// `executor.notify_captcha_pause()`.
    #[instrument(skip(self, executor, vars))]
    pub fn execute_signup_flow(
        &mut self,
        provider: Provider,
        executor: &mut dyn SignupExecutor,
        vars: &HashMap<String, String>,
    ) -> Result<SignupFlowResult, AccountError> {
        let steps = signup_steps(provider);
        if steps.is_empty() {
            return Err(AccountError::SignupFailed {
                step: 0,
                reason: format!("no signup flow defined for {}", provider.display_name()),
            });
        }

        let steps_total = steps.len();
        let mut steps_completed: usize = 0;
        let mut extracted_tokens: Vec<String> = Vec::new();
        let mut captcha_pauses: usize = 0;

        info!(
            provider = provider.display_name(),
            steps = steps_total,
            "starting signup flow execution"
        );

        for (i, step) in steps.iter().enumerate() {
            debug!(
                step = i,
                description = %step.description,
                "executing signup step"
            );

            let timeout = Duration::from_secs(step.timeout_secs);

            // Execute the action
            match &step.action {
                SignupAction::Navigate { url } => {
                    let resolved = resolve_template(url, vars);
                    executor
                        .navigate(&resolved)
                        .map_err(|e| AccountError::SignupFailed {
                            step: i,
                            reason: format!("navigate failed: {e}"),
                        })?;
                }
                SignupAction::Fill { selector, value } => {
                    let resolved_val = resolve_template(value, vars);
                    executor.fill(selector, &resolved_val).map_err(|e| {
                        AccountError::SignupFailed {
                            step: i,
                            reason: format!("fill '{selector}' failed: {e}"),
                        }
                    })?;
                }
                SignupAction::Click { selector } => {
                    executor
                        .click(selector)
                        .map_err(|e| AccountError::SignupFailed {
                            step: i,
                            reason: format!("click '{selector}' failed: {e}"),
                        })?;
                }
                SignupAction::WaitFor { selector } => {
                    executor.wait_for(selector, timeout).map_err(|e| {
                        AccountError::SignupFailed {
                            step: i,
                            reason: format!("wait_for '{selector}' timed out: {e}"),
                        }
                    })?;
                }
                SignupAction::WaitForUrl { pattern } => {
                    executor.wait_for_url(pattern, timeout).map_err(|e| {
                        AccountError::SignupFailed {
                            step: i,
                            reason: format!("wait_for_url '{pattern}' timed out: {e}"),
                        }
                    })?;
                }
                SignupAction::ExtractToken { selector } => {
                    let token = executor.extract_token(selector).map_err(|e| {
                        AccountError::SignupFailed {
                            step: i,
                            reason: format!("extract_token '{selector}' failed: {e}"),
                        }
                    })?;
                    extracted_tokens.push(token);
                }
                SignupAction::Screenshot { filename } => {
                    executor
                        .screenshot(filename)
                        .map_err(|e| AccountError::SignupFailed {
                            step: i,
                            reason: format!("screenshot failed: {e}"),
                        })?;
                }
                SignupAction::PauseForCaptcha { message } => {
                    captcha_pauses += 1;
                    info!(
                        provider = provider.display_name(),
                        pause = captcha_pauses,
                        "CAPTCHA pause — notifying owner via TUI"
                    );
                    executor.notify_captcha_pause(message).map_err(|e| {
                        AccountError::SignupFailed {
                            step: i,
                            reason: format!("captcha pause notification failed: {e}"),
                        }
                    })?;
                }
            }

            // CAPTCHA scanning: after steps that request it, fetch page content and scan
            if step.check_captcha {
                if let Ok(content) = executor.page_content() {
                    let detection = detect_captcha(&content, provider);
                    if detection.detected {
                        captcha_pauses += 1;
                        let msg = format!(
                            "CAPTCHA detected at step {} (indicator: {}). Complete it in the browser, then press Enter.",
                            i,
                            detection.indicator.as_deref().unwrap_or("unknown")
                        );
                        warn!(
                            provider = provider.display_name(),
                            indicator = detection.indicator.as_deref().unwrap_or("unknown"),
                            "auto-detected CAPTCHA — pausing for manual intervention"
                        );
                        executor.notify_captcha_pause(&msg).map_err(|e| {
                            AccountError::CaptchaDetected(format!(
                                "{}: notification failed: {e}",
                                provider.display_name()
                            ))
                        })?;
                    }
                }
            }

            steps_completed = i + 1;
        }

        // On success, register a credential record
        let cred_type = match auth_method_for(provider) {
            AuthMethod::CliLogin => CredentialType::CliSession,
            AuthMethod::ApiToken => CredentialType::ApiToken,
            AuthMethod::OAuth => CredentialType::OAuthToken,
            AuthMethod::None => CredentialType::ApiToken,
        };
        self.register_credential(provider, cred_type);
        self.set_authenticated(provider, None);

        info!(
            provider = provider.display_name(),
            steps_completed,
            captcha_pauses,
            tokens = extracted_tokens.len(),
            "signup flow completed successfully"
        );

        Ok(SignupFlowResult {
            provider,
            steps_completed,
            steps_total,
            extracted_tokens,
            captcha_pauses,
            success: true,
            error: None,
        })
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

/// Resolve `{{VAR}}` template placeholders in a string using the provided map.
///
/// Supports: `{{EMAIL}}`, `{{PASSWORD}}`, `{{USERNAME}}`, `{{FIRST_NAME}}`,
/// `{{LAST_NAME}}`, `{{COUNTRY}}`, and any other key in the map.
fn resolve_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
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
        mgr.credentials
            .get_mut(&Provider::GitHub)
            .unwrap()
            .last_rotated_at = current_epoch_secs() - (31 * 86400);

        let needing = mgr.credentials_needing_rotation();
        assert_eq!(needing.len(), 1);
        assert_eq!(needing[0], Provider::GitHub);
    }

    #[test]
    fn test_rotate_credential() {
        let mut mgr = AccountManager::new();
        mgr.register_credential(Provider::GitHub, CredentialType::CliSession);
        mgr.credentials
            .get_mut(&Provider::GitHub)
            .unwrap()
            .last_rotated_at = current_epoch_secs() - (31 * 86400);

        let result = mgr.rotate_credential(Provider::GitHub).unwrap();
        assert_eq!(result.provider, Provider::GitHub);
        assert_eq!(result.command, Some("gh auth refresh"));
        assert!(!mgr
            .get_credential(Provider::GitHub)
            .unwrap()
            .needs_rotation());
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
        let github_del = deletions
            .iter()
            .find(|d| d.provider == Provider::GitHub)
            .unwrap();
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
        assert_eq!(
            CaptchaAction::PauseForManual.to_string(),
            "pause_for_manual"
        );
        assert_eq!(
            CaptchaAction::RetryAfterDelay.to_string(),
            "retry_after_delay"
        );
        assert_eq!(CaptchaAction::SkipProvider.to_string(), "skip_provider");
    }

    // ── Signup flow tests ──────────────────────────────────────────────

    #[test]
    fn test_github_signup_steps() {
        let steps = signup_steps(Provider::GitHub);
        assert!(!steps.is_empty());
        // First step should navigate to signup
        assert!(
            matches!(&steps[0].action, SignupAction::Navigate { url } if url.contains("github.com/signup"))
        );
        // Should contain a CAPTCHA pause step
        assert!(steps
            .iter()
            .any(|s| matches!(&s.action, SignupAction::PauseForCaptcha { .. })));
    }

    #[test]
    fn test_cloudflare_signup_steps() {
        let steps = signup_steps(Provider::Cloudflare);
        assert!(!steps.is_empty());
        assert!(steps.iter().any(|s| matches!(&s.action, SignupAction::Navigate { url } if url.contains("cloudflare.com"))));
    }

    #[test]
    fn test_all_providers_have_signup_flows() {
        // These 14 providers should all have signup flows now
        let with_flows = [
            Provider::GitHub,
            Provider::Cloudflare,
            Provider::Vercel,
            Provider::Supabase,
            Provider::Upstash,
            Provider::Neon,
            Provider::FlyIo,
            Provider::Railway,
            Provider::Render,
            Provider::Netlify,
            Provider::DigitalOcean,
            Provider::Hetzner,
            Provider::Vultr,
            Provider::OracleCloud,
        ];
        for provider in &with_flows {
            let steps = signup_steps(*provider);
            assert!(!steps.is_empty(), "{:?} should have signup steps", provider);
            // Every flow should start with a Navigate
            assert!(
                matches!(&steps[0].action, SignupAction::Navigate { .. }),
                "{:?} first step should be Navigate",
                provider
            );
            // Every flow should have at least one PauseForCaptcha
            assert!(
                steps
                    .iter()
                    .any(|s| matches!(&s.action, SignupAction::PauseForCaptcha { .. })),
                "{:?} should have a CAPTCHA pause step",
                provider
            );
        }
    }

    #[test]
    fn test_has_signup_flow() {
        let mgr = AccountManager::new();
        assert!(mgr.has_signup_flow(Provider::GitHub));
        assert!(mgr.has_signup_flow(Provider::Cloudflare));
        assert!(mgr.has_signup_flow(Provider::Render));
        assert!(mgr.has_signup_flow(Provider::Neon));
        // Providers without flows
        assert!(!mgr.has_signup_flow(Provider::GoogleCloud));
        assert!(!mgr.has_signup_flow(Provider::AwsFreeTier));
    }

    #[test]
    fn test_signup_step_serde() {
        let step = SignupStep {
            description: "Navigate".into(),
            action: SignupAction::Navigate {
                url: "https://example.com".into(),
            },
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
        mgr.credentials
            .get_mut(&Provider::GitHub)
            .unwrap()
            .last_rotated_at = current_epoch_secs() - (31 * 86400);

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
        assert_eq!(
            AccountManager::healing_layer_for(AccountHealingAction::None),
            "none"
        );
        assert_eq!(
            AccountManager::healing_layer_for(AccountHealingAction::RotateCredential),
            "retry"
        );
        assert_eq!(
            AccountManager::healing_layer_for(AccountHealingAction::Reauthenticate),
            "alternative"
        );
        assert_eq!(
            AccountManager::healing_layer_for(AccountHealingAction::PauseAndAlert),
            "pause_and_alert"
        );
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
        assert_eq!(
            rotation_command(Provider::GoogleCloud),
            Some("gcloud auth login")
        );
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
        assert_eq!(
            record2.keychain_service.as_deref(),
            Some("phantom-oracle-cloud")
        );
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

    // ── Template resolution tests ─────────────────────────────────────

    #[test]
    fn test_resolve_template_basic() {
        let mut vars = HashMap::new();
        vars.insert("EMAIL".into(), "test@example.com".into());
        vars.insert("PASSWORD".into(), "s3cret".into());

        assert_eq!(resolve_template("{{EMAIL}}", &vars), "test@example.com");
        assert_eq!(
            resolve_template("user: {{EMAIL}} / {{PASSWORD}}", &vars),
            "user: test@example.com / s3cret"
        );
    }

    #[test]
    fn test_resolve_template_no_match() {
        let vars = HashMap::new();
        assert_eq!(resolve_template("{{UNKNOWN}}", &vars), "{{UNKNOWN}}");
    }

    #[test]
    fn test_resolve_template_multiple_occurrences() {
        let mut vars = HashMap::new();
        vars.insert("X".into(), "y".into());
        assert_eq!(resolve_template("{{X}}-{{X}}", &vars), "y-y");
    }

    // ── Signup executor mock tests ────────────────────────────────────

    /// Mock executor for testing `execute_signup_flow`.
    struct MockExecutor {
        navigations: Vec<String>,
        fills: Vec<(String, String)>,
        clicks: Vec<String>,
        captcha_pauses: usize,
        page_html: String,
        should_fail_at: Option<&'static str>,
    }

    impl MockExecutor {
        fn new() -> Self {
            Self {
                navigations: Vec::new(),
                fills: Vec::new(),
                clicks: Vec::new(),
                captcha_pauses: 0,
                page_html: String::new(),
                should_fail_at: None,
            }
        }

        fn with_captcha_page(mut self) -> Self {
            self.page_html = "<html><div class='g-recaptcha'></div></html>".into();
            self
        }
    }

    impl SignupExecutor for MockExecutor {
        fn navigate(&mut self, url: &str) -> Result<(), AccountError> {
            if self.should_fail_at == Some("navigate") {
                return Err(AccountError::SignupFailed {
                    step: 0,
                    reason: "mock nav fail".into(),
                });
            }
            self.navigations.push(url.to_string());
            Ok(())
        }
        fn fill(&mut self, selector: &str, value: &str) -> Result<(), AccountError> {
            self.fills.push((selector.to_string(), value.to_string()));
            Ok(())
        }
        fn click(&mut self, selector: &str) -> Result<(), AccountError> {
            self.clicks.push(selector.to_string());
            Ok(())
        }
        fn wait_for(&mut self, _selector: &str, _timeout: Duration) -> Result<(), AccountError> {
            Ok(())
        }
        fn wait_for_url(&mut self, _pattern: &str, _timeout: Duration) -> Result<(), AccountError> {
            Ok(())
        }
        fn extract_token(&mut self, _selector: &str) -> Result<String, AccountError> {
            Ok("mock-token-123".into())
        }
        fn screenshot(&mut self, _filename: &str) -> Result<(), AccountError> {
            Ok(())
        }
        fn page_content(&mut self) -> Result<String, AccountError> {
            Ok(self.page_html.clone())
        }
        fn notify_captcha_pause(&mut self, _message: &str) -> Result<(), AccountError> {
            self.captcha_pauses += 1;
            Ok(())
        }
    }

    #[test]
    fn test_execute_signup_flow_cloudflare() {
        let mut mgr = AccountManager::new();
        let mut executor = MockExecutor::new();
        let mut vars = HashMap::new();
        vars.insert("EMAIL".into(), "test@phantom.dev".into());
        vars.insert("PASSWORD".into(), "hunter2".into());

        let result = mgr
            .execute_signup_flow(Provider::Cloudflare, &mut executor, &vars)
            .unwrap();
        assert!(result.success);
        assert_eq!(result.steps_total, 6);
        assert_eq!(result.steps_completed, 6);
        // Should have navigated to cloudflare
        assert!(executor
            .navigations
            .iter()
            .any(|u| u.contains("cloudflare.com")));
        // Should have filled email and password
        assert!(executor.fills.iter().any(|(_, v)| v == "test@phantom.dev"));
        assert!(executor.fills.iter().any(|(_, v)| v == "hunter2"));
        // Should have at least 1 captcha pause (the PauseForCaptcha step)
        assert!(executor.captcha_pauses >= 1);
        // Should have registered credential
        assert!(mgr.get_credential(Provider::Cloudflare).is_some());
    }

    #[test]
    fn test_execute_signup_flow_with_auto_captcha_detection() {
        let mut mgr = AccountManager::new();
        let mut executor = MockExecutor::new().with_captcha_page();
        let vars = HashMap::new();

        let result = mgr
            .execute_signup_flow(Provider::Cloudflare, &mut executor, &vars)
            .unwrap();
        assert!(result.success);
        // Should have extra pauses from auto-detection (check_captcha steps scan page)
        assert!(result.captcha_pauses >= 2); // 1 from PauseForCaptcha + 1+ from auto-detect
    }

    #[test]
    fn test_execute_signup_flow_navigate_failure() {
        let mut mgr = AccountManager::new();
        let mut executor = MockExecutor::new();
        executor.should_fail_at = Some("navigate");
        let vars = HashMap::new();

        let err = mgr.execute_signup_flow(Provider::Cloudflare, &mut executor, &vars);
        assert!(err.is_err());
        match err.unwrap_err() {
            AccountError::SignupFailed { step, reason } => {
                assert_eq!(step, 0);
                assert!(reason.contains("navigate failed"));
            }
            other => panic!("expected SignupFailed, got: {other:?}"),
        }
    }

    #[test]
    fn test_execute_signup_flow_no_flow_defined() {
        let mut mgr = AccountManager::new();
        let mut executor = MockExecutor::new();
        let vars = HashMap::new();

        // GoogleCloud has no signup flow
        let err = mgr.execute_signup_flow(Provider::GoogleCloud, &mut executor, &vars);
        assert!(err.is_err());
    }

    #[test]
    fn test_execute_signup_flow_vercel() {
        let mut mgr = AccountManager::new();
        let mut executor = MockExecutor::new();
        let mut vars = HashMap::new();
        vars.insert("EMAIL".into(), "v@test.com".into());

        let result = mgr
            .execute_signup_flow(Provider::Vercel, &mut executor, &vars)
            .unwrap();
        assert!(result.success);
        assert!(executor
            .navigations
            .iter()
            .any(|u| u.contains("vercel.com")));
    }

    #[test]
    fn test_execute_signup_flow_registers_credential() {
        let mut mgr = AccountManager::new();
        let mut executor = MockExecutor::new();
        let vars = HashMap::new();

        mgr.execute_signup_flow(Provider::FlyIo, &mut executor, &vars)
            .unwrap();

        // Should have registered a CliSession credential (FlyIo uses CliLogin)
        let cred = mgr.get_credential(Provider::FlyIo).unwrap();
        assert_eq!(cred.credential_type, CredentialType::CliSession);
        assert!(!cred.revoked);

        // Should be marked authenticated
        assert!(mgr.get_status(Provider::FlyIo).unwrap().authenticated);
    }

    #[test]
    fn test_signup_step_count_per_provider() {
        // Verify each provider has a reasonable number of steps
        let providers_and_min_steps = [
            (Provider::Vercel, 5),
            (Provider::Supabase, 5),
            (Provider::Upstash, 5),
            (Provider::Neon, 5),
            (Provider::FlyIo, 6),
            (Provider::Railway, 5),
            (Provider::Render, 6),
            (Provider::Netlify, 6),
            (Provider::DigitalOcean, 5),
            (Provider::Hetzner, 4),
            (Provider::Vultr, 7),
            (Provider::OracleCloud, 9),
        ];
        for (provider, min) in &providers_and_min_steps {
            let steps = signup_steps(*provider);
            assert!(
                steps.len() >= *min,
                "{:?} should have at least {} steps, got {}",
                provider,
                min,
                steps.len()
            );
        }
    }
}
