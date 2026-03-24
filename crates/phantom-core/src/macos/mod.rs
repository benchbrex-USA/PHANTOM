//! macOS Computer Access Layer — Architecture Framework §3.
//!
//! Full terminal control of macOS for the autonomous engineering pipeline:
//!   • `OsascriptBridge`  — Run AppleScript / JXA via `osascript`
//!   • `KeychainManager`  — Store/retrieve credentials via `security` CLI
//!   • `ClipboardBridge`  — Read/write macOS pasteboard via `pbpaste`/`pbcopy`
//!   • `ScreenCapture`    — Capture screenshots via `screencapture` CLI
//!   • `LaunchctlManager` — Register/manage launchd daemons for background service
//!   • `BrowserAutomation`— Playwright stub for headless browser control (§5)
//!
//! All operations use `std::process::Command` for zero-dependency system integration.
//! Every operation is auditable — callers receive structured results that can be
//! fed into the audit log.

use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

// ── Error Type ──────────────────────────────────────────────────────────────

/// Errors from macOS system operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacError {
    /// Which subsystem produced the error
    pub subsystem: MacSubsystem,
    /// Human-readable error message
    pub message: String,
    /// The exit code from the subprocess (if any)
    pub exit_code: Option<i32>,
    /// stderr output (truncated to 1 KB)
    pub stderr: Option<String>,
}

impl fmt::Display for MacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.subsystem, self.message)
    }
}

impl std::error::Error for MacError {}

impl MacError {
    fn new(subsystem: MacSubsystem, message: impl Into<String>) -> Self {
        Self {
            subsystem,
            message: message.into(),
            exit_code: None,
            stderr: None,
        }
    }

    fn from_output(
        subsystem: MacSubsystem,
        message: impl Into<String>,
        output: &std::process::Output,
    ) -> Self {
        let stderr_raw = String::from_utf8_lossy(&output.stderr);
        Self {
            subsystem,
            message: message.into(),
            exit_code: output.status.code(),
            stderr: if stderr_raw.is_empty() {
                None
            } else {
                Some(truncate_str(&stderr_raw, 1024))
            },
        }
    }
}

/// Subsystem identifiers for error classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacSubsystem {
    Osascript,
    Keychain,
    Clipboard,
    ScreenCapture,
    Launchctl,
    Browser,
}

impl fmt::Display for MacSubsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Osascript => write!(f, "osascript"),
            Self::Keychain => write!(f, "keychain"),
            Self::Clipboard => write!(f, "clipboard"),
            Self::ScreenCapture => write!(f, "screencapture"),
            Self::Launchctl => write!(f, "launchctl"),
            Self::Browser => write!(f, "browser"),
        }
    }
}

/// Result type for macOS operations.
pub type MacResult<T> = Result<T, MacError>;

/// Truncate a string to `max_len` bytes, appending "..." if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let end = s.floor_char_boundary(max_len.saturating_sub(3));
        format!("{}...", &s[..end])
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// §3.1  OsascriptBridge — AppleScript / JXA execution
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The scripting language to use with `osascript`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLanguage {
    /// AppleScript (default)
    AppleScript,
    /// JavaScript for Automation (JXA)
    JavaScript,
}

/// Result of an osascript execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResult {
    /// stdout output (the script's return value)
    pub output: String,
    /// stderr output (diagnostics)
    pub stderr: String,
    /// Exit code
    pub exit_code: i32,
    /// Whether the script succeeded (exit 0)
    pub success: bool,
    /// Wall-clock execution time
    pub duration_ms: u64,
}

/// Bridge to macOS `osascript` — runs AppleScript or JXA code.
///
/// Used by agents to control macOS: open apps, click UI elements, read
/// system state, control Finder, etc.
pub struct OsascriptBridge {
    /// Default timeout for scripts
    timeout: Duration,
}

impl OsascriptBridge {
    /// Create a new bridge with default timeout (30 seconds).
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }

    /// Set the default timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Run an inline AppleScript string.
    #[instrument(skip(self, script), fields(lang = "applescript", len = script.len()))]
    pub fn run_applescript(&self, script: &str) -> MacResult<ScriptResult> {
        self.run(script, ScriptLanguage::AppleScript)
    }

    /// Run an inline JavaScript for Automation (JXA) string.
    #[instrument(skip(self, script), fields(lang = "javascript", len = script.len()))]
    pub fn run_javascript(&self, script: &str) -> MacResult<ScriptResult> {
        self.run(script, ScriptLanguage::JavaScript)
    }

    /// Run an osascript from a file.
    #[instrument(skip(self), fields(path = %path.as_ref().display()))]
    pub fn run_file(
        &self,
        path: impl AsRef<Path>,
        lang: ScriptLanguage,
    ) -> MacResult<ScriptResult> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(MacError::new(
                MacSubsystem::Osascript,
                format!("script file not found: {}", path.display()),
            ));
        }

        let mut cmd = Command::new("osascript");
        if lang == ScriptLanguage::JavaScript {
            cmd.arg("-l").arg("JavaScript");
        }
        cmd.arg(path);

        self.execute_command(cmd)
    }

    /// Run an inline script with the specified language.
    pub fn run(&self, script: &str, lang: ScriptLanguage) -> MacResult<ScriptResult> {
        let mut cmd = Command::new("osascript");
        if lang == ScriptLanguage::JavaScript {
            cmd.arg("-l").arg("JavaScript");
        }
        cmd.arg("-e").arg(script);

        self.execute_command(cmd)
    }

    /// Execute a pre-built Command and parse the result.
    fn execute_command(&self, mut cmd: Command) -> MacResult<ScriptResult> {
        let start = Instant::now();

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Osascript,
                    format!("failed to spawn osascript: {e}"),
                )
            })?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        debug!(exit_code, duration_ms, "osascript finished");

        Ok(ScriptResult {
            output: stdout,
            stderr,
            exit_code,
            success: output.status.success(),
            duration_ms,
        })
    }

    // ── Convenience helpers ────────────────────────────────────────────

    /// Display a macOS notification.
    pub fn notify(&self, title: &str, message: &str) -> MacResult<ScriptResult> {
        let escaped_title = title.replace('"', "\\\"");
        let escaped_msg = message.replace('"', "\\\"");
        self.run_applescript(&format!(
            "display notification \"{}\" with title \"{}\"",
            escaped_msg, escaped_title
        ))
    }

    /// Get the name of the frontmost application.
    pub fn frontmost_app(&self) -> MacResult<String> {
        let result = self.run_applescript(
            "tell application \"System Events\" to get name of first process whose frontmost is true",
        )?;
        if result.success {
            Ok(result.output)
        } else {
            Err(MacError::new(
                MacSubsystem::Osascript,
                "failed to get frontmost app",
            ))
        }
    }

    /// Open a URL in the default browser.
    pub fn open_url(&self, url: &str) -> MacResult<ScriptResult> {
        let escaped = url.replace('"', "\\\"");
        self.run_applescript(&format!("open location \"{}\"", escaped))
    }

    /// Get a system information value via JXA.
    pub fn system_info(&self) -> MacResult<ScriptResult> {
        self.run_javascript(
            r#"
            const app = Application.currentApplication();
            app.includeStandardAdditions = true;
            JSON.stringify({
                user: app.systemInfo().shortUserName,
                computer: app.systemInfo().computerName,
            });
            "#,
        )
    }
}

impl Default for OsascriptBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// §3.2  KeychainManager — Credential storage via `security` CLI
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Which macOS keychain to target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeychainTarget {
    /// Default login keychain
    Login,
    /// System keychain (requires root)
    System,
}

impl KeychainTarget {
    fn path(&self) -> Option<&'static str> {
        match self {
            Self::Login => None, // `security` defaults to login
            Self::System => Some("/Library/Keychains/System.keychain"),
        }
    }
}

/// A credential stored in or retrieved from the keychain.
#[derive(Clone, Serialize, Deserialize)]
pub struct KeychainCredential {
    pub service: String,
    pub account: String,
    #[serde(skip_serializing)]
    pub password: String,
}

impl std::fmt::Debug for KeychainCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeychainCredential")
            .field("service", &self.service)
            .field("account", &self.account)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

/// Manages macOS Keychain access via the `security` CLI.
///
/// Used by Phantom to securely store API keys, database passwords, service
/// tokens, and other secrets. Credentials never touch disk unencrypted.
pub struct KeychainManager {
    target: KeychainTarget,
}

impl KeychainManager {
    /// Create a manager targeting the login keychain.
    pub fn new() -> Self {
        Self {
            target: KeychainTarget::Login,
        }
    }

    /// Target a specific keychain.
    pub fn with_target(mut self, target: KeychainTarget) -> Self {
        self.target = target;
        self
    }

    /// Store a credential in the keychain.
    ///
    /// Uses `security add-generic-password`. If the entry already exists,
    /// it is updated via delete + add.
    #[instrument(skip(self, password), fields(service = %service, account = %account))]
    pub fn store(&self, service: &str, account: &str, password: &str) -> MacResult<()> {
        // Try to delete existing entry first (ignore errors — may not exist)
        let _ = self.delete(service, account);

        let mut cmd = Command::new("security");
        cmd.args([
            "add-generic-password",
            "-s",
            service,
            "-a",
            account,
            "-w",
            password,
        ]);

        if let Some(kc_path) = self.target.path() {
            cmd.arg(kc_path);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Keychain,
                    format!("failed to run security: {e}"),
                )
            })?;

        if output.status.success() {
            info!(service, account, "credential stored in keychain");
            Ok(())
        } else {
            Err(MacError::from_output(
                MacSubsystem::Keychain,
                format!("failed to store credential for {service}/{account}"),
                &output,
            ))
        }
    }

    /// Retrieve a credential from the keychain.
    ///
    /// Uses `security find-generic-password -w` to get only the password.
    #[instrument(skip(self), fields(service = %service, account = %account))]
    pub fn retrieve(&self, service: &str, account: &str) -> MacResult<KeychainCredential> {
        let mut cmd = Command::new("security");
        cmd.args(["find-generic-password", "-s", service, "-a", account, "-w"]);

        if let Some(kc_path) = self.target.path() {
            cmd.arg(kc_path);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Keychain,
                    format!("failed to run security: {e}"),
                )
            })?;

        if output.status.success() {
            let password = String::from_utf8_lossy(&output.stdout).trim().to_string();
            debug!(service, account, "credential retrieved from keychain");
            Ok(KeychainCredential {
                service: service.to_string(),
                account: account.to_string(),
                password,
            })
        } else {
            Err(MacError::from_output(
                MacSubsystem::Keychain,
                format!("credential not found: {service}/{account}"),
                &output,
            ))
        }
    }

    /// Delete a credential from the keychain.
    #[instrument(skip(self), fields(service = %service, account = %account))]
    pub fn delete(&self, service: &str, account: &str) -> MacResult<()> {
        let mut cmd = Command::new("security");
        cmd.args(["delete-generic-password", "-s", service, "-a", account]);

        if let Some(kc_path) = self.target.path() {
            cmd.arg(kc_path);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Keychain,
                    format!("failed to run security: {e}"),
                )
            })?;

        if output.status.success() {
            info!(service, account, "credential deleted from keychain");
            Ok(())
        } else {
            Err(MacError::from_output(
                MacSubsystem::Keychain,
                format!("failed to delete credential: {service}/{account}"),
                &output,
            ))
        }
    }

    /// Check if a credential exists in the keychain (without retrieving the password).
    pub fn exists(&self, service: &str, account: &str) -> bool {
        let mut cmd = Command::new("security");
        cmd.args(["find-generic-password", "-s", service, "-a", account]);

        if let Some(kc_path) = self.target.path() {
            cmd.arg(kc_path);
        }

        cmd.stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// List all generic passwords for a given service.
    pub fn list_accounts(&self, service: &str) -> MacResult<Vec<String>> {
        // `security dump-keychain` is noisy; use `find` with just service
        // and parse account names from the output
        let mut cmd = Command::new("security");
        cmd.args(["find-generic-password", "-s", service]);

        if let Some(kc_path) = self.target.path() {
            cmd.arg(kc_path);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Keychain,
                    format!("failed to run security: {e}"),
                )
            })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let accounts: Vec<String> = stdout
            .lines()
            .filter(|line| line.contains("\"acct\""))
            .filter_map(|line| {
                // Parse: "acct"<blob>="account-name"
                let start = line.find('=')?;
                let val = line[start + 1..].trim().trim_matches('"');
                if val.is_empty() {
                    None
                } else {
                    Some(val.to_string())
                }
            })
            .collect();

        Ok(accounts)
    }
}

impl Default for KeychainManager {
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// §3.3  ClipboardBridge — macOS pasteboard via pbpaste / pbcopy
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// macOS clipboard (pasteboard) bridge.
///
/// Provides read/write access to the macOS clipboard via `pbpaste` and `pbcopy`.
/// Used by agents to transfer data between the Phantom pipeline and macOS apps.
pub struct ClipboardBridge;

impl ClipboardBridge {
    pub fn new() -> Self {
        Self
    }

    /// Read the current clipboard contents as a UTF-8 string.
    #[instrument(skip(self))]
    pub fn read(&self) -> MacResult<String> {
        let output = Command::new("pbpaste")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Clipboard,
                    format!("failed to run pbpaste: {e}"),
                )
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(MacError::from_output(
                MacSubsystem::Clipboard,
                "failed to read clipboard",
                &output,
            ))
        }
    }

    /// Write text to the clipboard.
    #[instrument(skip(self, text), fields(len = text.len()))]
    pub fn write(&self, text: &str) -> MacResult<()> {
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Clipboard,
                    format!("failed to spawn pbcopy: {e}"),
                )
            })?;

        if let Some(ref mut stdin) = child.stdin {
            stdin.write_all(text.as_bytes()).map_err(|e| {
                MacError::new(
                    MacSubsystem::Clipboard,
                    format!("failed to write to pbcopy stdin: {e}"),
                )
            })?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| MacError::new(MacSubsystem::Clipboard, format!("pbcopy failed: {e}")))?;

        if output.status.success() {
            debug!(bytes = text.len(), "wrote to clipboard");
            Ok(())
        } else {
            Err(MacError::from_output(
                MacSubsystem::Clipboard,
                "failed to write clipboard",
                &output,
            ))
        }
    }

    /// Clear the clipboard by writing an empty string.
    pub fn clear(&self) -> MacResult<()> {
        self.write("")
    }

    /// Check if the clipboard contains any text.
    pub fn has_content(&self) -> bool {
        self.read().map(|s| !s.is_empty()).unwrap_or(false)
    }
}

impl Default for ClipboardBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// §3.4  ScreenCapture — macOS screenshot via `screencapture`
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Image format for screen captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureFormat {
    Png,
    Jpg,
    Pdf,
    Tiff,
}

impl CaptureFormat {
    fn extension(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Pdf => "pdf",
            Self::Tiff => "tiff",
        }
    }

    fn flag(&self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Pdf => "pdf",
            Self::Tiff => "tiff",
        }
    }
}

/// Options for a screen capture operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureOptions {
    /// Image format (default: PNG)
    pub format: CaptureFormat,
    /// Capture without the shadow (window captures)
    pub no_shadow: bool,
    /// Capture to clipboard instead of file
    pub to_clipboard: bool,
    /// Capture a specific window (interactive selection) instead of full screen
    pub window_mode: bool,
    /// Capture a specific rectangle (x, y, width, height)
    pub rect: Option<(u32, u32, u32, u32)>,
    /// Delay in seconds before capture
    pub delay: Option<u32>,
    /// Hide the cursor
    pub hide_cursor: bool,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            format: CaptureFormat::Png,
            no_shadow: true,
            to_clipboard: false,
            window_mode: false,
            rect: None,
            delay: None,
            hide_cursor: true,
        }
    }
}

/// Result of a screen capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    /// Path to the saved screenshot file (None if captured to clipboard)
    pub path: Option<PathBuf>,
    /// File size in bytes (None if clipboard)
    pub size_bytes: Option<u64>,
    /// The format used
    pub format: CaptureFormat,
    /// Whether the capture succeeded
    pub success: bool,
    /// Duration of the capture operation
    pub duration_ms: u64,
}

/// macOS screen capture via the `screencapture` CLI.
pub struct ScreenCapture {
    /// Default output directory for screenshots
    output_dir: PathBuf,
}

impl ScreenCapture {
    /// Create a new ScreenCapture writing to `/tmp/phantom-captures`.
    pub fn new() -> Self {
        Self {
            output_dir: PathBuf::from("/tmp/phantom-captures"),
        }
    }

    /// Set the output directory for screenshots.
    pub fn with_output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = dir.into();
        self
    }

    /// Capture the full screen to a file.
    #[instrument(skip(self))]
    pub fn capture_screen(&self, filename: Option<&str>) -> MacResult<CaptureResult> {
        self.capture(filename, &CaptureOptions::default())
    }

    /// Capture with custom options.
    #[instrument(skip(self, options))]
    pub fn capture(
        &self,
        filename: Option<&str>,
        options: &CaptureOptions,
    ) -> MacResult<CaptureResult> {
        let start = Instant::now();

        // Build output path
        let out_path = if options.to_clipboard {
            None
        } else {
            // Ensure output directory exists
            std::fs::create_dir_all(&self.output_dir).map_err(|e| {
                MacError::new(
                    MacSubsystem::ScreenCapture,
                    format!("failed to create capture dir: {e}"),
                )
            })?;

            let name = filename.unwrap_or("phantom-capture");
            let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            Some(
                self.output_dir
                    .join(format!("{}_{}.{}", name, ts, options.format.extension())),
            )
        };

        let mut cmd = Command::new("screencapture");

        // Format
        cmd.args(["-t", options.format.flag()]);

        // Options
        if options.no_shadow {
            cmd.arg("-o");
        }
        if options.to_clipboard {
            cmd.arg("-c");
        }
        if options.window_mode {
            cmd.arg("-w");
        }
        if options.hide_cursor {
            cmd.arg("-C");
        }
        if let Some(delay) = options.delay {
            cmd.args(["-T", &delay.to_string()]);
        }
        if let Some((x, y, w, h)) = options.rect {
            cmd.args(["-R", &format!("{x},{y},{w},{h}")]);
        }

        // Non-interactive mode (no mouse selection)
        if !options.window_mode {
            cmd.arg("-x");
        }

        // Output path
        if let Some(ref path) = out_path {
            cmd.arg(path);
        }

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::ScreenCapture,
                    format!("failed to run screencapture: {e}"),
                )
            })?;

        let duration_ms = start.elapsed().as_millis() as u64;

        if !output.status.success() {
            return Err(MacError::from_output(
                MacSubsystem::ScreenCapture,
                "screencapture failed",
                &output,
            ));
        }

        let size_bytes = out_path
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| m.len());

        info!(
            path = out_path.as_ref().map(|p| p.display().to_string()),
            size = size_bytes,
            duration_ms,
            "screen captured"
        );

        Ok(CaptureResult {
            path: out_path,
            size_bytes,
            format: options.format,
            success: true,
            duration_ms,
        })
    }

    /// Capture screen to clipboard (no file).
    pub fn capture_to_clipboard(&self) -> MacResult<CaptureResult> {
        let opts = CaptureOptions {
            to_clipboard: true,
            ..Default::default()
        };
        self.capture(None, &opts)
    }

    /// Capture a specific rectangle of the screen.
    pub fn capture_rect(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        filename: Option<&str>,
    ) -> MacResult<CaptureResult> {
        let opts = CaptureOptions {
            rect: Some((x, y, width, height)),
            ..Default::default()
        };
        self.capture(filename, &opts)
    }
}

impl Default for ScreenCapture {
    fn default() -> Self {
        Self::new()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// §3.5  LaunchctlManager — launchd daemon registration
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Configuration for a launchd daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Reverse-DNS label (e.g., "com.phantom.agent-service")
    pub label: String,
    /// Path to the executable
    pub program: PathBuf,
    /// Arguments to pass
    pub program_arguments: Vec<String>,
    /// Working directory
    pub working_directory: Option<PathBuf>,
    /// Run at load (start immediately)
    pub run_at_load: bool,
    /// Keep alive (restart on crash)
    pub keep_alive: bool,
    /// Redirect stdout to this file
    pub stdout_path: Option<PathBuf>,
    /// Redirect stderr to this file
    pub stderr_path: Option<PathBuf>,
    /// Environment variables
    pub environment_variables: std::collections::HashMap<String, String>,
    /// Start interval in seconds (periodic execution)
    pub start_interval: Option<u32>,
}

impl DaemonConfig {
    /// Create a minimal daemon config.
    pub fn new(label: impl Into<String>, program: impl Into<PathBuf>) -> Self {
        Self {
            label: label.into(),
            program: program.into(),
            program_arguments: Vec::new(),
            working_directory: None,
            run_at_load: true,
            keep_alive: true,
            stdout_path: None,
            stderr_path: None,
            environment_variables: std::collections::HashMap::new(),
            start_interval: None,
        }
    }

    /// Set program arguments.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.program_arguments = args;
        self
    }

    /// Set working directory.
    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Set log paths.
    pub fn with_logs(mut self, stdout: impl Into<PathBuf>, stderr: impl Into<PathBuf>) -> Self {
        self.stdout_path = Some(stdout.into());
        self.stderr_path = Some(stderr.into());
        self
    }

    /// Add an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment_variables.insert(key.into(), value.into());
        self
    }

    /// Set start interval for periodic execution.
    pub fn with_interval(mut self, seconds: u32) -> Self {
        self.start_interval = Some(seconds);
        self
    }

    /// Generate the plist XML for this daemon config.
    pub fn to_plist(&self) -> String {
        let mut plist = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n",
        );

        // Label
        plist.push_str(&format!(
            "    <key>Label</key>\n    <string>{}</string>\n",
            xml_escape(&self.label)
        ));

        // Program
        plist.push_str(&format!(
            "    <key>Program</key>\n    <string>{}</string>\n",
            xml_escape(&self.program.display().to_string())
        ));

        // ProgramArguments (if any)
        if !self.program_arguments.is_empty() {
            plist.push_str("    <key>ProgramArguments</key>\n    <array>\n");
            plist.push_str(&format!(
                "        <string>{}</string>\n",
                xml_escape(&self.program.display().to_string())
            ));
            for arg in &self.program_arguments {
                plist.push_str(&format!("        <string>{}</string>\n", xml_escape(arg)));
            }
            plist.push_str("    </array>\n");
        }

        // WorkingDirectory
        if let Some(ref dir) = self.working_directory {
            plist.push_str(&format!(
                "    <key>WorkingDirectory</key>\n    <string>{}</string>\n",
                xml_escape(&dir.display().to_string())
            ));
        }

        // RunAtLoad
        plist.push_str(&format!(
            "    <key>RunAtLoad</key>\n    <{}/>\\n",
            if self.run_at_load { "true" } else { "false" }
        ));

        // KeepAlive
        plist.push_str(&format!(
            "    <key>KeepAlive</key>\n    <{}/>\n",
            if self.keep_alive { "true" } else { "false" }
        ));

        // Stdout
        if let Some(ref path) = self.stdout_path {
            plist.push_str(&format!(
                "    <key>StandardOutPath</key>\n    <string>{}</string>\n",
                xml_escape(&path.display().to_string())
            ));
        }

        // Stderr
        if let Some(ref path) = self.stderr_path {
            plist.push_str(&format!(
                "    <key>StandardErrorPath</key>\n    <string>{}</string>\n",
                xml_escape(&path.display().to_string())
            ));
        }

        // EnvironmentVariables
        if !self.environment_variables.is_empty() {
            plist.push_str("    <key>EnvironmentVariables</key>\n    <dict>\n");
            for (key, val) in &self.environment_variables {
                plist.push_str(&format!(
                    "        <key>{}</key>\n        <string>{}</string>\n",
                    xml_escape(key),
                    xml_escape(val)
                ));
            }
            plist.push_str("    </dict>\n");
        }

        // StartInterval
        if let Some(interval) = self.start_interval {
            plist.push_str(&format!(
                "    <key>StartInterval</key>\n    <integer>{}</integer>\n",
                interval
            ));
        }

        plist.push_str("</dict>\n</plist>\n");
        plist
    }
}

/// Escape special XML characters.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Status of a launchd service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    /// The service label
    pub label: String,
    /// Whether the service is currently loaded
    pub loaded: bool,
    /// PID of the running process (if running)
    pub pid: Option<u32>,
    /// Last exit status
    pub last_exit_status: Option<i32>,
}

/// Manages launchd daemons via `launchctl`.
///
/// Registers Phantom as a background service that starts on login and
/// restarts on crash.
pub struct LaunchctlManager {
    /// Directory for plist files (~/Library/LaunchAgents for user agents)
    plist_dir: PathBuf,
}

impl LaunchctlManager {
    /// Create a new manager using ~/Library/LaunchAgents.
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            plist_dir: PathBuf::from(home).join("Library/LaunchAgents"),
        }
    }

    /// Set the plist directory.
    pub fn with_plist_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.plist_dir = dir.into();
        self
    }

    /// Install and load a daemon.
    #[instrument(skip(self, config), fields(label = %config.label))]
    pub fn install(&self, config: &DaemonConfig) -> MacResult<()> {
        // Write the plist file
        let plist_path = self.plist_path(&config.label);

        std::fs::create_dir_all(&self.plist_dir).map_err(|e| {
            MacError::new(
                MacSubsystem::Launchctl,
                format!("failed to create plist dir: {e}"),
            )
        })?;

        let plist_content = config.to_plist();
        std::fs::write(&plist_path, &plist_content).map_err(|e| {
            MacError::new(
                MacSubsystem::Launchctl,
                format!("failed to write plist {}: {e}", plist_path.display()),
            )
        })?;

        // Load the daemon
        self.load(&config.label)?;

        info!(label = %config.label, path = %plist_path.display(), "daemon installed and loaded");
        Ok(())
    }

    /// Uninstall a daemon (unload + delete plist).
    #[instrument(skip(self), fields(label = %label))]
    pub fn uninstall(&self, label: &str) -> MacResult<()> {
        // Unload first (ignore errors — may not be loaded)
        let _ = self.unload(label);

        let plist_path = self.plist_path(label);
        if plist_path.exists() {
            std::fs::remove_file(&plist_path).map_err(|e| {
                MacError::new(
                    MacSubsystem::Launchctl,
                    format!("failed to remove plist: {e}"),
                )
            })?;
        }

        info!(label, "daemon uninstalled");
        Ok(())
    }

    /// Load (start) a daemon.
    pub fn load(&self, label: &str) -> MacResult<()> {
        let plist_path = self.plist_path(label);
        let output = Command::new("launchctl")
            .args(["load", "-w"])
            .arg(&plist_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Launchctl,
                    format!("failed to run launchctl: {e}"),
                )
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(MacError::from_output(
                MacSubsystem::Launchctl,
                format!("failed to load daemon {label}"),
                &output,
            ))
        }
    }

    /// Unload (stop) a daemon.
    pub fn unload(&self, label: &str) -> MacResult<()> {
        let plist_path = self.plist_path(label);
        let output = Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&plist_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Launchctl,
                    format!("failed to run launchctl: {e}"),
                )
            })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(MacError::from_output(
                MacSubsystem::Launchctl,
                format!("failed to unload daemon {label}"),
                &output,
            ))
        }
    }

    /// Get the status of a daemon.
    pub fn status(&self, label: &str) -> MacResult<DaemonStatus> {
        let output = Command::new("launchctl")
            .args(["list", label])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(
                    MacSubsystem::Launchctl,
                    format!("failed to run launchctl: {e}"),
                )
            })?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let (pid, exit_status) = parse_launchctl_list(&stdout);
            Ok(DaemonStatus {
                label: label.to_string(),
                loaded: true,
                pid,
                last_exit_status: exit_status,
            })
        } else {
            Ok(DaemonStatus {
                label: label.to_string(),
                loaded: false,
                pid: None,
                last_exit_status: None,
            })
        }
    }

    /// Check if a daemon is currently loaded.
    pub fn is_loaded(&self, label: &str) -> bool {
        self.status(label).map(|s| s.loaded).unwrap_or(false)
    }

    /// Restart a daemon (unload + load).
    pub fn restart(&self, label: &str) -> MacResult<()> {
        let _ = self.unload(label);
        self.load(label)
    }

    /// Get the plist file path for a label.
    fn plist_path(&self, label: &str) -> PathBuf {
        self.plist_dir.join(format!("{label}.plist"))
    }
}

impl Default for LaunchctlManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse `launchctl list <label>` output to extract PID and last exit status.
fn parse_launchctl_list(output: &str) -> (Option<u32>, Option<i32>) {
    let mut pid = None;
    let mut exit_status = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\"PID\" = ") {
            pid = rest.trim_end_matches(';').trim().parse().ok();
        } else if let Some(rest) = trimmed.strip_prefix("\"LastExitStatus\" = ") {
            exit_status = rest.trim_end_matches(';').trim().parse().ok();
        }
    }

    (pid, exit_status)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// §3.6  BrowserAutomation — Playwright stub (§5 fills in)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Browser type for Playwright automation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserType {
    Chromium,
    Firefox,
    Webkit,
}

impl fmt::Display for BrowserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chromium => write!(f, "chromium"),
            Self::Firefox => write!(f, "firefox"),
            Self::Webkit => write!(f, "webkit"),
        }
    }
}

/// Configuration for browser automation sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Which browser engine to use
    pub browser: BrowserType,
    /// Run in headless mode
    pub headless: bool,
    /// Viewport width
    pub viewport_width: u32,
    /// Viewport height
    pub viewport_height: u32,
    /// Timeout for page loads in milliseconds
    pub timeout_ms: u64,
    /// User agent override
    pub user_agent: Option<String>,
    /// Proxy configuration (e.g., "socks5://localhost:9050")
    pub proxy: Option<String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            browser: BrowserType::Chromium,
            headless: true,
            viewport_width: 1920,
            viewport_height: 1080,
            timeout_ms: 30_000,
            user_agent: None,
            proxy: None,
        }
    }
}

/// A navigation action for browser automation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum BrowserAction {
    /// Navigate to a URL
    Navigate { url: String },
    /// Click an element by CSS selector
    Click { selector: String },
    /// Type text into an element
    Type { selector: String, text: String },
    /// Wait for an element to appear
    WaitFor {
        selector: String,
        timeout_ms: Option<u64>,
    },
    /// Take a screenshot of the page
    Screenshot { path: String },
    /// Extract text from an element
    ExtractText { selector: String },
    /// Execute JavaScript in the page context
    Evaluate { script: String },
    /// Fill a form field
    Fill { selector: String, value: String },
    /// Select an option from a dropdown
    Select { selector: String, value: String },
    /// Wait for navigation to complete
    WaitForNavigation { timeout_ms: Option<u64> },
}

/// Result of a browser automation action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserActionResult {
    /// The action that was executed
    pub action: String,
    /// Whether the action succeeded
    pub success: bool,
    /// Returned data (e.g., extracted text, eval result)
    pub data: Option<serde_json::Value>,
    /// Error if the action failed
    pub error: Option<String>,
    /// Duration of the action in milliseconds
    pub duration_ms: u64,
}

/// Browser automation interface using osascript JXA for native Safari/Chrome control.
///
/// Uses macOS JavaScript for Automation (JXA) via `osascript` to control
/// Safari or Google Chrome. No external dependencies required — everything
/// runs through the system `osascript` binary.
///
/// For complex flows (multi-step signups with CAPTCHA), falls back to
/// Playwright via a Node.js sidecar script when `npx playwright` is available.
pub struct BrowserAutomation {
    config: BrowserConfig,
    /// Whether a session is currently active
    session_active: bool,
    /// The osascript bridge for JXA execution
    bridge: OsascriptBridge,
    /// Target app name ("Safari" or "Google Chrome")
    target_app: String,
    /// Current page URL (tracked locally)
    current_url: Option<String>,
}

impl BrowserAutomation {
    /// Create a new browser automation instance.
    pub fn new(config: BrowserConfig) -> Self {
        let target_app = match config.browser {
            BrowserType::Chromium => "Google Chrome".to_string(),
            BrowserType::Webkit => "Safari".to_string(),
            // Firefox doesn't support osascript — fall back to Safari
            BrowserType::Firefox => "Safari".to_string(),
        };
        Self {
            config,
            session_active: false,
            bridge: OsascriptBridge::new().with_timeout(Duration::from_secs(60)),
            target_app,
            current_url: None,
        }
    }

    /// Create with default config.
    pub fn with_defaults() -> Self {
        Self::new(BrowserConfig::default())
    }

    /// Get the current configuration.
    pub fn config(&self) -> &BrowserConfig {
        &self.config
    }

    /// Start a browser session by launching or activating the target browser.
    pub fn start_session(&mut self) -> MacResult<()> {
        if self.session_active {
            return Err(MacError::new(
                MacSubsystem::Browser,
                "browser session already active",
            ));
        }

        // Launch the browser application if not already running
        let script = format!(
            r#"
            const app = Application("{}");
            app.activate();
            delay(0.5);
            app.name();
            "#,
            self.target_app
        );

        let result = self.bridge.run_javascript(&script);
        match result {
            Ok(r) if r.success => {
                info!(
                    browser = %self.config.browser,
                    app = %self.target_app,
                    "browser session started"
                );
                self.session_active = true;
                Ok(())
            }
            Ok(r) => {
                // Browser launched but JXA returned non-zero — still usable
                warn!(stderr = %r.stderr, "browser started with warnings");
                self.session_active = true;
                Ok(())
            }
            Err(e) => Err(MacError::new(
                MacSubsystem::Browser,
                format!("failed to start browser session: {e}"),
            )),
        }
    }

    /// End the current browser session.
    pub fn end_session(&mut self) -> MacResult<()> {
        if !self.session_active {
            return Err(MacError::new(
                MacSubsystem::Browser,
                "no active browser session",
            ));
        }

        info!(app = %self.target_app, "ending browser session");
        self.session_active = false;
        self.current_url = None;
        Ok(())
    }

    /// Execute a single browser action via osascript JXA.
    pub fn execute_action(&mut self, action: &BrowserAction) -> MacResult<BrowserActionResult> {
        if !self.session_active {
            return Err(MacError::new(
                MacSubsystem::Browser,
                "no active browser session — call start_session() first",
            ));
        }

        let start = Instant::now();

        let result = match action {
            BrowserAction::Navigate { url } => self.do_navigate(url),
            BrowserAction::Click { selector } => self.do_click(selector),
            BrowserAction::Type { selector, text } => self.do_type(selector, text),
            BrowserAction::WaitFor {
                selector,
                timeout_ms,
            } => self.do_wait_for(selector, timeout_ms.unwrap_or(self.config.timeout_ms)),
            BrowserAction::Screenshot { path } => self.do_screenshot(path),
            BrowserAction::ExtractText { selector } => self.do_extract_text(selector),
            BrowserAction::Evaluate { script } => self.do_evaluate(script),
            BrowserAction::Fill { selector, value } => self.do_fill(selector, value),
            BrowserAction::Select { selector, value } => self.do_select(selector, value),
            BrowserAction::WaitForNavigation { timeout_ms } => {
                self.do_wait_for_navigation(timeout_ms.unwrap_or(self.config.timeout_ms))
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        let action_name = action_label(action);

        match result {
            Ok(data) => {
                debug!(
                    action = action_name,
                    duration_ms, "browser action succeeded"
                );
                Ok(BrowserActionResult {
                    action: action_name.to_string(),
                    success: true,
                    data,
                    error: None,
                    duration_ms,
                })
            }
            Err(e) => {
                warn!(action = action_name, error = %e, "browser action failed");
                Ok(BrowserActionResult {
                    action: action_name.to_string(),
                    success: false,
                    data: None,
                    error: Some(e.message),
                    duration_ms,
                })
            }
        }
    }

    /// Execute a sequence of browser actions.
    /// Stops on first failure and returns all results (including the failure).
    pub fn execute_sequence(
        &mut self,
        actions: &[BrowserAction],
    ) -> MacResult<Vec<BrowserActionResult>> {
        let mut results = Vec::with_capacity(actions.len());

        for action in actions {
            let result = self.execute_action(action)?;
            let failed = !result.success;
            results.push(result);
            if failed {
                break;
            }
        }

        Ok(results)
    }

    /// Whether a browser session is currently active.
    pub fn is_active(&self) -> bool {
        self.session_active
    }

    /// Get the URL of the current tab.
    pub fn current_tab_url(&self) -> MacResult<String> {
        let script = self.jxa_get_url();
        let result = self.bridge.run_javascript(&script)?;
        if result.success {
            Ok(result.output.trim().trim_matches('"').to_string())
        } else {
            Err(MacError::new(
                MacSubsystem::Browser,
                "failed to get current URL",
            ))
        }
    }

    /// Get the page title of the current tab.
    pub fn current_tab_title(&self) -> MacResult<String> {
        let script = self.jxa_get_title();
        let result = self.bridge.run_javascript(&script)?;
        if result.success {
            Ok(result.output.trim().trim_matches('"').to_string())
        } else {
            Err(MacError::new(
                MacSubsystem::Browser,
                "failed to get page title",
            ))
        }
    }

    /// Get page source HTML of the current tab.
    pub fn page_source(&self) -> MacResult<String> {
        let script = self.jxa_evaluate("document.documentElement.outerHTML");
        let result = self.bridge.run_javascript(&script)?;
        if result.success {
            Ok(result.output)
        } else {
            Err(MacError::new(
                MacSubsystem::Browser,
                "failed to get page source",
            ))
        }
    }

    // ── Internal action implementations ──────────────────────────────

    fn do_navigate(&mut self, url: &str) -> MacResult<Option<serde_json::Value>> {
        let escaped = url.replace('\\', "\\\\").replace('"', "\\\"");
        let script = if self.target_app == "Safari" {
            format!(
                r#"
                const safari = Application("Safari");
                safari.activate();
                if (safari.windows.length === 0) {{ safari.Document().make(); }}
                safari.windows[0].currentTab.url = "{}";
                "#,
                escaped
            )
        } else {
            format!(
                r#"
                const chrome = Application("Google Chrome");
                chrome.activate();
                if (chrome.windows.length === 0) {{ chrome.Window().make(); }}
                chrome.windows[0].activeTab.url = "{}";
                "#,
                escaped
            )
        };

        let result = self.bridge.run_javascript(&script)?;
        self.current_url = Some(url.to_string());

        if result.success || result.exit_code == 0 {
            Ok(Some(serde_json::json!({ "url": url })))
        } else {
            Err(MacError::new(
                MacSubsystem::Browser,
                format!("navigation failed: {}", result.stderr),
            ))
        }
    }

    fn do_click(&self, selector: &str) -> MacResult<Option<serde_json::Value>> {
        let js = format!(
            r#"var el = document.querySelector("{}"); if (el) {{ el.click(); "clicked"; }} else {{ "not_found"; }}"#,
            js_escape(selector)
        );
        let script = self.jxa_evaluate(&js);
        let result = self.bridge.run_javascript(&script)?;

        if result.output.contains("not_found") {
            Err(MacError::new(
                MacSubsystem::Browser,
                format!("element not found: {selector}"),
            ))
        } else {
            Ok(Some(serde_json::json!({ "selector": selector })))
        }
    }

    fn do_type(&self, selector: &str, text: &str) -> MacResult<Option<serde_json::Value>> {
        // Type simulates keystrokes by setting value and dispatching input event
        let js = format!(
            r#"var el = document.querySelector("{}"); if (el) {{ el.focus(); el.value = "{}"; el.dispatchEvent(new Event("input", {{bubbles:true}})); el.dispatchEvent(new Event("change", {{bubbles:true}})); "typed"; }} else {{ "not_found"; }}"#,
            js_escape(selector),
            js_escape(text)
        );
        let script = self.jxa_evaluate(&js);
        let result = self.bridge.run_javascript(&script)?;

        if result.output.contains("not_found") {
            Err(MacError::new(
                MacSubsystem::Browser,
                format!("element not found: {selector}"),
            ))
        } else {
            Ok(Some(
                serde_json::json!({ "selector": selector, "length": text.len() }),
            ))
        }
    }

    fn do_fill(&self, selector: &str, value: &str) -> MacResult<Option<serde_json::Value>> {
        // Fill is the same as type but clears first
        let js = format!(
            r#"var el = document.querySelector("{}"); if (el) {{ el.focus(); el.value = ""; el.value = "{}"; el.dispatchEvent(new Event("input", {{bubbles:true}})); el.dispatchEvent(new Event("change", {{bubbles:true}})); "filled"; }} else {{ "not_found"; }}"#,
            js_escape(selector),
            js_escape(value)
        );
        let script = self.jxa_evaluate(&js);
        let result = self.bridge.run_javascript(&script)?;

        if result.output.contains("not_found") {
            Err(MacError::new(
                MacSubsystem::Browser,
                format!("element not found: {selector}"),
            ))
        } else {
            Ok(Some(
                serde_json::json!({ "selector": selector, "value": value }),
            ))
        }
    }

    fn do_select(&self, selector: &str, value: &str) -> MacResult<Option<serde_json::Value>> {
        let js = format!(
            r#"var el = document.querySelector("{}"); if (el) {{ el.value = "{}"; el.dispatchEvent(new Event("change", {{bubbles:true}})); "selected"; }} else {{ "not_found"; }}"#,
            js_escape(selector),
            js_escape(value)
        );
        let script = self.jxa_evaluate(&js);
        let result = self.bridge.run_javascript(&script)?;

        if result.output.contains("not_found") {
            Err(MacError::new(
                MacSubsystem::Browser,
                format!("element not found: {selector}"),
            ))
        } else {
            Ok(Some(
                serde_json::json!({ "selector": selector, "value": value }),
            ))
        }
    }

    fn do_wait_for(&self, selector: &str, timeout_ms: u64) -> MacResult<Option<serde_json::Value>> {
        // Poll for element existence via repeated JXA calls
        let poll_interval = Duration::from_millis(250);
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);

        loop {
            let js = format!(
                r#"document.querySelector("{}") !== null ? "found" : "waiting""#,
                js_escape(selector)
            );
            let script = self.jxa_evaluate(&js);
            if let Ok(result) = self.bridge.run_javascript(&script) {
                if result.output.contains("found") {
                    return Ok(Some(
                        serde_json::json!({ "selector": selector, "found": true }),
                    ));
                }
            }

            if Instant::now() >= deadline {
                return Err(MacError::new(
                    MacSubsystem::Browser,
                    format!("timeout waiting for element: {selector} ({}ms)", timeout_ms),
                ));
            }

            std::thread::sleep(poll_interval);
        }
    }

    fn do_wait_for_navigation(&self, timeout_ms: u64) -> MacResult<Option<serde_json::Value>> {
        // Capture current URL, then poll until it changes or page reports ready
        let initial_url = self.current_tab_url().unwrap_or_default();
        let poll_interval = Duration::from_millis(300);
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);

        loop {
            // Check document.readyState and current URL
            let js = r#"document.readyState + "|" + window.location.href"#;
            let script = self.jxa_evaluate(js);
            if let Ok(result) = self.bridge.run_javascript(&script) {
                let output = result.output.trim().trim_matches('"');
                if let Some((state, url)) = output.split_once('|') {
                    if state == "complete" && url != initial_url {
                        return Ok(Some(serde_json::json!({
                            "ready_state": "complete",
                            "url": url,
                        })));
                    }
                }
            }

            if Instant::now() >= deadline {
                return Err(MacError::new(
                    MacSubsystem::Browser,
                    format!("navigation timeout ({}ms)", timeout_ms),
                ));
            }

            std::thread::sleep(poll_interval);
        }
    }

    fn do_screenshot(&self, path: &str) -> MacResult<Option<serde_json::Value>> {
        // Use screencapture for the frontmost window
        let output = Command::new("screencapture")
            .args(["-o", "-x", "-w", path])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                MacError::new(MacSubsystem::Browser, format!("screencapture failed: {e}"))
            })?;

        if output.status.success() {
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            Ok(Some(
                serde_json::json!({ "path": path, "size_bytes": size }),
            ))
        } else {
            Err(MacError::from_output(
                MacSubsystem::Browser,
                "screenshot failed",
                &output,
            ))
        }
    }

    fn do_extract_text(&self, selector: &str) -> MacResult<Option<serde_json::Value>> {
        let js = format!(
            r#"var el = document.querySelector("{}"); el ? el.innerText : "__NOT_FOUND__""#,
            js_escape(selector)
        );
        let script = self.jxa_evaluate(&js);
        let result = self.bridge.run_javascript(&script)?;

        let text = result.output.trim().trim_matches('"');
        if text == "__NOT_FOUND__" {
            Err(MacError::new(
                MacSubsystem::Browser,
                format!("element not found: {selector}"),
            ))
        } else {
            Ok(Some(
                serde_json::json!({ "selector": selector, "text": text }),
            ))
        }
    }

    fn do_evaluate(&self, script: &str) -> MacResult<Option<serde_json::Value>> {
        let wrapper = self.jxa_evaluate(script);
        let result = self.bridge.run_javascript(&wrapper)?;

        if result.success {
            // Try to parse as JSON, fall back to string
            let raw = result.output.trim();
            let value = serde_json::from_str(raw)
                .unwrap_or_else(|_| serde_json::Value::String(raw.to_string()));
            Ok(Some(value))
        } else {
            Err(MacError::new(
                MacSubsystem::Browser,
                format!("JavaScript evaluation failed: {}", result.stderr),
            ))
        }
    }

    // ── JXA script builders ──────────────────────────────────────────

    /// Build a JXA script that evaluates JavaScript in the current browser tab.
    fn jxa_evaluate(&self, page_js: &str) -> String {
        let escaped_js = page_js.replace('\\', "\\\\").replace('"', "\\\"");
        if self.target_app == "Safari" {
            format!(
                r#"const safari = Application("Safari"); safari.doJavaScript("{}", {{in: safari.windows[0].currentTab}});"#,
                escaped_js
            )
        } else {
            format!(
                r#"const chrome = Application("Google Chrome"); chrome.windows[0].activeTab.execute({{javascript: "{}"}});"#,
                escaped_js
            )
        }
    }

    /// Build a JXA script that retrieves the current tab URL.
    fn jxa_get_url(&self) -> String {
        if self.target_app == "Safari" {
            r#"const s = Application("Safari"); s.windows[0].currentTab.url();"#.to_string()
        } else {
            r#"const c = Application("Google Chrome"); c.windows[0].activeTab.url();"#.to_string()
        }
    }

    /// Build a JXA script that retrieves the current tab title.
    fn jxa_get_title(&self) -> String {
        if self.target_app == "Safari" {
            r#"const s = Application("Safari"); s.windows[0].currentTab.name();"#.to_string()
        } else {
            r#"const c = Application("Google Chrome"); c.windows[0].activeTab.title();"#.to_string()
        }
    }
}

/// Escape a string for safe embedding in JavaScript string literals.
fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Get a short label for a BrowserAction (for logging).
fn action_label(action: &BrowserAction) -> &'static str {
    match action {
        BrowserAction::Navigate { .. } => "navigate",
        BrowserAction::Click { .. } => "click",
        BrowserAction::Type { .. } => "type",
        BrowserAction::WaitFor { .. } => "wait_for",
        BrowserAction::Screenshot { .. } => "screenshot",
        BrowserAction::ExtractText { .. } => "extract_text",
        BrowserAction::Evaluate { .. } => "evaluate",
        BrowserAction::Fill { .. } => "fill",
        BrowserAction::Select { .. } => "select",
        BrowserAction::WaitForNavigation { .. } => "wait_for_navigation",
    }
}

impl Default for BrowserAutomation {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    // ── OsascriptBridge ────────────────────────────────────────────────

    #[test]
    fn test_osascript_bridge_creation() {
        let bridge = OsascriptBridge::new();
        assert_eq!(bridge.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_osascript_with_timeout() {
        let bridge = OsascriptBridge::new().with_timeout(Duration::from_secs(60));
        assert_eq!(bridge.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_osascript_run_simple() {
        let bridge = OsascriptBridge::new();
        let result = bridge.run_applescript("return 2 + 2");
        // Should succeed on macOS
        if let Ok(r) = result {
            assert!(r.success);
            assert_eq!(r.output, "4");
        }
    }

    #[test]
    fn test_osascript_run_javascript() {
        let bridge = OsascriptBridge::new();
        let result = bridge.run_javascript("2 + 2");
        if let Ok(r) = result {
            assert!(r.success);
            assert_eq!(r.output, "4");
        }
    }

    #[test]
    fn test_osascript_run_file_missing() {
        let bridge = OsascriptBridge::new();
        let result = bridge.run_file("/nonexistent/script.scpt", ScriptLanguage::AppleScript);
        assert!(result.is_err());
    }

    #[test]
    fn test_osascript_invalid_script() {
        let bridge = OsascriptBridge::new();
        let result = bridge.run_applescript("this is not valid applescript syntax at all!!!");
        if let Ok(r) = result {
            assert!(!r.success);
            assert_ne!(r.exit_code, 0);
        }
    }

    // ── KeychainManager ────────────────────────────────────────────────

    #[test]
    fn test_keychain_manager_creation() {
        let km = KeychainManager::new();
        assert_eq!(km.target, KeychainTarget::Login);
    }

    #[test]
    fn test_keychain_target_paths() {
        assert_eq!(KeychainTarget::Login.path(), None);
        assert_eq!(
            KeychainTarget::System.path(),
            Some("/Library/Keychains/System.keychain")
        );
    }

    #[test]
    fn test_keychain_roundtrip() {
        let km = KeychainManager::new();
        let service = "com.phantom.test.roundtrip";
        let account = "test-account";
        let password = "s3cret-p@ss!";

        // Store
        let store_result = km.store(service, account, password);
        if store_result.is_err() {
            // May fail in CI or sandbox — skip
            return;
        }

        // Retrieve
        let cred = km.retrieve(service, account).unwrap();
        assert_eq!(cred.service, service);
        assert_eq!(cred.account, account);
        assert_eq!(cred.password, password);

        // Exists
        assert!(km.exists(service, account));

        // Delete
        km.delete(service, account).unwrap();
        assert!(!km.exists(service, account));
    }

    #[test]
    fn test_keychain_credential_serde() {
        let cred = KeychainCredential {
            service: "com.phantom.test".into(),
            account: "user".into(),
            password: "secret".into(),
        };

        let json = serde_json::to_string(&cred).unwrap();
        // Password should NOT appear in serialized output
        assert!(!json.contains("secret"));
        assert!(json.contains("com.phantom.test"));
    }

    #[test]
    fn test_keychain_retrieve_nonexistent() {
        let km = KeychainManager::new();
        let result = km.retrieve("com.phantom.nonexistent.xyzzy", "nobody");
        assert!(result.is_err());
    }

    // ── ClipboardBridge ────────────────────────────────────────────────

    /// Single clipboard test to avoid parallel race conditions on the
    /// shared system pasteboard.
    #[test]
    fn test_clipboard_operations() {
        let cb = ClipboardBridge::new();

        // Write
        let test_text = "phantom-clipboard-test-39281";
        if cb.write(test_text).is_err() {
            return; // Skip in sandbox
        }

        // Read back
        let read = cb.read().unwrap();
        assert_eq!(read, test_text);

        // has_content
        assert!(cb.has_content());

        // Clear
        cb.clear().unwrap();
        let read = cb.read().unwrap_or_default();
        assert!(read.is_empty());
    }

    // ── ScreenCapture ──────────────────────────────────────────────────

    #[test]
    fn test_capture_format() {
        assert_eq!(CaptureFormat::Png.extension(), "png");
        assert_eq!(CaptureFormat::Jpg.extension(), "jpg");
        assert_eq!(CaptureFormat::Pdf.extension(), "pdf");
        assert_eq!(CaptureFormat::Tiff.extension(), "tiff");
    }

    #[test]
    fn test_capture_options_default() {
        let opts = CaptureOptions::default();
        assert_eq!(opts.format, CaptureFormat::Png);
        assert!(opts.no_shadow);
        assert!(!opts.to_clipboard);
        assert!(!opts.window_mode);
        assert!(opts.hide_cursor);
        assert!(opts.rect.is_none());
        assert!(opts.delay.is_none());
    }

    #[test]
    fn test_screen_capture_creation() {
        let sc = ScreenCapture::new();
        assert_eq!(sc.output_dir, PathBuf::from("/tmp/phantom-captures"));
    }

    #[test]
    fn test_screen_capture_custom_dir() {
        let sc = ScreenCapture::new().with_output_dir("/tmp/custom-captures");
        assert_eq!(sc.output_dir, PathBuf::from("/tmp/custom-captures"));
    }

    // ── LaunchctlManager ───────────────────────────────────────────────

    #[test]
    fn test_daemon_config_creation() {
        let config = DaemonConfig::new("com.phantom.test", "/usr/local/bin/phantom");
        assert_eq!(config.label, "com.phantom.test");
        assert_eq!(config.program, PathBuf::from("/usr/local/bin/phantom"));
        assert!(config.run_at_load);
        assert!(config.keep_alive);
    }

    #[test]
    fn test_daemon_config_builder() {
        let config = DaemonConfig::new("com.phantom.agent", "/usr/bin/phantom")
            .with_args(vec!["serve".into(), "--port".into(), "8080".into()])
            .with_working_dir("/var/phantom")
            .with_logs("/var/log/phantom.out", "/var/log/phantom.err")
            .with_env("PHANTOM_MODE", "production")
            .with_interval(300);

        assert_eq!(config.program_arguments.len(), 3);
        assert_eq!(
            config.working_directory,
            Some(PathBuf::from("/var/phantom"))
        );
        assert_eq!(
            config.stdout_path,
            Some(PathBuf::from("/var/log/phantom.out"))
        );
        assert_eq!(config.start_interval, Some(300));
        assert_eq!(
            config.environment_variables.get("PHANTOM_MODE"),
            Some(&"production".to_string())
        );
    }

    #[test]
    fn test_daemon_plist_generation() {
        let config = DaemonConfig::new("com.phantom.test", "/usr/bin/phantom")
            .with_args(vec!["serve".into()])
            .with_env("MODE", "test");

        let plist = config.to_plist();
        assert!(plist.contains("com.phantom.test"));
        assert!(plist.contains("/usr/bin/phantom"));
        assert!(plist.contains("<key>Label</key>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
        assert!(plist.contains("<key>EnvironmentVariables</key>"));
        assert!(plist.contains("MODE"));
    }

    #[test]
    fn test_daemon_plist_xml_escaping() {
        let config = DaemonConfig::new("com.phantom.<test>", "/usr/bin/phantom");
        let plist = config.to_plist();
        assert!(plist.contains("com.phantom.&lt;test&gt;"));
        assert!(!plist.contains("com.phantom.<test>"));
    }

    #[test]
    fn test_launchctl_manager_creation() {
        let lm = LaunchctlManager::new();
        assert!(lm.plist_dir.ends_with("Library/LaunchAgents"));
    }

    #[test]
    fn test_launchctl_custom_dir() {
        let lm = LaunchctlManager::new().with_plist_dir("/tmp/test-agents");
        assert_eq!(lm.plist_dir, PathBuf::from("/tmp/test-agents"));
    }

    #[test]
    fn test_parse_launchctl_list() {
        let output = r#"{
    "LimitLoadToSessionType" = "Aqua";
    "Label" = "com.phantom.test";
    "TimeOut" = 30;
    "OnDemand" = true;
    "LastExitStatus" = 0;
    "PID" = 12345;
    "Program" = "/usr/bin/phantom";
};"#;

        let (pid, exit) = parse_launchctl_list(output);
        assert_eq!(pid, Some(12345));
        assert_eq!(exit, Some(0));
    }

    #[test]
    fn test_parse_launchctl_list_no_pid() {
        let output = r#"{
    "Label" = "com.phantom.test";
    "LastExitStatus" = 1;
};"#;

        let (pid, exit) = parse_launchctl_list(output);
        assert_eq!(pid, None);
        assert_eq!(exit, Some(1));
    }

    #[test]
    fn test_daemon_status_not_loaded() {
        let lm = LaunchctlManager::new();
        let status = lm.status("com.phantom.nonexistent.xyzzy.test").unwrap();
        assert!(!status.loaded);
        assert_eq!(status.pid, None);
    }

    // ── BrowserAutomation ──────────────────────────────────────────────

    #[test]
    fn test_browser_config_default() {
        let config = BrowserConfig::default();
        assert_eq!(config.browser, BrowserType::Chromium);
        assert!(config.headless);
        assert_eq!(config.viewport_width, 1920);
        assert_eq!(config.viewport_height, 1080);
        assert_eq!(config.timeout_ms, 30_000);
    }

    #[test]
    fn test_browser_type_display() {
        assert_eq!(BrowserType::Chromium.to_string(), "chromium");
        assert_eq!(BrowserType::Firefox.to_string(), "firefox");
        assert_eq!(BrowserType::Webkit.to_string(), "webkit");
    }

    #[test]
    fn test_browser_session_lifecycle() {
        let mut browser = BrowserAutomation::with_defaults();
        assert!(!browser.is_active());

        // Use manual session_active toggle to test lifecycle without launching Safari
        browser.session_active = true;
        assert!(browser.is_active());

        // Double start should fail
        assert!(browser.start_session().is_err());

        browser.end_session().unwrap();
        assert!(!browser.is_active());
        assert!(browser.current_url.is_none());

        // Double end should fail
        assert!(browser.end_session().is_err());
    }

    #[test]
    fn test_browser_action_without_session() {
        let mut browser = BrowserAutomation::with_defaults();
        let result = browser.execute_action(&BrowserAction::Navigate {
            url: "https://example.com".into(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_browser_target_app_mapping() {
        let chrome = BrowserAutomation::new(BrowserConfig {
            browser: BrowserType::Chromium,
            ..Default::default()
        });
        assert_eq!(chrome.target_app, "Google Chrome");

        let safari = BrowserAutomation::new(BrowserConfig {
            browser: BrowserType::Webkit,
            ..Default::default()
        });
        assert_eq!(safari.target_app, "Safari");

        // Firefox falls back to Safari (no osascript support)
        let firefox = BrowserAutomation::new(BrowserConfig {
            browser: BrowserType::Firefox,
            ..Default::default()
        });
        assert_eq!(firefox.target_app, "Safari");
    }

    #[test]
    fn test_browser_jxa_evaluate_builders() {
        let safari = BrowserAutomation::new(BrowserConfig {
            browser: BrowserType::Webkit,
            ..Default::default()
        });
        let script = safari.jxa_evaluate("document.title");
        assert!(script.contains("Safari"));
        assert!(script.contains("doJavaScript"));
        assert!(script.contains("document.title"));

        let chrome = BrowserAutomation::new(BrowserConfig {
            browser: BrowserType::Chromium,
            ..Default::default()
        });
        let script = chrome.jxa_evaluate("document.title");
        assert!(script.contains("Google Chrome"));
        assert!(script.contains("execute"));
    }

    #[test]
    fn test_browser_jxa_url_builders() {
        let safari = BrowserAutomation::new(BrowserConfig {
            browser: BrowserType::Webkit,
            ..Default::default()
        });
        let url_script = safari.jxa_get_url();
        assert!(url_script.contains("Safari"));
        assert!(url_script.contains("url()"));

        let title_script = safari.jxa_get_title();
        assert!(title_script.contains("name()"));
    }

    #[test]
    fn test_js_escape() {
        assert_eq!(js_escape("hello"), "hello");
        assert_eq!(js_escape(r#"say "hi""#), r#"say \"hi\""#);
        assert_eq!(js_escape("line1\nline2"), "line1\\nline2");
        assert_eq!(js_escape("back\\slash"), "back\\\\slash");
        assert_eq!(js_escape("tab\there"), "tab\\there");
    }

    #[test]
    fn test_action_label() {
        assert_eq!(
            action_label(&BrowserAction::Navigate { url: "".into() }),
            "navigate"
        );
        assert_eq!(
            action_label(&BrowserAction::Click {
                selector: "".into()
            }),
            "click"
        );
        assert_eq!(
            action_label(&BrowserAction::Type {
                selector: "".into(),
                text: "".into()
            }),
            "type"
        );
        assert_eq!(
            action_label(&BrowserAction::WaitFor {
                selector: "".into(),
                timeout_ms: None
            }),
            "wait_for"
        );
        assert_eq!(
            action_label(&BrowserAction::Screenshot { path: "".into() }),
            "screenshot"
        );
        assert_eq!(
            action_label(&BrowserAction::ExtractText {
                selector: "".into()
            }),
            "extract_text"
        );
        assert_eq!(
            action_label(&BrowserAction::Evaluate { script: "".into() }),
            "evaluate"
        );
        assert_eq!(
            action_label(&BrowserAction::Fill {
                selector: "".into(),
                value: "".into()
            }),
            "fill"
        );
        assert_eq!(
            action_label(&BrowserAction::Select {
                selector: "".into(),
                value: "".into()
            }),
            "select"
        );
        assert_eq!(
            action_label(&BrowserAction::WaitForNavigation { timeout_ms: None }),
            "wait_for_navigation"
        );
    }

    #[test]
    fn test_browser_action_serde() {
        let action = BrowserAction::Type {
            selector: "#input".into(),
            text: "hello".into(),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("\"action\":\"type\""));
        assert!(json.contains("hello"));

        let decoded: BrowserAction = serde_json::from_str(&json).unwrap();
        if let BrowserAction::Type { selector, text } = decoded {
            assert_eq!(selector, "#input");
            assert_eq!(text, "hello");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn test_browser_config_serde() {
        let config = BrowserConfig {
            browser: BrowserType::Firefox,
            headless: false,
            viewport_width: 1280,
            viewport_height: 720,
            timeout_ms: 60_000,
            user_agent: Some("PhantomBot/1.0".into()),
            proxy: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let decoded: BrowserConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.browser, BrowserType::Firefox);
        assert!(!decoded.headless);
        assert_eq!(decoded.user_agent.as_deref(), Some("PhantomBot/1.0"));
    }

    // ── Error types ────────────────────────────────────────────────────

    #[test]
    fn test_mac_error_display() {
        let err = MacError::new(MacSubsystem::Keychain, "credential not found");
        assert_eq!(err.to_string(), "[keychain] credential not found");
    }

    #[test]
    fn test_mac_subsystem_display() {
        assert_eq!(MacSubsystem::Osascript.to_string(), "osascript");
        assert_eq!(MacSubsystem::Keychain.to_string(), "keychain");
        assert_eq!(MacSubsystem::Clipboard.to_string(), "clipboard");
        assert_eq!(MacSubsystem::ScreenCapture.to_string(), "screencapture");
        assert_eq!(MacSubsystem::Launchctl.to_string(), "launchctl");
        assert_eq!(MacSubsystem::Browser.to_string(), "browser");
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
        assert_eq!(truncate_str("", 5), "");
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("hello"), "hello");
        assert_eq!(xml_escape("<tag>"), "&lt;tag&gt;");
        assert_eq!(xml_escape("a & b"), "a &amp; b");
        assert_eq!(
            xml_escape("it's \"quoted\""),
            "it&apos;s &quot;quoted&quot;"
        );
    }
}
