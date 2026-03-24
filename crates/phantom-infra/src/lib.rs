//! Phantom Infrastructure: provider clients, provisioner, health checks,
//! account creation, dependency installer.
//!
//! Core Law 6: Self-provisioning infrastructure.
//! Phantom finds, creates, and binds to free-tier servers autonomously.

pub mod accounts;
pub mod cloudflare;
pub mod dependencies;
pub mod doctor;
pub mod errors;
pub mod fly;
pub mod github_provider;
pub mod health;
pub mod installer;
pub mod neon;
pub mod oracle;
pub mod providers;
pub mod provisioner;
pub mod supabase;
pub mod traits;
pub mod upstash;
pub mod vercel;

pub use accounts::{
    AccountError, AccountHealingAction, AccountHealth, AccountHealthCheck, AccountManager,
    AccountStatus, AuthMethod, CaptchaAction, CaptchaDetection, CredentialRecord, CredentialType,
    DeletionRecord, OAuthFlowConfig, OAuthFlowState, RotationResult, SignupAction, SignupExecutor,
    SignupFlowResult, SignupStep,
};
pub use dependencies::{DependencyCheck, DependencyInstaller, DependencySummary, InstallPhase};
pub use doctor::{format_report, Doctor, DoctorReport, DoctorResult, DoctorStatus};
pub use errors::InfraError;
pub use health::{HealthCheckResult, HealthChecker, HealthStatus, HealthSummary};
pub use providers::{Provider, ProviderState, ProviderStatus, ResourceType, ALL_PROVIDERS};
pub use provisioner::{ProvisionRequest, ProvisionedResource, Provisioner, ResourceState};

pub use cloudflare::{
    CloudflareClient, CloudflareConfig, DnsRecord, DnsRecordType, DnsZone, R2Bucket, Worker,
};
pub use fly::{FlyApp, FlyClient, FlyConfig, FlyMachine, MachineConfig, MachineState};
pub use github_provider::{GitHubClient, Repository, WorkflowRun};
pub use installer::{DependencyInstaller as MacInstaller, SystemReport};
pub use neon::{NeonBranch, NeonClient, NeonEndpoint, NeonProject};
pub use oracle::{Instance, InstanceState, OracleClient, OracleConfig, Vcn};
pub use supabase::{ApiKey as SupabaseApiKey, SupabaseClient, SupabaseConfig, SupabaseProject};
pub use traits::{
    CloudProvider, FreeTierLimits, HealthStatus as ProviderHealthStatus, ProjectConfig, ProjectInfo,
};
pub use upstash::{RedisDatabase, UpstashClient};
pub use vercel::{
    EnvVar as VercelEnvVar, VercelClient, VercelDeployment, VercelDomain, VercelProject,
};
