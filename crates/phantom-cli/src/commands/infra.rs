//! `phantom infra` — Show infrastructure status across all providers.

use phantom_infra::dependencies::DependencyInstaller;
use phantom_infra::providers::ALL_PROVIDERS;

pub async fn run() -> anyhow::Result<()> {
    println!("\x1b[1mPhantom Infrastructure\x1b[0m\n");

    // Provider overview
    println!("\x1b[1;34mProviders (14)\x1b[0m");
    println!(
        "  {:<20} {:<8} {:<40} {:<10}",
        "PROVIDER", "PRI", "FREE TIER", "CLI"
    );
    println!("  {}", "-".repeat(78));

    for provider in ALL_PROVIDERS {
        let cli = provider.cli_tool().unwrap_or("-");
        let cli_installed = if let Some(tool) = provider.cli_tool() {
            std::process::Command::new("which")
                .arg(tool)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        } else {
            false
        };

        let cli_status = if provider.cli_tool().is_none() {
            "\x1b[90mAPI\x1b[0m"
        } else if cli_installed {
            "\x1b[32m\u{2713}\x1b[0m"
        } else {
            "\x1b[31m\u{2717}\x1b[0m"
        };

        println!(
            "  {:<20} {:<8} {:<40} {} {}",
            provider.display_name(),
            provider.priority(),
            provider.free_tier_description(),
            cli_status,
            cli,
        );
    }

    println!();

    // Resource types
    println!("\x1b[1;34mResource Capabilities\x1b[0m");
    let resource_types = [
        ("Compute", phantom_infra::providers::ResourceType::Compute),
        ("Storage", phantom_infra::providers::ResourceType::Storage),
        ("Database", phantom_infra::providers::ResourceType::Database),
        ("Cache", phantom_infra::providers::ResourceType::Cache),
        ("Edge", phantom_infra::providers::ResourceType::Edge),
        ("CI/CD", phantom_infra::providers::ResourceType::Ci),
    ];

    for (name, rt) in &resource_types {
        let providers = phantom_infra::providers::providers_for_resource(*rt);
        let names: Vec<&str> = providers.iter().map(|p| p.display_name()).collect();
        println!("  {:<12} {}", name, names.join(", "));
    }

    println!();

    // Dependency summary
    println!("\x1b[1;34mDependencies\x1b[0m");
    let installer = DependencyInstaller::new();
    let summary = installer.summary();
    println!(
        "  {}/{} installed ({} required missing)",
        summary.installed, summary.total, summary.missing_required
    );

    Ok(())
}
