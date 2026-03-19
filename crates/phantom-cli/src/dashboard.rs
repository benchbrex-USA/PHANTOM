//! ratatui-based live terminal dashboard for agent monitoring.
//!
//! Layout (when active):
//!   ┌─────────────────────────────────────────────┐
//!   │  PHANTOM — Autonomous AI Engineering        │
//!   ├──────────────────┬──────────────────────────┤
//!   │  Pipeline Phases │  Agent Status            │
//!   │  [1] Ingest  OK  │  CTO:      running      │
//!   │  [2] Infra   --  │  Architect: idle         │
//!   │  ...             │  Backend:  running       │
//!   ├──────────────────┴──────────────────────────┤
//!   │  Live Log Stream                            │
//!   │  [12:34:56] backend: Generated API routes   │
//!   └─────────────────────────────────────────────┘

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table},
    Frame,
};

/// Dashboard state — snapshot of the system for rendering.
#[derive(Debug, Clone)]
pub struct DashboardState {
    /// Current pipeline phase name
    pub current_phase: String,
    /// Phase completion (0.0 - 1.0)
    pub phase_progress: f64,
    /// Agent statuses: (name, status, task)
    pub agents: Vec<AgentDisplay>,
    /// Recent log lines
    pub log_lines: Vec<String>,
    /// System uptime
    pub uptime_secs: u64,
    /// Connected mesh peers
    pub peer_count: usize,
    /// Total tokens used
    pub tokens_used: u64,
    /// Estimated cost so far
    pub cost_usd: f64,
}

#[derive(Debug, Clone)]
pub struct AgentDisplay {
    pub name: String,
    pub status: String,
    pub current_task: Option<String>,
    pub tokens_used: u64,
}

impl Default for DashboardState {
    fn default() -> Self {
        let agents = phantom_ai::ALL_ROLES
            .iter()
            .map(|role| AgentDisplay {
                name: role.display_name().to_string(),
                status: "idle".to_string(),
                current_task: None,
                tokens_used: 0,
            })
            .collect();

        Self {
            current_phase: "idle".to_string(),
            phase_progress: 0.0,
            agents,
            log_lines: Vec::new(),
            uptime_secs: 0,
            peer_count: 0,
            tokens_used: 0,
            cost_usd: 0.0,
        }
    }
}

/// Render the dashboard to a ratatui frame.
pub fn render(frame: &mut Frame, state: &DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // Progress bar
            Constraint::Min(10),   // Main content
            Constraint::Length(8), // Logs
        ])
        .split(frame.area());

    render_header(frame, chunks[0], state);
    render_progress(frame, chunks[1], state);
    render_agents(frame, chunks[2], state);
    render_logs(frame, chunks[3], state);
}

fn render_header(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let uptime = format_duration(state.uptime_secs);
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " PHANTOM ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Phase: {} ", state.current_phase),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" | "),
        Span::raw(format!("Peers: {} ", state.peer_count)),
        Span::raw(" | "),
        Span::raw(format!("Tokens: {} ", format_tokens(state.tokens_used))),
        Span::raw(" | "),
        Span::raw(format!("Cost: ${:.2} ", state.cost_usd)),
        Span::raw(" | "),
        Span::styled(format!("Up: {}", uptime), Style::default().fg(Color::Cyan)),
    ]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, area);
}

fn render_progress(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Pipeline Progress"))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(state.phase_progress.clamp(0.0, 1.0))
        .label(format!(
            "{:.0}%",
            state.phase_progress * 100.0
        ));

    frame.render_widget(progress, area);
}

fn render_agents(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let rows: Vec<Row> = state
        .agents
        .iter()
        .map(|agent| {
            let status_style = match agent.status.as_str() {
                "running" => Style::default().fg(Color::Green),
                "error" => Style::default().fg(Color::Red),
                "idle" => Style::default().fg(Color::DarkGray),
                _ => Style::default(),
            };

            Row::new(vec![
                Cell::from(agent.name.clone()),
                Cell::from(agent.status.clone()).style(status_style),
                Cell::from(
                    agent
                        .current_task
                        .clone()
                        .unwrap_or_else(|| "-".to_string()),
                ),
                Cell::from(format_tokens(agent.tokens_used)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Min(30),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec!["Agent", "Status", "Current Task", "Tokens"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::ALL).title("Agents"));

    frame.render_widget(table, area);
}

fn render_logs(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let log_text: Vec<Line> = state
        .log_lines
        .iter()
        .rev()
        .take(area.height as usize - 2)
        .rev()
        .map(|line| Line::from(line.as_str()))
        .collect();

    let logs = Paragraph::new(log_text)
        .block(Block::default().borders(Borders::ALL).title("Logs"));

    frame.render_widget(logs, area);
}

fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    if hours > 0 {
        format!("{}h{}m{}s", hours, mins, s)
    } else if mins > 0 {
        format!("{}m{}s", mins, s)
    } else {
        format!("{}s", s)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_state_default() {
        let state = DashboardState::default();
        assert_eq!(state.agents.len(), 8);
        assert_eq!(state.current_phase, "idle");
        assert_eq!(state.phase_progress, 0.0);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(125), "2m5s");
        assert_eq!(format_duration(3661), "1h1m1s");
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1_500), "1.5K");
        assert_eq!(format_tokens(2_500_000), "2.5M");
    }
}
