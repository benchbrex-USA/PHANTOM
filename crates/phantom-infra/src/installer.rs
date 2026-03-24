//! Dependency installer for macOS.
//! Detects missing dependencies and installs them with user confirmation.
//! Architecture Framework Section 4.

use std::process::Command;

use serde::{Deserialize, Serialize};

/// A dependency that Phantom requires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub check_cmd: String,
    pub install_cmd: String,
    pub category: DependencyCategory,
    pub required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyCategory {
    SystemPrerequisite,
    Runtime,
    Database,
    AiTool,
    DeploymentCli,
}

impl std::fmt::Display for DependencyCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SystemPrerequisite => write!(f, "System Prerequisites"),
            Self::Runtime => write!(f, "Runtimes"),
            Self::Database => write!(f, "Databases"),
            Self::AiTool => write!(f, "AI Tools"),
            Self::DeploymentCli => write!(f, "Deployment CLIs"),
        }
    }
}

/// Result of checking a dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepCheckResult {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub category: DependencyCategory,
    pub required: bool,
}

/// A missing dependency that needs installation.
#[derive(Debug, Clone)]
pub struct MissingDep {
    pub dep: Dependency,
    pub reason: String,
}

/// System report from doctor check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemReport {
    pub checks: Vec<DepCheckResult>,
    pub total: usize,
    pub installed: usize,
    pub missing: usize,
    pub healthy: bool,
}

/// Dependency installer for macOS.
pub struct DependencyInstaller {
    dependencies: Vec<Dependency>,
}

impl DependencyInstaller {
    pub fn new() -> Self {
        Self {
            dependencies: all_dependencies(),
        }
    }

    /// Detect which dependencies are missing.
    pub fn detect_missing(&self) -> Vec<MissingDep> {
        self.dependencies
            .iter()
            .filter_map(|dep| {
                let result = check_command(&dep.check_cmd);
                if result.is_err() || !result.unwrap() {
                    Some(MissingDep {
                        dep: dep.clone(),
                        reason: format!("{} not found", dep.name),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Install a single dependency.
    pub fn install(&self, dep: &Dependency) -> Result<(), String> {
        let parts: Vec<&str> = dep.install_cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err("empty install command".into());
        }

        let output = Command::new(parts[0])
            .args(&parts[1..])
            .output()
            .map_err(|e| format!("failed to run {}: {}", dep.install_cmd, e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("install failed: {}", stderr.trim()))
        }
    }

    /// Run a full doctor check.
    pub fn doctor(&self) -> SystemReport {
        let mut checks = Vec::new();

        for dep in &self.dependencies {
            let (installed, version) = check_with_version(&dep.check_cmd);
            checks.push(DepCheckResult {
                name: dep.name.clone(),
                installed,
                version,
                category: dep.category,
                required: dep.required,
            });
        }

        let total = checks.len();
        let installed = checks.iter().filter(|c| c.installed).count();
        let missing = total - installed;
        let healthy = checks.iter().all(|c| c.installed || !c.required);

        SystemReport {
            checks,
            total,
            installed,
            missing,
            healthy,
        }
    }
}

impl Default for DependencyInstaller {
    fn default() -> Self {
        Self::new()
    }
}

fn check_command(cmd: &str) -> Result<bool, String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err("empty command".into());
    }

    let output = Command::new(parts[0])
        .args(&parts[1..])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(output.status.success())
}

fn check_with_version(cmd: &str) -> (bool, Option<String>) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return (false, None);
    }

    match Command::new(parts[0]).args(&parts[1..]).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let version = stdout
                .lines()
                .next()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty());
            (true, version)
        }
        _ => (false, None),
    }
}

/// All 18+ dependencies from Architecture Framework Section 4.
fn all_dependencies() -> Vec<Dependency> {
    use DependencyCategory::*;

    vec![
        Dependency {
            name: "Xcode CLI Tools".into(),
            check_cmd: "xcode-select -p".into(),
            install_cmd: "xcode-select --install".into(),
            category: SystemPrerequisite,
            required: true,
        },
        Dependency {
            name: "Homebrew".into(),
            check_cmd: "brew --version".into(),
            install_cmd: "/bin/bash -c $(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)".into(),
            category: SystemPrerequisite,
            required: true,
        },
        Dependency {
            name: "Git".into(),
            check_cmd: "git --version".into(),
            install_cmd: "brew install git".into(),
            category: SystemPrerequisite,
            required: true,
        },
        Dependency {
            name: "curl".into(),
            check_cmd: "curl --version".into(),
            install_cmd: "brew install curl".into(),
            category: SystemPrerequisite,
            required: true,
        },
        Dependency {
            name: "jq".into(),
            check_cmd: "jq --version".into(),
            install_cmd: "brew install jq".into(),
            category: SystemPrerequisite,
            required: false,
        },
        Dependency {
            name: "ripgrep".into(),
            check_cmd: "rg --version".into(),
            install_cmd: "brew install ripgrep".into(),
            category: SystemPrerequisite,
            required: false,
        },
        Dependency {
            name: "Node.js".into(),
            check_cmd: "node --version".into(),
            install_cmd: "brew install node@20".into(),
            category: Runtime,
            required: true,
        },
        Dependency {
            name: "Python 3".into(),
            check_cmd: "python3 --version".into(),
            install_cmd: "brew install python@3.12".into(),
            category: Runtime,
            required: true,
        },
        Dependency {
            name: "Rust".into(),
            check_cmd: "rustc --version".into(),
            install_cmd: "curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y".into(),
            category: Runtime,
            required: true,
        },
        Dependency {
            name: "PostgreSQL Client".into(),
            check_cmd: "psql --version".into(),
            install_cmd: "brew install postgresql@16".into(),
            category: Database,
            required: false,
        },
        Dependency {
            name: "Redis Client".into(),
            check_cmd: "redis-cli --version".into(),
            install_cmd: "brew install redis".into(),
            category: Database,
            required: false,
        },
        Dependency {
            name: "Docker".into(),
            check_cmd: "docker --version".into(),
            install_cmd: "brew install --cask docker".into(),
            category: Database,
            required: false,
        },
        Dependency {
            name: "GitHub CLI".into(),
            check_cmd: "gh --version".into(),
            install_cmd: "brew install gh".into(),
            category: DeploymentCli,
            required: true,
        },
        Dependency {
            name: "Vercel CLI".into(),
            check_cmd: "vercel --version".into(),
            install_cmd: "npm install -g vercel".into(),
            category: DeploymentCli,
            required: false,
        },
        Dependency {
            name: "Supabase CLI".into(),
            check_cmd: "supabase --version".into(),
            install_cmd: "npm install -g supabase".into(),
            category: DeploymentCli,
            required: false,
        },
        Dependency {
            name: "Wrangler CLI".into(),
            check_cmd: "wrangler --version".into(),
            install_cmd: "npm install -g wrangler".into(),
            category: DeploymentCli,
            required: false,
        },
        Dependency {
            name: "Fly CLI".into(),
            check_cmd: "flyctl version".into(),
            install_cmd: "brew install flyctl".into(),
            category: DeploymentCli,
            required: false,
        },
        Dependency {
            name: "sentence-transformers".into(),
            check_cmd: "python3 -c import sentence_transformers".into(),
            install_cmd: "pip3 install sentence-transformers".into(),
            category: AiTool,
            required: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_dependencies_populated() {
        let deps = all_dependencies();
        assert!(deps.len() >= 18, "expected 18+ deps, got {}", deps.len());
    }

    #[test]
    fn test_dependency_has_valid_fields() {
        for dep in all_dependencies() {
            assert!(!dep.name.is_empty());
            assert!(!dep.check_cmd.is_empty());
            assert!(!dep.install_cmd.is_empty());
        }
    }

    #[test]
    fn test_installer_creation() {
        let installer = DependencyInstaller::new();
        assert!(!installer.dependencies.is_empty());
    }

    #[test]
    fn test_doctor_runs() {
        let installer = DependencyInstaller::new();
        let report = installer.doctor();
        assert_eq!(report.total, installer.dependencies.len());
        assert_eq!(report.installed + report.missing, report.total);
    }

    #[test]
    fn test_detect_missing_runs() {
        let installer = DependencyInstaller::new();
        let _missing = installer.detect_missing();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_check_command_exists() {
        // 'echo' should exist on every system
        assert!(check_command("echo hello").unwrap());
    }

    #[test]
    fn test_check_command_not_exists() {
        let result = check_command("__nonexistent_command_12345__ --version");
        assert!(result.is_err() || !result.unwrap());
    }

    #[test]
    fn test_system_report_serde() {
        let report = SystemReport {
            checks: vec![],
            total: 18,
            installed: 15,
            missing: 3,
            healthy: true,
        };
        let json = serde_json::to_string(&report).unwrap();
        let restored: SystemReport = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total, 18);
    }
}
