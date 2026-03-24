//! `phantom activate --key <KEY>` — License activation + full system bootstrap.

use phantom_crypto::fingerprint::collect_machine_identifiers;
use phantom_crypto::license::LicenseKey;
use phantom_infra::doctor::{Doctor, DoctorStatus};

pub async fn run(key: &str) -> anyhow::Result<()> {
    println!("\x1b[1mPhantom Activation\x1b[0m\n");

    // ── Step 1: Decode license key ──────────────────────────────────────────
    println!("\x1b[1;34m▶ Step 1: License Validation\x1b[0m");
    let license = LicenseKey::decode(key)?;
    println!(
        "  \x1b[32m\u{2713}\x1b[0m Decoded: tier={}, v={}, capabilities={:?}",
        license.payload.tier, license.payload.v, license.payload.cap
    );

    // Check expiration
    let now = chrono::Utc::now().timestamp();
    if now > license.payload.exp {
        let expiry_str = chrono::DateTime::from_timestamp(license.payload.exp, 0)
            .map(|d: chrono::DateTime<chrono::Utc>| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".into());
        anyhow::bail!("License expired on {}", expiry_str);
    }
    let days_remaining = (license.payload.exp - now) / 86400;
    println!(
        "  \x1b[32m\u{2713}\x1b[0m Expiry: {} days remaining",
        days_remaining
    );

    // ── Step 2: Machine fingerprint verification ────────────────────────────
    println!("\n\x1b[1;34m▶ Step 2: Machine Identity\x1b[0m");
    let ids = collect_machine_identifiers();
    let salt = b"phantom-license-fingerprint-salt-v1";
    let current_fp = ids.fingerprint(salt)?;
    let current_mid = hex::encode(current_fp);

    let mid_short = &current_mid[..16];
    println!("  Machine fingerprint: {}...", mid_short);

    if current_mid == license.payload.mid {
        println!("  \x1b[32m\u{2713}\x1b[0m Machine fingerprint matches license");
    } else {
        let license_mid_short = if license.payload.mid.len() > 16 {
            &license.payload.mid[..16]
        } else {
            &license.payload.mid
        };
        anyhow::bail!(
            "Machine fingerprint mismatch.\n  License bound to: {}...\n  Current machine:  {}...\n  This license was issued for a different machine.",
            license_mid_short,
            mid_short
        );
    }

    // ── Step 3: Dependency check ────────────────────────────────────────────
    println!("\n\x1b[1;34m▶ Step 3: Dependencies\x1b[0m");
    let doctor = Doctor::new();
    let report = doctor.run();

    let mut current_category = String::new();
    for result in &report.results {
        if result.category != current_category {
            if !current_category.is_empty() {
                println!();
            }
            current_category = result.category.clone();
        }

        let (icon, color) = match result.status {
            DoctorStatus::Ok => ("\u{2713}", "\x1b[32m"),
            DoctorStatus::Missing => ("\u{2717}", "\x1b[31m"),
            DoctorStatus::Warning => ("!", "\x1b[33m"),
            DoctorStatus::Error => ("\u{2717}", "\x1b[31m"),
        };

        let info = result
            .version
            .as_deref()
            .or(result.message.as_deref())
            .unwrap_or("");

        println!("  {color}{icon}\x1b[0m {:<28} {}", result.name, info);
    }

    println!(
        "\n  Summary: {} OK, {} missing, {} warnings",
        report.ok_count, report.missing_count, report.warning_count
    );

    if !report.healthy {
        println!("\n  \x1b[33m!\x1b[0m Some required dependencies are missing.");
        println!("  Phantom will attempt to install them during provisioning.");
    }

    // ── Step 4: Provider authentication ─────────────────────────────────────
    println!("\n\x1b[1;34m▶ Step 4: Provider Authentication\x1b[0m");
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
        println!("  {} {:<20} {}", icon, status.provider.display_name(), msg);
    }

    // ── Step 5: Infrastructure readiness ────────────────────────────────────
    println!("\n\x1b[1;34m▶ Step 5: Infrastructure Readiness\x1b[0m");
    let installer = phantom_infra::DependencyInstaller::new();
    let summary = installer.summary();
    println!(
        "  Dependencies: {}/{} installed",
        summary.installed, summary.total
    );
    println!("  Providers: {}/{} authenticated", authed, total);
    println!("  Knowledge Brain: \x1b[90mready after `phantom master init`\x1b[0m");
    println!("  P2P Mesh: \x1b[90mready after provisioning\x1b[0m");

    // ── Summary ─────────────────────────────────────────────────────────────
    println!("\n\x1b[32m\u{2713} Phantom activated successfully.\x1b[0m");
    println!();
    println!("Next steps:");
    println!("  1. \x1b[1mphantom master init\x1b[0m     — Initialize master key + TOTP 2FA");
    println!("  2. \x1b[1mphantom doctor\x1b[0m          — Verify all dependencies");
    println!("  3. \x1b[1mphantom build --framework <file>\x1b[0m — Build from architecture spec");

    Ok(())
}
