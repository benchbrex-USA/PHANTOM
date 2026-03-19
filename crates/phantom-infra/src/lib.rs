//! Phantom Infrastructure: provider clients, provisioner, health checks,
//! account creation, dependency installer.
//!
//! Core Law 6: Self-provisioning infrastructure.
//! Phantom finds, creates, and binds to free-tier servers autonomously.

pub mod accounts;
pub mod dependencies;
pub mod doctor;
pub mod errors;
pub mod health;
pub mod providers;
pub mod provisioner;

pub use accounts::{AccountManager, AccountStatus, AuthMethod};
pub use dependencies::{DependencyCheck, DependencyInstaller, DependencySummary, InstallPhase};
pub use doctor::{Doctor, DoctorReport, DoctorResult, DoctorStatus};
pub use errors::InfraError;
pub use health::{HealthCheckResult, HealthChecker, HealthStatus, HealthSummary};
pub use providers::{Provider, ProviderState, ProviderStatus, ResourceType, ALL_PROVIDERS};
pub use provisioner::{ProvisionRequest, ProvisionedResource, Provisioner, ResourceState};
