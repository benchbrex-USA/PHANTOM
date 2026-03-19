//! `phantom logs` — Stream audit log entries.

pub async fn run(agent: Option<String>) -> anyhow::Result<()> {
    println!("\x1b[1mPhantom Logs\x1b[0m\n");

    match &agent {
        Some(name) => {
            println!("No log entries for agent: {}", name);
        }
        None => {
            println!("No log entries yet. Logs will appear here during a build.\n");
            println!("Log format:");
            println!("  \x1b[90m[timestamp]\x1b[0m \x1b[34m[agent]\x1b[0m \x1b[33m[action]\x1b[0m detail");
            println!();
            println!("Filter by agent:");
            println!("  phantom logs --agent cto");
            println!("  phantom logs --agent backend");
            println!("  phantom logs --agent security");
        }
    }

    Ok(())
}
