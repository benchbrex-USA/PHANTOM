//! 14+ cloud provider clients for self-discovering infrastructure.
//!
//! Each provider is modeled as:
//!   • Static metadata (name, free-tier limits, required CLI tool)
//!   • ProviderStatus (credentials present, quota remaining, health)
//!   • Resource types it can provide (compute, storage, database, edge, ci)

use std::fmt;

use serde::{Deserialize, Serialize};

/// Supported infrastructure providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    OracleCloud,
    GoogleCloud,
    AwsFreeTier,
    Cloudflare,
    FlyIo,
    Railway,
    Vercel,
    Netlify,
    Supabase,
    Neon,
    Upstash,
    Render,
    GitHub,
    CloudflareR2,
}

/// All providers in priority order (primary first).
pub const ALL_PROVIDERS: &[Provider] = &[
    Provider::OracleCloud,
    Provider::GoogleCloud,
    Provider::AwsFreeTier,
    Provider::Cloudflare,
    Provider::FlyIo,
    Provider::Railway,
    Provider::Vercel,
    Provider::Netlify,
    Provider::Supabase,
    Provider::Neon,
    Provider::Upstash,
    Provider::Render,
    Provider::GitHub,
    Provider::CloudflareR2,
];

impl Provider {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::OracleCloud => "Oracle Cloud",
            Self::GoogleCloud => "Google Cloud",
            Self::AwsFreeTier => "AWS Free Tier",
            Self::Cloudflare => "Cloudflare",
            Self::FlyIo => "Fly.io",
            Self::Railway => "Railway",
            Self::Vercel => "Vercel",
            Self::Netlify => "Netlify",
            Self::Supabase => "Supabase",
            Self::Neon => "Neon",
            Self::Upstash => "Upstash",
            Self::Render => "Render",
            Self::GitHub => "GitHub",
            Self::CloudflareR2 => "Cloudflare R2",
        }
    }

    pub fn free_tier_description(&self) -> &'static str {
        match self {
            Self::OracleCloud => "2 VMs + 200GB (primary compute)",
            Self::GoogleCloud => "e2-micro (secondary)",
            Self::AwsFreeTier => "t2.micro 12mo (backup)",
            Self::Cloudflare => "Workers + R2 + DNS (edge)",
            Self::FlyIo => "3 shared VMs (P2P mesh)",
            Self::Railway => "$5/mo credit (ephemeral builds)",
            Self::Vercel => "Serverless (frontend)",
            Self::Netlify => "100GB bw/mo (fallback frontend)",
            Self::Supabase => "500MB PG (database)",
            Self::Neon => "0.5GB PG (backup database)",
            Self::Upstash => "10K cmd/day (Redis)",
            Self::Render => "Static + DB (static hosting)",
            Self::GitHub => "Unlimited repos (code + CI/CD)",
            Self::CloudflareR2 => "10GB (encrypted blob storage)",
        }
    }

    /// The CLI tool required to interact with this provider.
    pub fn cli_tool(&self) -> Option<&'static str> {
        match self {
            Self::OracleCloud => Some("oci"),
            Self::GoogleCloud => Some("gcloud"),
            Self::AwsFreeTier => Some("aws"),
            Self::Cloudflare => Some("wrangler"),
            Self::FlyIo => Some("flyctl"),
            Self::Railway => Some("railway"),
            Self::Vercel => Some("vercel"),
            Self::Netlify => Some("netlify"),
            Self::Supabase => Some("supabase"),
            Self::Neon => Some("neonctl"),
            Self::Upstash => None, // API-only
            Self::Render => None,  // API-only
            Self::GitHub => Some("gh"),
            Self::CloudflareR2 => Some("wrangler"),
        }
    }

    /// Resource categories this provider can fulfill.
    pub fn resource_types(&self) -> Vec<ResourceType> {
        match self {
            Self::OracleCloud => vec![ResourceType::Compute, ResourceType::Storage],
            Self::GoogleCloud => vec![ResourceType::Compute],
            Self::AwsFreeTier => vec![ResourceType::Compute, ResourceType::Storage],
            Self::Cloudflare => vec![ResourceType::Edge, ResourceType::Storage, ResourceType::Dns],
            Self::FlyIo => vec![ResourceType::Compute],
            Self::Railway => vec![ResourceType::Compute],
            Self::Vercel => vec![ResourceType::Edge],
            Self::Netlify => vec![ResourceType::Edge],
            Self::Supabase => vec![ResourceType::Database],
            Self::Neon => vec![ResourceType::Database],
            Self::Upstash => vec![ResourceType::Cache],
            Self::Render => vec![ResourceType::Compute, ResourceType::Database],
            Self::GitHub => vec![ResourceType::Ci, ResourceType::CodeHost],
            Self::CloudflareR2 => vec![ResourceType::Storage],
        }
    }

    /// Priority rank (lower = try first). Based on Architecture Framework §9.
    pub fn priority(&self) -> u8 {
        match self {
            Self::OracleCloud => 1,  // Primary compute
            Self::GoogleCloud => 2,  // Secondary compute
            Self::AwsFreeTier => 3,  // Backup compute
            Self::Cloudflare => 4,   // Edge + DNS
            Self::CloudflareR2 => 5, // Blob storage
            Self::GitHub => 6,       // Code + CI
            Self::Supabase => 7,     // Primary DB
            Self::Neon => 8,         // Backup DB
            Self::FlyIo => 9,        // P2P mesh nodes
            Self::Upstash => 10,     // Cache
            Self::Railway => 11,     // Ephemeral
            Self::Vercel => 12,      // Frontend
            Self::Netlify => 13,     // Frontend backup
            Self::Render => 14,      // Static hosting
        }
    }

    /// Providers that can serve as fallback for this provider's role.
    pub fn fallbacks(&self) -> Vec<Provider> {
        match self {
            Self::OracleCloud => vec![Self::GoogleCloud, Self::AwsFreeTier, Self::FlyIo],
            Self::GoogleCloud => vec![Self::AwsFreeTier, Self::FlyIo],
            Self::AwsFreeTier => vec![Self::FlyIo, Self::Railway],
            Self::Supabase => vec![Self::Neon, Self::Render],
            Self::Neon => vec![Self::Supabase, Self::Render],
            Self::CloudflareR2 => vec![Self::AwsFreeTier],
            Self::Vercel => vec![Self::Netlify, Self::Render],
            Self::Netlify => vec![Self::Vercel, Self::Render],
            _ => Vec::new(),
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Types of infrastructure resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// Virtual machines / containers
    Compute,
    /// Object / blob storage
    Storage,
    /// SQL/Postgres database
    Database,
    /// Redis / key-value cache
    Cache,
    /// Edge functions / serverless
    Edge,
    /// DNS management
    Dns,
    /// CI/CD pipelines
    Ci,
    /// Source code hosting
    CodeHost,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compute => write!(f, "compute"),
            Self::Storage => write!(f, "storage"),
            Self::Database => write!(f, "database"),
            Self::Cache => write!(f, "cache"),
            Self::Edge => write!(f, "edge"),
            Self::Dns => write!(f, "dns"),
            Self::Ci => write!(f, "ci"),
            Self::CodeHost => write!(f, "code_host"),
        }
    }
}

/// Runtime status of a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub provider: Provider,
    pub state: ProviderState,
    /// Whether CLI tool is installed
    pub cli_installed: bool,
    /// Whether credentials / auth is configured
    pub authenticated: bool,
    /// Human-readable status message
    pub message: Option<String>,
}

impl ProviderStatus {
    pub fn unchecked(provider: Provider) -> Self {
        Self {
            provider,
            state: ProviderState::Unknown,
            cli_installed: false,
            authenticated: false,
            message: None,
        }
    }

    pub fn available(provider: Provider) -> Self {
        Self {
            provider,
            state: ProviderState::Available,
            cli_installed: true,
            authenticated: true,
            message: None,
        }
    }

    pub fn unavailable(provider: Provider, reason: impl Into<String>) -> Self {
        Self {
            provider,
            state: ProviderState::Unavailable,
            cli_installed: false,
            authenticated: false,
            message: Some(reason.into()),
        }
    }
}

/// Provider availability state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderState {
    /// Provider status not yet checked
    Unknown,
    /// CLI installed, authenticated, quota available
    Available,
    /// CLI missing or auth not configured
    Unavailable,
    /// Quota exhausted but otherwise working
    QuotaExhausted,
    /// Provider is responding but degraded
    Degraded,
}

/// Find providers that can fulfill a given resource type, sorted by priority.
pub fn providers_for_resource(resource: ResourceType) -> Vec<Provider> {
    let mut providers: Vec<Provider> = ALL_PROVIDERS
        .iter()
        .filter(|p| p.resource_types().contains(&resource))
        .copied()
        .collect();
    providers.sort_by_key(|p| p.priority());
    providers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_providers_count() {
        assert_eq!(ALL_PROVIDERS.len(), 14);
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(Provider::OracleCloud.to_string(), "Oracle Cloud");
        assert_eq!(Provider::CloudflareR2.to_string(), "Cloudflare R2");
    }

    #[test]
    fn test_provider_cli_tools() {
        assert_eq!(Provider::GitHub.cli_tool(), Some("gh"));
        assert_eq!(Provider::Upstash.cli_tool(), None);
    }

    #[test]
    fn test_provider_resource_types() {
        let oracle = Provider::OracleCloud.resource_types();
        assert!(oracle.contains(&ResourceType::Compute));
        assert!(oracle.contains(&ResourceType::Storage));

        let supabase = Provider::Supabase.resource_types();
        assert!(supabase.contains(&ResourceType::Database));
    }

    #[test]
    fn test_providers_for_resource() {
        let compute = providers_for_resource(ResourceType::Compute);
        assert!(compute.len() >= 4);
        // Oracle should be first (priority 1)
        assert_eq!(compute[0], Provider::OracleCloud);

        let db = providers_for_resource(ResourceType::Database);
        assert!(db.contains(&Provider::Supabase));
        assert!(db.contains(&Provider::Neon));
    }

    #[test]
    fn test_provider_fallbacks() {
        let fallbacks = Provider::OracleCloud.fallbacks();
        assert!(fallbacks.contains(&Provider::GoogleCloud));
        assert!(fallbacks.contains(&Provider::AwsFreeTier));
    }

    #[test]
    fn test_provider_priority_ordering() {
        assert!(Provider::OracleCloud.priority() < Provider::GoogleCloud.priority());
        assert!(Provider::GoogleCloud.priority() < Provider::Render.priority());
    }

    #[test]
    fn test_provider_status() {
        let status = ProviderStatus::available(Provider::GitHub);
        assert_eq!(status.state, ProviderState::Available);
        assert!(status.authenticated);

        let unavail = ProviderStatus::unavailable(Provider::FlyIo, "CLI not installed");
        assert_eq!(unavail.state, ProviderState::Unavailable);
        assert_eq!(unavail.message, Some("CLI not installed".into()));
    }

    #[test]
    fn test_provider_serde() {
        let p = Provider::CloudflareR2;
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"cloudflare_r2\"");
        let decoded: Provider = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, Provider::CloudflareR2);
    }
}
