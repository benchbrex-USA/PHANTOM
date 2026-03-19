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
    println!("\x1b[1mPhantom Status\x1b[0m\n");

    println!("\x1b[1;34mSystem\x1b[0m");
    println!("  Status:        \x1b[33midle\x1b[0m");
    println!("  License:       not activated");
    println!("  Master key:    not initialized");
    println!();

    println!("\x1b[1;34mBuild Pipeline\x1b[0m");
    for phase in BuildPhase::all() {
        println!("  {:<28} \x1b[90m--\x1b[0m", phase.display_name());
    }
    println!();

    println!("\x1b[1;34mAgents\x1b[0m");
    for role in phantom_ai::ALL_ROLES {
        println!("  {:<20} \x1b[90midle\x1b[0m", role.display_name());
    }
    println!();

    println!("\x1b[1;34mInfrastructure\x1b[0m");
    println!("  Providers:     0/14 authenticated");
    println!("  Resources:     0 provisioned");
    println!("  Mesh peers:    0 connected");
    println!();

    println!("\x1b[1;34mStorage\x1b[0m");
    println!("  Vault entries: 0");
    println!("  R2 blobs:      0");
    println!("  State keys:    0");
    println!();

    println!("Run `phantom status --live` for the real-time TUI dashboard.");

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
