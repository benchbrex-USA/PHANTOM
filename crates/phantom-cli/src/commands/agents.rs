//! `phantom agents` — List the 8-agent team and their configuration.

use phantom_ai::agents::{AgentConfig, ALL_ROLES};

pub async fn run() -> anyhow::Result<()> {
    println!("\x1b[1mPhantom Agent Team\x1b[0m\n");

    println!(
        "  {:<20} {:<25} {:<8} {:<10} {:<12}",
        "ROLE", "MODEL", "TEMP", "MAX TOK", "BUDGET"
    );
    println!("  {}", "-".repeat(75));

    for role in ALL_ROLES {
        let config = AgentConfig::for_role(*role);
        let model_short = config.model.replace("claude-", "");

        let delegates = if role.can_delegate() { " [D]" } else { "" };
        let code_exec = if role.needs_code_exec() { " [X]" } else { "" };

        println!(
            "  {:<20} {:<25} {:<8.1} {:<10} {:<12}{}{}",
            role.display_name(),
            model_short,
            config.temperature,
            config.max_tokens,
            format_tokens(config.task_token_budget),
            delegates,
            code_exec,
        );
    }

    println!();
    println!("  [D] = can delegate tasks   [X] = has code execution");
    println!();

    // Knowledge scope summary
    println!("\x1b[1mKnowledge Scope\x1b[0m\n");
    for role in ALL_ROLES {
        let scope = role.knowledge_scope();
        println!(
            "  {:<20} {} files: {}",
            role.display_name(),
            scope.len(),
            scope.join(", ")
        );
    }

    Ok(())
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{}M", tokens / 1_000_000)
    } else if tokens >= 1_000 {
        format!("{}K", tokens / 1_000)
    } else {
        tokens.to_string()
    }
}
