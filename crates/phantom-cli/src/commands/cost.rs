//! `phantom cost estimate` — Estimate API and infrastructure costs.

use phantom_ai::agents::{AgentConfig, ALL_ROLES};

pub async fn estimate(framework: &str) -> anyhow::Result<()> {
    println!("\x1b[1mPhantom Cost Estimate\x1b[0m\n");

    if !std::path::Path::new(framework).exists() {
        anyhow::bail!("Framework file not found: {}", framework);
    }

    // Read framework to estimate complexity
    let content = std::fs::read_to_string(framework)?;
    let line_count = content.lines().count();
    let word_count = content.split_whitespace().count();

    println!("Framework: {}", framework);
    println!("  Lines: {}", line_count);
    println!("  Words: {}", word_count);
    println!();

    // Estimate tokens per agent based on framework size
    let complexity_factor = (word_count as f64 / 1000.0).clamp(1.0, 10.0);

    println!("\x1b[1;34mEstimated API Usage\x1b[0m");
    println!(
        "  {:<20} {:<15} {:<15} {:<12} {:<10}",
        "AGENT", "INPUT TOK", "OUTPUT TOK", "MODEL", "EST COST"
    );
    println!("  {}", "-".repeat(72));

    let mut total_cost = 0.0f64;

    for role in ALL_ROLES {
        let config = AgentConfig::for_role(*role);

        // Estimate based on role budget and complexity
        let estimated_input = (config.task_token_budget as f64 * 0.6 * complexity_factor) as u64;
        let estimated_output = (config.task_token_budget as f64 * 0.2 * complexity_factor) as u64;

        // Model pricing (per million tokens)
        let (input_price, output_price) = match config.model.as_str() {
            "claude-opus-4-6" => (15.0, 75.0),
            "claude-sonnet-4-6" => (3.0, 15.0),
            "claude-haiku-4-5-20251001" => (0.25, 1.25),
            _ => (3.0, 15.0),
        };

        let cost = (estimated_input as f64 * input_price / 1_000_000.0)
            + (estimated_output as f64 * output_price / 1_000_000.0);
        total_cost += cost;

        let model_short = config
            .model
            .replace("claude-", "")
            .chars()
            .take(12)
            .collect::<String>();

        println!(
            "  {:<20} {:<15} {:<15} {:<12} ${:<.2}",
            role.display_name(),
            format_tokens(estimated_input),
            format_tokens(estimated_output),
            model_short,
            cost,
        );
    }

    println!("  {}", "-".repeat(72));
    println!(
        "  {:<20} {:<15} {:<15} {:<12} \x1b[1m${:.2}\x1b[0m",
        "TOTAL", "", "", "", total_cost
    );

    println!();
    println!("\x1b[1;34mInfrastructure\x1b[0m");
    println!("  All providers use free tiers — \x1b[32m$0.00\x1b[0m");
    println!();
    println!(
        "\x1b[1mEstimated total cost: ${:.2}\x1b[0m (API usage only)",
        total_cost
    );
    println!(
        "\x1b[90mNote: Estimates based on framework complexity. Actual usage may vary.\x1b[0m"
    );

    Ok(())
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}
