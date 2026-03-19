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
//!
//! Modes:
//!   • `phantom status --live`  — Full dashboard with all panels
//!   • `phantom logs`           — Log-only view (scrollable)
//!   • `phantom brain search`   — Interactive Knowledge Brain search

use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};
use tokio::sync::RwLock;

use phantom_core::agent_manager::{AgentManager, AgentState};
use phantom_core::audit::AuditLog;
use phantom_core::message_bus::MessageKind;
use phantom_core::pipeline::{BuildPhase, BuildPipeline, PhaseState};

// ── Dashboard State ─────────────────────────────────────────────────────────

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
    /// Per-phase status (for the phase panel)
    pub phases: Vec<PhaseDisplay>,
    /// Tasks completed / total
    pub tasks_completed: usize,
    pub tasks_total: usize,
    /// Log scroll offset (0 = bottom)
    pub log_scroll: usize,
    /// Max log lines to keep
    pub max_log_lines: usize,
    /// Whether the pipeline is halted
    pub halted: bool,
}

/// Per-agent display data.
#[derive(Debug, Clone)]
pub struct AgentDisplay {
    pub name: String,
    pub status: String,
    pub current_task: Option<String>,
    pub tokens_used: u64,
    pub tasks_completed: u32,
    pub tasks_failed: u32,
}

/// Per-phase display data.
#[derive(Debug, Clone)]
pub struct PhaseDisplay {
    pub name: String,
    pub status: String,
    pub tasks_done: usize,
    pub tasks_total: usize,
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
                tasks_completed: 0,
                tasks_failed: 0,
            })
            .collect();

        let phases = BuildPhase::all()
            .iter()
            .map(|p| PhaseDisplay {
                name: p.display_name().to_string(),
                status: "pending".to_string(),
                tasks_done: 0,
                tasks_total: 0,
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
            phases,
            tasks_completed: 0,
            tasks_total: 0,
            log_scroll: 0,
            max_log_lines: 500,
            halted: false,
        }
    }
}

impl DashboardState {
    /// Update state from live system components.
    #[allow(dead_code)] // Called by pipeline executor during live builds
    pub fn update_from_pipeline(&mut self, pipeline: &BuildPipeline) {
        // Current phase
        self.current_phase = pipeline
            .current_phase
            .map(|p| p.display_name().to_string())
            .unwrap_or_else(|| {
                if pipeline.is_complete() {
                    "complete".to_string()
                } else {
                    "idle".to_string()
                }
            });

        self.halted = pipeline.halted;

        // Phase progress = completed phases / total phases
        let completed = pipeline.completed_phases().len();
        self.phase_progress = completed as f64 / 8.0;

        // Per-phase status
        for ps in &pipeline.phases {
            if let Some(pd) = self.phases.iter_mut().find(|p| p.name == ps.phase.display_name()) {
                pd.status = match ps.status {
                    PhaseState::Pending => "pending".to_string(),
                    PhaseState::Running => "running".to_string(),
                    PhaseState::Completed => "done".to_string(),
                    PhaseState::Failed => "failed".to_string(),
                    PhaseState::Skipped => "skipped".to_string(),
                };
                pd.tasks_done = ps.tasks_completed;
                pd.tasks_total = ps.tasks_total;
            }
        }

        // Task stats from graph
        let stats = pipeline.task_graph.stats();
        self.tasks_completed = stats.completed;
        self.tasks_total = stats.total;
    }

    /// Update state from the agent manager.
    #[allow(dead_code)] // Called by pipeline executor during live builds
    pub fn update_from_agents(&mut self, manager: &AgentManager) {
        let mut total_tokens = 0u64;

        for agent_display in &mut self.agents {
            // Match agent display to manager by role name
            let role_name = agent_display.name.as_str();
            let handle = manager.agents().find(|a| a.role.display_name() == role_name);

            if let Some(h) = handle {
                agent_display.status = match h.state {
                    AgentState::Idle => "idle".to_string(),
                    AgentState::Working => "running".to_string(),
                    AgentState::Waiting => "waiting".to_string(),
                    AgentState::Healing => "healing".to_string(),
                    AgentState::Stopped => "stopped".to_string(),
                    AgentState::Halted => "halted".to_string(),
                };
                agent_display.current_task = h.current_task.clone();
                agent_display.tokens_used = h.tokens_consumed;
                agent_display.tasks_completed = h.tasks_completed;
                agent_display.tasks_failed = h.tasks_failed;
                total_tokens += h.tokens_consumed;
            }
        }

        self.tokens_used = total_tokens;
        // Blended cost estimate: ~$10/M input, ~$50/M output ≈ ~$15/M average
        self.cost_usd = total_tokens as f64 * 15.0 / 1_000_000.0;
    }

    /// Update state from the audit log (extract recent entries as log lines).
    #[allow(dead_code)] // Called by pipeline executor during live builds
    pub fn update_from_audit(&mut self, audit: &AuditLog) {
        let entries = audit.entries();
        let new_count = entries.len();
        let existing = self.log_lines.len();

        if new_count > existing {
            for entry in entries.iter().skip(existing) {
                let ts = entry.timestamp.format("%H:%M:%S");
                let line = format!(
                    "[{}] [{}] [{}] {}",
                    ts, entry.agent_id, entry.action, entry.description
                );
                self.log_lines.push(line);

                // Cap log buffer
                if self.log_lines.len() > self.max_log_lines {
                    self.log_lines.remove(0);
                }
            }
        }
    }

    /// Push a raw log line (from message bus progress events).
    pub fn push_log(&mut self, line: String) {
        self.log_lines.push(line);
        if self.log_lines.len() > self.max_log_lines {
            self.log_lines.remove(0);
        }
    }

    /// Scroll logs up.
    pub fn scroll_up(&mut self) {
        if self.log_scroll < self.log_lines.len().saturating_sub(1) {
            self.log_scroll += 1;
        }
    }

    /// Scroll logs down.
    pub fn scroll_down(&mut self) {
        self.log_scroll = self.log_scroll.saturating_sub(1);
    }

    /// Reset scroll to bottom (most recent).
    pub fn scroll_bottom(&mut self) {
        self.log_scroll = 0;
    }
}

// ── Dashboard Render ────────────────────────────────────────────────────────

/// Render the full dashboard to a ratatui frame.
pub fn render(frame: &mut Frame, state: &DashboardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // Progress bar
            Constraint::Min(10),   // Main content (phases + agents)
            Constraint::Length(10), // Logs
        ])
        .split(frame.area());

    render_header(frame, chunks[0], state);
    render_progress(frame, chunks[1], state);

    // Split main content into phases (left) and agents (right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(40)])
        .split(chunks[2]);

    render_phases(frame, main_chunks[0], state);
    render_agents(frame, main_chunks[1], state);
    render_logs(frame, chunks[3], state);
}

fn render_header(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let uptime = format_duration(state.uptime_secs);
    let status_color = if state.halted {
        Color::Red
    } else if state.current_phase == "complete" {
        Color::Green
    } else {
        Color::Yellow
    };

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
            Style::default().fg(status_color),
        ),
        Span::raw(" | "),
        Span::raw(format!(
            "Tasks: {}/{} ",
            state.tasks_completed, state.tasks_total
        )),
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
    let label = if state.halted {
        "HALTED".to_string()
    } else {
        format!(
            "{:.0}% — {}",
            state.phase_progress * 100.0,
            state.current_phase
        )
    };

    let gauge_color = if state.halted { Color::Red } else { Color::Green };

    let progress = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Pipeline Progress"),
        )
        .gauge_style(Style::default().fg(gauge_color))
        .ratio(state.phase_progress.clamp(0.0, 1.0))
        .label(label);

    frame.render_widget(progress, area);
}

/// Render the phase panel (left column).
fn render_phases(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let items: Vec<ListItem> = state
        .phases
        .iter()
        .map(|phase| {
            let (icon, color) = match phase.status.as_str() {
                "done" => ("✓", Color::Green),
                "running" => ("▶", Color::Yellow),
                "failed" => ("✗", Color::Red),
                "skipped" => ("–", Color::DarkGray),
                _ => (" ", Color::DarkGray),
            };

            let task_info = if phase.tasks_total > 0 {
                format!(" ({}/{})", phase.tasks_done, phase.tasks_total)
            } else {
                String::new()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", icon), Style::default().fg(color)),
                Span::styled(
                    format!("{}{}", phase.name, task_info),
                    Style::default().fg(if phase.status == "running" {
                        Color::White
                    } else {
                        color
                    }),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Phases"));

    frame.render_widget(list, area);
}

fn render_agents(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let rows: Vec<Row> = state
        .agents
        .iter()
        .map(|agent| {
            let status_style = match agent.status.as_str() {
                "running" => Style::default().fg(Color::Green),
                "waiting" => Style::default().fg(Color::Yellow),
                "healing" => Style::default().fg(Color::Magenta),
                "halted" | "stopped" => Style::default().fg(Color::Red),
                "error" => Style::default().fg(Color::Red),
                "idle" => Style::default().fg(Color::DarkGray),
                _ => Style::default(),
            };

            let task_display = agent
                .current_task
                .as_deref()
                .unwrap_or("-")
                .to_string();

            let stats = format!(
                "{}/{}",
                agent.tasks_completed, agent.tasks_completed + agent.tasks_failed
            );

            Row::new(vec![
                Cell::from(agent.name.clone()),
                Cell::from(agent.status.clone()).style(status_style),
                Cell::from(task_display),
                Cell::from(format_tokens(agent.tokens_used)),
                Cell::from(stats),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Length(9),
            Constraint::Min(20),
            Constraint::Length(10),
            Constraint::Length(7),
        ],
    )
    .header(
        Row::new(vec!["Agent", "Status", "Current Task", "Tokens", "Done"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::ALL).title("Agents"));

    frame.render_widget(table, area);
}

fn render_logs(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let visible_lines = (area.height as usize).saturating_sub(2);

    let log_text: Vec<Line> = state
        .log_lines
        .iter()
        .rev()
        .skip(state.log_scroll)
        .take(visible_lines)
        .rev()
        .map(|line| {
            // Color-code log lines by content
            let style = if line.contains("[TaskFailed]") || line.contains("error") {
                Style::default().fg(Color::Red)
            } else if line.contains("[TaskCompleted]") || line.contains("completed") {
                Style::default().fg(Color::Green)
            } else if line.contains("[SelfHealing]") || line.contains("healing") {
                Style::default().fg(Color::Magenta)
            } else if line.contains("[system]") {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::styled(line.as_str(), style)
        })
        .collect();

    let scroll_indicator = if state.log_scroll > 0 {
        format!("Logs [↑{}]", state.log_scroll)
    } else {
        "Logs".to_string()
    };

    let logs = Paragraph::new(log_text)
        .block(Block::default().borders(Borders::ALL).title(scroll_indicator))
        .wrap(Wrap { trim: false });

    frame.render_widget(logs, area);
}

// ── Log-only View ───────────────────────────────────────────────────────────

/// Render a log-only view (for `phantom logs`).
pub fn render_logs_view(frame: &mut Frame, state: &DashboardState, agent_filter: Option<&str>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(frame.area());

    // Header
    let title = match agent_filter {
        Some(agent) => format!("Phantom Logs — filtered: {}", agent),
        None => "Phantom Logs — all agents".to_string(),
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " PHANTOM LOGS ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::raw(format!(
            "{} entries | q=quit | ↑/↓=scroll",
            state.log_lines.len()
        )),
    ]))
    .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(header, chunks[0]);

    // Logs
    let visible_lines = (chunks[1].height as usize).saturating_sub(2);
    let filtered_lines: Vec<&String> = match agent_filter {
        Some(agent) => state
            .log_lines
            .iter()
            .filter(|l| l.contains(&format!("[{}]", agent)))
            .collect(),
        None => state.log_lines.iter().collect(),
    };

    let log_text: Vec<Line> = filtered_lines
        .iter()
        .rev()
        .skip(state.log_scroll)
        .take(visible_lines)
        .rev()
        .map(|line| {
            let style = if line.contains("[TaskFailed]") || line.contains("error") {
                Style::default().fg(Color::Red)
            } else if line.contains("[TaskCompleted]") {
                Style::default().fg(Color::Green)
            } else if line.contains("[SelfHealing]") {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::styled(line.as_str(), style)
        })
        .collect();

    let scroll_indicator = if state.log_scroll > 0 {
        format!(" [↑{}] ", state.log_scroll)
    } else {
        String::new()
    };

    let logs = Paragraph::new(log_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Log Stream{}", scroll_indicator)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(logs, chunks[1]);
}

// ── Brain Search View ───────────────────────────────────────────────────────

/// State for the interactive brain search panel.
#[derive(Debug, Clone)]
pub struct BrainSearchState {
    /// Current search query
    pub query: String,
    /// Search results
    pub results: Vec<BrainSearchResult>,
    /// Whether a search is in progress
    pub searching: bool,
    /// Error message (if any)
    pub error: Option<String>,
    /// Selected result index
    pub selected: usize,
    /// ChromaDB status
    pub chromadb_status: String,
}

/// A single Knowledge Brain search result.
#[derive(Debug, Clone)]
pub struct BrainSearchResult {
    pub source: String,
    pub heading: String,
    pub content: String,
    pub score: f64,
}

impl Default for BrainSearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            searching: false,
            error: None,
            selected: 0,
            chromadb_status: "unknown".to_string(),
        }
    }
}

/// Render the brain search TUI panel.
pub fn render_brain_search(frame: &mut Frame, state: &BrainSearchState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Search input
            Constraint::Min(10),  // Results
        ])
        .split(frame.area());

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " KNOWLEDGE BRAIN ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("ChromaDB: {} ", state.chromadb_status),
            Style::default().fg(if state.chromadb_status == "connected" {
                Color::Green
            } else {
                Color::Red
            }),
        ),
        Span::raw(" | "),
        Span::raw(format!("{} results", state.results.len())),
        Span::raw(" | q=quit ↑/↓=navigate"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    // Search input
    let search_label = if state.searching {
        "Searching..."
    } else {
        "Query"
    };
    let input = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::styled(&state.query, Style::default().fg(Color::Yellow)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(search_label),
    );
    frame.render_widget(input, chunks[1]);

    // Results
    if let Some(ref err) = state.error {
        let error_para = Paragraph::new(Line::styled(
            err.as_str(),
            Style::default().fg(Color::Red),
        ))
        .block(Block::default().borders(Borders::ALL).title("Error"));
        frame.render_widget(error_para, chunks[2]);
    } else if state.results.is_empty() && !state.query.is_empty() {
        let empty = Paragraph::new("No results found.")
            .block(Block::default().borders(Borders::ALL).title("Results"));
        frame.render_widget(empty, chunks[2]);
    } else {
        // Split results area: list on left, detail on right
        let result_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[2]);

        // Result list
        let items: Vec<ListItem> = state
            .results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let style = if i == state.selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:.0}% ", r.score * 100.0), style),
                    Span::styled(
                        format!("{}/{}", r.source, truncate_display(&r.heading, 20)),
                        style,
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Results"));
        frame.render_widget(list, result_chunks[0]);

        // Detail panel
        let detail = if let Some(result) = state.results.get(state.selected) {
            let content = format!(
                "Source: {}\nSection: {}\nRelevance: {:.1}%\n\n{}",
                result.source,
                result.heading,
                result.score * 100.0,
                result.content,
            );
            Paragraph::new(content)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title("Detail"))
        } else {
            Paragraph::new("Select a result to view details.")
                .block(Block::default().borders(Borders::ALL).title("Detail"))
        };
        frame.render_widget(detail, result_chunks[1]);
    }
}

// ── Terminal Runner ─────────────────────────────────────────────────────────

/// Run the live dashboard TUI event loop.
///
/// Reads from the shared state on each tick and re-renders.
/// Exits on 'q' or Ctrl+C.
pub async fn run_live_dashboard(
    state: Arc<RwLock<DashboardState>>,
    mut bus_mailbox: Option<phantom_core::message_bus::AgentMailbox>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let start = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        // Poll for message bus events
        if let Some(ref mut mailbox) = bus_mailbox {
            while let Some(msg) = mailbox.try_recv_broadcast() {
                let mut s = state.write().await;
                let ts = msg.timestamp.format("%H:%M:%S");
                match msg.kind {
                    MessageKind::ProgressUpdate => {
                        let detail = msg
                            .payload
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .unwrap_or("progress");
                        s.push_log(format!("[{}] [{}] [{}] {}", ts, msg.from, detail, msg.payload));
                    }
                    MessageKind::TaskCompleted => {
                        s.push_log(format!(
                            "[{}] [{}] [TaskCompleted] {}",
                            ts, msg.from, msg.payload
                        ));
                    }
                    MessageKind::TaskFailed => {
                        s.push_log(format!(
                            "[{}] [{}] [TaskFailed] {}",
                            ts, msg.from, msg.payload
                        ));
                    }
                    MessageKind::Halt => {
                        s.push_log(format!("[{}] [system] [HALT] {}", ts, msg.payload));
                        s.halted = true;
                    }
                    _ => {
                        s.push_log(format!(
                            "[{}] [{}] [{:?}] {}",
                            ts, msg.from, msg.kind, msg.payload
                        ));
                    }
                }
            }
        }

        // Update uptime
        {
            let mut s = state.write().await;
            s.uptime_secs = start.elapsed().as_secs();
        }

        // Render
        let snapshot = state.read().await.clone();
        terminal.draw(|frame| render(frame, &snapshot))?;

        // Handle input
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.write().await.scroll_up();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        state.write().await.scroll_down();
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        state.write().await.scroll_bottom();
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Run the log-only TUI view.
pub async fn run_logs_view(
    state: Arc<RwLock<DashboardState>>,
    agent_filter: Option<String>,
    mut bus_mailbox: Option<phantom_core::message_bus::AgentMailbox>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);

    loop {
        // Poll bus
        if let Some(ref mut mailbox) = bus_mailbox {
            while let Some(msg) = mailbox.try_recv_broadcast() {
                let mut s = state.write().await;
                let ts = msg.timestamp.format("%H:%M:%S");
                s.push_log(format!(
                    "[{}] [{}] [{:?}] {}",
                    ts, msg.from, msg.kind, msg.payload
                ));
            }
        }

        let snapshot = state.read().await.clone();
        let filter = agent_filter.as_deref();
        terminal.draw(|frame| render_logs_view(frame, &snapshot, filter))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Up | KeyCode::Char('k') => {
                        state.write().await.scroll_up();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        state.write().await.scroll_down();
                    }
                    KeyCode::End | KeyCode::Char('G') => {
                        state.write().await.scroll_bottom();
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Run the brain search TUI view.
pub async fn run_brain_search(initial_query: &str) -> anyhow::Result<()> {
    let mut search_state = BrainSearchState {
        query: initial_query.to_string(),
        results: Vec::new(),
        searching: false,
        error: None,
        selected: 0,
        chromadb_status: "checking...".to_string(),
    };

    // Check ChromaDB status
    let config = phantom_brain::config::BrainConfig::default();
    let client = phantom_brain::chromadb::ChromaClient::new(&config.chromadb_url);
    match client.health_check().await {
        Ok(true) => search_state.chromadb_status = "connected".to_string(),
        _ => search_state.chromadb_status = "offline".to_string(),
    }

    // Run initial search if query provided
    if !initial_query.is_empty() && search_state.chromadb_status == "connected" {
        search_state.searching = true;
        // The actual search would call ChromaDB here.
        // For now, show that the interface is ready.
        search_state.searching = false;
        if search_state.results.is_empty() {
            search_state.error = Some(
                "Knowledge Brain not yet ingested. Run `phantom brain update --file <path>` first."
                    .to_string(),
            );
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|frame| render_brain_search(frame, &search_state))?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Up | KeyCode::Char('k') => {
                        if search_state.selected > 0 {
                            search_state.selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if search_state.selected + 1 < search_state.results.len() {
                            search_state.selected += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

// ── Utilities ───────────────────────────────────────────────────────────────

pub fn format_duration(secs: u64) -> String {
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

pub fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Truncate a string for display purposes.
fn truncate_display(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let end = s.floor_char_boundary(max.saturating_sub(1));
        format!("{}…", &s[..end])
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_state_default() {
        let state = DashboardState::default();
        assert_eq!(state.agents.len(), 8);
        assert_eq!(state.phases.len(), 8);
        assert_eq!(state.current_phase, "idle");
        assert_eq!(state.phase_progress, 0.0);
        assert_eq!(state.tasks_completed, 0);
        assert_eq!(state.tasks_total, 0);
        assert!(!state.halted);
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

    #[test]
    fn test_push_log() {
        let mut state = DashboardState::default();
        state.push_log("test line 1".into());
        state.push_log("test line 2".into());
        assert_eq!(state.log_lines.len(), 2);
        assert_eq!(state.log_lines[0], "test line 1");
    }

    #[test]
    fn test_push_log_capped() {
        let mut state = DashboardState::default();
        state.max_log_lines = 3;
        for i in 0..5 {
            state.push_log(format!("line {}", i));
        }
        assert_eq!(state.log_lines.len(), 3);
        assert_eq!(state.log_lines[0], "line 2");
        assert_eq!(state.log_lines[2], "line 4");
    }

    #[test]
    fn test_log_scroll() {
        let mut state = DashboardState::default();
        for i in 0..20 {
            state.push_log(format!("line {}", i));
        }
        assert_eq!(state.log_scroll, 0);

        state.scroll_up();
        assert_eq!(state.log_scroll, 1);
        state.scroll_up();
        assert_eq!(state.log_scroll, 2);

        state.scroll_down();
        assert_eq!(state.log_scroll, 1);

        state.scroll_bottom();
        assert_eq!(state.log_scroll, 0);
    }

    #[test]
    fn test_scroll_up_capped() {
        let mut state = DashboardState::default();
        state.push_log("only one".into());

        state.scroll_up();
        assert_eq!(state.log_scroll, 0); // Can't scroll past the only line
    }

    #[test]
    fn test_scroll_down_at_zero() {
        let mut state = DashboardState::default();
        state.scroll_down();
        assert_eq!(state.log_scroll, 0); // Already at bottom
    }

    #[test]
    fn test_update_from_pipeline() {
        let mut state = DashboardState::default();
        let mut pipeline = BuildPipeline::new(Some("test.md".into()));
        pipeline.start();
        pipeline.complete_current_phase(); // Ingest → Infrastructure

        state.update_from_pipeline(&pipeline);

        assert_eq!(state.current_phase, "Phase 1: Infrastructure");
        assert!(!state.halted);
        // 1 phase completed out of 8
        assert!((state.phase_progress - 1.0 / 8.0).abs() < 0.01);
    }

    #[test]
    fn test_update_from_pipeline_halted() {
        let mut state = DashboardState::default();
        let mut pipeline = BuildPipeline::new(None);
        pipeline.start();
        pipeline.halt();

        state.update_from_pipeline(&pipeline);
        assert!(state.halted);
    }

    #[test]
    fn test_update_from_pipeline_complete() {
        let mut state = DashboardState::default();
        let mut pipeline = BuildPipeline::new(None);
        pipeline.start();
        while pipeline.complete_current_phase().is_some() {}

        state.update_from_pipeline(&pipeline);
        assert_eq!(state.current_phase, "complete");
        assert!((state.phase_progress - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_update_from_agents() {
        let mut state = DashboardState::default();
        let mut manager = AgentManager::new();
        let ids = manager.spawn_full_team().unwrap();

        // Find the backend agent ID (format: "backend-agent-0")
        let backend_id = ids.iter().find(|id| id.starts_with("backend")).unwrap();

        // Simulate backend working on a task
        if let Some(agent) = manager.get_mut(backend_id) {
            agent.assign_task("task-123");
            agent.record_tokens(5000, 2000);
        }

        state.update_from_agents(&manager);

        // Find the Backend Agent in display
        let backend = state.agents.iter().find(|a| a.name == "Backend Agent").unwrap();
        assert_eq!(backend.status, "running");
        assert_eq!(backend.current_task.as_deref(), Some("task-123"));
        assert_eq!(backend.tokens_used, 7000);

        assert!(state.tokens_used > 0);
        assert!(state.cost_usd > 0.0);
    }

    #[test]
    fn test_update_from_audit() {
        use phantom_core::audit::{AuditAction, AuditLog};

        let mut state = DashboardState::default();
        let mut audit = AuditLog::new();
        audit.record(
            "cto-0",
            AuditAction::TaskStarted,
            "Started task: design API",
            serde_json::json!({}),
            None,
        );
        audit.record(
            "backend-0",
            AuditAction::TaskCompleted,
            "Completed task: implement auth",
            serde_json::json!({}),
            None,
        );

        state.update_from_audit(&audit);
        assert_eq!(state.log_lines.len(), 2);
        assert!(state.log_lines[0].contains("[cto-0]"));
        assert!(state.log_lines[1].contains("[backend-0]"));

        // Second update should only add new entries
        audit.record(
            "qa-0",
            AuditAction::TaskStarted,
            "Started testing",
            serde_json::json!({}),
            None,
        );
        state.update_from_audit(&audit);
        assert_eq!(state.log_lines.len(), 3);
    }

    #[test]
    fn test_phase_display_status() {
        let mut state = DashboardState::default();
        let mut pipeline = BuildPipeline::new(None);
        pipeline.start();

        state.update_from_pipeline(&pipeline);

        // Ingest should be running
        let ingest = state.phases.iter().find(|p| p.name.contains("Ingest")).unwrap();
        assert_eq!(ingest.status, "running");

        // Infrastructure should be pending
        let infra = state
            .phases
            .iter()
            .find(|p| p.name.contains("Infrastructure"))
            .unwrap();
        assert_eq!(infra.status, "pending");
    }

    #[test]
    fn test_agent_display_default() {
        let state = DashboardState::default();
        for agent in &state.agents {
            assert_eq!(agent.status, "idle");
            assert!(agent.current_task.is_none());
            assert_eq!(agent.tokens_used, 0);
            assert_eq!(agent.tasks_completed, 0);
            assert_eq!(agent.tasks_failed, 0);
        }
    }

    #[test]
    fn test_brain_search_state_default() {
        let state = BrainSearchState::default();
        assert!(state.query.is_empty());
        assert!(state.results.is_empty());
        assert!(!state.searching);
        assert!(state.error.is_none());
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_truncate_display() {
        assert_eq!(truncate_display("hello", 10), "hello");
        assert_eq!(truncate_display("hello world", 6), "hello…");
        assert_eq!(truncate_display("", 5), "");
    }
}
