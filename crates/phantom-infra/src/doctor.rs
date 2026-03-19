//! `phantom doctor` — verify all dependencies are installed and healthy.
//!
//! Runs comprehensive checks across:
//!   • System dependencies (git, docker, node, python, rust)
//!   • AI tools (aider, chromadb, sentence-transformers)
//!   • Provider CLIs (gh, flyctl, wrangler, vercel, etc.)
//!   • Service connectivity (ChromaDB, Supabase, etc.)

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::dependencies::{DependencyCheck, DependencyInstaller};
use crate::providers::ALL_PROVIDERS;

/// Result of a single doctor check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorResult {
    pub name: String,
    pub category: String,
    pub version: Option<String>,
    pub status: DoctorStatus,
    pub message: Option<String>,
}

impl DoctorResult {
    pub fn ok(
        name: impl Into<String>,
        category: impl Into<String>,
        version: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            category: category.into(),
            version,
            status: DoctorStatus::Ok,
            message: None,
        }
    }

    pub fn missing(
        name: impl Into<String>,
        category: impl Into<String>,
        msg: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            category: category.into(),
            version: None,
            status: DoctorStatus::Missing,
            message: Some(msg.into()),
        }
    }

    pub fn warning(
        name: impl Into<String>,
        category: impl Into<String>,
        msg: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            category: category.into(),
            version: None,
            status: DoctorStatus::Warning,
            message: Some(msg.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoctorStatus {
    Ok,
    Missing,
    Warning,
    Error,
}

impl std::fmt::Display for DoctorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Missing => write!(f, "MISSING"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}

/// Overall doctor report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub results: Vec<DoctorResult>,
    pub total: usize,
    pub ok_count: usize,
    pub missing_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    /// True if all required checks pass
    pub healthy: bool,
}

impl DoctorReport {
    pub fn from_results(results: Vec<DoctorResult>) -> Self {
        let total = results.len();
        let ok_count = results
            .iter()
            .filter(|r| r.status == DoctorStatus::Ok)
            .count();
        let missing_count = results
            .iter()
            .filter(|r| r.status == DoctorStatus::Missing)
            .count();
        let warning_count = results
            .iter()
            .filter(|r| r.status == DoctorStatus::Warning)
            .count();
        let error_count = results
            .iter()
            .filter(|r| r.status == DoctorStatus::Error)
            .count();
        let healthy = error_count == 0 && missing_count == 0;

        Self {
            results,
            total,
            ok_count,
            missing_count,
            warning_count,
            error_count,
            healthy,
        }
    }
}

/// Run all dependency health checks.
pub struct Doctor {
    installer: DependencyInstaller,
}

impl Default for Doctor {
    fn default() -> Self {
        Self::new()
    }
}

impl Doctor {
    pub fn new() -> Self {
        Self {
            installer: DependencyInstaller::new(),
        }
    }

    /// Run full doctor check (dependencies + providers).
    pub fn run(&self) -> DoctorReport {
        let mut results = Vec::new();

        // Check all dependencies
        results.extend(self.check_dependencies());

        // Check provider CLI availability
        results.extend(self.check_provider_clis());

        let report = DoctorReport::from_results(results);
        info!(
            total = report.total,
            ok = report.ok_count,
            missing = report.missing_count,
            "doctor check complete"
        );
        report
    }

    /// Check only dependencies.
    pub fn check_dependencies(&self) -> Vec<DoctorResult> {
        self.installer
            .check_all()
            .into_iter()
            .map(dep_check_to_result)
            .collect()
    }

    /// Check provider CLI availability.
    pub fn check_provider_clis(&self) -> Vec<DoctorResult> {
        ALL_PROVIDERS
            .iter()
            .filter_map(|provider| {
                let tool = provider.cli_tool()?;
                let output = std::process::Command::new("which")
                    .arg(tool)
                    .output()
                    .ok()?;

                if output.status.success() {
                    Some(DoctorResult::ok(
                        format!("{} CLI", provider.display_name()),
                        "Provider CLI",
                        Some(tool.to_string()),
                    ))
                } else {
                    Some(DoctorResult::warning(
                        format!("{} CLI", provider.display_name()),
                        "Provider CLI",
                        format!(
                            "`{}` not found — {} features will be unavailable",
                            tool,
                            provider.display_name()
                        ),
                    ))
                }
            })
            .collect()
    }
}

fn dep_check_to_result(check: DependencyCheck) -> DoctorResult {
    let category = check.phase.to_string();
    if check.installed {
        DoctorResult::ok(check.name, category, check.version)
    } else if check.required {
        DoctorResult::missing(check.name, category, "required dependency not installed")
    } else {
        DoctorResult::warning(check.name, category, "optional dependency not installed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doctor_runs() {
        let doctor = Doctor::new();
        let report = doctor.run();
        assert!(report.total > 0);
        assert_eq!(
            report.total,
            report.ok_count + report.missing_count + report.warning_count + report.error_count
        );
    }

    #[test]
    fn test_doctor_report_from_results() {
        let results = vec![
            DoctorResult::ok("Git", "System", Some("2.40".into())),
            DoctorResult::missing("Docker", "System", "not installed"),
            DoctorResult::warning("flyctl", "Provider CLI", "optional"),
        ];
        let report = DoctorReport::from_results(results);
        assert_eq!(report.total, 3);
        assert_eq!(report.ok_count, 1);
        assert_eq!(report.missing_count, 1);
        assert_eq!(report.warning_count, 1);
        assert!(!report.healthy); // Missing counts as unhealthy
    }

    #[test]
    fn test_doctor_healthy_report() {
        let results = vec![
            DoctorResult::ok("Git", "System", Some("2.40".into())),
            DoctorResult::warning("flyctl", "Provider CLI", "optional"),
        ];
        let report = DoctorReport::from_results(results);
        assert!(report.healthy); // Warnings don't make it unhealthy
    }

    #[test]
    fn test_doctor_status_display() {
        assert_eq!(DoctorStatus::Ok.to_string(), "OK");
        assert_eq!(DoctorStatus::Missing.to_string(), "MISSING");
    }
}
