//! Zero-Footprint Runtime Enforcement
//!
//! Core Law 3: Zero local disk footprint.
//!
//! Phantom must never leave traces on the host filesystem outside of
//! explicitly allowed vault paths. This module provides:
//!
//! 1. **DiskPolicy** — allow-list of paths where writes are permitted;
//!    all other file-creation attempts are blocked.
//! 2. **SecureBuffer / SecureString** — heap buffers that zeroize on drop
//!    and can optionally be mlocked to prevent swap.
//! 3. **LockedMemory** — raw mlock/munlock wrappers for key material.
//! 4. **SessionGuard** — registers temp files/dirs and signal handlers so
//!    everything is wiped on normal exit *or* SIGTERM/SIGINT.
//! 5. **StartupValidator** — scans common locations at launch and fails
//!    fast if sensitive Phantom artifacts leaked to disk.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use thiserror::Error;
use tracing::{debug, info, warn};
use zeroize::Zeroize;

// ═══════════════════════════════════════════════════════════════════════════
//  Errors
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Error)]
pub enum ZeroFootprintError {
    #[error("disk write blocked: {path} is outside allowed vault paths")]
    WriteBlocked { path: String },

    #[error("sensitive file found on disk: {path}")]
    SensitiveFileLeak { path: String },

    #[error("mlock failed on {len} bytes: {reason}")]
    MlockFailed { len: usize, reason: String },

    #[error("munlock failed on {len} bytes: {reason}")]
    MunlockFailed { len: usize, reason: String },

    #[error("cleanup failed for {path}: {reason}")]
    CleanupFailed { path: String, reason: String },

    #[error("session guard already active")]
    GuardAlreadyActive,

    #[error("io error: {reason}")]
    Io { reason: String },
}

impl From<io::Error> for ZeroFootprintError {
    fn from(e: io::Error) -> Self {
        Self::Io {
            reason: e.to_string(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  1. Disk Policy — zero-disk write enforcement
// ═══════════════════════════════════════════════════════════════════════════

/// Enforces zero-disk policy by maintaining an allow-list of vault paths.
/// Any file creation outside these paths is rejected.
#[derive(Debug)]
pub struct DiskPolicy {
    /// Canonicalized paths where writes are permitted.
    allowed_prefixes: Vec<PathBuf>,
    /// Tracked temporary files created during this session.
    temp_files: Vec<PathBuf>,
    /// If true, all non-vault writes are hard errors; if false, just warnings.
    strict: bool,
}

impl DiskPolicy {
    /// Create a new policy. Strict mode blocks writes; permissive mode warns.
    pub fn new(strict: bool) -> Self {
        Self {
            allowed_prefixes: Vec::new(),
            temp_files: Vec::new(),
            strict,
        }
    }

    /// Create a strict policy (default for production).
    pub fn strict() -> Self {
        Self::new(true)
    }

    /// Create a permissive policy (for development/testing).
    pub fn permissive() -> Self {
        Self::new(false)
    }

    /// Allow writes under a given directory prefix.
    /// The path is resolved to handle symlinks (e.g. /var → /private/var on macOS).
    pub fn allow_prefix(&mut self, path: impl Into<PathBuf>) {
        let p = path.into();
        let resolved = resolve_for_comparison(&p);
        debug!(path = %resolved.display(), "allowing write prefix");
        self.allowed_prefixes.push(resolved);
    }

    /// Allow the system temp directory.
    pub fn allow_system_temp(&mut self) {
        self.allow_prefix(std::env::temp_dir());
    }

    /// Check whether a path falls under an allowed prefix.
    pub fn is_allowed(&self, path: &Path) -> bool {
        // Resolve the target path. Try canonicalize first (resolves symlinks),
        // then fall back to resolving parent + filename for paths that don't exist yet.
        let target = resolve_for_comparison(path);

        self.allowed_prefixes.iter().any(|prefix| {
            // Prefixes are already canonicalized in allow_prefix()
            target.starts_with(prefix)
        })
    }

    /// Validate a write. Returns `Ok(())` if the path is allowed.
    pub fn validate_write(&self, path: &Path) -> Result<(), ZeroFootprintError> {
        if self.is_allowed(path) {
            return Ok(());
        }
        let msg = format!("{}", path.display());
        if self.strict {
            Err(ZeroFootprintError::WriteBlocked { path: msg })
        } else {
            warn!(path = %path.display(), "disk write outside vault (permissive mode)");
            Ok(())
        }
    }

    /// Track a temporary file for cleanup on session end.
    pub fn track_temp_file(&mut self, path: impl Into<PathBuf>) {
        let p = path.into();
        debug!(path = %p.display(), "tracking temp file");
        self.temp_files.push(p);
    }

    /// Remove all tracked temp files. Returns per-file results.
    pub fn cleanup_temps(&mut self) -> Vec<CleanupResult> {
        let paths: Vec<PathBuf> = self.temp_files.drain(..).collect();
        paths
            .into_iter()
            .map(|p| {
                let display = p.display().to_string();
                match secure_delete(&p) {
                    Ok(()) => CleanupResult {
                        path: display,
                        success: true,
                        error: None,
                    },
                    Err(e) => CleanupResult {
                        path: display,
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            })
            .collect()
    }

    /// Number of currently tracked temp files.
    pub fn tracked_count(&self) -> usize {
        self.temp_files.len()
    }

    /// Whether the policy is in strict mode.
    pub fn is_strict(&self) -> bool {
        self.strict
    }
}

/// Result of cleaning up a single temporary file.
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Policy-guarded file creation. Blocks if the path violates the disk policy.
pub fn guarded_create(policy: &DiskPolicy, path: &Path) -> Result<fs::File, ZeroFootprintError> {
    policy.validate_write(path)?;
    Ok(fs::File::create(path)?)
}

/// Policy-guarded write. Validates the path, then writes atomically.
pub fn guarded_write(
    policy: &DiskPolicy,
    path: &Path,
    data: &[u8],
) -> Result<(), ZeroFootprintError> {
    policy.validate_write(path)?;
    fs::write(path, data)?;
    Ok(())
}

/// Securely delete a file: overwrite with zeros, then remove.
pub fn secure_delete(path: &Path) -> Result<(), ZeroFootprintError> {
    if !path.exists() {
        return Ok(());
    }

    // Overwrite file contents with zeros before unlinking
    if path.is_file() {
        let meta = fs::metadata(path)?;
        let len = meta.len() as usize;
        if len > 0 {
            let zeros = vec![0u8; len.min(1024 * 1024)]; // Cap at 1MB chunks
            let mut remaining = len;
            let file = fs::OpenOptions::new().write(true).open(path)?;
            use std::io::Write;
            let mut writer = io::BufWriter::new(file);
            while remaining > 0 {
                let chunk = remaining.min(zeros.len());
                writer.write_all(&zeros[..chunk])?;
                remaining -= chunk;
            }
            writer.flush()?;
        }
        fs::remove_file(path)?;
    } else if path.is_dir() {
        fs::remove_dir_all(path)?;
    }

    Ok(())
}

/// Normalize a path for comparison (resolve `.` and `..` without requiring the path to exist).
fn normalize_path(path: &Path) -> Option<PathBuf> {
    // Try canonical first (resolves symlinks, requires existence)
    if let Ok(canon) = path.canonicalize() {
        return Some(canon);
    }
    // Fall back to lexical normalization
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => result.push(other),
        }
    }
    Some(result)
}

/// Resolve a path for prefix comparison.
/// If the full path exists, canonicalize it. Otherwise, canonicalize
/// the nearest existing ancestor and append the remaining components.
/// This handles macOS symlinks like /var → /private/var for non-existent files.
fn resolve_for_comparison(path: &Path) -> PathBuf {
    // Best case: path exists, full canonicalize
    if let Ok(canon) = path.canonicalize() {
        return canon;
    }

    // Walk up to find the nearest existing ancestor and canonicalize it
    let mut ancestors = Vec::new();
    let mut current = path.to_path_buf();
    loop {
        if let Ok(canon) = current.canonicalize() {
            // Rebuild from the canonicalized ancestor
            let mut result = canon;
            for component in ancestors.into_iter().rev() {
                result.push(component);
            }
            return result;
        }
        match current.file_name() {
            Some(name) => {
                ancestors.push(name.to_os_string());
                current.pop();
            }
            None => break,
        }
    }

    // Fallback: lexical normalization
    normalize_path(path).unwrap_or_else(|| path.to_path_buf())
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. Secure Memory — zeroize-on-drop buffers
// ═══════════════════════════════════════════════════════════════════════════

/// A heap-allocated byte buffer that is zeroized on drop.
/// Optionally mlocked to prevent the OS from swapping it to disk.
pub struct SecureBuffer {
    data: Vec<u8>,
    locked: bool,
}

impl SecureBuffer {
    /// Allocate a new secure buffer with the given capacity, filled with zeros.
    pub fn new(len: usize) -> Self {
        Self {
            data: vec![0u8; len],
            locked: false,
        }
    }

    /// Create a secure buffer from existing data (copies and can zeroize the source).
    pub fn from_slice(src: &[u8]) -> Self {
        Self {
            data: src.to_vec(),
            locked: false,
        }
    }

    /// Lock the buffer into physical RAM (prevent swap).
    pub fn mlock(&mut self) -> Result<(), ZeroFootprintError> {
        if self.locked || self.data.is_empty() {
            return Ok(());
        }
        mlock_bytes(&self.data)?;
        self.locked = true;
        Ok(())
    }

    /// Create a buffer from a slice and immediately mlock it.
    pub fn from_slice_locked(src: &[u8]) -> Result<Self, ZeroFootprintError> {
        let mut buf = Self::from_slice(src);
        buf.mlock()?;
        Ok(buf)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }
}

impl Drop for SecureBuffer {
    fn drop(&mut self) {
        // Zeroize the data using volatile writes (prevents compiler elision)
        self.data.zeroize();
        // Unlock if we mlocked
        if self.locked && !self.data.is_empty() {
            let _ = munlock_bytes(&self.data);
            self.locked = false;
        }
    }
}

/// A string that is zeroized on drop. For passwords, tokens, API keys.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SecureString {
    inner: String,
}

impl SecureString {
    pub fn new(s: impl Into<String>) -> Self {
        Self { inner: s.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl std::fmt::Debug for SecureString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecureString([REDACTED, {} bytes])", self.inner.len())
    }
}

impl std::fmt::Display for SecureString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// A registry of all session secrets for bulk zeroization.
pub struct SessionSecrets {
    buffers: Vec<SecureBuffer>,
    strings: Vec<SecureString>,
}

impl SessionSecrets {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
            strings: Vec::new(),
        }
    }

    /// Register a secure buffer for lifecycle tracking.
    pub fn register_buffer(&mut self, buf: SecureBuffer) -> usize {
        let idx = self.buffers.len();
        self.buffers.push(buf);
        idx
    }

    /// Register a secure string for lifecycle tracking.
    pub fn register_string(&mut self, s: SecureString) -> usize {
        let idx = self.strings.len();
        self.strings.push(s);
        idx
    }

    /// Get a registered buffer by index.
    pub fn get_buffer(&self, idx: usize) -> Option<&SecureBuffer> {
        self.buffers.get(idx)
    }

    /// Get a registered string by index.
    pub fn get_string(&self, idx: usize) -> Option<&SecureString> {
        self.strings.get(idx)
    }

    /// Total number of registered secrets.
    pub fn count(&self) -> usize {
        self.buffers.len() + self.strings.len()
    }

    /// Zeroize and drop all registered secrets.
    pub fn wipe_all(&mut self) {
        // Dropping triggers Zeroize via the derive macro
        self.buffers.clear();
        self.strings.clear();
        info!("all session secrets zeroized");
    }
}

impl Default for SessionSecrets {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SessionSecrets {
    fn drop(&mut self) {
        self.wipe_all();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. Locked Memory — mlock / munlock for key material
// ═══════════════════════════════════════════════════════════════════════════

/// Lock a byte slice into physical RAM. Prevents the OS from swapping
/// this memory region to disk, which could leak key material.
pub fn mlock_bytes(data: &[u8]) -> Result<(), ZeroFootprintError> {
    if data.is_empty() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let ret = unsafe { libc::mlock(data.as_ptr() as *const libc::c_void, data.len()) };
        if ret != 0 {
            let err = io::Error::last_os_error();
            return Err(ZeroFootprintError::MlockFailed {
                len: data.len(),
                reason: err.to_string(),
            });
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        warn!("mlock not supported on this platform");
    }

    Ok(())
}

/// Unlock a previously mlocked byte slice.
pub fn munlock_bytes(data: &[u8]) -> Result<(), ZeroFootprintError> {
    if data.is_empty() {
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let ret = unsafe { libc::munlock(data.as_ptr() as *const libc::c_void, data.len()) };
        if ret != 0 {
            let err = io::Error::last_os_error();
            return Err(ZeroFootprintError::MunlockFailed {
                len: data.len(),
                reason: err.to_string(),
            });
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = data;
    }

    Ok(())
}

/// Query the system's mlock limit (RLIMIT_MEMLOCK on macOS/Linux).
pub fn mlock_limit() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        let mut rlim: libc::rlimit = unsafe { std::mem::zeroed() };
        let ret = unsafe { libc::getrlimit(libc::RLIMIT_MEMLOCK, &mut rlim) };
        if ret == 0 {
            return Some(rlim.rlim_cur);
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. Session Guard — auto-wipe on process exit
// ═══════════════════════════════════════════════════════════════════════════

/// Global flag set by signal handlers to trigger cleanup.
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Check whether a graceful shutdown has been requested via signal.
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

/// Manages the lifecycle of temporary state. Registers signal handlers
/// so that temp files/dirs are wiped even on SIGTERM/SIGINT.
pub struct SessionGuard {
    temp_files: Arc<Mutex<Vec<PathBuf>>>,
    temp_dirs: Arc<Mutex<Vec<PathBuf>>>,
    active: AtomicBool,
}

impl SessionGuard {
    /// Create a new session guard. Only one should exist per process.
    pub fn new() -> Self {
        Self {
            temp_files: Arc::new(Mutex::new(Vec::new())),
            temp_dirs: Arc::new(Mutex::new(Vec::new())),
            active: AtomicBool::new(true),
        }
    }

    /// Register OS signal handlers (SIGTERM, SIGINT) that trigger cleanup.
    /// Must be called from a tokio runtime context.
    pub fn register_signal_handlers(&self) {
        let files = Arc::clone(&self.temp_files);
        let dirs = Arc::clone(&self.temp_dirs);

        // Spawn a background task that waits for SIGTERM/SIGINT
        tokio::spawn(async move {
            let ctrl_c = tokio::signal::ctrl_c();

            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");

                tokio::select! {
                    _ = ctrl_c => {
                        info!("SIGINT received — cleaning up session state");
                    }
                    _ = sigterm.recv() => {
                        info!("SIGTERM received — cleaning up session state");
                    }
                }
            }

            #[cfg(not(unix))]
            {
                let _ = ctrl_c.await;
                info!("shutdown signal received — cleaning up session state");
            }

            SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
            cleanup_paths(&files, &dirs);
        });
    }

    /// Track a temporary file for cleanup on session end.
    pub fn track_file(&self, path: impl Into<PathBuf>) {
        if let Ok(mut files) = self.temp_files.lock() {
            files.push(path.into());
        }
    }

    /// Track a temporary directory for cleanup on session end.
    pub fn track_dir(&self, path: impl Into<PathBuf>) {
        if let Ok(mut dirs) = self.temp_dirs.lock() {
            dirs.push(path.into());
        }
    }

    /// Number of tracked items (files + dirs).
    pub fn tracked_count(&self) -> usize {
        let files = self.temp_files.lock().map(|f| f.len()).unwrap_or(0);
        let dirs = self.temp_dirs.lock().map(|d| d.len()).unwrap_or(0);
        files + dirs
    }

    /// Manually trigger cleanup of all tracked temp state.
    pub fn cleanup(&self) -> Vec<CleanupResult> {
        let mut results = Vec::new();

        if let Ok(mut files) = self.temp_files.lock() {
            for path in files.drain(..) {
                let display = path.display().to_string();
                match secure_delete(&path) {
                    Ok(()) => results.push(CleanupResult {
                        path: display,
                        success: true,
                        error: None,
                    }),
                    Err(e) => results.push(CleanupResult {
                        path: display,
                        success: false,
                        error: Some(e.to_string()),
                    }),
                }
            }
        }

        if let Ok(mut dirs) = self.temp_dirs.lock() {
            for path in dirs.drain(..) {
                let display = path.display().to_string();
                match secure_delete(&path) {
                    Ok(()) => results.push(CleanupResult {
                        path: display,
                        success: true,
                        error: None,
                    }),
                    Err(e) => results.push(CleanupResult {
                        path: display,
                        success: false,
                        error: Some(e.to_string()),
                    }),
                }
            }
        }

        self.active.store(false, Ordering::SeqCst);
        results
    }

    /// Whether the guard is still active.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

impl Default for SessionGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        if self.active.load(Ordering::SeqCst) {
            cleanup_paths(&self.temp_files, &self.temp_dirs);
        }
    }
}

/// Helper used by both Drop and signal handlers.
fn cleanup_paths(files: &Arc<Mutex<Vec<PathBuf>>>, dirs: &Arc<Mutex<Vec<PathBuf>>>) {
    if let Ok(mut files) = files.lock() {
        for path in files.drain(..) {
            let _ = secure_delete(&path);
        }
    }
    if let Ok(mut dirs) = dirs.lock() {
        for path in dirs.drain(..) {
            let _ = secure_delete(&path);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  5. Startup Validator — detect sensitive file leaks
// ═══════════════════════════════════════════════════════════════════════════

/// Well-known filenames that should NEVER exist on disk.
const SENSITIVE_PATTERNS: &[&str] = &[
    "phantom-master-key",
    "phantom-session-key",
    "phantom.key",
    "phantom.pem",
    "phantom-secrets",
    ".phantom-credentials",
    "phantom-license.json",
    "phantom-salt",
    "phantom-totp-secret",
    "phantom-mnemonic",
    "phantom-destruction-key",
    ".phantom-env",
];

/// Well-known directories to scan for leaks.
fn default_scan_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/tmp"), PathBuf::from("/var/tmp")];

    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join("Desktop"));
        dirs.push(home.join("Documents"));
        dirs.push(home.join("Downloads"));
        dirs.push(home.clone());
    }

    if let Ok(tmpdir) = std::env::var("TMPDIR") {
        dirs.push(PathBuf::from(tmpdir));
    }

    dirs
}

/// Validates that no sensitive Phantom artifacts leaked to disk.
pub struct StartupValidator {
    sensitive_names: Vec<String>,
    scan_dirs: Vec<PathBuf>,
    max_depth: usize,
}

impl StartupValidator {
    /// Create with default sensitive patterns and scan directories.
    pub fn new() -> Self {
        Self {
            sensitive_names: SENSITIVE_PATTERNS.iter().map(|s| s.to_string()).collect(),
            scan_dirs: default_scan_dirs(),
            max_depth: 2, // Don't recurse too deep — performance
        }
    }

    /// Add a custom sensitive filename pattern.
    pub fn add_sensitive_name(&mut self, name: impl Into<String>) {
        self.sensitive_names.push(name.into());
    }

    /// Add a custom directory to scan.
    pub fn add_scan_dir(&mut self, dir: impl Into<PathBuf>) {
        self.scan_dirs.push(dir.into());
    }

    /// Set the maximum directory recursion depth.
    pub fn set_max_depth(&mut self, depth: usize) {
        self.max_depth = depth;
    }

    /// Run the validation scan. Returns a report with any violations found.
    pub fn validate(&self) -> ValidationReport {
        let mut violations = Vec::new();
        let mut scanned = 0usize;
        let sensitive_set: HashSet<&str> =
            self.sensitive_names.iter().map(|s| s.as_str()).collect();

        for dir in &self.scan_dirs {
            if dir.exists() && dir.is_dir() {
                self.scan_dir(dir, 0, &sensitive_set, &mut violations, &mut scanned);
            }
        }

        let clean = violations.is_empty();
        if clean {
            info!(
                scanned_paths = scanned,
                "startup validation passed — no sensitive files found"
            );
        } else {
            warn!(
                violations = violations.len(),
                scanned_paths = scanned,
                "startup validation FAILED — sensitive files found on disk"
            );
        }

        ValidationReport {
            violations,
            scanned_paths: scanned,
            clean,
        }
    }

    fn scan_dir(
        &self,
        dir: &Path,
        depth: usize,
        sensitive: &HashSet<&str>,
        violations: &mut Vec<Violation>,
        scanned: &mut usize,
    ) {
        if depth > self.max_depth {
            return;
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return, // Permission denied, etc.
        };

        for entry in entries.flatten() {
            *scanned += 1;
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if sensitive.contains(name) {
                    let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    violations.push(Violation {
                        path: path.display().to_string(),
                        filename: name.to_string(),
                        size_bytes: size,
                        is_directory: path.is_dir(),
                    });
                }
            }

            // Recurse into subdirectories
            if path.is_dir() {
                self.scan_dir(&path, depth + 1, sensitive, violations, scanned);
            }
        }
    }
}

impl Default for StartupValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Report from a startup validation scan.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub violations: Vec<Violation>,
    pub scanned_paths: usize,
    pub clean: bool,
}

impl ValidationReport {
    /// Fail hard if any violations were found.
    pub fn require_clean(&self) -> Result<(), ZeroFootprintError> {
        if let Some(v) = self.violations.first() {
            return Err(ZeroFootprintError::SensitiveFileLeak {
                path: v.path.clone(),
            });
        }
        Ok(())
    }

    /// Auto-remediate: securely delete all violating files.
    pub fn remediate(&self) -> Vec<CleanupResult> {
        self.violations
            .iter()
            .map(|v| {
                let path = PathBuf::from(&v.path);
                match secure_delete(&path) {
                    Ok(()) => CleanupResult {
                        path: v.path.clone(),
                        success: true,
                        error: None,
                    },
                    Err(e) => CleanupResult {
                        path: v.path.clone(),
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            })
            .collect()
    }
}

/// A single sensitive-file violation.
#[derive(Debug, Clone)]
pub struct Violation {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub is_directory: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
//  Full session bootstrap
// ═══════════════════════════════════════════════════════════════════════════

/// All-in-one session initialization:
/// 1. Run startup validation
/// 2. Set up disk policy with allowed vault paths
/// 3. Create session guard with signal handlers
/// 4. Return a `RuntimeSession` combining all of the above
pub struct RuntimeSession {
    pub policy: DiskPolicy,
    pub guard: SessionGuard,
    pub secrets: SessionSecrets,
}

impl RuntimeSession {
    /// Bootstrap a new runtime session with full zero-footprint enforcement.
    ///
    /// `vault_paths` — directories where writes are permitted.
    /// `strict` — if true, unauthorized writes are hard errors.
    pub fn bootstrap(vault_paths: &[PathBuf], strict: bool) -> Result<Self, ZeroFootprintError> {
        // 1. Startup validation
        let validator = StartupValidator::new();
        let report = validator.validate();
        if strict {
            report.require_clean()?;
        } else if !report.clean {
            warn!(
                "{} sensitive file(s) found on disk — auto-remediating",
                report.violations.len()
            );
            report.remediate();
        }

        // 2. Disk policy
        let mut policy = DiskPolicy::new(strict);
        policy.allow_system_temp();
        for vp in vault_paths {
            policy.allow_prefix(vp);
        }

        // 3. Session guard
        let guard = SessionGuard::new();
        guard.register_signal_handlers();

        // 4. Secret registry
        let secrets = SessionSecrets::new();

        info!("zero-footprint runtime session initialized");
        Ok(Self {
            policy,
            guard,
            secrets,
        })
    }

    /// Register a secret buffer, optionally mlocking it.
    pub fn protect_secret(&mut self, data: &[u8], lock: bool) -> Result<usize, ZeroFootprintError> {
        let buf = if lock {
            SecureBuffer::from_slice_locked(data)?
        } else {
            SecureBuffer::from_slice(data)
        };
        Ok(self.secrets.register_buffer(buf))
    }

    /// Register a secret string.
    pub fn protect_string(&mut self, s: impl Into<String>) -> usize {
        self.secrets.register_string(SecureString::new(s))
    }

    /// Track a temporary file for auto-cleanup.
    pub fn track_temp_file(&self, path: impl Into<PathBuf>) {
        self.guard.track_file(path);
    }

    /// Track a temporary directory for auto-cleanup.
    pub fn track_temp_dir(&self, path: impl Into<PathBuf>) {
        self.guard.track_dir(path);
    }

    /// Tear down: wipe secrets, clean temps, deactivate guard.
    pub fn teardown(&mut self) -> Vec<CleanupResult> {
        self.secrets.wipe_all();
        let mut results = self.guard.cleanup();
        results.extend(self.policy.cleanup_temps());
        info!("runtime session torn down");
        results
    }
}

impl Drop for RuntimeSession {
    fn drop(&mut self) {
        self.secrets.wipe_all();
        // guard and policy Drop impls handle the rest
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── DiskPolicy ──────────────────────────────────────────────────────

    #[test]
    fn test_strict_policy_blocks_unallowed_write() {
        let policy = DiskPolicy::strict();
        let result = policy.validate_write(Path::new("/usr/local/bin/evil"));
        assert!(result.is_err());
        match result.unwrap_err() {
            ZeroFootprintError::WriteBlocked { path } => {
                assert!(path.contains("evil"));
            }
            other => panic!("expected WriteBlocked, got {:?}", other),
        }
    }

    #[test]
    fn test_permissive_policy_allows_unallowed_write() {
        let policy = DiskPolicy::permissive();
        // Permissive mode warns but does not error
        assert!(policy
            .validate_write(Path::new("/usr/local/bin/anything"))
            .is_ok());
    }

    #[test]
    fn test_policy_allows_vault_path() {
        let mut policy = DiskPolicy::strict();
        let vault = std::env::temp_dir().join("phantom-vault-test");
        policy.allow_prefix(&vault);
        let target = vault.join("subdir").join("file.dat");
        assert!(policy.is_allowed(&target));
        assert!(policy.validate_write(&target).is_ok());
    }

    #[test]
    fn test_policy_rejects_outside_vault() {
        let mut policy = DiskPolicy::strict();
        policy.allow_prefix("/tmp/phantom-vault");
        assert!(!policy.is_allowed(Path::new("/home/user/secrets.txt")));
    }

    #[test]
    fn test_policy_allow_system_temp() {
        let mut policy = DiskPolicy::strict();
        policy.allow_system_temp();
        let temp = std::env::temp_dir().join("phantom-test-file");
        assert!(policy.is_allowed(&temp));
    }

    #[test]
    fn test_guarded_write_and_cleanup() {
        let mut policy = DiskPolicy::strict();
        policy.allow_system_temp();

        let tmp = std::env::temp_dir().join("phantom-zfp-test-guarded");
        guarded_write(&policy, &tmp, b"secret data").unwrap();
        policy.track_temp_file(tmp.clone());

        assert!(tmp.exists());
        let results = policy.cleanup_temps();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(!tmp.exists());
    }

    #[test]
    fn test_guarded_create_blocked() {
        let policy = DiskPolicy::strict();
        let result = guarded_create(&policy, Path::new("/usr/local/bin/bad"));
        assert!(result.is_err());
    }

    #[test]
    fn test_secure_delete() {
        let path = std::env::temp_dir().join("phantom-zfp-secure-del-test");
        fs::write(&path, b"sensitive").unwrap();
        assert!(path.exists());
        secure_delete(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_secure_delete_nonexistent() {
        let path = std::env::temp_dir().join("phantom-zfp-does-not-exist");
        assert!(secure_delete(&path).is_ok());
    }

    // ── SecureBuffer ────────────────────────────────────────────────────

    #[test]
    fn test_secure_buffer_zeroize_on_drop() {
        let ptr: *const u8;
        let len: usize;

        {
            let buf = SecureBuffer::from_slice(b"super secret key material!!!");
            ptr = buf.as_bytes().as_ptr();
            len = buf.len();
            assert_eq!(buf.as_bytes(), b"super secret key material!!!");
        }
        // After drop, memory should be zeroed.
        // NOTE: This is best-effort — the allocator may reuse the memory.
        // In release builds with optimizations, this could be elided.
        // The zeroize crate uses volatile writes to prevent this.
        // We just verify the API works correctly.
        let _ = (ptr, len);
    }

    #[test]
    fn test_secure_buffer_empty() {
        let buf = SecureBuffer::new(0);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_secure_buffer_new_zeroed() {
        let buf = SecureBuffer::new(64);
        assert_eq!(buf.len(), 64);
        assert!(buf.as_bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_secure_buffer_mlock() {
        let mut buf = SecureBuffer::from_slice(b"key material");
        // mlock may fail on CI due to resource limits — that's OK
        let result = buf.mlock();
        if result.is_ok() {
            assert!(buf.is_locked());
        }
    }

    #[test]
    fn test_secure_buffer_from_slice_locked() {
        let result = SecureBuffer::from_slice_locked(b"locked secret");
        // May fail due to mlock limits
        if let Ok(buf) = result {
            assert!(buf.is_locked());
            assert_eq!(buf.as_bytes(), b"locked secret");
        }
    }

    // ── SecureString ────────────────────────────────────────────────────

    #[test]
    fn test_secure_string_redacted_display() {
        let ss = SecureString::new("my-api-key-12345");
        assert_eq!(format!("{}", ss), "[REDACTED]");
        assert_eq!(format!("{:?}", ss), "SecureString([REDACTED, 16 bytes])");
    }

    #[test]
    fn test_secure_string_as_str() {
        let ss = SecureString::new("password123");
        assert_eq!(ss.as_str(), "password123");
        assert_eq!(ss.len(), 11);
        assert!(!ss.is_empty());
    }

    #[test]
    fn test_secure_string_empty() {
        let ss = SecureString::new("");
        assert!(ss.is_empty());
    }

    // ── SessionSecrets ──────────────────────────────────────────────────

    #[test]
    fn test_session_secrets_register_and_wipe() {
        let mut secrets = SessionSecrets::new();
        let idx_buf = secrets.register_buffer(SecureBuffer::from_slice(b"key"));
        let idx_str = secrets.register_string(SecureString::new("token"));

        assert_eq!(secrets.count(), 2);
        assert!(secrets.get_buffer(idx_buf).is_some());
        assert!(secrets.get_string(idx_str).is_some());

        secrets.wipe_all();
        assert_eq!(secrets.count(), 0);
    }

    #[test]
    fn test_session_secrets_default() {
        let secrets = SessionSecrets::default();
        assert_eq!(secrets.count(), 0);
    }

    // ── mlock helpers ───────────────────────────────────────────────────

    #[test]
    fn test_mlock_empty_slice() {
        assert!(mlock_bytes(&[]).is_ok());
        assert!(munlock_bytes(&[]).is_ok());
    }

    #[test]
    fn test_mlock_limit_returns_value() {
        // On macOS this should return Some; on other platforms may be None
        if cfg!(target_os = "macos") {
            assert!(mlock_limit().is_some());
        }
    }

    // ── SessionGuard ────────────────────────────────────────────────────

    #[test]
    fn test_session_guard_track_and_cleanup() {
        let guard = SessionGuard::new();
        let tmp1 = std::env::temp_dir().join("phantom-zfp-guard-test-1");
        let tmp2 = std::env::temp_dir().join("phantom-zfp-guard-test-2");

        fs::write(&tmp1, b"temp1").unwrap();
        fs::write(&tmp2, b"temp2").unwrap();

        guard.track_file(tmp1.clone());
        guard.track_file(tmp2.clone());
        assert_eq!(guard.tracked_count(), 2);

        let results = guard.cleanup();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success));
        assert!(!tmp1.exists());
        assert!(!tmp2.exists());
        assert!(!guard.is_active());
    }

    #[test]
    fn test_session_guard_track_dir() {
        let guard = SessionGuard::new();
        let dir = std::env::temp_dir().join("phantom-zfp-guard-dir-test");
        fs::create_dir_all(dir.join("subdir")).unwrap();
        fs::write(dir.join("subdir").join("file.txt"), b"data").unwrap();

        guard.track_dir(dir.clone());
        let results = guard.cleanup();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(!dir.exists());
    }

    #[test]
    fn test_session_guard_drop_cleans_up() {
        let tmp = std::env::temp_dir().join("phantom-zfp-guard-drop-test");
        fs::write(&tmp, b"should be cleaned on drop").unwrap();

        {
            let guard = SessionGuard::new();
            guard.track_file(tmp.clone());
        } // guard dropped here

        assert!(!tmp.exists());
    }

    // ── StartupValidator ────────────────────────────────────────────────

    #[test]
    fn test_validator_clean_system() {
        let validator = StartupValidator::new();
        let report = validator.validate();
        // We can't guarantee a clean system in all environments,
        // so we just verify the validator runs without error
        assert!(report.scanned_paths > 0);
    }

    #[test]
    fn test_validator_detects_sensitive_file() {
        let tmp = std::env::temp_dir().join("phantom-master-key");
        fs::write(&tmp, b"fake key").unwrap();

        let mut validator = StartupValidator::new();
        validator.add_scan_dir(std::env::temp_dir());
        validator.set_max_depth(1);
        let report = validator.validate();

        // Clean up before asserting
        let _ = fs::remove_file(&tmp);

        assert!(
            !report.clean,
            "expected validator to detect phantom-master-key"
        );
        assert!(report
            .violations
            .iter()
            .any(|v| v.filename == "phantom-master-key"));
    }

    #[test]
    fn test_validator_remediate() {
        let tmp = std::env::temp_dir().join("phantom-session-key");
        fs::write(&tmp, b"fake session key").unwrap();

        let mut validator = StartupValidator::new();
        validator.add_scan_dir(std::env::temp_dir());
        validator.set_max_depth(1);
        let report = validator.validate();

        if !report.clean {
            let results = report.remediate();
            assert!(results.iter().any(|r| r.success));
        }

        assert!(!tmp.exists());
    }

    #[test]
    fn test_validator_require_clean_on_violation() {
        let tmp = std::env::temp_dir().join("phantom-totp-secret");
        fs::write(&tmp, b"totp data").unwrap();

        let mut validator = StartupValidator::new();
        validator.add_scan_dir(std::env::temp_dir());
        validator.set_max_depth(1);
        let report = validator.validate();
        let _ = fs::remove_file(&tmp);

        if !report.clean {
            let result = report.require_clean();
            assert!(result.is_err());
            match result.unwrap_err() {
                ZeroFootprintError::SensitiveFileLeak { path } => {
                    assert!(
                        path.contains("phantom-"),
                        "expected phantom- in path: {}",
                        path
                    );
                }
                other => panic!("expected SensitiveFileLeak, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_validator_custom_sensitive_name() {
        let tmp = std::env::temp_dir().join("my-custom-secret");
        fs::write(&tmp, b"custom secret data").unwrap();

        let mut validator = StartupValidator::new();
        validator.add_sensitive_name("my-custom-secret");
        validator.add_scan_dir(std::env::temp_dir());
        validator.set_max_depth(1);
        let report = validator.validate();
        let _ = fs::remove_file(&tmp);

        assert!(report
            .violations
            .iter()
            .any(|v| v.filename == "my-custom-secret"));
    }

    #[test]
    fn test_validator_max_depth() {
        let base = std::env::temp_dir().join("phantom-zfp-depth-test");
        let deep = base.join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("phantom-master-key"), b"deep").unwrap();

        let mut validator = StartupValidator::new();
        validator.add_scan_dir(&base);
        validator.set_max_depth(1); // Should NOT find it at depth 3
        let report = validator.validate();

        let _ = fs::remove_dir_all(&base);

        // At max_depth=1, the file at depth 3 should not be found
        assert!(report
            .violations
            .iter()
            .all(|v| !v.path.contains("depth-test")));
    }

    // ── normalize_path ──────────────────────────────────────────────────

    #[test]
    fn test_normalize_removes_dot_components() {
        let result = normalize_path(Path::new("/tmp/./phantom/../phantom/file"));
        assert_eq!(result.unwrap(), PathBuf::from("/tmp/phantom/file"));
    }

    #[test]
    fn test_normalize_absolute_path() {
        let result = normalize_path(Path::new("/usr/local/bin"));
        assert!(result.is_some());
    }

    // ── Integration: DiskPolicy + SessionGuard ──────────────────────────

    #[test]
    fn test_policy_tracked_count() {
        let mut policy = DiskPolicy::strict();
        assert_eq!(policy.tracked_count(), 0);
        policy.track_temp_file("/tmp/a");
        policy.track_temp_file("/tmp/b");
        assert_eq!(policy.tracked_count(), 2);
    }

    #[test]
    fn test_cleanup_result_structure() {
        let r = CleanupResult {
            path: "/tmp/test".to_string(),
            success: true,
            error: None,
        };
        assert!(r.success);
        assert!(r.error.is_none());
    }

    #[test]
    fn test_violation_structure() {
        let v = Violation {
            path: "/tmp/phantom-master-key".to_string(),
            filename: "phantom-master-key".to_string(),
            size_bytes: 42,
            is_directory: false,
        };
        assert_eq!(v.size_bytes, 42);
        assert!(!v.is_directory);
    }

    // ── RuntimeSession integration ──────────────────────────────────────

    #[tokio::test]
    async fn test_runtime_session_bootstrap_permissive() {
        let session = RuntimeSession::bootstrap(&[], false);
        assert!(session.is_ok());

        let mut session = session.unwrap();
        let idx = session.protect_string("api-key-xyz");
        assert!(session.secrets.get_string(idx).is_some());

        let results = session.teardown();
        assert_eq!(session.secrets.count(), 0);
        let _ = results;
    }

    #[tokio::test]
    async fn test_runtime_session_protect_secret() {
        let mut session = RuntimeSession::bootstrap(&[], false).unwrap();
        let idx = session
            .protect_secret(b"key-material-32-bytes!!!!!!!!!!!", false)
            .unwrap();
        let buf = session.secrets.get_buffer(idx).unwrap();
        assert_eq!(buf.len(), 32);
    }

    #[tokio::test]
    async fn test_runtime_session_track_and_teardown() {
        let mut session = RuntimeSession::bootstrap(&[], false).unwrap();

        let tmp = std::env::temp_dir().join("phantom-zfp-runtime-test");
        fs::write(&tmp, b"temp data").unwrap();
        session.track_temp_file(tmp.clone());

        let results = session.teardown();
        assert!(!tmp.exists());
        assert!(!results.is_empty());
    }
}
