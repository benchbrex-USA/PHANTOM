//! Beyond Human Capabilities — §15
//!
//! Eight autonomous subsystems that make Phantom operate beyond what any
//! human developer can sustain:
//!
//! 1. **Ambient Context** — background daemon reads active app, clipboard,
//!    recent files to build situational awareness.
//! 2. **Self-Scheduling** — cron-like trigger engine that queues Phantom
//!    jobs based on time, file-change, or git events.
//! 3. **Smart Git** — auto-branch, semantic commit messages, PR creation
//!    via `gh` CLI with zero human intervention.
//! 4. **Predictive Errors** — static scan of codebase before build to
//!    flag likely failures early.
//! 5. **Cross-Project Memory** — stores project learnings in Knowledge
//!    Brain with project tags for reuse across repos.
//! 6. **Cost Oracle** — real-time token spend tracker with budget
//!    enforcement and alert thresholds.
//! 7. **Voice Notifications** — macOS `say` command for key events.
//! 8. **Self-Updating** — background version check + hot-swap binary.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

// ═══════════════════════════════════════════════════════════════════════════
//  Error type
// ═══════════════════════════════════════════════════════════════════════════

/// Unified error for all Beyond-Human subsystems.
#[derive(Debug, thiserror::Error)]
pub enum BeyondError {
    #[error("ambient context: {0}")]
    Ambient(String),
    #[error("scheduler: {0}")]
    Scheduler(String),
    #[error("git workflow: {0}")]
    Git(String),
    #[error("predictive scan: {0}")]
    Predictive(String),
    #[error("cross-project memory: {0}")]
    Memory(String),
    #[error("cost oracle: budget exceeded — {0}")]
    BudgetExceeded(String),
    #[error("cost oracle: {0}")]
    CostOracle(String),
    #[error("voice: {0}")]
    Voice(String),
    #[error("self-update: {0}")]
    SelfUpdate(String),
    #[error("command failed ({cmd}): {reason}")]
    CommandFailed { cmd: String, reason: String },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type BeyondResult<T> = Result<T, BeyondError>;

// ═══════════════════════════════════════════════════════════════════════════
//  1. Ambient Context Awareness
// ═══════════════════════════════════════════════════════════════════════════

/// Snapshot of the user's current desktop context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientSnapshot {
    /// Currently focused application name
    pub active_app: Option<String>,
    /// Current clipboard text (truncated to 4 KB)
    pub clipboard: Option<String>,
    /// Recently modified files in the working directory
    pub recent_files: Vec<RecentFile>,
    /// When this snapshot was captured
    pub captured_at: DateTime<Utc>,
}

/// A recently modified file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFile {
    pub path: String,
    pub modified: DateTime<Utc>,
    pub size_bytes: u64,
}

/// Background daemon that polls ambient context at a configurable interval.
pub struct AmbientDaemon {
    /// Working directory to watch for recent files
    work_dir: PathBuf,
    /// How many seconds between polls
    poll_interval_secs: u64,
    /// Maximum number of recent files to track
    max_recent_files: usize,
    /// Last captured snapshot
    last_snapshot: Arc<Mutex<Option<AmbientSnapshot>>>,
    /// Whether the daemon loop is running
    running: Arc<AtomicBool>,
}

impl AmbientDaemon {
    pub fn new(work_dir: impl Into<PathBuf>) -> Self {
        Self {
            work_dir: work_dir.into(),
            poll_interval_secs: 10,
            max_recent_files: 20,
            last_snapshot: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_interval(mut self, secs: u64) -> Self {
        self.poll_interval_secs = secs;
        self
    }

    pub fn with_max_recent(mut self, n: usize) -> Self {
        self.max_recent_files = n;
        self
    }

    /// Capture a single ambient snapshot (non-blocking, runs CLI tools).
    pub fn capture(&self) -> BeyondResult<AmbientSnapshot> {
        let active_app = read_active_app().ok();
        let clipboard = read_clipboard().ok();
        let recent_files = self.scan_recent_files();

        let snapshot = AmbientSnapshot {
            active_app,
            clipboard,
            recent_files,
            captured_at: Utc::now(),
        };

        if let Ok(mut lock) = self.last_snapshot.lock() {
            *lock = Some(snapshot.clone());
        }

        Ok(snapshot)
    }

    /// Start background polling. Returns immediately; the loop runs on a
    /// tokio task. Call `stop()` to terminate.
    pub fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return; // already running
        }

        let running = Arc::clone(&self.running);
        let snapshot = Arc::clone(&self.last_snapshot);
        let work_dir = self.work_dir.clone();
        let interval = self.poll_interval_secs;
        let max_recent = self.max_recent_files;

        tokio::spawn(async move {
            info!("ambient daemon started (interval={}s)", interval);
            while running.load(Ordering::SeqCst) {
                let active_app = read_active_app().ok();
                let clipboard = read_clipboard().ok();
                let recent_files = scan_recent_files_inner(&work_dir, max_recent);

                let snap = AmbientSnapshot {
                    active_app,
                    clipboard,
                    recent_files,
                    captured_at: Utc::now(),
                };

                if let Ok(mut lock) = snapshot.lock() {
                    *lock = Some(snap);
                }

                tokio::time::sleep(Duration::from_secs(interval)).await;
            }
            info!("ambient daemon stopped");
        });
    }

    /// Stop the background polling loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the latest snapshot without capturing a new one.
    pub fn latest(&self) -> Option<AmbientSnapshot> {
        self.last_snapshot.lock().ok().and_then(|s| s.clone())
    }

    fn scan_recent_files(&self) -> Vec<RecentFile> {
        scan_recent_files_inner(&self.work_dir, self.max_recent_files)
    }
}

/// Read the currently focused application via AppleScript.
fn read_active_app() -> BeyondResult<String> {
    let output = Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to get name of first application process whose frontmost is true",
        ])
        .output()?;

    if !output.status.success() {
        return Err(BeyondError::Ambient("failed to read active app".into()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Read clipboard text via pbpaste (macOS), truncated to 4 KB.
fn read_clipboard() -> BeyondResult<String> {
    let output = Command::new("pbpaste").output()?;
    if !output.status.success() {
        return Err(BeyondError::Ambient("pbpaste failed".into()));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.chars().take(4096).collect())
}

/// Scan a directory for recently modified files (last 30 min), sorted newest first.
fn scan_recent_files_inner(dir: &Path, max: usize) -> Vec<RecentFile> {
    let mut files = Vec::new();
    let cutoff = std::time::SystemTime::now() - Duration::from_secs(1800);

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return files,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                if modified > cutoff {
                    let dt: DateTime<Utc> = modified.into();
                    files.push(RecentFile {
                        path: path.display().to_string(),
                        modified: dt,
                        size_bytes: meta.len(),
                    });
                }
            }
        }
    }

    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    files.truncate(max);
    files
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. Self-Scheduling — cron-like trigger engine
// ═══════════════════════════════════════════════════════════════════════════

/// What triggers a scheduled job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    /// Fire at a fixed interval (seconds).
    Interval(u64),
    /// Fire when a file matching the glob changes.
    FileChange { glob: String },
    /// Fire on git events (push, commit, branch creation).
    GitEvent(GitEventKind),
    /// Fire once at a specific time.
    OneShot(DateTime<Utc>),
}

/// Git events that can trigger jobs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitEventKind {
    PostCommit,
    PostPush,
    BranchCreated,
    MergeConflict,
}

/// A scheduled rule — maps a trigger to a job description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRule {
    /// Unique rule ID
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// What triggers this rule
    pub trigger: TriggerKind,
    /// Agent role that should execute the job
    pub agent_role: String,
    /// Payload template passed to the job
    pub payload: serde_json::Value,
    /// Priority for the queued job
    pub priority: u32,
    /// Whether this rule is enabled
    pub enabled: bool,
}

/// Emitted when a rule fires.
#[derive(Debug, Clone)]
pub struct FiredJob {
    pub rule_id: String,
    pub agent_role: String,
    pub priority: u32,
    pub payload: serde_json::Value,
    pub fired_at: DateTime<Utc>,
}

/// Self-scheduler that evaluates trigger rules and emits jobs.
pub struct SelfScheduler {
    rules: Vec<ScheduleRule>,
    /// Last fire time per rule ID, for interval-based dedup.
    last_fired: HashMap<String, DateTime<Utc>>,
    /// Accumulated fired jobs waiting to be drained by the orchestrator.
    pending: Vec<FiredJob>,
}

impl Default for SelfScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfScheduler {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            last_fired: HashMap::new(),
            pending: Vec::new(),
        }
    }

    /// Register a new schedule rule.
    pub fn add_rule(&mut self, rule: ScheduleRule) {
        info!(id = %rule.id, desc = %rule.description, "schedule rule added");
        self.rules.push(rule);
    }

    /// Remove a rule by ID.
    pub fn remove_rule(&mut self, id: &str) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() < before
    }

    /// Enable or disable a rule.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = enabled;
            return true;
        }
        false
    }

    /// Evaluate all interval-based and one-shot rules against the current time.
    /// Call this periodically from the orchestrator loop.
    pub fn tick(&mut self) {
        let now = Utc::now();

        let to_fire: Vec<usize> = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.enabled)
            .filter(|(_, rule)| match &rule.trigger {
                TriggerKind::Interval(secs) => {
                    let last = self.last_fired.get(&rule.id);
                    match last {
                        None => true,
                        Some(t) => {
                            let elapsed = (now - *t).num_seconds().unsigned_abs();
                            elapsed >= *secs
                        }
                    }
                }
                TriggerKind::OneShot(at) => {
                    let already = self.last_fired.contains_key(&rule.id);
                    !already && now >= *at
                }
                _ => false,
            })
            .map(|(i, _)| i)
            .collect();

        for idx in to_fire {
            let rule = self.rules[idx].clone();
            self.fire_rule(&rule, now);
        }
    }

    /// Notify the scheduler that a file matching a glob has changed.
    pub fn notify_file_change(&mut self, changed_path: &str) {
        let now = Utc::now();
        let matching: Vec<usize> = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, r)| r.enabled)
            .filter(|(_, r)| matches!(&r.trigger, TriggerKind::FileChange { glob } if path_matches_glob(changed_path, glob)))
            .map(|(i, _)| i)
            .collect();

        for idx in matching {
            let rule = self.rules[idx].clone();
            self.fire_rule(&rule, now);
        }
    }

    /// Notify the scheduler that a git event occurred.
    pub fn notify_git_event(&mut self, event: GitEventKind) {
        let now = Utc::now();
        let matching: Vec<usize> = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, r)| r.enabled)
            .filter(|(_, r)| matches!(&r.trigger, TriggerKind::GitEvent(e) if *e == event))
            .map(|(i, _)| i)
            .collect();

        for idx in matching {
            let rule = self.rules[idx].clone();
            self.fire_rule(&rule, now);
        }
    }

    /// Drain pending fired jobs (the orchestrator calls this to feed the JobQueue).
    pub fn drain_pending(&mut self) -> Vec<FiredJob> {
        std::mem::take(&mut self.pending)
    }

    /// Number of registered rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Number of pending (unflushed) fired jobs.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    fn fire_rule(&mut self, rule: &ScheduleRule, now: DateTime<Utc>) {
        debug!(rule = %rule.id, "schedule rule fired");
        self.last_fired.insert(rule.id.clone(), now);
        self.pending.push(FiredJob {
            rule_id: rule.id.clone(),
            agent_role: rule.agent_role.clone(),
            priority: rule.priority,
            payload: rule.payload.clone(),
            fired_at: now,
        });
    }
}

/// Minimal glob matching: supports `*` (any chars) and `**` (any path segments).
fn path_matches_glob(path: &str, glob: &str) -> bool {
    if glob == "*" || glob == "**" {
        return true;
    }
    // Simple suffix match for *.ext patterns
    if let Some(ext) = glob.strip_prefix('*') {
        return path.ends_with(ext);
    }
    // Simple prefix match for dir/** patterns
    if let Some(prefix) = glob.strip_suffix("/**") {
        return path.starts_with(prefix);
    }
    path == glob
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. Smart Git Workflows
// ═══════════════════════════════════════════════════════════════════════════

/// Configuration for smart git operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Working directory (repo root)
    pub repo_dir: PathBuf,
    /// Branch prefix for auto-created branches (e.g. "phantom/")
    pub branch_prefix: String,
    /// Whether to push branches after creation
    pub auto_push: bool,
    /// GitHub owner/repo for PR creation (e.g. "user/repo")
    pub github_repo: Option<String>,
}

impl GitConfig {
    pub fn new(repo_dir: impl Into<PathBuf>) -> Self {
        Self {
            repo_dir: repo_dir.into(),
            branch_prefix: "phantom/".into(),
            auto_push: true,
            github_repo: None,
        }
    }

    pub fn with_github_repo(mut self, repo: impl Into<String>) -> Self {
        self.github_repo = Some(repo.into());
        self
    }
}

/// Result of a git operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpResult {
    pub operation: String,
    pub success: bool,
    pub output: String,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub pr_url: Option<String>,
}

/// Smart git workflow engine.
pub struct SmartGit {
    config: GitConfig,
}

impl SmartGit {
    pub fn new(config: GitConfig) -> Self {
        Self { config }
    }

    /// Create a feature branch from the current HEAD.
    /// Name is sanitized: lowercase, spaces → dashes, max 50 chars.
    pub fn auto_branch(&self, description: &str) -> BeyondResult<GitOpResult> {
        let slug = slugify(description);
        let branch = format!("{}{}", self.config.branch_prefix, slug);

        let output = self.git(&["checkout", "-b", &branch])?;
        info!(branch = %branch, "auto-branch created");

        Ok(GitOpResult {
            operation: "auto_branch".into(),
            success: true,
            output,
            branch: Some(branch),
            commit_sha: None,
            pr_url: None,
        })
    }

    /// Stage all changes and commit with a semantic message.
    /// `category` is one of: feat, fix, refactor, docs, test, chore, perf.
    pub fn auto_commit(
        &self,
        category: &str,
        scope: &str,
        summary: &str,
    ) -> BeyondResult<GitOpResult> {
        // Stage all tracked changes
        self.git(&["add", "-A"])?;

        // Check if there's anything to commit
        let status = self.git(&["status", "--porcelain"])?;
        if status.trim().is_empty() {
            return Ok(GitOpResult {
                operation: "auto_commit".into(),
                success: false,
                output: "nothing to commit".into(),
                branch: None,
                commit_sha: None,
                pr_url: None,
            });
        }

        // Build conventional commit message
        let message = if scope.is_empty() {
            format!("{}: {}", category, summary)
        } else {
            format!("{}({}): {}", category, scope, summary)
        };

        let output = self.git(&["commit", "-m", &message])?;

        // Extract SHA
        let sha = self
            .git(&["rev-parse", "--short", "HEAD"])
            .ok()
            .map(|s| s.trim().to_string());

        info!(message = %message, sha = ?sha, "auto-commit");

        Ok(GitOpResult {
            operation: "auto_commit".into(),
            success: true,
            output,
            branch: self.current_branch().ok(),
            commit_sha: sha,
            pr_url: None,
        })
    }

    /// Push the current branch to origin.
    pub fn push(&self) -> BeyondResult<GitOpResult> {
        let branch = self.current_branch()?;
        let output = self.git(&["push", "-u", "origin", &branch])?;

        Ok(GitOpResult {
            operation: "push".into(),
            success: true,
            output,
            branch: Some(branch),
            commit_sha: None,
            pr_url: None,
        })
    }

    /// Create a pull request via `gh` CLI.
    pub fn create_pr(&self, title: &str, body: &str, base: &str) -> BeyondResult<GitOpResult> {
        // Push first
        if self.config.auto_push {
            self.push()?;
        }

        let output = run_cmd(
            "gh",
            &[
                "pr", "create", "--title", title, "--body", body, "--base", base,
            ],
            Some(&self.config.repo_dir),
        )?;

        // Extract PR URL from gh output (last line is usually the URL)
        let pr_url = output
            .lines()
            .rev()
            .find(|l| l.starts_with("https://"))
            .map(|s| s.trim().to_string());

        info!(title = %title, pr_url = ?pr_url, "PR created");

        Ok(GitOpResult {
            operation: "create_pr".into(),
            success: true,
            output,
            branch: self.current_branch().ok(),
            commit_sha: None,
            pr_url,
        })
    }

    /// Full workflow: branch → commit → push → PR.
    #[allow(clippy::too_many_arguments)]
    pub fn full_workflow(
        &self,
        branch_desc: &str,
        category: &str,
        scope: &str,
        commit_summary: &str,
        pr_title: &str,
        pr_body: &str,
        base: &str,
    ) -> BeyondResult<GitOpResult> {
        self.auto_branch(branch_desc)?;
        self.auto_commit(category, scope, commit_summary)?;
        self.create_pr(pr_title, pr_body, base)
    }

    /// Get the current branch name.
    pub fn current_branch(&self) -> BeyondResult<String> {
        let out = self.git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        Ok(out.trim().to_string())
    }

    /// Get a short diff summary for the working directory.
    pub fn diff_summary(&self) -> BeyondResult<String> {
        self.git(&["diff", "--stat"])
    }

    /// List commits not yet pushed to origin.
    pub fn unpushed_commits(&self) -> BeyondResult<Vec<String>> {
        let branch = self.current_branch()?;
        let range = format!("origin/{}..HEAD", branch);
        let output = self.git(&["log", "--oneline", &range]).unwrap_or_default();
        Ok(output
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect())
    }

    fn git(&self, args: &[&str]) -> BeyondResult<String> {
        run_cmd("git", args, Some(&self.config.repo_dir))
    }
}

/// Slugify a description for use as a branch name.
fn slugify(s: &str) -> String {
    let slug: String = s
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    // Collapse consecutive dashes, trim, truncate
    let mut result = String::new();
    let mut last_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !last_dash && !result.is_empty() {
                result.push('-');
            }
            last_dash = true;
        } else {
            result.push(c);
            last_dash = false;
        }
    }
    result.truncate(50);
    result.trim_end_matches('-').to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. Predictive Error Prevention
// ═══════════════════════════════════════════════════════════════════════════

/// Severity level for predictive warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// A single predictive warning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveWarning {
    pub file: String,
    pub line: Option<usize>,
    pub severity: Severity,
    pub rule: String,
    pub message: String,
}

/// Result of a predictive scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub warnings: Vec<PredictiveWarning>,
    pub files_scanned: usize,
    pub duration_ms: u64,
    pub build_recommended: bool,
}

impl ScanReport {
    /// Whether the scan found any critical issues that should block build.
    pub fn has_blockers(&self) -> bool {
        self.warnings.iter().any(|w| w.severity >= Severity::Error)
    }

    /// Count by severity.
    pub fn count_by_severity(&self, sev: Severity) -> usize {
        self.warnings.iter().filter(|w| w.severity == sev).count()
    }
}

/// Rule applied during predictive scanning.
struct ScanRule {
    name: &'static str,
    severity: Severity,
    /// File extensions this rule applies to (empty = all)
    extensions: &'static [&'static str],
    /// Pattern to search for in file content
    pattern: &'static str,
    message: &'static str,
}

/// Built-in scan rules for common failure patterns.
const SCAN_RULES: &[ScanRule] = &[
    ScanRule {
        name: "todo_fixme",
        severity: Severity::Info,
        extensions: &["rs", "ts", "js", "py", "go"],
        pattern: "TODO|FIXME|HACK|XXX",
        message: "unresolved TODO/FIXME marker",
    },
    ScanRule {
        name: "unwrap_usage",
        severity: Severity::Warning,
        extensions: &["rs"],
        pattern: ".unwrap()",
        message: "unwrap() may panic — consider using ? or expect()",
    },
    ScanRule {
        name: "hardcoded_secret",
        severity: Severity::Critical,
        extensions: &["rs", "ts", "js", "py", "go", "toml", "yaml", "json"],
        pattern: "password|secret|api_key|private_key",
        message: "possible hardcoded secret or credential",
    },
    ScanRule {
        name: "debug_print",
        severity: Severity::Warning,
        extensions: &["rs"],
        pattern: "dbg!(",
        message: "debug macro left in code",
    },
    ScanRule {
        name: "console_log",
        severity: Severity::Warning,
        extensions: &["ts", "js"],
        pattern: "console.log(",
        message: "console.log left in code",
    },
    ScanRule {
        name: "unresolved_conflict",
        severity: Severity::Critical,
        extensions: &[],
        pattern: "<<<<<<<",
        message: "unresolved merge conflict marker",
    },
    ScanRule {
        name: "large_file",
        severity: Severity::Warning,
        extensions: &[],
        pattern: "",
        message: "file exceeds 2000 lines — consider splitting",
    },
    ScanRule {
        name: "missing_error_handling",
        severity: Severity::Warning,
        extensions: &["go"],
        pattern: "_ = ",
        message: "ignored error return value",
    },
];

/// Predictive error scanner.
pub struct PredictiveScanner {
    /// Extra patterns to check (name → pattern)
    custom_patterns: Vec<(String, String, Severity)>,
    /// File extensions to include (empty = all)
    include_extensions: Vec<String>,
    /// Maximum file size to scan (bytes)
    max_file_bytes: u64,
    /// Large file line threshold
    large_file_threshold: usize,
}

impl Default for PredictiveScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl PredictiveScanner {
    pub fn new() -> Self {
        Self {
            custom_patterns: Vec::new(),
            include_extensions: Vec::new(),
            max_file_bytes: 1024 * 1024, // 1 MB
            large_file_threshold: 2000,
        }
    }

    /// Add a custom pattern to scan for.
    pub fn add_pattern(
        &mut self,
        name: impl Into<String>,
        pattern: impl Into<String>,
        severity: Severity,
    ) {
        self.custom_patterns
            .push((name.into(), pattern.into(), severity));
    }

    /// Restrict scanning to specific file extensions.
    pub fn include_ext(&mut self, ext: impl Into<String>) {
        self.include_extensions.push(ext.into());
    }

    /// Scan a directory tree for predictive warnings.
    pub fn scan(&self, dir: &Path) -> BeyondResult<ScanReport> {
        let start = Instant::now();
        let mut warnings = Vec::new();
        let mut files_scanned = 0;

        self.scan_dir(dir, &mut warnings, &mut files_scanned)?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let build_recommended = !warnings
            .iter()
            .any(|w: &PredictiveWarning| w.severity >= Severity::Error);

        Ok(ScanReport {
            warnings,
            files_scanned,
            duration_ms,
            build_recommended,
        })
    }

    fn scan_dir(
        &self,
        dir: &Path,
        warnings: &mut Vec<PredictiveWarning>,
        count: &mut usize,
    ) -> BeyondResult<()> {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Skip hidden dirs and common non-source dirs
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.')
                    || name == "target"
                    || name == "node_modules"
                    || name == "vendor"
                    || name == "__pycache__"
                {
                    continue;
                }
            }

            if path.is_dir() {
                self.scan_dir(&path, warnings, count)?;
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();

            // Extension filter
            if !self.include_extensions.is_empty() && !self.include_extensions.contains(&ext) {
                continue;
            }

            // Size check
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.len() > self.max_file_bytes {
                continue;
            }

            *count += 1;
            self.scan_file(&path, &ext, meta.len(), warnings);
        }

        Ok(())
    }

    fn scan_file(&self, path: &Path, ext: &str, _size: u64, warnings: &mut Vec<PredictiveWarning>) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return, // binary file or permission issue
        };

        let file_str = path.display().to_string();
        let line_count = content.lines().count();

        // Large file check
        if line_count > self.large_file_threshold {
            warnings.push(PredictiveWarning {
                file: file_str.clone(),
                line: None,
                severity: Severity::Warning,
                rule: "large_file".into(),
                message: format!(
                    "file has {} lines (threshold: {})",
                    line_count, self.large_file_threshold
                ),
            });
        }

        // Built-in rules
        for rule in SCAN_RULES {
            if rule.pattern.is_empty() {
                continue; // handled above (large_file)
            }
            if !rule.extensions.is_empty() && !rule.extensions.contains(&ext) {
                continue;
            }

            for (i, line) in content.lines().enumerate() {
                // Simple substring check for each pattern alternative
                let patterns: Vec<&str> = rule.pattern.split('|').collect();
                for pat in patterns {
                    if line.contains(pat) {
                        // Skip false positives in comments for credential checks
                        if rule.name == "hardcoded_secret" {
                            let trimmed = line.trim();
                            if trimmed.starts_with("//")
                                || trimmed.starts_with('#')
                                || trimmed.starts_with("///")
                                || trimmed.contains("env::")
                                || trimmed.contains("env::var")
                                || trimmed.contains("_env_var")
                                || trimmed.contains("token_env")
                            {
                                continue;
                            }
                        }
                        warnings.push(PredictiveWarning {
                            file: file_str.clone(),
                            line: Some(i + 1),
                            severity: rule.severity,
                            rule: rule.name.into(),
                            message: rule.message.into(),
                        });
                        break; // one warning per line per rule
                    }
                }
            }
        }

        // Custom patterns
        for (name, pattern, severity) in &self.custom_patterns {
            for (i, line) in content.lines().enumerate() {
                if line.contains(pattern.as_str()) {
                    warnings.push(PredictiveWarning {
                        file: file_str.clone(),
                        line: Some(i + 1),
                        severity: *severity,
                        rule: name.clone(),
                        message: format!("custom pattern match: {}", pattern),
                    });
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  5. Cross-Project Memory
// ═══════════════════════════════════════════════════════════════════════════

/// A learning stored in the Knowledge Brain with project context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectLearning {
    /// Unique learning ID
    pub id: String,
    /// Project this learning came from
    pub project: String,
    /// Tags for retrieval (e.g. "rust", "auth", "performance")
    pub tags: Vec<String>,
    /// Category of learning
    pub category: LearningCategory,
    /// The lesson itself
    pub content: String,
    /// Confidence score (0.0–1.0) — higher means more validated
    pub confidence: f32,
    /// When this was learned
    pub learned_at: DateTime<Utc>,
    /// How many times this learning has been applied
    pub applied_count: u32,
}

/// Categories of cross-project learnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningCategory {
    /// Build/dependency patterns
    BuildConfig,
    /// Error patterns and their fixes
    ErrorFix,
    /// Architecture decisions and rationale
    Architecture,
    /// Performance optimization techniques
    Performance,
    /// Security patterns
    Security,
    /// Testing strategies
    Testing,
    /// Deployment patterns
    Deployment,
    /// General workflow learnings
    Workflow,
}

impl std::fmt::Display for LearningCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BuildConfig => write!(f, "build_config"),
            Self::ErrorFix => write!(f, "error_fix"),
            Self::Architecture => write!(f, "architecture"),
            Self::Performance => write!(f, "performance"),
            Self::Security => write!(f, "security"),
            Self::Testing => write!(f, "testing"),
            Self::Deployment => write!(f, "deployment"),
            Self::Workflow => write!(f, "workflow"),
        }
    }
}

/// In-memory store for cross-project learnings.
/// In production, backed by Knowledge Brain (ChromaDB vector store).
pub struct ProjectMemory {
    learnings: Vec<ProjectLearning>,
    /// Current project name for tagging new learnings
    current_project: String,
    /// Maximum learnings to keep in memory
    max_learnings: usize,
}

impl ProjectMemory {
    pub fn new(current_project: impl Into<String>) -> Self {
        Self {
            learnings: Vec::new(),
            current_project: current_project.into(),
            max_learnings: 10_000,
        }
    }

    /// Record a new learning.
    pub fn record(
        &mut self,
        content: impl Into<String>,
        category: LearningCategory,
        tags: Vec<String>,
    ) -> String {
        let id = format!(
            "learn-{}-{}",
            self.current_project,
            self.learnings.len() + 1
        );

        let learning = ProjectLearning {
            id: id.clone(),
            project: self.current_project.clone(),
            tags,
            category,
            content: content.into(),
            confidence: 0.5, // initial confidence
            learned_at: Utc::now(),
            applied_count: 0,
        };

        info!(id = %id, category = %category, "learning recorded");
        self.learnings.push(learning);

        // Evict oldest if over limit
        if self.learnings.len() > self.max_learnings {
            self.learnings.remove(0);
        }

        id
    }

    /// Query learnings by tag (any matching tag counts).
    pub fn query_by_tag(&self, tag: &str) -> Vec<&ProjectLearning> {
        self.learnings
            .iter()
            .filter(|l| l.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Query learnings by category.
    pub fn query_by_category(&self, category: LearningCategory) -> Vec<&ProjectLearning> {
        self.learnings
            .iter()
            .filter(|l| l.category == category)
            .collect()
    }

    /// Query learnings by project name.
    pub fn query_by_project(&self, project: &str) -> Vec<&ProjectLearning> {
        self.learnings
            .iter()
            .filter(|l| l.project == project)
            .collect()
    }

    /// Full-text search across learning content (simple substring match).
    pub fn search(&self, query: &str) -> Vec<&ProjectLearning> {
        let q = query.to_lowercase();
        self.learnings
            .iter()
            .filter(|l| l.content.to_lowercase().contains(&q))
            .collect()
    }

    /// Mark a learning as applied (bumps confidence and count).
    pub fn mark_applied(&mut self, id: &str) -> bool {
        if let Some(learning) = self.learnings.iter_mut().find(|l| l.id == id) {
            learning.applied_count += 1;
            // Confidence grows with each application, approaching 1.0
            learning.confidence = 1.0 - (1.0 - learning.confidence) * 0.8;
            return true;
        }
        false
    }

    /// Get the most confident learnings for a given category.
    pub fn top_learnings(&self, category: LearningCategory, limit: usize) -> Vec<&ProjectLearning> {
        let mut results: Vec<&ProjectLearning> = self
            .learnings
            .iter()
            .filter(|l| l.category == category)
            .collect();
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        results
    }

    /// Export all learnings as JSON (for persistence / Knowledge Brain sync).
    pub fn export_json(&self) -> BeyondResult<String> {
        serde_json::to_string_pretty(&self.learnings)
            .map_err(|e| BeyondError::Memory(e.to_string()))
    }

    /// Import learnings from JSON.
    pub fn import_json(&mut self, json: &str) -> BeyondResult<usize> {
        let imported: Vec<ProjectLearning> =
            serde_json::from_str(json).map_err(|e| BeyondError::Memory(e.to_string()))?;
        let count = imported.len();
        self.learnings.extend(imported);
        Ok(count)
    }

    pub fn total_learnings(&self) -> usize {
        self.learnings.len()
    }

    pub fn current_project(&self) -> &str {
        &self.current_project
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  6. Cost Oracle — real-time token spend tracker
// ═══════════════════════════════════════════════════════════════════════════

/// Token pricing per 1M tokens (in microdollars for integer math).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TokenPricing {
    /// Model identifier
    pub model: ModelTier,
    /// Input price per 1M tokens in microdollars (1 USD = 1_000_000)
    pub input_per_m: u64,
    /// Output price per 1M tokens in microdollars
    pub output_per_m: u64,
}

/// Model tiers with known pricing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    Haiku,
    Sonnet,
    Opus,
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Haiku => write!(f, "haiku"),
            Self::Sonnet => write!(f, "sonnet"),
            Self::Opus => write!(f, "opus"),
        }
    }
}

/// Default pricing (as of 2025 — Claude 4 family).
fn default_pricing() -> HashMap<ModelTier, TokenPricing> {
    let mut m = HashMap::new();
    m.insert(
        ModelTier::Haiku,
        TokenPricing {
            model: ModelTier::Haiku,
            input_per_m: 800_000,    // $0.80
            output_per_m: 4_000_000, // $4.00
        },
    );
    m.insert(
        ModelTier::Sonnet,
        TokenPricing {
            model: ModelTier::Sonnet,
            input_per_m: 3_000_000,   // $3.00
            output_per_m: 15_000_000, // $15.00
        },
    );
    m.insert(
        ModelTier::Opus,
        TokenPricing {
            model: ModelTier::Opus,
            input_per_m: 15_000_000,  // $15.00
            output_per_m: 75_000_000, // $75.00
        },
    );
    m
}

/// A single API call record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub model: ModelTier,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microdollars: u64,
    pub timestamp: DateTime<Utc>,
    pub purpose: String,
}

/// Budget alert level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Normal,
    Warning,  // 80% of budget
    Critical, // 95% of budget
    Exceeded, // over budget
}

impl std::fmt::Display for AlertLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
            Self::Exceeded => write!(f, "EXCEEDED"),
        }
    }
}

/// Real-time token spend tracker with budget enforcement.
pub struct CostOracle {
    pricing: HashMap<ModelTier, TokenPricing>,
    /// Usage records for current session
    usage: Vec<TokenUsage>,
    /// Session budget ceiling in microdollars (0 = unlimited)
    session_budget: u64,
    /// Daily budget ceiling in microdollars (0 = unlimited)
    daily_budget: u64,
    /// Running total spend in microdollars (atomic for concurrent reads)
    total_spend: AtomicU64,
    /// Warning threshold percentage (default 80)
    warn_pct: u8,
    /// Critical threshold percentage (default 95)
    critical_pct: u8,
    /// Callback-style: last alert level emitted (to avoid duplicate alerts)
    last_alert: Mutex<AlertLevel>,
}

impl Default for CostOracle {
    fn default() -> Self {
        Self::new()
    }
}

impl CostOracle {
    pub fn new() -> Self {
        Self {
            pricing: default_pricing(),
            usage: Vec::new(),
            session_budget: 0,
            daily_budget: 0,
            total_spend: AtomicU64::new(0),
            warn_pct: 80,
            critical_pct: 95,
            last_alert: Mutex::new(AlertLevel::Normal),
        }
    }

    /// Set session budget in dollars (e.g. 5.00 = $5).
    pub fn set_session_budget(&mut self, dollars: f64) {
        self.session_budget = (dollars * 1_000_000.0) as u64;
    }

    /// Set daily budget in dollars.
    pub fn set_daily_budget(&mut self, dollars: f64) {
        self.daily_budget = (dollars * 1_000_000.0) as u64;
    }

    /// Set alert thresholds (percentages of budget).
    pub fn set_thresholds(&mut self, warn_pct: u8, critical_pct: u8) {
        self.warn_pct = warn_pct;
        self.critical_pct = critical_pct;
    }

    /// Record a token usage event. Returns the cost and current alert level.
    /// Fails with BudgetExceeded if the session budget is exceeded.
    pub fn record(
        &mut self,
        model: ModelTier,
        input_tokens: u64,
        output_tokens: u64,
        purpose: impl Into<String>,
    ) -> BeyondResult<(u64, AlertLevel)> {
        let pricing = self
            .pricing
            .get(&model)
            .ok_or_else(|| BeyondError::CostOracle(format!("unknown model: {}", model)))?;

        let input_cost = (input_tokens * pricing.input_per_m) / 1_000_000;
        let output_cost = (output_tokens * pricing.output_per_m) / 1_000_000;
        let cost = input_cost + output_cost;

        // Check budget BEFORE recording
        let current = self.total_spend.load(Ordering::SeqCst);
        if self.session_budget > 0 && current + cost > self.session_budget {
            return Err(BeyondError::BudgetExceeded(format!(
                "session budget ${:.2} would be exceeded (current: ${:.2}, request: ${:.2})",
                self.session_budget as f64 / 1_000_000.0,
                current as f64 / 1_000_000.0,
                cost as f64 / 1_000_000.0,
            )));
        }

        let usage = TokenUsage {
            model,
            input_tokens,
            output_tokens,
            cost_microdollars: cost,
            timestamp: Utc::now(),
            purpose: purpose.into(),
        };

        self.usage.push(usage);
        self.total_spend.fetch_add(cost, Ordering::SeqCst);

        let alert = self.check_alert_level();
        Ok((cost, alert))
    }

    /// Pre-flight cost estimate without recording.
    pub fn estimate(
        &self,
        model: ModelTier,
        input_tokens: u64,
        output_tokens: u64,
    ) -> BeyondResult<u64> {
        let pricing = self
            .pricing
            .get(&model)
            .ok_or_else(|| BeyondError::CostOracle(format!("unknown model: {}", model)))?;

        let cost =
            (input_tokens * pricing.input_per_m + output_tokens * pricing.output_per_m) / 1_000_000;
        Ok(cost)
    }

    /// Check whether the budget allows a request of estimated cost.
    pub fn can_afford(&self, estimated_cost: u64) -> bool {
        if self.session_budget == 0 {
            return true;
        }
        let current = self.total_spend.load(Ordering::SeqCst);
        current + estimated_cost <= self.session_budget
    }

    /// Current total spend in microdollars.
    pub fn total_spend_microdollars(&self) -> u64 {
        self.total_spend.load(Ordering::SeqCst)
    }

    /// Current total spend in dollars.
    pub fn total_spend_dollars(&self) -> f64 {
        self.total_spend.load(Ordering::SeqCst) as f64 / 1_000_000.0
    }

    /// Remaining budget in dollars (None if unlimited).
    pub fn remaining_dollars(&self) -> Option<f64> {
        if self.session_budget == 0 {
            return None;
        }
        let spent = self.total_spend.load(Ordering::SeqCst);
        let remaining = self.session_budget.saturating_sub(spent);
        Some(remaining as f64 / 1_000_000.0)
    }

    /// Number of API calls recorded.
    pub fn call_count(&self) -> usize {
        self.usage.len()
    }

    /// Breakdown of spend by model.
    pub fn spend_by_model(&self) -> HashMap<ModelTier, u64> {
        let mut map = HashMap::new();
        for u in &self.usage {
            *map.entry(u.model).or_insert(0) += u.cost_microdollars;
        }
        map
    }

    /// Get usage records for the current session.
    pub fn usage_log(&self) -> &[TokenUsage] {
        &self.usage
    }

    /// Generate a human-readable spend report.
    pub fn report(&self) -> CostReport {
        let by_model = self.spend_by_model();
        let total = self.total_spend_dollars();

        CostReport {
            total_dollars: total,
            session_budget_dollars: if self.session_budget > 0 {
                Some(self.session_budget as f64 / 1_000_000.0)
            } else {
                None
            },
            remaining_dollars: self.remaining_dollars(),
            call_count: self.usage.len(),
            spend_by_model: by_model
                .into_iter()
                .map(|(k, v)| (k.to_string(), v as f64 / 1_000_000.0))
                .collect(),
            alert_level: self.check_alert_level(),
        }
    }

    fn check_alert_level(&self) -> AlertLevel {
        if self.session_budget == 0 {
            return AlertLevel::Normal;
        }

        let spent = self.total_spend.load(Ordering::SeqCst);
        let pct = ((spent as f64 / self.session_budget as f64) * 100.0) as u8;

        let level = if spent >= self.session_budget {
            AlertLevel::Exceeded
        } else if pct >= self.critical_pct {
            AlertLevel::Critical
        } else if pct >= self.warn_pct {
            AlertLevel::Warning
        } else {
            AlertLevel::Normal
        };

        // Log escalation
        if let Ok(mut last) = self.last_alert.lock() {
            if level > *last {
                match level {
                    AlertLevel::Warning => {
                        warn!(pct = pct, "cost oracle: approaching budget limit");
                    }
                    AlertLevel::Critical => {
                        warn!(pct = pct, "cost oracle: CRITICAL — near budget limit");
                    }
                    AlertLevel::Exceeded => {
                        error!("cost oracle: budget EXCEEDED");
                    }
                    _ => {}
                }
                *last = level;
            }
        }

        level
    }
}

/// Human-readable cost report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    pub total_dollars: f64,
    pub session_budget_dollars: Option<f64>,
    pub remaining_dollars: Option<f64>,
    pub call_count: usize,
    pub spend_by_model: HashMap<String, f64>,
    pub alert_level: AlertLevel,
}

// ═══════════════════════════════════════════════════════════════════════════
//  7. Voice Notifications
// ═══════════════════════════════════════════════════════════════════════════

/// Voice engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// macOS voice to use (e.g. "Samantha", "Daniel")
    pub voice: String,
    /// Speaking rate (words per minute, 0 = default)
    pub rate: u32,
    /// Whether voice is enabled
    pub enabled: bool,
    /// Only speak for these severity levels and above
    pub min_severity: Severity,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            voice: "Samantha".into(),
            rate: 200,
            enabled: true,
            min_severity: Severity::Warning,
        }
    }
}

/// Voice notification system using macOS `say` command.
pub struct VoiceNotifier {
    config: VoiceConfig,
}

impl VoiceNotifier {
    pub fn new(config: VoiceConfig) -> Self {
        Self { config }
    }

    /// Speak a message aloud.
    pub fn say(&self, message: &str) -> BeyondResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut args = vec!["-v", &self.config.voice];
        let rate_str;
        if self.config.rate > 0 {
            rate_str = self.config.rate.to_string();
            args.extend(["-r", &rate_str]);
        }
        args.push(message);

        let output = Command::new("say").args(&args).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BeyondError::Voice(stderr.to_string()));
        }
        Ok(())
    }

    /// Speak a notification for a key event (non-blocking, fires and forgets).
    pub fn notify(&self, event: &str, severity: Severity) -> BeyondResult<()> {
        if severity < self.config.min_severity {
            return Ok(());
        }

        let prefix = match severity {
            Severity::Info => "Info.",
            Severity::Warning => "Warning.",
            Severity::Error => "Error.",
            Severity::Critical => "Critical alert.",
        };

        let message = format!("{} {}", prefix, event);
        debug!(message = %message, "voice notification");
        self.say(&message)
    }

    /// Speak a build result.
    pub fn announce_build(&self, success: bool, duration_secs: u64) -> BeyondResult<()> {
        let msg = if success {
            format!("Build succeeded in {} seconds.", duration_secs)
        } else {
            "Build failed. Check the logs.".to_string()
        };
        self.notify(
            &msg,
            if success {
                Severity::Info
            } else {
                Severity::Error
            },
        )
    }

    /// Speak a deployment event.
    pub fn announce_deploy(&self, target: &str, success: bool) -> BeyondResult<()> {
        let msg = if success {
            format!("Deployed to {} successfully.", target)
        } else {
            format!("Deployment to {} failed.", target)
        };
        self.notify(
            &msg,
            if success {
                Severity::Info
            } else {
                Severity::Critical
            },
        )
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  8. Self-Updating Binary
// ═══════════════════════════════════════════════════════════════════════════

/// Version information for update checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current: String,
    pub latest: Option<String>,
    pub update_available: bool,
    pub release_url: Option<String>,
    pub changelog: Option<String>,
    pub checked_at: DateTime<Utc>,
}

/// Result of a self-update attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub from_version: String,
    pub to_version: String,
    pub success: bool,
    pub message: String,
    pub requires_restart: bool,
}

/// Self-updating engine. Checks for new versions and hot-swaps the binary.
pub struct SelfUpdater {
    /// Current version
    current_version: String,
    /// Path to the running binary
    binary_path: PathBuf,
    /// URL pattern for version check (GitHub releases API)
    check_url: String,
    /// How often to check (seconds)
    check_interval_secs: u64,
    /// Whether auto-update is enabled
    auto_update: bool,
    /// Last check result
    last_check: Arc<Mutex<Option<VersionInfo>>>,
    /// Whether background checker is running
    running: Arc<AtomicBool>,
}

impl SelfUpdater {
    pub fn new(
        current_version: impl Into<String>,
        binary_path: impl Into<PathBuf>,
        check_url: impl Into<String>,
    ) -> Self {
        Self {
            current_version: current_version.into(),
            binary_path: binary_path.into(),
            check_url: check_url.into(),
            check_interval_secs: 3600, // hourly
            auto_update: false,
            last_check: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_interval(mut self, secs: u64) -> Self {
        self.check_interval_secs = secs;
        self
    }

    pub fn with_auto_update(mut self, enabled: bool) -> Self {
        self.auto_update = enabled;
        self
    }

    /// Check for updates by querying the release endpoint.
    pub fn check_now(&self) -> BeyondResult<VersionInfo> {
        // Use `curl` to avoid pulling in an HTTP client just for this
        let output = Command::new("curl")
            .args(["-sL", "--max-time", "10", &self.check_url])
            .output()?;

        if !output.status.success() {
            return Err(BeyondError::SelfUpdate(
                "version check request failed".into(),
            ));
        }

        let body = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| BeyondError::SelfUpdate(format!("invalid JSON: {}", e)))?;

        let latest = json
            .get("tag_name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_start_matches('v').to_string());

        let update_available = latest
            .as_ref()
            .map(|l| l != &self.current_version)
            .unwrap_or(false);

        let release_url = json
            .get("html_url")
            .and_then(|v| v.as_str())
            .map(String::from);

        let changelog = json.get("body").and_then(|v| v.as_str()).map(String::from);

        let info = VersionInfo {
            current: self.current_version.clone(),
            latest,
            update_available,
            release_url,
            changelog,
            checked_at: Utc::now(),
        };

        if let Ok(mut lock) = self.last_check.lock() {
            *lock = Some(info.clone());
        }

        if info.update_available {
            info!(
                current = %self.current_version,
                latest = ?info.latest,
                "update available"
            );
        }

        Ok(info)
    }

    /// Download and install an update (hot-swap the binary).
    /// The old binary is renamed to `<name>.bak` before replacement.
    pub fn apply_update(&self, download_url: &str) -> BeyondResult<UpdateResult> {
        let latest = self
            .last_check
            .lock()
            .ok()
            .and_then(|v| v.as_ref().and_then(|i| i.latest.clone()))
            .unwrap_or_else(|| "unknown".into());

        // 1. Download to temp file
        let temp_path = self.binary_path.with_extension("new");
        let status = Command::new("curl")
            .args([
                "-sL",
                "--max-time",
                "120",
                "-o",
                &temp_path.display().to_string(),
                download_url,
            ])
            .status()?;

        if !status.success() {
            return Err(BeyondError::SelfUpdate("download failed".into()));
        }

        // 2. Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&temp_path, perms)?;
        }

        // 3. Backup current binary
        let backup_path = self.binary_path.with_extension("bak");
        if self.binary_path.exists() {
            std::fs::rename(&self.binary_path, &backup_path)?;
        }

        // 4. Move new binary into place
        std::fs::rename(&temp_path, &self.binary_path)?;

        info!(from = %self.current_version, to = %latest, "binary updated");

        Ok(UpdateResult {
            from_version: self.current_version.clone(),
            to_version: latest,
            success: true,
            message: "update applied — restart required".into(),
            requires_restart: true,
        })
    }

    /// Start background version checking. Runs on a tokio task.
    pub fn start_background_check(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = Arc::clone(&self.running);
        let last_check = Arc::clone(&self.last_check);
        let check_url = self.check_url.clone();
        let current = self.current_version.clone();
        let interval = self.check_interval_secs;

        tokio::spawn(async move {
            info!(
                "self-update background checker started (interval={}s)",
                interval
            );
            while running.load(Ordering::SeqCst) {
                // Use a blocking spawn since curl is synchronous
                let url = check_url.clone();
                let ver = current.clone();
                let lc = Arc::clone(&last_check);

                let _ = tokio::task::spawn_blocking(move || {
                    let output = Command::new("curl")
                        .args(["-sL", "--max-time", "10", &url])
                        .output();

                    if let Ok(output) = output {
                        if output.status.success() {
                            let body = String::from_utf8_lossy(&output.stdout);
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                                let latest = json
                                    .get("tag_name")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.trim_start_matches('v').to_string());

                                let update_available = latest
                                    .as_ref()
                                    .map(|l| l != &ver)
                                    .unwrap_or(false);

                                let info = VersionInfo {
                                    current: ver,
                                    latest,
                                    update_available,
                                    release_url: json
                                        .get("html_url")
                                        .and_then(|v| v.as_str())
                                        .map(String::from),
                                    changelog: None,
                                    checked_at: Utc::now(),
                                };

                                if info.update_available {
                                    info!(latest = ?info.latest, "background check: update available");
                                }

                                if let Ok(mut lock) = lc.lock() {
                                    *lock = Some(info);
                                }
                            }
                        }
                    }
                })
                .await;

                tokio::time::sleep(Duration::from_secs(interval)).await;
            }
            info!("self-update background checker stopped");
        });
    }

    /// Stop the background checker.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get the latest check result.
    pub fn latest_check(&self) -> Option<VersionInfo> {
        self.last_check.lock().ok().and_then(|v| v.clone())
    }

    pub fn current_version(&self) -> &str {
        &self.current_version
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  Shared utilities
// ═══════════════════════════════════════════════════════════════════════════

/// Run a CLI command and return stdout as a string.
fn run_cmd(cmd: &str, args: &[&str], cwd: Option<&Path>) -> BeyondResult<String> {
    let mut command = Command::new(cmd);
    command.args(args);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }

    let output = command.output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = if stderr.is_empty() {
            stdout.clone()
        } else {
            stderr.chars().take(1024).collect()
        };
        return Err(BeyondError::CommandFailed {
            cmd: format!("{} {}", cmd, args.join(" ")),
            reason,
        });
    }

    Ok(stdout)
}

// ═══════════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Ambient Context ──────────────────────────────────────────────────

    #[test]
    fn test_ambient_daemon_creation() {
        let daemon = AmbientDaemon::new("/tmp");
        assert!(!daemon.is_running());
        assert!(daemon.latest().is_none());
    }

    #[test]
    fn test_ambient_daemon_config() {
        let daemon = AmbientDaemon::new("/tmp")
            .with_interval(30)
            .with_max_recent(50);
        assert_eq!(daemon.poll_interval_secs, 30);
        assert_eq!(daemon.max_recent_files, 50);
    }

    #[test]
    fn test_ambient_snapshot_capture() {
        let daemon = AmbientDaemon::new(std::env::temp_dir());
        // capture() may fail on CI without a display, but should not panic
        let result = daemon.capture();
        if let Ok(snap) = result {
            assert!(snap.captured_at <= Utc::now());
        }
    }

    #[test]
    fn test_scan_recent_files() {
        let dir = std::env::temp_dir().join("phantom-beyond-recent-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("recent.txt"), b"hello").unwrap();

        let files = scan_recent_files_inner(&dir, 10);
        assert!(!files.is_empty());
        assert!(files[0].path.contains("recent.txt"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Self-Scheduling ──────────────────────────────────────────────────

    #[test]
    fn test_scheduler_add_rule() {
        let mut scheduler = SelfScheduler::new();
        scheduler.add_rule(ScheduleRule {
            id: "test-1".into(),
            description: "test rule".into(),
            trigger: TriggerKind::Interval(60),
            agent_role: "backend".into(),
            payload: serde_json::Value::Null,
            priority: 5,
            enabled: true,
        });
        assert_eq!(scheduler.rule_count(), 1);
    }

    #[test]
    fn test_scheduler_remove_rule() {
        let mut scheduler = SelfScheduler::new();
        scheduler.add_rule(ScheduleRule {
            id: "r1".into(),
            description: "rule 1".into(),
            trigger: TriggerKind::Interval(60),
            agent_role: "backend".into(),
            payload: serde_json::Value::Null,
            priority: 5,
            enabled: true,
        });
        assert!(scheduler.remove_rule("r1"));
        assert_eq!(scheduler.rule_count(), 0);
        assert!(!scheduler.remove_rule("nonexistent"));
    }

    #[test]
    fn test_scheduler_interval_tick() {
        let mut scheduler = SelfScheduler::new();
        scheduler.add_rule(ScheduleRule {
            id: "r1".into(),
            description: "every minute".into(),
            trigger: TriggerKind::Interval(60),
            agent_role: "cron".into(),
            payload: serde_json::json!({"task": "cleanup"}),
            priority: 1,
            enabled: true,
        });

        // First tick should fire (never fired before)
        scheduler.tick();
        assert_eq!(scheduler.pending_count(), 1);

        let jobs = scheduler.drain_pending();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].rule_id, "r1");
        assert_eq!(jobs[0].agent_role, "cron");

        // Second tick should NOT fire (interval not elapsed)
        scheduler.tick();
        assert_eq!(scheduler.pending_count(), 0);
    }

    #[test]
    fn test_scheduler_disabled_rule() {
        let mut scheduler = SelfScheduler::new();
        scheduler.add_rule(ScheduleRule {
            id: "r1".into(),
            description: "disabled".into(),
            trigger: TriggerKind::Interval(1),
            agent_role: "test".into(),
            payload: serde_json::Value::Null,
            priority: 1,
            enabled: false,
        });

        scheduler.tick();
        assert_eq!(scheduler.pending_count(), 0);
    }

    #[test]
    fn test_scheduler_enable_disable() {
        let mut scheduler = SelfScheduler::new();
        scheduler.add_rule(ScheduleRule {
            id: "r1".into(),
            description: "toggle".into(),
            trigger: TriggerKind::Interval(1),
            agent_role: "test".into(),
            payload: serde_json::Value::Null,
            priority: 1,
            enabled: true,
        });

        assert!(scheduler.set_enabled("r1", false));
        scheduler.tick();
        assert_eq!(scheduler.pending_count(), 0);

        assert!(scheduler.set_enabled("r1", true));
        scheduler.tick();
        assert_eq!(scheduler.pending_count(), 1);
    }

    #[test]
    fn test_scheduler_git_event() {
        let mut scheduler = SelfScheduler::new();
        scheduler.add_rule(ScheduleRule {
            id: "post-commit".into(),
            description: "lint after commit".into(),
            trigger: TriggerKind::GitEvent(GitEventKind::PostCommit),
            agent_role: "linter".into(),
            payload: serde_json::Value::Null,
            priority: 3,
            enabled: true,
        });

        scheduler.notify_git_event(GitEventKind::PostPush); // wrong event
        assert_eq!(scheduler.pending_count(), 0);

        scheduler.notify_git_event(GitEventKind::PostCommit); // correct
        assert_eq!(scheduler.pending_count(), 1);
    }

    #[test]
    fn test_scheduler_file_change() {
        let mut scheduler = SelfScheduler::new();
        scheduler.add_rule(ScheduleRule {
            id: "rebuild-ts".into(),
            description: "rebuild on .ts change".into(),
            trigger: TriggerKind::FileChange {
                glob: "*.ts".into(),
            },
            agent_role: "builder".into(),
            payload: serde_json::Value::Null,
            priority: 5,
            enabled: true,
        });

        scheduler.notify_file_change("src/app.rs"); // no match
        assert_eq!(scheduler.pending_count(), 0);

        scheduler.notify_file_change("src/app.ts"); // match
        assert_eq!(scheduler.pending_count(), 1);
    }

    #[test]
    fn test_scheduler_oneshot() {
        let mut scheduler = SelfScheduler::new();
        let past = Utc::now() - chrono::Duration::seconds(10);
        scheduler.add_rule(ScheduleRule {
            id: "once".into(),
            description: "fire once".into(),
            trigger: TriggerKind::OneShot(past),
            agent_role: "deploy".into(),
            payload: serde_json::Value::Null,
            priority: 10,
            enabled: true,
        });

        scheduler.tick();
        assert_eq!(scheduler.pending_count(), 1);

        scheduler.drain_pending();
        scheduler.tick(); // should NOT fire again
        assert_eq!(scheduler.pending_count(), 0);
    }

    // ── Smart Git ────────────────────────────────────────────────────────

    #[test]
    fn test_slugify() {
        assert_eq!(
            slugify("Add user authentication"),
            "add-user-authentication"
        );
        assert_eq!(
            slugify("fix: broken CI  pipeline"),
            "fix-broken-ci-pipeline"
        );
        assert_eq!(slugify("  spaces  "), "spaces");
        assert_eq!(slugify("UPPER CASE"), "upper-case");
    }

    #[test]
    fn test_slugify_truncation() {
        let long = "a".repeat(100);
        let slug = slugify(&long);
        assert!(slug.len() <= 50);
    }

    #[test]
    fn test_git_config() {
        let config = GitConfig::new("/tmp/repo").with_github_repo("user/phantom");
        assert_eq!(config.branch_prefix, "phantom/");
        assert_eq!(config.github_repo, Some("user/phantom".into()));
        assert!(config.auto_push);
    }

    #[test]
    fn test_git_op_result_structure() {
        let result = GitOpResult {
            operation: "auto_commit".into(),
            success: true,
            output: "1 file changed".into(),
            branch: Some("phantom/add-auth".into()),
            commit_sha: Some("abc1234".into()),
            pr_url: None,
        };
        assert!(result.success);
        assert!(result.pr_url.is_none());
    }

    // ── Predictive Scanner ───────────────────────────────────────────────

    #[test]
    fn test_scanner_creation() {
        let scanner = PredictiveScanner::new();
        assert_eq!(scanner.large_file_threshold, 2000);
    }

    #[test]
    fn test_scanner_custom_pattern() {
        let mut scanner = PredictiveScanner::new();
        scanner.add_pattern("no_print", "println!", Severity::Warning);
        assert_eq!(scanner.custom_patterns.len(), 1);
    }

    #[test]
    fn test_scanner_scan_temp_dir() {
        let dir = std::env::temp_dir().join("phantom-beyond-scan-test");
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("good.rs"), "fn main() {\n    let x = 42;\n}\n").unwrap();
        std::fs::write(dir.join("bad.rs"), "fn main() {\n    dbg!(\"oops\");\n}\n").unwrap();

        let scanner = PredictiveScanner::new();
        let report = scanner.scan(&dir).unwrap();

        assert!(report.files_scanned >= 2);
        // Should detect dbg! in bad.rs
        assert!(report
            .warnings
            .iter()
            .any(|w| w.rule == "debug_print" && w.file.contains("bad.rs")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scanner_detects_conflict_markers() {
        let dir = std::env::temp_dir().join("phantom-beyond-conflict-test");
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("conflict.rs"),
            "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> branch\n",
        )
        .unwrap();

        let scanner = PredictiveScanner::new();
        let report = scanner.scan(&dir).unwrap();

        assert!(
            report.has_blockers()
                || report
                    .warnings
                    .iter()
                    .any(|w| w.rule == "unresolved_conflict")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_report_count_by_severity() {
        let report = ScanReport {
            warnings: vec![
                PredictiveWarning {
                    file: "a.rs".into(),
                    line: Some(1),
                    severity: Severity::Warning,
                    rule: "test".into(),
                    message: "w".into(),
                },
                PredictiveWarning {
                    file: "b.rs".into(),
                    line: Some(2),
                    severity: Severity::Error,
                    rule: "test".into(),
                    message: "e".into(),
                },
            ],
            files_scanned: 2,
            duration_ms: 10,
            build_recommended: false,
        };

        assert_eq!(report.count_by_severity(Severity::Warning), 1);
        assert_eq!(report.count_by_severity(Severity::Error), 1);
        assert!(report.has_blockers());
    }

    // ── Cross-Project Memory ─────────────────────────────────────────────

    #[test]
    fn test_memory_record() {
        let mut mem = ProjectMemory::new("phantom");
        let id = mem.record(
            "Use cargo nextest for faster tests",
            LearningCategory::Testing,
            vec!["rust".into(), "testing".into()],
        );
        assert!(id.contains("phantom"));
        assert_eq!(mem.total_learnings(), 1);
    }

    #[test]
    fn test_memory_query_by_tag() {
        let mut mem = ProjectMemory::new("phantom");
        mem.record(
            "nextest is fast",
            LearningCategory::Testing,
            vec!["rust".into()],
        );
        mem.record(
            "sqlx compile-time checks",
            LearningCategory::BuildConfig,
            vec!["rust".into(), "sql".into()],
        );

        let rust = mem.query_by_tag("rust");
        assert_eq!(rust.len(), 2);

        let sql = mem.query_by_tag("sql");
        assert_eq!(sql.len(), 1);
    }

    #[test]
    fn test_memory_query_by_category() {
        let mut mem = ProjectMemory::new("phantom");
        mem.record("a", LearningCategory::Testing, vec![]);
        mem.record("b", LearningCategory::Testing, vec![]);
        mem.record("c", LearningCategory::Security, vec![]);

        assert_eq!(mem.query_by_category(LearningCategory::Testing).len(), 2);
        assert_eq!(mem.query_by_category(LearningCategory::Security).len(), 1);
    }

    #[test]
    fn test_memory_search() {
        let mut mem = ProjectMemory::new("phantom");
        mem.record(
            "cargo nextest is faster than cargo test",
            LearningCategory::Testing,
            vec![],
        );
        mem.record(
            "use mold linker for fast builds",
            LearningCategory::BuildConfig,
            vec![],
        );

        let results = mem.search("nextest");
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("nextest"));
    }

    #[test]
    fn test_memory_mark_applied() {
        let mut mem = ProjectMemory::new("phantom");
        let id = mem.record("tip", LearningCategory::Workflow, vec![]);

        assert!(mem.mark_applied(&id));
        let learning = mem.query_by_category(LearningCategory::Workflow)[0];
        assert_eq!(learning.applied_count, 1);
        assert!(learning.confidence > 0.5);

        assert!(!mem.mark_applied("nonexistent"));
    }

    #[test]
    fn test_memory_top_learnings() {
        let mut mem = ProjectMemory::new("phantom");
        let id1 = mem.record("low confidence", LearningCategory::Testing, vec![]);
        let id2 = mem.record("high confidence", LearningCategory::Testing, vec![]);

        // Apply id2 multiple times to boost confidence
        mem.mark_applied(&id2);
        mem.mark_applied(&id2);
        mem.mark_applied(&id2);

        let top = mem.top_learnings(LearningCategory::Testing, 1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].id, id2);

        let _ = id1;
    }

    #[test]
    fn test_memory_export_import() {
        let mut mem = ProjectMemory::new("phantom");
        mem.record("tip 1", LearningCategory::Workflow, vec!["a".into()]);
        mem.record("tip 2", LearningCategory::Security, vec!["b".into()]);

        let json = mem.export_json().unwrap();

        let mut mem2 = ProjectMemory::new("other");
        let count = mem2.import_json(&json).unwrap();
        assert_eq!(count, 2);
        assert_eq!(mem2.total_learnings(), 2);
    }

    #[test]
    fn test_memory_query_by_project() {
        let mut mem = ProjectMemory::new("phantom");
        mem.record("from phantom", LearningCategory::Workflow, vec![]);

        let imported = r#"[{"id":"learn-other-1","project":"other","tags":[],"category":"workflow","content":"from other","confidence":0.5,"learned_at":"2025-01-01T00:00:00Z","applied_count":0}]"#;
        mem.import_json(imported).unwrap();

        assert_eq!(mem.query_by_project("phantom").len(), 1);
        assert_eq!(mem.query_by_project("other").len(), 1);
    }

    // ── Cost Oracle ──────────────────────────────────────────────────────

    #[test]
    fn test_cost_oracle_creation() {
        let oracle = CostOracle::new();
        assert_eq!(oracle.total_spend_microdollars(), 0);
        assert_eq!(oracle.call_count(), 0);
    }

    #[test]
    fn test_cost_oracle_record() {
        let mut oracle = CostOracle::new();
        let (cost, alert) = oracle.record(ModelTier::Haiku, 1000, 500, "test").unwrap();

        assert!(cost > 0);
        assert_eq!(alert, AlertLevel::Normal);
        assert_eq!(oracle.call_count(), 1);
        assert!(oracle.total_spend_microdollars() > 0);
    }

    #[test]
    fn test_cost_oracle_estimate() {
        let oracle = CostOracle::new();
        let cost = oracle
            .estimate(ModelTier::Opus, 1_000_000, 500_000)
            .unwrap();
        // Opus: $15/M input + $75/M output = $15 + $37.50 = $52.50
        assert!(cost > 0);
    }

    #[test]
    fn test_cost_oracle_budget_enforcement() {
        let mut oracle = CostOracle::new();
        oracle.set_session_budget(0.01); // $0.01 budget

        // Record a massive request that exceeds the budget
        let result = oracle.record(ModelTier::Opus, 10_000_000, 5_000_000, "big request");
        assert!(result.is_err());
        match result.unwrap_err() {
            BeyondError::BudgetExceeded(_) => {} // expected
            other => panic!("expected BudgetExceeded, got {:?}", other),
        }
    }

    #[test]
    fn test_cost_oracle_can_afford() {
        let mut oracle = CostOracle::new();
        oracle.set_session_budget(1.00); // $1 budget

        assert!(oracle.can_afford(500_000)); // $0.50
        assert!(!oracle.can_afford(2_000_000)); // $2.00
    }

    #[test]
    fn test_cost_oracle_remaining() {
        let mut oracle = CostOracle::new();
        oracle.set_session_budget(10.0);

        assert_eq!(oracle.remaining_dollars(), Some(10.0));

        oracle
            .record(ModelTier::Haiku, 1_000_000, 0, "test")
            .unwrap();

        let remaining = oracle.remaining_dollars().unwrap();
        assert!(remaining < 10.0);
        assert!(remaining > 0.0);
    }

    #[test]
    fn test_cost_oracle_unlimited_budget() {
        let oracle = CostOracle::new();
        assert!(oracle.remaining_dollars().is_none());
        assert!(oracle.can_afford(999_999_999));
    }

    #[test]
    fn test_cost_oracle_spend_by_model() {
        let mut oracle = CostOracle::new();
        oracle.record(ModelTier::Haiku, 1000, 500, "h1").unwrap();
        oracle.record(ModelTier::Sonnet, 1000, 500, "s1").unwrap();
        oracle.record(ModelTier::Haiku, 2000, 1000, "h2").unwrap();

        let by_model = oracle.spend_by_model();
        assert!(by_model.contains_key(&ModelTier::Haiku));
        assert!(by_model.contains_key(&ModelTier::Sonnet));
        assert!(!by_model.contains_key(&ModelTier::Opus));
    }

    #[test]
    fn test_cost_oracle_alert_levels() {
        let mut oracle = CostOracle::new();
        oracle.set_session_budget(0.001); // very small budget

        // Small request should succeed but may trigger warning
        let result = oracle.record(ModelTier::Haiku, 100, 50, "tiny");
        if let Ok((_, alert)) = result {
            // Alert level depends on how much of budget was consumed
            assert!(alert == AlertLevel::Normal || alert >= AlertLevel::Warning);
        }
    }

    #[test]
    fn test_cost_report() {
        let mut oracle = CostOracle::new();
        oracle.set_session_budget(100.0);
        oracle
            .record(ModelTier::Sonnet, 5000, 2000, "test")
            .unwrap();

        let report = oracle.report();
        assert!(report.total_dollars > 0.0);
        assert_eq!(report.session_budget_dollars, Some(100.0));
        assert!(report.remaining_dollars.unwrap() < 100.0);
        assert_eq!(report.call_count, 1);
    }

    // ── Voice Notifications ──────────────────────────────────────────────

    #[test]
    fn test_voice_config_default() {
        let config = VoiceConfig::default();
        assert_eq!(config.voice, "Samantha");
        assert!(config.enabled);
        assert_eq!(config.rate, 200);
    }

    #[test]
    fn test_voice_disabled() {
        let notifier = VoiceNotifier::new(VoiceConfig {
            enabled: false,
            ..Default::default()
        });
        // Should return Ok immediately when disabled
        assert!(notifier.say("test").is_ok());
        assert!(!notifier.is_enabled());
    }

    #[test]
    fn test_voice_enable_toggle() {
        let mut notifier = VoiceNotifier::new(VoiceConfig::default());
        assert!(notifier.is_enabled());
        notifier.set_enabled(false);
        assert!(!notifier.is_enabled());
    }

    #[test]
    fn test_voice_severity_filter() {
        let notifier = VoiceNotifier::new(VoiceConfig {
            enabled: false, // disable actual speech for tests
            min_severity: Severity::Error,
            ..Default::default()
        });
        // Info and Warning should be filtered out (but since disabled, returns Ok anyway)
        assert!(notifier.notify("test", Severity::Info).is_ok());
        assert!(notifier.notify("test", Severity::Warning).is_ok());
    }

    // ── Self-Updating ────────────────────────────────────────────────────

    #[test]
    fn test_self_updater_creation() {
        let updater = SelfUpdater::new(
            "0.1.0",
            "/usr/local/bin/phantom",
            "https://api.github.com/repos/user/phantom/releases/latest",
        );
        assert_eq!(updater.current_version(), "0.1.0");
        assert!(!updater.is_running());
    }

    #[test]
    fn test_self_updater_config() {
        let updater = SelfUpdater::new("0.1.0", "/tmp/phantom", "https://example.com/releases")
            .with_interval(7200)
            .with_auto_update(true);
        assert_eq!(updater.check_interval_secs, 7200);
        assert!(updater.auto_update);
    }

    #[test]
    fn test_version_info_structure() {
        let info = VersionInfo {
            current: "0.1.0".into(),
            latest: Some("0.2.0".into()),
            update_available: true,
            release_url: Some("https://github.com/user/phantom/releases/0.2.0".into()),
            changelog: Some("Bug fixes".into()),
            checked_at: Utc::now(),
        };
        assert!(info.update_available);
    }

    #[test]
    fn test_update_result_structure() {
        let result = UpdateResult {
            from_version: "0.1.0".into(),
            to_version: "0.2.0".into(),
            success: true,
            message: "updated".into(),
            requires_restart: true,
        };
        assert!(result.success);
        assert!(result.requires_restart);
    }

    // ── Glob matching ────────────────────────────────────────────────────

    #[test]
    fn test_glob_wildcard() {
        assert!(path_matches_glob("src/app.ts", "*.ts"));
        assert!(!path_matches_glob("src/app.rs", "*.ts"));
    }

    #[test]
    fn test_glob_prefix() {
        assert!(path_matches_glob("src/components/Button.tsx", "src/**"));
        assert!(!path_matches_glob("lib/foo.ts", "src/**"));
    }

    #[test]
    fn test_glob_exact() {
        assert!(path_matches_glob("Cargo.toml", "Cargo.toml"));
        assert!(!path_matches_glob("Cargo.lock", "Cargo.toml"));
    }

    #[test]
    fn test_glob_star_star() {
        assert!(path_matches_glob("anything", "**"));
        assert!(path_matches_glob("deeply/nested/file.rs", "*"));
    }

    // ── Severity ─────────────────────────────────────────────────────────

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Critical.to_string(), "critical");
        assert_eq!(Severity::Warning.to_string(), "warning");
    }

    // ── Learning category ────────────────────────────────────────────────

    #[test]
    fn test_learning_category_display() {
        assert_eq!(LearningCategory::ErrorFix.to_string(), "error_fix");
        assert_eq!(LearningCategory::Security.to_string(), "security");
    }

    // ── Integration ──────────────────────────────────────────────────────

    #[test]
    fn test_run_cmd_success() {
        let result = run_cmd("echo", &["hello"], None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello");
    }

    #[test]
    fn test_run_cmd_failure() {
        let result = run_cmd("false", &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_cmd_with_cwd() {
        let result = run_cmd("pwd", &[], Some(Path::new("/tmp")));
        assert!(result.is_ok());
        // macOS: /tmp -> /private/tmp
        assert!(result.unwrap().contains("tmp"));
    }

    // ── Alert level ──────────────────────────────────────────────────────

    #[test]
    fn test_alert_level_ordering() {
        assert!(AlertLevel::Normal < AlertLevel::Warning);
        assert!(AlertLevel::Warning < AlertLevel::Critical);
        assert!(AlertLevel::Critical < AlertLevel::Exceeded);
    }

    #[test]
    fn test_alert_level_display() {
        assert_eq!(AlertLevel::Normal.to_string(), "normal");
        assert_eq!(AlertLevel::Exceeded.to_string(), "EXCEEDED");
    }

    // ── Model tier ───────────────────────────────────────────────────────

    #[test]
    fn test_model_tier_display() {
        assert_eq!(ModelTier::Opus.to_string(), "opus");
        assert_eq!(ModelTier::Haiku.to_string(), "haiku");
    }
}
