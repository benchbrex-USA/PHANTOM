//! `phantom activate --key <KEY>` — License activation + full system bootstrap.

use phantom_crypto::fingerprint::collect_machine_identifiers;
use phantom_crypto::license::LicenseKey;
use phantom_infra::doctor::{Doctor, DoctorStatus};

pub async fn run(key: &str) -> anyhow::Result<()> {
    println!();
    println!(
        "  \x1b[1mphantom activate\x1b[0m \x1b[2m·\x1b[0m \x1b[2mlicense activation\x1b[0m"
    );
    println!("  \x1b[2m{}\x1b[0m", "─".repeat(44));
    println!();

    // ── Step 1: Decode license key ──────────────────────────────────────────
    println!("  \x1b[1mStep 1\x1b[0m \x1b[2m·\x1b[0m License Validation");
    let license = LicenseKey::decode(key)?;
    println!(
        "  \x1b[2m│\x1b[0m  \x1b[32m✓\x1b[0m Decoded: tier={}, v={}, capabilities={:?}",
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
        "  \x1b[2m│\x1b[0m  \x1b[32m✓\x1b[0m Expiry: \x1b[32m{} days\x1b[0m remaining",
        days_remaining
    );
    println!();

    // ── Step 2: Machine fingerprint verification ────────────────────────────
    println!("  \x1b[1mStep 2\x1b[0m \x1b[2m·\x1b[0m Machine Identity");
    let ids = collect_machine_identifiers();
    let salt = b"phantom-license-fingerprint-salt-v1";
    let current_fp = ids.fingerprint(salt)?;
    let current_mid = hex::encode(current_fp);

    let mid_short = &current_mid[..16];
    println!(
        "  \x1b[2m│\x1b[0m  Fingerprint: \x1b[2m{}…\x1b[0m",
        mid_short
    );

    if current_mid == license.payload.mid {
        println!("  \x1b[2m│\x1b[0m  \x1b[32m✓\x1b[0m Machine fingerprint matches license");
    } else {
        let license_mid_short = if license.payload.mid.len() > 16 {
            &license.payload.mid[..16]
        } else {
            &license.payload.mid
        };
        anyhow::bail!(
            "Machine fingerprint mismatch.\n  License bound to: {}…\n  Current machine:  {}…\n  This license was issued for a different machine.",
            license_mid_short,
            mid_short
        );
    }
    println!();

    // ── Step 3: Dependency check ────────────────────────────────────────────
    println!("  \x1b[1mStep 3\x1b[0m \x1b[2m·\x1b[0m Dependencies");
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
            DoctorStatus::Ok => ("✓", "\x1b[32m"),
            DoctorStatus::Missing => ("✗", "\x1b[31m"),
            DoctorStatus::Warning => ("!", "\x1b[33m"),
            DoctorStatus::Error => ("✗", "\x1b[31m"),
        };

        let info = result
            .version
            .as_deref()
            .or(result.message.as_deref())
            .unwrap_or("");

        println!(
            "  \x1b[2m│\x1b[0m  {color}{icon}\x1b[0m {:<26} \x1b[2m{}\x1b[0m",
            result.name, info
        );
    }

    println!();
    println!(
        "  \x1b[2m│\x1b[0m  {} passed, {} missing, {} warnings",
        report.ok_count, report.missing_count, report.warning_count
    );

    if !report.healthy {
        println!(
            "  \x1b[2m│\x1b[0m  \x1b[33m!\x1b[0m Missing dependencies will be installed during provisioning."
        );
    }
    println!();

    // ── Step 4: Provider authentication ─────────────────────────────────────
    println!("  \x1b[1mStep 4\x1b[0m \x1b[2m·\x1b[0m Provider Authentication");
    let mut accounts = phantom_infra::AccountManager::new();
    let statuses = accounts.check_all();
    let authed = statuses.iter().filter(|s| s.authenticated).count();
    let total = statuses.len();
    println!(
        "  \x1b[2m│\x1b[0m  {}/{} providers authenticated",
        authed, total
    );

    for status in statuses {
        let icon = if status.authenticated {
            "\x1b[32m✓\x1b[0m"
        } else {
            "\x1b[2m○\x1b[0m"
        };
        let msg = status.message.as_deref().unwrap_or("");
        println!(
            "  \x1b[2m│\x1b[0m  {} {:<18} \x1b[2m{}\x1b[0m",
            icon,
            status.provider.display_name(),
            msg
        );
    }
    println!();

    // ── Step 5: Infrastructure readiness ────────────────────────────────────
    println!("  \x1b[1mStep 5\x1b[0m \x1b[2m·\x1b[0m Infrastructure Readiness");
    let installer = phantom_infra::DependencyInstaller::new();
    let summary = installer.summary();
    println!(
        "  \x1b[2m│\x1b[0m  Dependencies:    {}/{} installed",
        summary.installed, summary.total
    );
    println!(
        "  \x1b[2m│\x1b[0m  Providers:       {}/{} authenticated",
        authed, total
    );
    println!(
        "  \x1b[2m│\x1b[0m  Knowledge Brain: \x1b[2mready after \x1b[0mphantom master init\x1b[0m"
    );
    println!(
        "  \x1b[2m│\x1b[0m  P2P Mesh:        \x1b[2mready after provisioning\x1b[0m"
    );

    // ── Summary ─────────────────────────────────────────────────────────────
    println!();
    println!("  \x1b[2m{}\x1b[0m", "─".repeat(44));
    println!("  \x1b[32m●\x1b[0m Phantom activated successfully.");
    println!();
    println!("  \x1b[1mNext steps\x1b[0m");
    println!(
        "  \x1b[2m│\x1b[0m  \x1b[1m1.\x1b[0m phantom master init         \x1b[2m· initialize master key + TOTP 2FA\x1b[0m"
    );
    println!(
        "  \x1b[2m│\x1b[0m  \x1b[1m2.\x1b[0m phantom doctor              \x1b[2m· verify all dependencies\x1b[0m"
    );
    println!(
        "  \x1b[2m│\x1b[0m  \x1b[1m3.\x1b[0m phantom build --framework   \x1b[2m· build from architecture spec\x1b[0m"
    );
    println!();

    Ok(())
}
