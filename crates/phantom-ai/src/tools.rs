//! Tool definitions and execution for Phantom AI agents.
//!
//! Implements the Anthropic Messages API tool_use protocol: agents declare tools,
//! the model requests tool executions via `tool_use` content blocks, and we
//! execute them and feed results back as `tool_result` blocks.
//!
//! Tools: file_write, file_read, shell_exec, http_request, grep_search, list_files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::debug;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// A tool definition in the Anthropic API format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// A tool invocation requested by the model (parsed from a `tool_use` content block).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The `id` field from the `tool_use` block — must be echoed back in the result.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// JSON object of input parameters.
    pub input: Value,
}

/// The result of executing a tool, ready to be sent back as a `tool_result` content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Must match the `id` from the originating `ToolCall`.
    pub tool_use_id: String,
    /// The textual output of the tool.
    pub content: String,
    /// Whether the tool execution failed.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

impl ToolResult {
    fn success(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_use_id: id.into(),
            content: content.into(),
            is_error: false,
        }
    }

    fn error(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_use_id: id.into(),
            content: content.into(),
            is_error: true,
        }
    }

    /// Serialize this result into the `tool_result` content block format the
    /// Anthropic API expects inside a user message.
    pub fn to_content_block(&self) -> Value {
        let mut block = json!({
            "type": "tool_result",
            "tool_use_id": self.tool_use_id,
            "content": self.content,
        });
        if self.is_error {
            block["is_error"] = json!(true);
        }
        block
    }
}

// ---------------------------------------------------------------------------
// Tool registry
// ---------------------------------------------------------------------------

/// Registry of all tools available to Phantom agents.
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: Vec<ToolDefinition>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create a registry pre-loaded with all built-in Phantom tools.
    pub fn new() -> Self {
        Self {
            tools: vec![
                Self::file_write_def(),
                Self::file_read_def(),
                Self::shell_exec_def(),
                Self::http_request_def(),
                Self::grep_search_def(),
                Self::list_files_def(),
            ],
        }
    }

    /// Return tool definitions as `Vec<Value>` suitable for injection into an
    /// Anthropic Messages API request body under the `tools` key.
    pub fn to_api_tools(&self) -> Vec<Value> {
        self.tools
            .iter()
            .map(|t| serde_json::to_value(t).expect("tool serialization cannot fail"))
            .collect()
    }

    /// Look up a tool definition by name.
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// All tool names.
    pub fn names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name.as_str()).collect()
    }

    // -- individual definitions -------------------------------------------

    fn file_write_def() -> ToolDefinition {
        ToolDefinition {
            name: "file_write".into(),
            description: "Write content to a file. Creates the file if it does not exist. \
                          Optionally creates parent directories."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the file to write."
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file."
                    },
                    "create_dirs": {
                        "type": "boolean",
                        "description": "If true, create parent directories as needed. Default false."
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn file_read_def() -> ToolDefinition {
        ToolDefinition {
            name: "file_read".into(),
            description: "Read the contents of a file. Optionally read a slice via offset/limit \
                          (line-based)."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the file to read."
                    },
                    "offset": {
                        "type": "integer",
                        "description": "0-based line offset to start reading from."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to return."
                    }
                },
                "required": ["path"]
            }),
        }
    }

    fn shell_exec_def() -> ToolDefinition {
        ToolDefinition {
            name: "shell_exec".into(),
            description: "Execute a shell command and return stdout/stderr. The command runs \
                          inside /bin/sh with a configurable timeout (default 30s)."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    },
                    "working_dir": {
                        "type": "string",
                        "description": "Working directory for the command. Defaults to cwd."
                    },
                    "timeout_secs": {
                        "type": "integer",
                        "description": "Timeout in seconds. Default 30."
                    }
                },
                "required": ["command"]
            }),
        }
    }

    fn http_request_def() -> ToolDefinition {
        ToolDefinition {
            name: "http_request".into(),
            description: "Make an HTTP request and return the response status and body.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "description": "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD)."
                    },
                    "url": {
                        "type": "string",
                        "description": "The URL to request."
                    },
                    "headers": {
                        "type": "object",
                        "description": "Optional HTTP headers as key-value pairs.",
                        "additionalProperties": { "type": "string" }
                    },
                    "body": {
                        "type": "string",
                        "description": "Optional request body."
                    }
                },
                "required": ["method", "url"]
            }),
        }
    }

    fn grep_search_def() -> ToolDefinition {
        ToolDefinition {
            name: "grep_search".into(),
            description: "Search file contents for a pattern (substring match). Returns matching \
                          lines with file paths and line numbers."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The search pattern (substring match)."
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory or file to search in. Defaults to cwd."
                    },
                    "file_glob": {
                        "type": "string",
                        "description": "Optional glob pattern to filter files, e.g. '*.rs'."
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    fn list_files_def() -> ToolDefinition {
        ToolDefinition {
            name: "list_files".into(),
            description: "List files in a directory, optionally filtered by a glob-like pattern. \
                          Can recurse into subdirectories."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Absolute path to the directory to list."
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Optional filename pattern to filter (e.g. '*.rs'). \
                                        Simple suffix matching."
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Recurse into subdirectories. Default false."
                    }
                },
                "required": ["path"]
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing tool_use blocks from Anthropic responses
// ---------------------------------------------------------------------------

/// Parse `ToolCall`s from the content blocks of an Anthropic completion response.
///
/// The API returns content blocks like:
/// ```json
/// { "type": "tool_use", "id": "toolu_xxx", "name": "file_read", "input": {...} }
/// ```
pub fn parse_tool_calls(content_blocks: &[Value]) -> Vec<ToolCall> {
    content_blocks
        .iter()
        .filter_map(|block| {
            if block.get("type")?.as_str()? != "tool_use" {
                return None;
            }
            Some(ToolCall {
                id: block.get("id")?.as_str()?.to_string(),
                name: block.get("name")?.as_str()?.to_string(),
                input: block.get("input")?.clone(),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tool execution
// ---------------------------------------------------------------------------

/// Execute a single tool call and return its result.
///
/// This is the main dispatch: it inspects `call.name`, extracts parameters from
/// `call.input`, and calls the appropriate async implementation.
pub async fn execute_tool(call: &ToolCall) -> ToolResult {
    debug!(tool = %call.name, id = %call.id, "executing tool");

    match call.name.as_str() {
        "file_write" => exec_file_write(call).await,
        "file_read" => exec_file_read(call).await,
        "shell_exec" => exec_shell_exec(call).await,
        "http_request" => exec_http_request(call).await,
        "grep_search" => exec_grep_search(call).await,
        "list_files" => exec_list_files(call).await,
        other => ToolResult::error(&call.id, format!("Unknown tool: {other}")),
    }
}

/// Execute all tool calls in sequence, returning results in the same order.
pub async fn execute_tool_calls(calls: &[ToolCall]) -> Vec<ToolResult> {
    let mut results = Vec::with_capacity(calls.len());
    for call in calls {
        results.push(execute_tool(call).await);
    }
    results
}

// -- file_write -----------------------------------------------------------

async fn exec_file_write(call: &ToolCall) -> ToolResult {
    let path = match call.input.get("path").and_then(|v| v.as_str()) {
        Some(p) => PathBuf::from(p),
        None => return ToolResult::error(&call.id, "Missing required parameter: path"),
    };
    let content = match call.input.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return ToolResult::error(&call.id, "Missing required parameter: content"),
    };
    let create_dirs = call
        .input
        .get("create_dirs")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if create_dirs {
        if let Some(parent) = path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                return ToolResult::error(&call.id, format!("Failed to create directories: {e}"));
            }
        }
    }

    match tokio::fs::write(&path, content).await {
        Ok(()) => ToolResult::success(
            &call.id,
            format!("Wrote {} bytes to {}", content.len(), path.display()),
        ),
        Err(e) => ToolResult::error(&call.id, format!("Failed to write file: {e}")),
    }
}

// -- file_read ------------------------------------------------------------

async fn exec_file_read(call: &ToolCall) -> ToolResult {
    let path = match call.input.get("path").and_then(|v| v.as_str()) {
        Some(p) => PathBuf::from(p),
        None => return ToolResult::error(&call.id, "Missing required parameter: path"),
    };
    let offset = call
        .input
        .get("offset")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);
    let limit = call
        .input
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize);

    let content = match tokio::fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(e) => return ToolResult::error(&call.id, format!("Failed to read file: {e}")),
    };

    let lines: Vec<&str> = content.lines().collect();
    let start = offset.unwrap_or(0).min(lines.len());
    let end = match limit {
        Some(lim) => (start + lim).min(lines.len()),
        None => lines.len(),
    };

    let slice: String = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:>6}\t{}", start + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    ToolResult::success(&call.id, slice)
}

// -- shell_exec -----------------------------------------------------------

async fn exec_shell_exec(call: &ToolCall) -> ToolResult {
    let command = match call.input.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return ToolResult::error(&call.id, "Missing required parameter: command"),
    };
    let working_dir = call.input.get("working_dir").and_then(|v| v.as_str());
    let timeout_secs = call
        .input
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(30);

    // Cap timeout at 5 minutes for safety
    let timeout_secs = timeout_secs.min(300);

    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg(command);

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return ToolResult::error(&call.id, format!("Failed to spawn command: {e}")),
    };

    let timeout = Duration::from_secs(timeout_secs);
    match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);

            let mut result = format!("exit code: {exit_code}\n");
            if !stdout.is_empty() {
                result.push_str(&format!("--- stdout ---\n{stdout}"));
            }
            if !stderr.is_empty() {
                result.push_str(&format!("--- stderr ---\n{stderr}"));
            }

            if output.status.success() {
                ToolResult::success(&call.id, result)
            } else {
                ToolResult::error(&call.id, result)
            }
        }
        Ok(Err(e)) => ToolResult::error(&call.id, format!("Command failed: {e}")),
        Err(_) => ToolResult::error(&call.id, format!("Command timed out after {timeout_secs}s")),
    }
}

// -- http_request ---------------------------------------------------------

async fn exec_http_request(call: &ToolCall) -> ToolResult {
    let method_str = match call.input.get("method").and_then(|v| v.as_str()) {
        Some(m) => m.to_uppercase(),
        None => return ToolResult::error(&call.id, "Missing required parameter: method"),
    };
    let url = match call.input.get("url").and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return ToolResult::error(&call.id, "Missing required parameter: url"),
    };
    let headers: HashMap<String, String> = call
        .input
        .get("headers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let body = call.input.get("body").and_then(|v| v.as_str());

    let method = match method_str.as_str() {
        "GET" => reqwest::Method::GET,
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        "PATCH" => reqwest::Method::PATCH,
        "HEAD" => reqwest::Method::HEAD,
        other => return ToolResult::error(&call.id, format!("Unsupported HTTP method: {other}")),
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => return ToolResult::error(&call.id, format!("Failed to create HTTP client: {e}")),
    };

    let mut request = client.request(method, url);
    for (key, value) in &headers {
        request = request.header(key.as_str(), value.as_str());
    }
    if let Some(b) = body {
        request = request.body(b.to_string());
    }

    match request.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let resp_headers: Vec<String> = resp
                .headers()
                .iter()
                .take(20) // limit header count
                .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or("<binary>")))
                .collect();
            let body_text = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));

            // Truncate very large response bodies
            let body_text = if body_text.len() > 50_000 {
                format!(
                    "{}... [truncated, {} bytes total]",
                    &body_text[..50_000],
                    body_text.len()
                )
            } else {
                body_text
            };

            let output = format!(
                "HTTP {status}\n--- headers ---\n{}\n--- body ---\n{body_text}",
                resp_headers.join("\n")
            );
            ToolResult::success(&call.id, output)
        }
        Err(e) => ToolResult::error(&call.id, format!("HTTP request failed: {e}")),
    }
}

// -- grep_search ----------------------------------------------------------

async fn exec_grep_search(call: &ToolCall) -> ToolResult {
    let pattern = match call.input.get("pattern").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => return ToolResult::error(&call.id, "Missing required parameter: pattern"),
    };
    let search_path = call
        .input
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let file_glob = call.input.get("file_glob").and_then(|v| v.as_str());

    let root = PathBuf::from(search_path);
    if !root.exists() {
        return ToolResult::error(&call.id, format!("Path does not exist: {search_path}"));
    }

    let mut matches = Vec::new();
    let max_matches = 200;

    if root.is_file() {
        search_file(&root, &pattern, &mut matches, max_matches);
    } else {
        // Walk directory recursively using a stack (no walkdir dependency)
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            if matches.len() >= max_matches {
                break;
            }
            let mut entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            while let Some(Ok(entry)) = entries.next() {
                if matches.len() >= max_matches {
                    break;
                }
                let path = entry.path();
                if path.is_dir() {
                    // Skip hidden directories and common noise
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with('.')
                        || name_str == "node_modules"
                        || name_str == "target"
                    {
                        continue;
                    }
                    stack.push(path);
                } else if path.is_file() {
                    // Apply file_glob filter if provided
                    if let Some(glob) = file_glob {
                        if !matches_simple_glob(&path, glob) {
                            continue;
                        }
                    }
                    search_file(&path, &pattern, &mut matches, max_matches);
                }
            }
        }
    }

    if matches.is_empty() {
        ToolResult::success(&call.id, "No matches found.")
    } else {
        let truncated = if matches.len() >= max_matches {
            format!("\n... (truncated at {max_matches} matches)")
        } else {
            String::new()
        };
        ToolResult::success(&call.id, format!("{}{truncated}", matches.join("\n")))
    }
}

/// Search a single file for lines containing `pattern`.
fn search_file(path: &Path, pattern: &str, matches: &mut Vec<String>, max: usize) {
    // Skip binary files by checking extension
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let binary_exts = [
        "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "woff", "woff2", "ttf", "eot", "zip",
        "tar", "gz", "bz2", "xz", "7z", "rar", "pdf", "doc", "docx", "xls", "xlsx", "exe", "dll",
        "so", "dylib", "o", "a", "class", "jar", "wasm",
    ];
    if binary_exts.contains(&ext.to_lowercase().as_str()) {
        return;
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return, // skip unreadable files
    };

    for (i, line) in content.lines().enumerate() {
        if matches.len() >= max {
            break;
        }
        if line.contains(pattern) {
            matches.push(format!("{}:{}:{}", path.display(), i + 1, line));
        }
    }
}

/// Very simple glob matching: supports `*.ext` (suffix match) and exact name.
fn matches_simple_glob(path: &Path, glob: &str) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };
    if let Some(suffix) = glob.strip_prefix('*') {
        file_name.ends_with(suffix)
    } else {
        file_name == glob
    }
}

// -- list_files -----------------------------------------------------------

async fn exec_list_files(call: &ToolCall) -> ToolResult {
    let path = match call.input.get("path").and_then(|v| v.as_str()) {
        Some(p) => PathBuf::from(p),
        None => return ToolResult::error(&call.id, "Missing required parameter: path"),
    };
    let pattern = call.input.get("pattern").and_then(|v| v.as_str());
    let recursive = call
        .input
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !path.exists() {
        return ToolResult::error(&call.id, format!("Path does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return ToolResult::error(&call.id, format!("Not a directory: {}", path.display()));
    }

    let mut files = Vec::new();
    let max_files = 1000;

    if recursive {
        let mut stack = vec![path.clone()];
        while let Some(dir) = stack.pop() {
            if files.len() >= max_files {
                break;
            }
            let mut entries = match tokio::fs::read_dir(&dir).await {
                Ok(e) => e,
                Err(_) => continue,
            };
            while let Ok(Some(entry)) = entries.next_entry().await {
                if files.len() >= max_files {
                    break;
                }
                let entry_path = entry.path();
                let is_dir = entry_path.is_dir();

                if is_dir {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if !name_str.starts_with('.') {
                        stack.push(entry_path.clone());
                    }
                }

                if let Some(glob) = pattern {
                    if !matches_simple_glob(&entry_path, glob) {
                        continue;
                    }
                }

                let suffix = if is_dir { "/" } else { "" };
                files.push(format!("{}{suffix}", entry_path.display()));
            }
        }
    } else {
        let mut entries = match tokio::fs::read_dir(&path).await {
            Ok(e) => e,
            Err(e) => return ToolResult::error(&call.id, format!("Failed to read directory: {e}")),
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            if files.len() >= max_files {
                break;
            }
            let entry_path = entry.path();

            if let Some(glob) = pattern {
                if !matches_simple_glob(&entry_path, glob) {
                    continue;
                }
            }

            let suffix = if entry_path.is_dir() { "/" } else { "" };
            files.push(format!("{}{suffix}", entry_path.display()));
        }
    }

    files.sort();

    if files.is_empty() {
        ToolResult::success(&call.id, "No files found.")
    } else {
        let truncated = if files.len() >= max_files {
            format!("\n... (truncated at {max_files} entries)")
        } else {
            String::new()
        };
        ToolResult::success(&call.id, format!("{}{truncated}", files.join("\n")))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- ToolRegistry tests -----------------------------------------------

    #[test]
    fn test_registry_has_all_tools() {
        let registry = ToolRegistry::new();
        let names = registry.names();
        assert_eq!(names.len(), 6);
        assert!(names.contains(&"file_write"));
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"shell_exec"));
        assert!(names.contains(&"http_request"));
        assert!(names.contains(&"grep_search"));
        assert!(names.contains(&"list_files"));
    }

    #[test]
    fn test_registry_to_api_tools() {
        let registry = ToolRegistry::new();
        let tools = registry.to_api_tools();
        assert_eq!(tools.len(), 6);

        // Each tool must have name, description, and input_schema
        for tool in &tools {
            assert!(tool.get("name").is_some());
            assert!(tool.get("description").is_some());
            let schema = tool.get("input_schema").unwrap();
            assert_eq!(schema["type"], "object");
            assert!(schema.get("properties").is_some());
            assert!(schema.get("required").is_some());
        }
    }

    #[test]
    fn test_tool_definition_serialization() {
        let registry = ToolRegistry::new();
        let tool = registry.get("file_write").unwrap();
        let value = serde_json::to_value(tool).unwrap();
        assert_eq!(value["name"], "file_write");
        assert_eq!(value["input_schema"]["type"], "object");
        let required = value["input_schema"]["required"].as_array().unwrap();
        assert!(required.contains(&json!("path")));
        assert!(required.contains(&json!("content")));
    }

    #[test]
    fn test_registry_get_unknown_tool() {
        let registry = ToolRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    // -- ToolCall / ToolResult tests --------------------------------------

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("id123", "all good");
        assert_eq!(result.tool_use_id, "id123");
        assert_eq!(result.content, "all good");
        assert!(!result.is_error);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("id456", "bad input");
        assert!(result.is_error);
    }

    #[test]
    fn test_tool_result_to_content_block() {
        let result = ToolResult::success("id789", "output");
        let block = result.to_content_block();
        assert_eq!(block["type"], "tool_result");
        assert_eq!(block["tool_use_id"], "id789");
        assert_eq!(block["content"], "output");
        assert!(block.get("is_error").is_none());

        let err_result = ToolResult::error("idabc", "failed");
        let block = err_result.to_content_block();
        assert_eq!(block["is_error"], true);
    }

    // -- parse_tool_calls tests -------------------------------------------

    #[test]
    fn test_parse_tool_calls_from_content_blocks() {
        let blocks = vec![
            json!({
                "type": "text",
                "text": "Let me read that file."
            }),
            json!({
                "type": "tool_use",
                "id": "toolu_abc123",
                "name": "file_read",
                "input": { "path": "/tmp/test.txt" }
            }),
            json!({
                "type": "tool_use",
                "id": "toolu_def456",
                "name": "shell_exec",
                "input": { "command": "ls -la" }
            }),
        ];

        let calls = parse_tool_calls(&blocks);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "file_read");
        assert_eq!(calls[0].id, "toolu_abc123");
        assert_eq!(calls[0].input["path"], "/tmp/test.txt");
        assert_eq!(calls[1].name, "shell_exec");
    }

    #[test]
    fn test_parse_tool_calls_empty() {
        let blocks = vec![json!({"type": "text", "text": "hello"})];
        assert!(parse_tool_calls(&blocks).is_empty());
    }

    // -- matches_simple_glob tests ----------------------------------------

    #[test]
    fn test_simple_glob_suffix() {
        assert!(matches_simple_glob(Path::new("/foo/bar.rs"), "*.rs"));
        assert!(!matches_simple_glob(Path::new("/foo/bar.ts"), "*.rs"));
    }

    #[test]
    fn test_simple_glob_exact() {
        assert!(matches_simple_glob(
            Path::new("/foo/Cargo.toml"),
            "Cargo.toml"
        ));
        assert!(!matches_simple_glob(
            Path::new("/foo/Cargo.lock"),
            "Cargo.toml"
        ));
    }

    // -- Tool execution tests (async) -------------------------------------

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let call = ToolCall {
            id: "test1".into(),
            name: "nonexistent".into(),
            input: json!({}),
        };
        let result = execute_tool(&call).await;
        assert!(result.is_error);
        assert!(result.content.contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_file_write_and_read() {
        let tmp = std::env::temp_dir().join("phantom_test_tools_rw.txt");
        let path_str = tmp.to_string_lossy().to_string();

        // Write
        let write_call = ToolCall {
            id: "w1".into(),
            name: "file_write".into(),
            input: json!({
                "path": path_str,
                "content": "line one\nline two\nline three\n"
            }),
        };
        let result = execute_tool(&write_call).await;
        assert!(!result.is_error, "write failed: {}", result.content);

        // Read full
        let read_call = ToolCall {
            id: "r1".into(),
            name: "file_read".into(),
            input: json!({ "path": path_str }),
        };
        let result = execute_tool(&read_call).await;
        assert!(!result.is_error);
        assert!(result.content.contains("line one"));
        assert!(result.content.contains("line three"));

        // Read with offset/limit
        let read_call2 = ToolCall {
            id: "r2".into(),
            name: "file_read".into(),
            input: json!({ "path": path_str, "offset": 1, "limit": 1 }),
        };
        let result = execute_tool(&read_call2).await;
        assert!(!result.is_error);
        assert!(result.content.contains("line two"));
        assert!(!result.content.contains("line one"));

        // Cleanup
        let _ = tokio::fs::remove_file(&tmp).await;
    }

    #[tokio::test]
    async fn test_file_write_create_dirs() {
        let tmp = std::env::temp_dir()
            .join("phantom_test_tools_dirs")
            .join("sub")
            .join("deep")
            .join("file.txt");
        let path_str = tmp.to_string_lossy().to_string();

        let call = ToolCall {
            id: "wd1".into(),
            name: "file_write".into(),
            input: json!({
                "path": path_str,
                "content": "nested content",
                "create_dirs": true
            }),
        };
        let result = execute_tool(&call).await;
        assert!(
            !result.is_error,
            "write with dirs failed: {}",
            result.content
        );
        assert!(tmp.exists());

        // Cleanup
        let _ =
            tokio::fs::remove_dir_all(std::env::temp_dir().join("phantom_test_tools_dirs")).await;
    }

    #[tokio::test]
    async fn test_file_read_missing_file() {
        let call = ToolCall {
            id: "rm1".into(),
            name: "file_read".into(),
            input: json!({ "path": "/tmp/phantom_nonexistent_file_xyz.txt" }),
        };
        let result = execute_tool(&call).await;
        assert!(result.is_error);
        assert!(result.content.contains("Failed to read file"));
    }

    #[tokio::test]
    async fn test_file_write_missing_param() {
        let call = ToolCall {
            id: "mp1".into(),
            name: "file_write".into(),
            input: json!({ "path": "/tmp/x.txt" }),
        };
        let result = execute_tool(&call).await;
        assert!(result.is_error);
        assert!(result.content.contains("Missing required parameter"));
    }

    #[tokio::test]
    async fn test_shell_exec_basic() {
        let call = ToolCall {
            id: "s1".into(),
            name: "shell_exec".into(),
            input: json!({ "command": "echo hello" }),
        };
        let result = execute_tool(&call).await;
        assert!(!result.is_error, "shell failed: {}", result.content);
        assert!(result.content.contains("hello"));
        assert!(result.content.contains("exit code: 0"));
    }

    #[tokio::test]
    async fn test_shell_exec_failure() {
        let call = ToolCall {
            id: "s2".into(),
            name: "shell_exec".into(),
            input: json!({ "command": "exit 42" }),
        };
        let result = execute_tool(&call).await;
        assert!(result.is_error);
        assert!(result.content.contains("exit code: 42"));
    }

    #[tokio::test]
    async fn test_shell_exec_timeout() {
        let call = ToolCall {
            id: "s3".into(),
            name: "shell_exec".into(),
            input: json!({
                "command": "sleep 60",
                "timeout_secs": 1
            }),
        };
        let result = execute_tool(&call).await;
        assert!(result.is_error);
        assert!(result.content.contains("timed out"));
    }

    #[tokio::test]
    async fn test_shell_exec_working_dir() {
        let call = ToolCall {
            id: "s4".into(),
            name: "shell_exec".into(),
            input: json!({
                "command": "pwd",
                "working_dir": "/tmp"
            }),
        };
        let result = execute_tool(&call).await;
        assert!(!result.is_error);
        // macOS may resolve /tmp -> /private/tmp
        assert!(
            result.content.contains("/tmp"),
            "expected /tmp in output: {}",
            result.content
        );
    }

    #[tokio::test]
    async fn test_grep_search_basic() {
        // Write a test file
        let dir = std::env::temp_dir().join("phantom_test_grep");
        let _ = tokio::fs::create_dir_all(&dir).await;
        tokio::fs::write(dir.join("test.txt"), "foo bar\nbaz quux\nfoo again\n")
            .await
            .unwrap();

        let call = ToolCall {
            id: "g1".into(),
            name: "grep_search".into(),
            input: json!({
                "pattern": "foo",
                "path": dir.to_string_lossy()
            }),
        };
        let result = execute_tool(&call).await;
        assert!(!result.is_error, "grep failed: {}", result.content);
        assert!(result.content.contains("foo bar"));
        assert!(result.content.contains("foo again"));
        assert!(!result.content.contains("baz quux"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn test_grep_search_with_glob() {
        let dir = std::env::temp_dir().join("phantom_test_grep_glob");
        let _ = tokio::fs::create_dir_all(&dir).await;
        tokio::fs::write(dir.join("a.rs"), "fn main() {}\n")
            .await
            .unwrap();
        tokio::fs::write(dir.join("b.txt"), "fn not_this() {}\n")
            .await
            .unwrap();

        let call = ToolCall {
            id: "g2".into(),
            name: "grep_search".into(),
            input: json!({
                "pattern": "fn",
                "path": dir.to_string_lossy(),
                "file_glob": "*.rs"
            }),
        };
        let result = execute_tool(&call).await;
        assert!(!result.is_error);
        assert!(result.content.contains("a.rs"));
        assert!(!result.content.contains("b.txt"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn test_list_files_basic() {
        let dir = std::env::temp_dir().join("phantom_test_list");
        let _ = tokio::fs::create_dir_all(&dir).await;
        tokio::fs::write(dir.join("alpha.txt"), "").await.unwrap();
        tokio::fs::write(dir.join("beta.rs"), "").await.unwrap();

        let call = ToolCall {
            id: "l1".into(),
            name: "list_files".into(),
            input: json!({ "path": dir.to_string_lossy() }),
        };
        let result = execute_tool(&call).await;
        assert!(!result.is_error, "list failed: {}", result.content);
        assert!(result.content.contains("alpha.txt"));
        assert!(result.content.contains("beta.rs"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn test_list_files_with_pattern() {
        let dir = std::env::temp_dir().join("phantom_test_list_pat");
        let _ = tokio::fs::create_dir_all(&dir).await;
        tokio::fs::write(dir.join("a.rs"), "").await.unwrap();
        tokio::fs::write(dir.join("b.txt"), "").await.unwrap();

        let call = ToolCall {
            id: "l2".into(),
            name: "list_files".into(),
            input: json!({
                "path": dir.to_string_lossy(),
                "pattern": "*.rs"
            }),
        };
        let result = execute_tool(&call).await;
        assert!(!result.is_error);
        assert!(result.content.contains("a.rs"));
        assert!(!result.content.contains("b.txt"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn test_list_files_recursive() {
        let dir = std::env::temp_dir().join("phantom_test_list_rec");
        let sub = dir.join("sub");
        let _ = tokio::fs::create_dir_all(&sub).await;
        tokio::fs::write(dir.join("top.rs"), "").await.unwrap();
        tokio::fs::write(sub.join("nested.rs"), "").await.unwrap();

        let call = ToolCall {
            id: "l3".into(),
            name: "list_files".into(),
            input: json!({
                "path": dir.to_string_lossy(),
                "recursive": true
            }),
        };
        let result = execute_tool(&call).await;
        assert!(!result.is_error);
        assert!(result.content.contains("top.rs"));
        assert!(result.content.contains("nested.rs"));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn test_list_files_nonexistent() {
        let call = ToolCall {
            id: "l4".into(),
            name: "list_files".into(),
            input: json!({ "path": "/tmp/phantom_definitely_not_a_dir_xyz" }),
        };
        let result = execute_tool(&call).await;
        assert!(result.is_error);
        assert!(result.content.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_execute_tool_calls_multiple() {
        let calls = vec![
            ToolCall {
                id: "m1".into(),
                name: "shell_exec".into(),
                input: json!({ "command": "echo one" }),
            },
            ToolCall {
                id: "m2".into(),
                name: "shell_exec".into(),
                input: json!({ "command": "echo two" }),
            },
        ];
        let results = execute_tool_calls(&calls).await;
        assert_eq!(results.len(), 2);
        assert!(results[0].content.contains("one"));
        assert!(results[1].content.contains("two"));
    }
}
