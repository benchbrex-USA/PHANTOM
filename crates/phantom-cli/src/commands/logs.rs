//! `phantom logs` — Stream audit log entries with optional agent filtering.
//! Launches a scrollable TUI log viewer.

use std::sync::Arc;
use tokio::sync::RwLock;

use phantom_core::message_bus::MessageBus;

use crate::dashboard::{self, DashboardState};

pub async fn run(agent: Option<String>) -> anyhow::Result<()> {
    // Create shared state
    let state = Arc::new(RwLock::new(DashboardState::default()));

    // Register on the message bus for live events
    let bus = Arc::new(MessageBus::new(64));
    let mailbox = bus.register_agent("logs-viewer-0").await.ok();

    // Seed with existing audit entries
    {
        let mut s = state.write().await;
        // In production, we'd load the audit log from disk/R2 here.
        // For now, seed with instructions.
        s.push_log("[system] Log viewer started.".into());
        s.push_log("[system] Showing log entries from the current session.".into());
        if let Some(ref a) = agent {
            s.push_log(format!("[system] Filtering by agent: {}", a));
        }
        s.push_log("[system] Press 'q' to quit, ↑/↓ to scroll, G to go to bottom.".into());
    }

    // Run the log viewer TUI
    dashboard::run_logs_view(state, agent, mailbox).await
}
