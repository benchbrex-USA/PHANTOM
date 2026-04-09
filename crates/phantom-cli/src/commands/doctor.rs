//! `phantom doctor` — Verify all dependencies and provider CLIs.

use phantom_infra::doctor::{Doctor, DoctorStatus};

pub async fn run() -> anyhow::Result<()> {
    println!();
    println!(
        "  \x1b[1mphantom doctor\x1b[0m \x1b[2m·\x1b[0m \x1b[2msystem health check\x1b[0m"
    );
    println!("  \x1b[2m{}\x1b[0m", "─".repeat(44));
    println!();

    let doctor = Doctor::new();
    let report = doctor.run();

    // Group by category
    let mut current_category = String::new();
    for result in &report.results {
        if result.category != current_category {
            if !current_category.is_empty() {
                println!();
            }
            println!("  \x1b[1m{}\x1b[0m", result.category);
            current_category = result.category.clone();
        }

        let (icon, color) = match result.status {
            DoctorStatus::Ok => ("✓", "\x1b[32m"),
            DoctorStatus::Missing => ("✗", "\x1b[31m"),
            DoctorStatus::Warning => ("!", "\x1b[33m"),
            DoctorStatus::Error => ("✗", "\x1b[31m"),
        };

        let version_or_msg = result
            .version
            .as_deref()
            .or(result.message.as_deref())
            .unwrap_or("");

        println!(
            "  \x1b[2m│\x1b[0m  {color}{icon}\x1b[0m {:<28} \x1b[2m{}\x1b[0m",
            result.name, version_or_msg
        );
    }

    // Summary
    println!();
    println!("  \x1b[2m{}\x1b[0m", "─".repeat(44));

    let summary_parts = [
        (report.ok_count, "\x1b[32m", "passed"),
        (report.missing_count, "\x1b[31m", "missing"),
        (report.warning_count, "\x1b[33m", "warnings"),
        (report.error_count, "\x1b[31m", "errors"),
    ];

    let summary: Vec<String> = summary_parts
        .iter()
        .filter(|(count, _, _)| *count > 0)
        .map(|(count, color, label)| format!("{color}{} {}\x1b[0m", count, label))
        .collect();

    println!("  {}", summary.join(" \x1b[2m·\x1b[0m "));

    if report.healthy {
        println!();
        println!("  \x1b[32m●\x1b[0m System is healthy.");
    } else {
        println!();
        println!("  \x1b[31m●\x1b[0m Some required dependencies are missing.");
        println!(
            "  \x1b[2m  Run \x1b[0mphantom activate --key <KEY>\x1b[2m to proceed.\x1b[0m"
        );
    }
    println!();

    Ok(())
}
