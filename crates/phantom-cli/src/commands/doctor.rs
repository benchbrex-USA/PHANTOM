//! `phantom doctor` — Verify all dependencies and provider CLIs.

use phantom_infra::doctor::{Doctor, DoctorStatus};

pub async fn run() -> anyhow::Result<()> {
    println!("\x1b[1mPhantom Doctor\x1b[0m");
    println!("Checking system dependencies and provider CLIs...\n");

    let doctor = Doctor::new();
    let report = doctor.run();

    // Group by category
    let mut current_category = String::new();
    for result in &report.results {
        if result.category != current_category {
            if !current_category.is_empty() {
                println!();
            }
            println!("\x1b[1;34m{}\x1b[0m", result.category);
            current_category = result.category.clone();
        }

        let (icon, color) = match result.status {
            DoctorStatus::Ok => ("\u{2713}", "\x1b[32m"),       // green check
            DoctorStatus::Missing => ("\u{2717}", "\x1b[31m"),  // red X
            DoctorStatus::Warning => ("!", "\x1b[33m"),         // yellow !
            DoctorStatus::Error => ("\u{2717}", "\x1b[31m"),    // red X
        };

        let version_or_msg = result
            .version
            .as_deref()
            .or(result.message.as_deref())
            .unwrap_or("");

        println!("  {color}{icon}\x1b[0m {:<30} {}", result.name, version_or_msg);
    }

    println!();
    println!(
        "\x1b[1mSummary:\x1b[0m {} OK, {} missing, {} warnings, {} errors",
        report.ok_count, report.missing_count, report.warning_count, report.error_count
    );

    if report.healthy {
        println!("\n\x1b[32mSystem is healthy.\x1b[0m");
    } else {
        println!("\n\x1b[31mSome required dependencies are missing.\x1b[0m");
        println!("Run `phantom activate --key <KEY>` to install missing dependencies.");
    }

    Ok(())
}
