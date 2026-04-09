//! `phantom status` — Show system status overview.
//! `phantom status --live` — Launch the live TUI dashboard.

use std::sync::Arc;
use tokio::sync::RwLock;

use phantom_core::message_bus::MessageBus;
use phantom_core::pipeline::BuildPhase;

use crate::dashboard::{self, DashboardState};

pub async fn run(live: bool) -> anyhow::Result<()> {
    if live {
        return run_live().await;
    }

    // Static status output
    println!();
    println!(
        "  \x1b[1mphantom\x1b[0m \x1b[2m·\x1b[0m \x1b[2mautonomous AI engineering system\x1b[0m"
    );
    println!("  \x1b[2m{}\x1b[0m", "─".repeat(44));
    println!();

    // System
    println!("  \x1b[1mSystem\x1b[0m");
    println!(
        "  \x1b[2m│\x1b[0m  Status         \x1b[33m●\x1b[0m \x1b[33midle\x1b[0m"
    );
    println!("  \x1b[2m│\x1b[0m  License        \x1b[2m○\x1b[0m \x1b[2mnot activated\x1b[0m");
    println!("  \x1b[2m│\x1b[0m  Master key     \x1b[2m○\x1b[0m \x1b[2mnot initialized\x1b[0m");
    println!();

    // Build Pipeline
    println!("  \x1b[1mPipeline\x1b[0m");
    for phase in BuildPhase::all() {
        println!(
            "  \x1b[2m│\x1b[0m  {:<26} \x1b[2m–\x1b[0m",
            phase.display_name()
        );
    }
    println!();

    // Agents
    println!("  \x1b[1mAgents\x1b[0m");
    for role in phantom_ai::ALL_ROLES {
        println!(
            "  \x1b[2m│\x1b[0m  {:<18} \x1b[2m○\x1b[0m \x1b[2midle\x1b[0m",
            role.display_name()
        );
    }
    println!();

    // Infrastructure
    println!("  \x1b[1mInfrastructure\x1b[0m");
    println!(
        "  \x1b[2m│\x1b[0m  Providers       \x1b[2m0/14 authenticated\x1b[0m"
    );
    println!("  \x1b[2m│\x1b[0m  Resources       \x1b[2m0 provisioned\x1b[0m");
    println!("  \x1b[2m│\x1b[0m  Mesh peers      \x1b[2m0 connected\x1b[0m");
    println!();

    // Storage
    println!("  \x1b[1mStorage\x1b[0m");
    println!("  \x1b[2m│\x1b[0m  Vault entries   \x1b[2m0\x1b[0m");
    println!("  \x1b[2m│\x1b[0m  R2 blobs        \x1b[2m0\x1b[0m");
    println!("  \x1b[2m│\x1b[0m  State keys      \x1b[2m0\x1b[0m");
    println!();

    println!(
        "  \x1b[2mRun \x1b[0mphantom status --live\x1b[2m for the real-time dashboard.\x1b[0m"
    );
    println!();

    Ok(())
}

/// Launch the live TUI dashboard.
async fn run_live() -> anyhow::Result<()> {
    // Create shared state
    let state = Arc::new(RwLock::new(DashboardState::default()));

    // Create a message bus and register a dashboard listener
    let bus = Arc::new(MessageBus::new(64));
    let mailbox = bus.register_agent("dashboard-0").await.ok();

    // Seed with current system state
    {
        let mut s = state.write().await;
        s.push_log("[system] Dashboard started. Waiting for pipeline events...".into());
        s.push_log("[system] Press 'q' to quit, ↑/↓ to scroll logs.".into());
    }

    // Run the live dashboard (blocks until user quits)
    dashboard::run_live_dashboard(state, mailbox).await
}
