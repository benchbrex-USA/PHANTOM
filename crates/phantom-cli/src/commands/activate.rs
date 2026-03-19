//! `phantom activate --key <KEY>` — License activation + full system bootstrap.

pub async fn run(key: &str) -> anyhow::Result<()> {
    println!("\x1b[1mPhantom Activation\x1b[0m\n");

    // Step 1: Decode and verify license key
    println!("Step 1: Validating license key...");
    let license = phantom_crypto::license::LicenseKey::decode(key)?;
    println!(
        "  \x1b[32m\u{2713}\x1b[0m License decoded: tier={}, capabilities={:?}",
        license.payload.tier, license.payload.cap
    );

    // Step 2: Show machine ID from license
    println!("\nStep 2: Machine identity...");
    let mid_short = if license.payload.mid.len() > 16 {
        &license.payload.mid[..16]
    } else {
        &license.payload.mid
    };
    println!("  License bound to machine: {}...", mid_short);

    // Step 3: Check dependencies
    println!("\nStep 3: Checking dependencies...");
    let installer = phantom_infra::DependencyInstaller::new();
    let summary = installer.summary();
    println!(
        "  {}/{} dependencies installed ({} required missing, {} optional missing)",
        summary.installed, summary.total, summary.missing_required, summary.missing_optional
    );

    if summary.ready {
        println!("  \x1b[32m\u{2713}\x1b[0m All required dependencies satisfied");
    } else {
        println!("  \x1b[33m!\x1b[0m Some required dependencies are missing");
        println!("  Run `phantom doctor` for details");
    }

    // Step 4: Check provider authentication
    println!("\nStep 4: Checking provider authentication...");
    let mut accounts = phantom_infra::AccountManager::new();
    let statuses = accounts.check_all();
    let authed = statuses.iter().filter(|s| s.authenticated).count();
    let total = statuses.len();
    println!("  {}/{} providers authenticated", authed, total);

    for status in statuses {
        let icon = if status.authenticated {
            "\x1b[32m\u{2713}\x1b[0m"
        } else {
            "\x1b[90m-\x1b[0m"
        };
        let msg = status.message.as_deref().unwrap_or("");
        println!(
            "  {} {:<20} {}",
            icon,
            status.provider.display_name(),
            msg
        );
    }

    println!("\n\x1b[32mPhantom activated successfully.\x1b[0m");

    Ok(())
}
