//! Phantom Infrastructure: provider clients, provisioner, health checks,
//! account creation, dependency installer.
//!
//! Core Law 6: Self-provisioning infrastructure.
//! Phantom finds, creates, and binds to free-tier servers autonomously.

pub mod providers;
pub mod provisioner;
pub mod health;
pub mod accounts;
pub mod dependencies;
pub mod doctor;
pub mod errors;

pub use errors::InfraError;
pub use providers::{Provider, ProviderStatus, ProviderState, ResourceType, ALL_PROVIDERS};
pub use provisioner::{Provisioner, ProvisionedResource, ProvisionRequest, ResourceState};
pub use health::{HealthChecker, HealthCheckResult, HealthStatus, HealthSummary};
pub use accounts::{AccountManager, AccountStatus, AuthMethod};
pub use dependencies::{DependencyInstaller, DependencyCheck, DependencySummary, InstallPhase};
pub use doctor::{Doctor, DoctorReport, DoctorResult, DoctorStatus};
