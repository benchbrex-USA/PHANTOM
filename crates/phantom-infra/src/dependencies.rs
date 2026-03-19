//! Autonomous dependency installation pipeline.
//! Detects, installs, and verifies all 18+ required dependencies.
//!
//! 3-phase installation:
//!   Phase 1 — System prerequisites (brew, git, docker, node, python, rust)
//!   Phase 2 — AI tools (aider, chromadb, sentence-transformers)
//!   Phase 3 — Deployment tools (provider CLIs: oci, gcloud, aws, flyctl, etc.)

use std::process::Command;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::errors::InfraError;

/// A system dependency that Phantom requires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Human-readable name
    pub name: String,
    /// Command to check if installed (e.g. "git --version")
    pub check_command: String,
    /// Command to install (e.g. "brew install git")
    pub install_command: String,
    /// Expected output pattern (substring match on check output)
    pub version_pattern: Option<String>,
    /// Installation phase
    pub phase: InstallPhase,
    /// Whether this dependency is required (vs optional)
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallPhase {
    SystemPrerequisites,
    AiTools,
    DeploymentTools,
}

impl std::fmt::Display for InstallPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SystemPrerequisites => write!(f, "System Prerequisites"),
            Self::AiTools => write!(f, "AI Tools"),
            Self::DeploymentTools => write!(f, "Deployment Tools"),
        }
    }
}

/// Result of checking a single dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCheck {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub phase: InstallPhase,
    pub required: bool,
}

/// Build the full dependency manifest.
pub fn all_dependencies() -> Vec<Dependency> {
    vec![
        // Phase 1: System Prerequisites
        Dependency {
            name: "Homebrew".into(),
            check_command: "brew --version".into(),
            install_command: "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"".into(),
            version_pattern: Some("Homebrew".into()),
            phase: InstallPhase::SystemPrerequisites,
            required: true,
        },
        Dependency {
            name: "Git".into(),
            check_command: "git --version".into(),
            install_command: "brew install git".into(),
            version_pattern: Some("git version".into()),
            phase: InstallPhase::SystemPrerequisites,
            required: true,
        },
        Dependency {
            name: "Docker".into(),
            check_command: "docker --version".into(),
            install_command: "brew install --cask docker".into(),
            version_pattern: Some("Docker version".into()),
            phase: InstallPhase::SystemPrerequisites,
            required: true,
        },
        Dependency {
            name: "Node.js".into(),
            check_command: "node --version".into(),
            install_command: "brew install node".into(),
            version_pattern: Some("v".into()),
            phase: InstallPhase::SystemPrerequisites,
            required: true,
        },
        Dependency {
            name: "Python 3".into(),
            check_command: "python3 --version".into(),
            install_command: "brew install python3".into(),
            version_pattern: Some("Python 3".into()),
            phase: InstallPhase::SystemPrerequisites,
            required: true,
        },
        Dependency {
            name: "Rust".into(),
            check_command: "rustc --version".into(),
            install_command: "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y".into(),
            version_pattern: Some("rustc".into()),
            phase: InstallPhase::SystemPrerequisites,
            required: true,
        },
        Dependency {
            name: "jq".into(),
            check_command: "jq --version".into(),
            install_command: "brew install jq".into(),
            version_pattern: Some("jq-".into()),
            phase: InstallPhase::SystemPrerequisites,
            required: true,
        },
        Dependency {
            name: "curl".into(),
            check_command: "curl --version".into(),
            install_command: "brew install curl".into(),
            version_pattern: Some("curl".into()),
            phase: InstallPhase::SystemPrerequisites,
            required: true,
        },
        // Phase 2: AI Tools
        Dependency {
            name: "Aider".into(),
            check_command: "aider --version".into(),
            install_command: "pip3 install aider-chat".into(),
            version_pattern: None,
            phase: InstallPhase::AiTools,
            required: true,
        },
        Dependency {
            name: "ChromaDB".into(),
            check_command: "python3 -c \"import chromadb; print(chromadb.__version__)\"".into(),
            install_command: "pip3 install chromadb".into(),
            version_pattern: None,
            phase: InstallPhase::AiTools,
            required: true,
        },
        Dependency {
            name: "sentence-transformers".into(),
            check_command: "python3 -c \"import sentence_transformers; print(sentence_transformers.__version__)\"".into(),
            install_command: "pip3 install sentence-transformers".into(),
            version_pattern: None,
            phase: InstallPhase::AiTools,
            required: true,
        },
        // Phase 3: Deployment Tools
        Dependency {
            name: "GitHub CLI".into(),
            check_command: "gh --version".into(),
            install_command: "brew install gh".into(),
            version_pattern: Some("gh version".into()),
            phase: InstallPhase::DeploymentTools,
            required: true,
        },
        Dependency {
            name: "Fly.io CLI".into(),
            check_command: "flyctl version".into(),
            install_command: "brew install flyctl".into(),
            version_pattern: Some("flyctl".into()),
            phase: InstallPhase::DeploymentTools,
            required: false,
        },
        Dependency {
            name: "Wrangler (Cloudflare)".into(),
            check_command: "wrangler --version".into(),
            install_command: "npm install -g wrangler".into(),
            version_pattern: Some("wrangler".into()),
            phase: InstallPhase::DeploymentTools,
            required: false,
        },
        Dependency {
            name: "Vercel CLI".into(),
            check_command: "vercel --version".into(),
            install_command: "npm install -g vercel".into(),
            version_pattern: None,
            phase: InstallPhase::DeploymentTools,
            required: false,
        },
        Dependency {
            name: "Railway CLI".into(),
            check_command: "railway --version".into(),
            install_command: "brew install railway".into(),
            version_pattern: None,
            phase: InstallPhase::DeploymentTools,
            required: false,
        },
        Dependency {
            name: "Supabase CLI".into(),
            check_command: "supabase --version".into(),
            install_command: "brew install supabase/tap/supabase".into(),
            version_pattern: None,
            phase: InstallPhase::DeploymentTools,
            required: false,
        },
        Dependency {
            name: "AWS CLI".into(),
            check_command: "aws --version".into(),
            install_command: "brew install awscli".into(),
            version_pattern: Some("aws-cli".into()),
            phase: InstallPhase::DeploymentTools,
            required: false,
        },
        Dependency {
            name: "Google Cloud CLI".into(),
            check_command: "gcloud --version".into(),
            install_command: "brew install --cask google-cloud-sdk".into(),
            version_pattern: Some("Google Cloud SDK".into()),
            phase: InstallPhase::DeploymentTools,
            required: false,
        },
    ]
}

/// Dependency installer and verifier.
pub struct DependencyInstaller {
    deps: Vec<Dependency>,
}

impl DependencyInstaller {
    pub fn new() -> Self {
        Self {
            deps: all_dependencies(),
        }
    }

    /// Check a single dependency.
    pub fn check_one(&self, dep: &Dependency) -> DependencyCheck {
        let parts: Vec<&str> = dep.check_command.split_whitespace().collect();
        if parts.is_empty() {
            return DependencyCheck {
                name: dep.name.clone(),
                installed: false,
                version: None,
                phase: dep.phase,
                required: dep.required,
            };
        }

        let result = Command::new(parts[0])
            .args(&parts[1..])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let combined = format!("{}{}", stdout, stderr);
                let version = combined.lines().next().map(|l| l.trim().to_string());

                debug!(dep = %dep.name, version = ?version, "dependency found");
                DependencyCheck {
                    name: dep.name.clone(),
                    installed: true,
                    version,
                    phase: dep.phase,
                    required: dep.required,
                }
            }
            _ => {
                debug!(dep = %dep.name, "dependency not found");
                DependencyCheck {
                    name: dep.name.clone(),
                    installed: false,
                    version: None,
                    phase: dep.phase,
                    required: dep.required,
                }
            }
        }
    }

    /// Check all dependencies.
    pub fn check_all(&self) -> Vec<DependencyCheck> {
        self.deps.iter().map(|dep| self.check_one(dep)).collect()
    }

    /// Check only dependencies in a specific phase.
    pub fn check_phase(&self, phase: InstallPhase) -> Vec<DependencyCheck> {
        self.deps
            .iter()
            .filter(|d| d.phase == phase)
            .map(|dep| self.check_one(dep))
            .collect()
    }

    /// Get all dependencies that are missing.
    pub fn missing(&self) -> Vec<&Dependency> {
        self.deps
            .iter()
            .filter(|dep| !self.check_one(dep).installed)
            .collect()
    }

    /// Get all required dependencies that are missing.
    pub fn missing_required(&self) -> Vec<&Dependency> {
        self.deps
            .iter()
            .filter(|dep| dep.required && !self.check_one(dep).installed)
            .collect()
    }

    /// Install a single dependency. Returns Ok if successful.
    pub fn install_one(&self, dep: &Dependency) -> Result<(), InfraError> {
        info!(dep = %dep.name, cmd = %dep.install_command, "installing dependency");

        let output = Command::new("sh")
            .arg("-c")
            .arg(&dep.install_command)
            .output()
            .map_err(|e| InfraError::CommandFailed(e.to_string()))?;

        if output.status.success() {
            info!(dep = %dep.name, "dependency installed successfully");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(dep = %dep.name, error = %stderr, "dependency installation failed");
            Err(InfraError::DependencyFailed {
                dep: format!("{}: {}", dep.name, stderr),
            })
        }
    }

    /// Summary statistics.
    pub fn summary(&self) -> DependencySummary {
        let checks = self.check_all();
        let total = checks.len();
        let installed = checks.iter().filter(|c| c.installed).count();
        let missing_required = checks
            .iter()
            .filter(|c| !c.installed && c.required)
            .count();
        let missing_optional = checks
            .iter()
            .filter(|c| !c.installed && !c.required)
            .count();

        DependencySummary {
            total,
            installed,
            missing_required,
            missing_optional,
            ready: missing_required == 0,
        }
    }

    /// Get the full dependency list.
    pub fn all(&self) -> &[Dependency] {
        &self.deps
    }
}

/// Summary of dependency status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySummary {
    pub total: usize,
    pub installed: usize,
    pub missing_required: usize,
    pub missing_optional: usize,
    /// True if all required dependencies are installed
    pub ready: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_dependencies_manifest() {
        let deps = all_dependencies();
        assert!(deps.len() >= 18);

        // Check phases are represented
        let phases: Vec<InstallPhase> = deps.iter().map(|d| d.phase).collect();
        assert!(phases.contains(&InstallPhase::SystemPrerequisites));
        assert!(phases.contains(&InstallPhase::AiTools));
        assert!(phases.contains(&InstallPhase::DeploymentTools));
    }

    #[test]
    fn test_dependency_check_git() {
        let installer = DependencyInstaller::new();
        let git_dep = installer
            .all()
            .iter()
            .find(|d| d.name == "Git")
            .unwrap();
        let check = installer.check_one(git_dep);
        // Git should be installed on any dev machine
        assert_eq!(check.name, "Git");
        // Don't assert installed — CI might not have it
    }

    #[test]
    fn test_dependency_installer_check_all() {
        let installer = DependencyInstaller::new();
        let checks = installer.check_all();
        assert_eq!(checks.len(), installer.all().len());
    }

    #[test]
    fn test_dependency_summary() {
        let installer = DependencyInstaller::new();
        let summary = installer.summary();
        assert!(summary.total >= 18);
        assert!(summary.installed <= summary.total);
    }

    #[test]
    fn test_install_phase_display() {
        assert_eq!(
            InstallPhase::SystemPrerequisites.to_string(),
            "System Prerequisites"
        );
        assert_eq!(InstallPhase::AiTools.to_string(), "AI Tools");
    }

    #[test]
    fn test_check_phase() {
        let installer = DependencyInstaller::new();
        let system = installer.check_phase(InstallPhase::SystemPrerequisites);
        assert!(system.len() >= 6);
        assert!(system.iter().all(|c| c.phase == InstallPhase::SystemPrerequisites));
    }
}
