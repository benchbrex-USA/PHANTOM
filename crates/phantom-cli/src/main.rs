//! Phantom — The Autonomous AI Engineering Team
//!
//! Single binary entry point. License-gated. Master-key-controlled.
//! Core Law 1: No installation without a valid license key.

use clap::{Parser, Subcommand};

mod commands;
pub(crate) mod dashboard;

#[derive(Parser)]
#[command(
    name = "phantom",
    about = "Phantom — The Autonomous AI Engineering Team",
    version,
    long_about = "A terminal-native, license-gated, master-key-controlled, zero-footprint, \
                  self-provisioning, knowledge-driven autonomous AI engineering system."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Activate Phantom with a license key and bootstrap all dependencies
    Activate {
        /// License key (format: PH1-<payload>-<signature>)
        #[arg(long)]
        key: String,
    },

    /// Build a project from an Architecture Framework
    Build {
        /// Path to the Architecture Framework markdown file
        #[arg(long)]
        framework: Option<String>,

        /// Resume an interrupted build
        #[arg(long)]
        resume: bool,

        /// Build a single component
        #[arg(long)]
        component: Option<String>,

        /// Run tests only
        #[arg(long)]
        test_only: bool,

        /// Deploy existing build only
        #[arg(long)]
        deploy_only: bool,
    },

    /// Show live agent dashboard
    Status {
        /// Enable live updating
        #[arg(long)]
        live: bool,
    },

    /// Verify all dependencies are installed and healthy
    Doctor,

    /// List agent status
    Agents,

    /// Stream logs
    Logs {
        /// Filter logs by agent name
        #[arg(long)]
        agent: Option<String>,
    },

    /// Show infrastructure status
    Infra,

    /// Update Phantom to the latest release
    SelfUpdate {
        /// Force update even if already on latest version
        #[arg(long)]
        force: bool,
    },

    /// Query the Knowledge Brain directly
    Brain {
        #[command(subcommand)]
        action: BrainAction,
    },

    /// Cost estimation
    Cost {
        #[command(subcommand)]
        action: CostAction,
    },

    /// Master key operations (requires passphrase)
    Master {
        #[command(subcommand)]
        action: MasterAction,
    },
}

#[derive(Subcommand)]
enum BrainAction {
    /// Search the Knowledge Brain
    Search {
        /// Semantic query
        query: String,
    },
    /// Update a knowledge file
    Update {
        /// Path to the knowledge file
        #[arg(long)]
        file: String,
    },
}

#[derive(Subcommand)]
enum CostAction {
    /// Estimate costs for a project
    Estimate {
        /// Path to the Architecture Framework
        #[arg(long)]
        framework: String,
    },
}

#[derive(Subcommand)]
enum MasterAction {
    /// Initialize master key (first time setup)
    Init,
    /// Issue a new license
    Issue {
        #[arg(long)]
        email: String,
    },
    /// Revoke a license
    Revoke {
        #[arg(long)]
        key: String,
    },
    /// List all installations
    List,
    /// Remote-kill an installation
    Kill {
        /// Installation ID
        id: String,
    },
    /// Full system destruction (requires TOTP 2FA)
    Destroy,
    /// Rotate all keys
    Rotate,
    /// Export audit log
    Audit,
    /// Transfer ownership
    Transfer {
        #[arg(long)]
        to: String,
    },
    /// Emergency stop all agents
    Halt,
}

/// Check for a valid license before executing commands.
/// Returns Ok(()) if:
///   - The command is `activate` or `doctor` (always allowed)
///   - PHANTOM_LICENSE env var is set and contains a valid, non-expired license
fn check_license(command: &Commands) -> anyhow::Result<()> {
    // These commands are always allowed without a license
    if matches!(command, Commands::Activate { .. } | Commands::Doctor) {
        return Ok(());
    }

    let key_str = match std::env::var("PHANTOM_LICENSE") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            anyhow::bail!(
                "No license found. Set PHANTOM_LICENSE or run:\n  \
                 phantom activate --key <YOUR_KEY>"
            );
        }
    };

    // Decode and verify the license
    let license = phantom_crypto::license::LicenseKey::decode(&key_str)
        .map_err(|e| anyhow::anyhow!("Invalid license: {}", e))?;

    // Check expiration
    let now = chrono::Utc::now().timestamp();
    if now > license.payload.exp {
        anyhow::bail!("License expired. Please renew your license.");
    }

    // Verify machine fingerprint
    let ids = phantom_crypto::fingerprint::collect_machine_identifiers();
    let salt = b"phantom-license-fingerprint-salt-v1";
    let current_fp = ids.fingerprint(salt)?;
    let current_mid = hex::encode(current_fp);

    if current_mid != license.payload.mid {
        anyhow::bail!(
            "License is bound to a different machine.\n  \
             Re-activate on this machine with: phantom activate --key <YOUR_KEY>"
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("phantom=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    // License gate: verify before executing any gated command
    check_license(&cli.command)?;

    match cli.command {
        Commands::Activate { key } => commands::activate::run(&key).await,
        Commands::Build {
            framework,
            resume,
            component,
            test_only,
            deploy_only,
        } => commands::build::run(framework, resume, component, test_only, deploy_only).await,
        Commands::Status { live } => commands::status::run(live).await,
        Commands::Doctor => commands::doctor::run().await,
        Commands::Agents => commands::agents::run().await,
        Commands::Logs { agent } => commands::logs::run(agent).await,
        Commands::Infra => commands::infra::run().await,
        Commands::SelfUpdate { force } => commands::self_update::run(force).await,
        Commands::Brain { action } => match action {
            BrainAction::Search { query } => commands::brain::search(&query).await,
            BrainAction::Update { file } => commands::brain::update(&file).await,
        },
        Commands::Cost { action } => match action {
            CostAction::Estimate { framework } => commands::cost::estimate(&framework).await,
        },
        Commands::Master { action } => commands::master::run(action).await,
    }
}
