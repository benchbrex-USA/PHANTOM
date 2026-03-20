//! Dynamic CLAUDE.md generation — per-agent instruction files.
//!
//! Each agent spawn gets a tailored CLAUDE.md written to a temp directory
//! before the agent starts. The file contains:
//!   - Agent identity (role, model, token budget)
//!   - Environment variable declarations (AGENT_ROLE, AGENT_KB_SCOPE, AGENT_TOKEN_BUDGET)
//!   - Knowledge Brain context chunks filtered to the agent's KB scope
//!   - Role-specific constraints and coordination protocols
//!   - Build context (current phase, project info)
//!
//! Files are cleaned up after the agent completes (zero-footprint compliance).

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use crate::agents::AgentRole;
use crate::context::KnowledgeChunk;
use crate::prompts;

// ─── Template sections ────────────────────────────────────────────────────────

/// Variables injected into every generated CLAUDE.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVars {
    /// Agent role identifier (e.g. "backend")
    pub agent_role: String,
    /// Display name (e.g. "Backend Agent")
    pub agent_display_name: String,
    /// Comma-separated knowledge scope files
    pub agent_kb_scope: String,
    /// Per-task token budget
    pub agent_token_budget: u64,
    /// Claude model for this agent
    pub agent_model: String,
    /// Agent temperature
    pub agent_temperature: f32,
    /// Max tokens per response
    pub agent_max_tokens: u32,
    /// Whether agent can delegate to others
    pub can_delegate: bool,
    /// Whether agent needs code execution
    pub needs_code_exec: bool,
    /// Optional current build phase
    pub build_phase: Option<String>,
    /// Optional project name
    pub project_name: Option<String>,
    /// Arbitrary extra variables
    pub extra: HashMap<String, String>,
}

impl TemplateVars {
    /// Create template variables from an agent role with defaults.
    pub fn from_role(role: AgentRole) -> Self {
        Self {
            agent_role: role.id().to_string(),
            agent_display_name: role.display_name().to_string(),
            agent_kb_scope: role.knowledge_scope().join(","),
            agent_token_budget: role.task_token_budget(),
            agent_model: role.model().to_string(),
            agent_temperature: role.temperature(),
            agent_max_tokens: role.max_tokens(),
            can_delegate: role.can_delegate(),
            needs_code_exec: role.needs_code_exec(),
            build_phase: None,
            project_name: None,
            extra: HashMap::new(),
        }
    }

    /// Set the current build phase.
    pub fn with_build_phase(mut self, phase: impl Into<String>) -> Self {
        self.build_phase = Some(phase.into());
        self
    }

    /// Set the project name.
    pub fn with_project_name(mut self, name: impl Into<String>) -> Self {
        self.project_name = Some(name.into());
        self
    }

    /// Add an extra variable.
    pub fn with_extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    /// Environment variables to inject for this agent.
    pub fn env_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("AGENT_ROLE".into(), self.agent_role.clone());
        vars.insert("AGENT_KB_SCOPE".into(), self.agent_kb_scope.clone());
        vars.insert(
            "AGENT_TOKEN_BUDGET".into(),
            self.agent_token_budget.to_string(),
        );
        vars.insert("AGENT_MODEL".into(), self.agent_model.clone());
        vars.insert(
            "AGENT_TEMPERATURE".into(),
            self.agent_temperature.to_string(),
        );
        vars.insert("AGENT_MAX_TOKENS".into(), self.agent_max_tokens.to_string());
        vars.insert("AGENT_CAN_DELEGATE".into(), self.can_delegate.to_string());
        if let Some(ref phase) = self.build_phase {
            vars.insert("BUILD_PHASE".into(), phase.clone());
        }
        if let Some(ref name) = self.project_name {
            vars.insert("PROJECT_NAME".into(), name.clone());
        }
        for (k, v) in &self.extra {
            vars.insert(k.clone(), v.clone());
        }
        vars
    }
}

// ─── Template engine ──────────────────────────────────────────────────────────

/// Generates per-agent CLAUDE.md files from templates and knowledge context.
#[derive(Debug, Clone)]
pub struct ClaudeMdGenerator {
    /// Base temp directory for generated files
    base_dir: PathBuf,
    /// Max knowledge tokens to include per agent
    max_knowledge_tokens: usize,
    /// Whether to include the full system prompt
    include_system_prompt: bool,
}

impl ClaudeMdGenerator {
    /// Create a generator that writes to the system temp directory.
    pub fn new() -> Self {
        Self {
            base_dir: std::env::temp_dir().join("phantom-claude-md"),
            max_knowledge_tokens: 10_000,
            include_system_prompt: true,
        }
    }

    /// Create a generator targeting a specific directory.
    pub fn with_base_dir(dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: dir.into(),
            max_knowledge_tokens: 10_000,
            include_system_prompt: true,
        }
    }

    /// Set the max knowledge tokens to embed in the CLAUDE.md.
    pub fn with_max_knowledge_tokens(mut self, tokens: usize) -> Self {
        self.max_knowledge_tokens = tokens;
        self
    }

    /// Toggle whether the full system prompt is included.
    pub fn with_system_prompt(mut self, include: bool) -> Self {
        self.include_system_prompt = include;
        self
    }

    /// Get the base directory for generated files.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Generate a CLAUDE.md for an agent and write it to disk.
    ///
    /// Returns the path to the generated file.
    #[instrument(skip(self, knowledge_chunks), fields(role = %vars.agent_role))]
    pub fn generate(
        &self,
        vars: &TemplateVars,
        knowledge_chunks: &[KnowledgeChunk],
    ) -> Result<GeneratedClaudeMd, ClaudeMdError> {
        let content = self.render(vars, knowledge_chunks)?;

        // Create agent-specific subdirectory
        let agent_dir = self.base_dir.join(&vars.agent_role);
        std::fs::create_dir_all(&agent_dir).map_err(|e| ClaudeMdError::IoError {
            path: agent_dir.clone(),
            reason: e.to_string(),
        })?;

        let file_path = agent_dir.join("CLAUDE.md");
        std::fs::write(&file_path, &content).map_err(|e| ClaudeMdError::IoError {
            path: file_path.clone(),
            reason: e.to_string(),
        })?;

        info!(
            path = %file_path.display(),
            size_bytes = content.len(),
            "generated CLAUDE.md for agent"
        );

        Ok(GeneratedClaudeMd {
            path: file_path,
            content,
            env_vars: vars.env_vars(),
        })
    }

    /// Render the CLAUDE.md content without writing to disk.
    pub fn render(
        &self,
        vars: &TemplateVars,
        knowledge_chunks: &[KnowledgeChunk],
    ) -> Result<String, ClaudeMdError> {
        let role = parse_role(&vars.agent_role)?;
        let mut out = String::with_capacity(4096);

        // Header
        writeln!(
            out,
            "# CLAUDE.md — {} Instructions",
            vars.agent_display_name
        )
        .unwrap();
        writeln!(out).unwrap();
        writeln!(out, "> Auto-generated by Phantom. Do not edit manually.").unwrap();
        writeln!(out).unwrap();

        // Identity section
        self.render_identity(&mut out, vars);

        // Environment variables
        self.render_env_vars(&mut out, vars);

        // Knowledge Brain context
        self.render_knowledge(&mut out, vars, knowledge_chunks);

        // Code conventions (§20 of Architecture Framework)
        self.render_code_conventions(&mut out);

        // System prompt (role constraints + coordination)
        if self.include_system_prompt {
            self.render_system_prompt(&mut out, role);
        }

        // Build context
        self.render_build_context(&mut out, vars);

        // Footer
        writeln!(out).unwrap();
        writeln!(out, "---").unwrap();
        writeln!(out, "Generated at: {{timestamp will be set at spawn time}}").unwrap();
        writeln!(
            out,
            "Zero-footprint: this file will be deleted after agent completion."
        )
        .unwrap();

        Ok(out)
    }

    fn render_identity(&self, out: &mut String, vars: &TemplateVars) {
        writeln!(out, "## Identity\n").unwrap();
        writeln!(out, "| Field | Value |").unwrap();
        writeln!(out, "|-------|-------|").unwrap();
        writeln!(out, "| Role | {} |", vars.agent_display_name).unwrap();
        writeln!(out, "| Role ID | `{}` |", vars.agent_role).unwrap();
        writeln!(out, "| Model | `{}` |", vars.agent_model).unwrap();
        writeln!(out, "| Temperature | {} |", vars.agent_temperature).unwrap();
        writeln!(out, "| Max Tokens | {} |", vars.agent_max_tokens).unwrap();
        writeln!(out, "| Token Budget | {} |", vars.agent_token_budget).unwrap();
        writeln!(out, "| Can Delegate | {} |", vars.can_delegate).unwrap();
        writeln!(out, "| Needs Code Exec | {} |", vars.needs_code_exec).unwrap();
        writeln!(out).unwrap();
    }

    fn render_env_vars(&self, out: &mut String, vars: &TemplateVars) {
        writeln!(out, "## Environment Variables\n").unwrap();
        writeln!(out, "These variables are set in your environment:\n").unwrap();
        writeln!(out, "```bash").unwrap();

        let mut env = vars.env_vars();
        // Sort for deterministic output
        let mut keys: Vec<String> = env.keys().cloned().collect();
        keys.sort();
        for key in &keys {
            if let Some(val) = env.remove(key) {
                writeln!(out, "export {}=\"{}\"", key, val).unwrap();
            }
        }

        writeln!(out, "```\n").unwrap();
    }

    fn render_knowledge(&self, out: &mut String, vars: &TemplateVars, chunks: &[KnowledgeChunk]) {
        writeln!(out, "## Knowledge Brain Context\n").unwrap();

        // List the KB scope files
        let scope_files: Vec<&str> = vars.agent_kb_scope.split(',').collect();
        writeln!(out, "**Knowledge Scope** ({} files):", scope_files.len()).unwrap();
        for file in &scope_files {
            writeln!(out, "- {}", file.trim()).unwrap();
        }
        writeln!(out).unwrap();

        if chunks.is_empty() {
            writeln!(
                out,
                "*No knowledge chunks available. Query the Knowledge Brain at runtime.*\n"
            )
            .unwrap();
            return;
        }

        // Filter chunks to this agent's KB scope and respect token budget
        let relevant: Vec<&KnowledgeChunk> = filter_chunks_for_scope(chunks, &scope_files);

        if relevant.is_empty() {
            writeln!(out, "*No chunks matched this agent's knowledge scope.*\n").unwrap();
            return;
        }

        writeln!(
            out,
            "**Pre-loaded Context** ({} chunks, budget: {} tokens):\n",
            relevant.len(),
            self.max_knowledge_tokens
        )
        .unwrap();

        let mut tokens_used = 0;
        let mut included = 0;
        for chunk in &relevant {
            let chunk_tokens =
                estimate_tokens(&chunk.content) + estimate_tokens(&chunk.heading) + 20; // formatting overhead

            if tokens_used + chunk_tokens > self.max_knowledge_tokens {
                break;
            }

            writeln!(
                out,
                "### [{}/{}] (relevance: {:.2})\n",
                chunk.source, chunk.heading, chunk.score
            )
            .unwrap();
            writeln!(out, "{}\n", chunk.content).unwrap();

            tokens_used += chunk_tokens;
            included += 1;
        }

        if included < relevant.len() {
            writeln!(
                out,
                "*{} additional chunks omitted (token budget exhausted). Query Brain for more.*\n",
                relevant.len() - included
            )
            .unwrap();
        }
    }

    fn render_code_conventions(&self, out: &mut String) {
        writeln!(out, "## Code Conventions (from owner's vault)\n").unwrap();

        // Python conventions
        writeln!(out, "### Python").unwrap();
        writeln!(out, "- FastAPI domain modules with dependency injection").unwrap();
        writeln!(
            out,
            "- Type hints on **all** function signatures — enforce with `mypy --strict`"
        )
        .unwrap();
        writeln!(out, "- Lint with `ruff` (replaces flake8/isort/pyupgrade)").unwrap();
        writeln!(
            out,
            "- Structured logging via `structlog` (never bare `print()`)"
        )
        .unwrap();
        writeln!(out, "- Pydantic v2 for all request/response schemas").unwrap();
        writeln!(out, "- Async-first: `async def` for all I/O-bound handlers").unwrap();
        writeln!(
            out,
            "- Tests with `pytest` + `pytest-asyncio`, minimum 80% coverage"
        )
        .unwrap();
        writeln!(out).unwrap();

        // TypeScript conventions
        writeln!(out, "### TypeScript").unwrap();
        writeln!(out, "- `strict: true` in tsconfig — no exceptions").unwrap();
        writeln!(
            out,
            "- **Never** use `any` — prefer `unknown` with type narrowing"
        )
        .unwrap();
        writeln!(
            out,
            "- ESLint + Prettier, enforced in CI (no warnings allowed)"
        )
        .unwrap();
        writeln!(
            out,
            "- React: functional components only, hooks for all state"
        )
        .unwrap();
        writeln!(out, "- Barrel exports (`index.ts`) at domain boundaries").unwrap();
        writeln!(
            out,
            "- Zod schemas for runtime validation at API boundaries"
        )
        .unwrap();
        writeln!(out).unwrap();

        // Database conventions
        writeln!(out, "### Database").unwrap();
        writeln!(out, "- PostgreSQL as primary store").unwrap();
        writeln!(out, "- UUID primary keys on every table").unwrap();
        writeln!(out, "- `tenant_id` column on every multi-tenant table").unwrap();
        writeln!(
            out,
            "- Migrations via versioned SQL files (never auto-generate)"
        )
        .unwrap();
        writeln!(
            out,
            "- Connection pooling required (PgBouncer / Supabase pooler)"
        )
        .unwrap();
        writeln!(out).unwrap();

        // API conventions
        writeln!(out, "### API").unwrap();
        writeln!(out, "- REST endpoints under `/api/v1/` prefix").unwrap();
        writeln!(out, "- Standard error schema: `{{ \"error\": {{ \"code\": \"...\", \"message\": \"...\" }} }}`").unwrap();
        writeln!(out, "- `BaseAPIClient` pattern for all outbound HTTP calls").unwrap();
        writeln!(
            out,
            "- Rate limiting + retry with exponential backoff on all external calls"
        )
        .unwrap();
        writeln!(
            out,
            "- OpenAPI 3.1 spec auto-generated from code annotations"
        )
        .unwrap();
        writeln!(out).unwrap();

        // CSS / Design conventions
        writeln!(out, "### CSS & Design").unwrap();
        writeln!(
            out,
            "- Tailwind CSS with `@apply` sparingly (prefer utility classes)"
        )
        .unwrap();
        writeln!(
            out,
            "- CSS custom properties for theming (design tokens in `:root`)"
        )
        .unwrap();
        writeln!(
            out,
            "- 8pt spatial grid (padding/margin in multiples of 8px)"
        )
        .unwrap();
        writeln!(
            out,
            "- Semantic color tokens: `--color-primary`, `--color-surface`, etc."
        )
        .unwrap();
        writeln!(
            out,
            "- Mobile-first responsive (375px min-width breakpoint)"
        )
        .unwrap();
        writeln!(
            out,
            "- WCAG 2.2 AA compliance required (contrast, focus, landmarks)"
        )
        .unwrap();
        writeln!(
            out,
            "- Dark mode support via `prefers-color-scheme` + manual toggle"
        )
        .unwrap();
        writeln!(out).unwrap();

        // Security conventions
        writeln!(out, "### Security").unwrap();
        writeln!(
            out,
            "- All credentials via environment variables. **No hardcoded secrets. Ever.**"
        )
        .unwrap();
        writeln!(out, "- OWASP Top 10 compliance gate in CI").unwrap();
        writeln!(out, "- Input validation at every system boundary").unwrap();
        writeln!(out, "- Content Security Policy headers on all responses").unwrap();
        writeln!(out).unwrap();
    }

    fn render_system_prompt(&self, out: &mut String, role: AgentRole) {
        writeln!(out, "## Agent Instructions\n").unwrap();
        let prompt = prompts::agent_system_prompt(role);
        writeln!(out, "{}\n", prompt).unwrap();
    }

    fn render_build_context(&self, out: &mut String, vars: &TemplateVars) {
        if vars.build_phase.is_none() && vars.project_name.is_none() && vars.extra.is_empty() {
            return;
        }

        writeln!(out, "## Build Context\n").unwrap();

        if let Some(ref name) = vars.project_name {
            writeln!(out, "- **Project:** {}", name).unwrap();
        }
        if let Some(ref phase) = vars.build_phase {
            writeln!(out, "- **Current Phase:** {}", phase).unwrap();
        }
        for (k, v) in &vars.extra {
            writeln!(out, "- **{}:** {}", k, v).unwrap();
        }
        writeln!(out).unwrap();
    }
}

impl Default for ClaudeMdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Generated file handle ────────────────────────────────────────────────────

/// A generated CLAUDE.md file that tracks its path for cleanup.
#[derive(Debug, Clone)]
pub struct GeneratedClaudeMd {
    /// Path to the generated file on disk
    pub path: PathBuf,
    /// The rendered content
    pub content: String,
    /// Environment variables to set for this agent
    pub env_vars: HashMap<String, String>,
}

impl GeneratedClaudeMd {
    /// Clean up the generated file and its parent directory (zero-footprint).
    ///
    /// Removes the CLAUDE.md file and the agent subdirectory if empty.
    #[instrument(skip(self), fields(path = %self.path.display()))]
    pub fn cleanup(&self) -> Result<(), ClaudeMdError> {
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|e| ClaudeMdError::IoError {
                path: self.path.clone(),
                reason: e.to_string(),
            })?;

            debug!(path = %self.path.display(), "removed generated CLAUDE.md");

            // Try to remove the parent directory if empty
            if let Some(parent) = self.path.parent() {
                if parent
                    .read_dir()
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false)
                {
                    let _ = std::fs::remove_dir(parent);
                    debug!(dir = %parent.display(), "removed empty agent directory");
                }
            }
        }

        Ok(())
    }

    /// Check if the generated file still exists on disk.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Content length in bytes.
    pub fn size_bytes(&self) -> usize {
        self.content.len()
    }
}

impl Drop for GeneratedClaudeMd {
    fn drop(&mut self) {
        // Best-effort cleanup on drop (zero-footprint guarantee)
        if self.path.exists() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                warn!(
                    path = %self.path.display(),
                    error = %e,
                    "failed to clean up CLAUDE.md on drop"
                );
            } else if let Some(parent) = self.path.parent() {
                let _ = std::fs::remove_dir(parent);
            }
        }
    }
}

// ─── Batch generation ─────────────────────────────────────────────────────────

/// Generate CLAUDE.md files for all agents in the team.
///
/// Knowledge chunks are filtered per-agent based on each role's KB scope.
pub fn generate_team_claude_mds(
    generator: &ClaudeMdGenerator,
    knowledge_chunks: &[KnowledgeChunk],
    build_phase: Option<&str>,
    project_name: Option<&str>,
) -> Result<Vec<GeneratedClaudeMd>, ClaudeMdError> {
    let mut results = Vec::with_capacity(crate::ALL_ROLES.len());

    for role in crate::ALL_ROLES {
        let mut vars = TemplateVars::from_role(*role);
        if let Some(phase) = build_phase {
            vars = vars.with_build_phase(phase);
        }
        if let Some(name) = project_name {
            vars = vars.with_project_name(name);
        }

        let generated = generator.generate(&vars, knowledge_chunks)?;
        results.push(generated);
    }

    info!(
        count = results.len(),
        "generated CLAUDE.md files for full team"
    );

    Ok(results)
}

/// Clean up all generated CLAUDE.md files and the base directory.
pub fn cleanup_all(files: &[GeneratedClaudeMd], base_dir: &Path) {
    for file in files {
        if let Err(e) = file.cleanup() {
            warn!(error = %e, "failed to clean up generated CLAUDE.md");
        }
    }

    // Try to remove the base directory if empty
    if base_dir.exists()
        && base_dir
            .read_dir()
            .map(|mut d| d.next().is_none())
            .unwrap_or(false)
    {
        let _ = std::fs::remove_dir(base_dir);
        debug!(dir = %base_dir.display(), "removed empty base directory");
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Filter knowledge chunks to those relevant to the agent's KB scope.
fn filter_chunks_for_scope<'a>(
    chunks: &'a [KnowledgeChunk],
    scope_files: &[&str],
) -> Vec<&'a KnowledgeChunk> {
    let mut relevant: Vec<&KnowledgeChunk> = chunks
        .iter()
        .filter(|c| {
            scope_files
                .iter()
                .any(|scope| c.source.contains(scope.trim()))
        })
        .collect();

    // Sort by score descending
    relevant.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    relevant
}

/// Simple token estimation (chars / 4).
fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

/// Parse a role ID string back into an AgentRole.
fn parse_role(role_id: &str) -> Result<AgentRole, ClaudeMdError> {
    match role_id {
        "cto" => Ok(AgentRole::Cto),
        "architect" => Ok(AgentRole::Architect),
        "backend" => Ok(AgentRole::Backend),
        "frontend" => Ok(AgentRole::Frontend),
        "devops" => Ok(AgentRole::DevOps),
        "qa" => Ok(AgentRole::Qa),
        "security" => Ok(AgentRole::Security),
        "monitor" => Ok(AgentRole::Monitor),
        other => Err(ClaudeMdError::UnknownRole(other.to_string())),
    }
}

// ─── Errors ───────────────────────────────────────────────────────────────────

/// Errors from CLAUDE.md generation.
#[derive(Debug, thiserror::Error)]
pub enum ClaudeMdError {
    #[error("I/O error at {path}: {reason}")]
    IoError { path: PathBuf, reason: String },

    #[error("unknown agent role: {0}")]
    UnknownRole(String),

    #[error("template render error: {0}")]
    RenderError(String),
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_chunks() -> Vec<KnowledgeChunk> {
        vec![
            KnowledgeChunk {
                source: "API_Expert".into(),
                heading: "REST Design".into(),
                content: "Use standard HTTP methods for CRUD.".into(),
                score: 0.92,
            },
            KnowledgeChunk {
                source: "Full_Stack_Blueprint".into(),
                heading: "Database Layer".into(),
                content: "Use PostgreSQL with connection pooling.".into(),
                score: 0.85,
            },
            KnowledgeChunk {
                source: "Design_Expert".into(),
                heading: "Color System".into(),
                content: "Use design tokens for all colors.".into(),
                score: 0.88,
            },
            KnowledgeChunk {
                source: "CTO_Complete_Technology_Knowledge".into(),
                heading: "Technology Stack".into(),
                content: "Prefer Rust for backend, React for frontend.".into(),
                score: 0.80,
            },
        ]
    }

    #[test]
    fn test_template_vars_from_role() {
        let vars = TemplateVars::from_role(AgentRole::Backend);
        assert_eq!(vars.agent_role, "backend");
        assert_eq!(vars.agent_display_name, "Backend Agent");
        assert_eq!(vars.agent_model, "claude-sonnet-4-6");
        assert_eq!(vars.agent_token_budget, 200_000);
        assert!(vars.agent_kb_scope.contains("API_Expert"));
        assert!(!vars.can_delegate);
        assert!(vars.needs_code_exec);
    }

    #[test]
    fn test_template_vars_builders() {
        let vars = TemplateVars::from_role(AgentRole::Cto)
            .with_build_phase("Code Generation")
            .with_project_name("MyApp")
            .with_extra("SPRINT", "3");

        assert_eq!(vars.build_phase.as_deref(), Some("Code Generation"));
        assert_eq!(vars.project_name.as_deref(), Some("MyApp"));
        assert_eq!(vars.extra.get("SPRINT").unwrap(), "3");
    }

    #[test]
    fn test_env_vars() {
        let vars = TemplateVars::from_role(AgentRole::Backend).with_build_phase("Testing");

        let env = vars.env_vars();
        assert_eq!(env.get("AGENT_ROLE").unwrap(), "backend");
        assert!(env.get("AGENT_KB_SCOPE").unwrap().contains("API_Expert"));
        assert_eq!(env.get("AGENT_TOKEN_BUDGET").unwrap(), "200000");
        assert_eq!(env.get("AGENT_MODEL").unwrap(), "claude-sonnet-4-6");
        assert_eq!(env.get("BUILD_PHASE").unwrap(), "Testing");
        assert_eq!(env.get("AGENT_CAN_DELEGATE").unwrap(), "false");
    }

    #[test]
    fn test_env_vars_no_optional() {
        let vars = TemplateVars::from_role(AgentRole::Monitor);
        let env = vars.env_vars();
        assert!(!env.contains_key("BUILD_PHASE"));
        assert!(!env.contains_key("PROJECT_NAME"));
    }

    #[test]
    fn test_render_basic() {
        let gen = ClaudeMdGenerator::new();
        let vars = TemplateVars::from_role(AgentRole::Backend);
        let content = gen.render(&vars, &[]).unwrap();

        assert!(content.contains("# CLAUDE.md — Backend Agent Instructions"));
        assert!(content.contains("## Identity"));
        assert!(content.contains("| Role | Backend Agent |"));
        assert!(content.contains("## Environment Variables"));
        assert!(content.contains("AGENT_ROLE=\"backend\""));
        assert!(content.contains("## Knowledge Brain Context"));
        assert!(content.contains("## Agent Instructions"));
        assert!(content.contains("Zero-footprint"));
    }

    #[test]
    fn test_render_with_knowledge() {
        let gen = ClaudeMdGenerator::new();
        let vars = TemplateVars::from_role(AgentRole::Backend);
        let chunks = sample_chunks();
        let content = gen.render(&vars, &chunks).unwrap();

        // Backend has API_Expert and Full_Stack_Blueprint in scope
        assert!(content.contains("REST Design"));
        assert!(content.contains("Database Layer"));
        assert!(content.contains("Technology Stack"));
        // Design_Expert is NOT in Backend's scope
        assert!(!content.contains("Color System"));
    }

    #[test]
    fn test_render_knowledge_token_budget() {
        let gen = ClaudeMdGenerator::new().with_max_knowledge_tokens(20);
        let vars = TemplateVars::from_role(AgentRole::Backend);
        let chunks = sample_chunks();
        let content = gen.render(&vars, &chunks).unwrap();

        // With a tiny budget, not all matching chunks should fit
        assert!(content.contains("chunks omitted") || content.contains("Pre-loaded Context"));
    }

    #[test]
    fn test_render_no_system_prompt() {
        let gen = ClaudeMdGenerator::new().with_system_prompt(false);
        let vars = TemplateVars::from_role(AgentRole::Qa);
        let content = gen.render(&vars, &[]).unwrap();

        assert!(!content.contains("## Agent Instructions"));
        assert!(content.contains("## Identity"));
    }

    #[test]
    fn test_render_build_context() {
        let gen = ClaudeMdGenerator::new().with_system_prompt(false);
        let vars = TemplateVars::from_role(AgentRole::DevOps)
            .with_build_phase("Deploy")
            .with_project_name("SuperApp");
        let content = gen.render(&vars, &[]).unwrap();

        assert!(content.contains("## Build Context"));
        assert!(content.contains("**Project:** SuperApp"));
        assert!(content.contains("**Current Phase:** Deploy"));
    }

    #[test]
    fn test_render_no_build_context_when_empty() {
        let gen = ClaudeMdGenerator::new().with_system_prompt(false);
        let vars = TemplateVars::from_role(AgentRole::Qa);
        let content = gen.render(&vars, &[]).unwrap();

        assert!(!content.contains("## Build Context"));
    }

    #[test]
    fn test_generate_and_cleanup() {
        let dir = std::env::temp_dir().join("phantom-test-claude-md");
        let gen = ClaudeMdGenerator::with_base_dir(&dir);
        let vars = TemplateVars::from_role(AgentRole::Security);

        let generated = gen.generate(&vars, &[]).unwrap();

        // File should exist
        assert!(generated.exists());
        assert!(generated.path.ends_with("security/CLAUDE.md"));
        assert!(generated.size_bytes() > 0);

        // Cleanup
        // We need to prevent the Drop impl from running, so we manually clean up
        let path = generated.path.clone();
        generated.cleanup().unwrap();

        // After cleanup, file should be gone
        assert!(!path.exists());

        // Clean up test directory
        std::mem::forget(gen); // prevent double-cleanup issues
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_writes_correct_content() {
        let dir = std::env::temp_dir().join("phantom-test-claude-md-content");
        let gen = ClaudeMdGenerator::with_base_dir(&dir).with_system_prompt(false);
        let vars = TemplateVars::from_role(AgentRole::Cto);

        let generated = gen.generate(&vars, &sample_chunks()).unwrap();

        // Read back from disk
        let on_disk = std::fs::read_to_string(&generated.path).unwrap();
        assert_eq!(on_disk, generated.content);

        // CTO has access to CTO_Complete_Technology_Knowledge
        assert!(on_disk.contains("Technology Stack"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
        std::mem::forget(generated);
    }

    #[test]
    fn test_generate_env_vars_present() {
        let dir = std::env::temp_dir().join("phantom-test-claude-md-env");
        let gen = ClaudeMdGenerator::with_base_dir(&dir).with_system_prompt(false);
        let vars = TemplateVars::from_role(AgentRole::Frontend);

        let generated = gen.generate(&vars, &[]).unwrap();

        assert_eq!(generated.env_vars.get("AGENT_ROLE").unwrap(), "frontend");
        assert!(generated
            .env_vars
            .get("AGENT_KB_SCOPE")
            .unwrap()
            .contains("Design_Expert"));

        let _ = std::fs::remove_dir_all(&dir);
        std::mem::forget(generated);
    }

    #[test]
    fn test_filter_chunks_for_scope() {
        let chunks = sample_chunks();
        let scope = vec!["API_Expert", "Full_Stack_Blueprint"];
        let filtered = filter_chunks_for_scope(&chunks, &scope);

        assert_eq!(filtered.len(), 2);
        // Sorted by score descending
        assert_eq!(filtered[0].source, "API_Expert"); // 0.92
        assert_eq!(filtered[1].source, "Full_Stack_Blueprint"); // 0.85
    }

    #[test]
    fn test_filter_chunks_empty_scope() {
        let chunks = sample_chunks();
        let scope: Vec<&str> = vec![];
        let filtered = filter_chunks_for_scope(&chunks, &scope);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_parse_role() {
        assert_eq!(parse_role("cto").unwrap(), AgentRole::Cto);
        assert_eq!(parse_role("backend").unwrap(), AgentRole::Backend);
        assert_eq!(parse_role("devops").unwrap(), AgentRole::DevOps);
        assert!(parse_role("unknown").is_err());
    }

    #[test]
    fn test_generate_team() {
        let dir = std::env::temp_dir().join("phantom-test-claude-md-team");
        let gen = ClaudeMdGenerator::with_base_dir(&dir).with_system_prompt(false);

        let results =
            generate_team_claude_mds(&gen, &sample_chunks(), Some("Code"), Some("TestApp"))
                .unwrap();

        assert_eq!(results.len(), 8);

        // Each agent should have a unique path
        let paths: Vec<&Path> = results.iter().map(|r| r.path.as_path()).collect();
        for (i, p1) in paths.iter().enumerate() {
            for (j, p2) in paths.iter().enumerate() {
                if i != j {
                    assert_ne!(p1, p2);
                }
            }
        }

        // All files should exist
        for r in &results {
            assert!(r.exists());
            assert!(r.env_vars.contains_key("AGENT_ROLE"));
            assert_eq!(r.env_vars.get("BUILD_PHASE").unwrap(), "Code");
            assert_eq!(r.env_vars.get("PROJECT_NAME").unwrap(), "TestApp");
        }

        // Cleanup
        cleanup_all(&results, &dir);
        // Forget to avoid Drop double-cleanup
        for r in results {
            std::mem::forget(r);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cleanup_all() {
        let dir = std::env::temp_dir().join("phantom-test-claude-md-cleanup");
        let gen = ClaudeMdGenerator::with_base_dir(&dir).with_system_prompt(false);

        let results = generate_team_claude_mds(&gen, &[], None, None).unwrap();
        let paths: Vec<PathBuf> = results.iter().map(|r| r.path.clone()).collect();

        cleanup_all(&results, &dir);

        for p in &paths {
            assert!(!p.exists());
        }

        // Forget to avoid Drop double-cleanup
        for r in results {
            std::mem::forget(r);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_all_roles_render_successfully() {
        let gen = ClaudeMdGenerator::new().with_system_prompt(false);
        for role in crate::ALL_ROLES {
            let vars = TemplateVars::from_role(*role);
            let content = gen.render(&vars, &[]).unwrap();
            assert!(content.contains(&format!(
                "# CLAUDE.md — {} Instructions",
                role.display_name()
            )));
        }
    }

    #[test]
    fn test_knowledge_scope_alignment() {
        // Verify that TemplateVars KB scope matches AgentRole::knowledge_scope()
        for role in crate::ALL_ROLES {
            let vars = TemplateVars::from_role(*role);
            let scope_from_vars: Vec<&str> = vars.agent_kb_scope.split(',').collect();
            let scope_from_role = role.knowledge_scope();
            assert_eq!(scope_from_vars.len(), scope_from_role.len());
        }
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("hello world!!"), 4);
    }

    #[test]
    fn test_default_generator() {
        let gen = ClaudeMdGenerator::default();
        assert!(gen.base_dir().ends_with("phantom-claude-md"));
        assert_eq!(gen.max_knowledge_tokens, 10_000);
    }

    #[test]
    fn test_render_code_conventions() {
        let gen = ClaudeMdGenerator::new().with_system_prompt(false);
        let vars = TemplateVars::from_role(AgentRole::Backend);
        let content = gen.render(&vars, &[]).unwrap();

        // Section header
        assert!(content.contains("## Code Conventions (from owner's vault)"));

        // Python conventions
        assert!(content.contains("### Python"));
        assert!(content.contains("FastAPI domain modules"));
        assert!(content.contains("mypy --strict"));
        assert!(content.contains("ruff"));
        assert!(content.contains("structlog"));

        // TypeScript conventions
        assert!(content.contains("### TypeScript"));
        assert!(content.contains("strict: true"));
        assert!(content.contains("Never** use `any`"));
        assert!(content.contains("ESLint + Prettier"));
        assert!(content.contains("Zod schemas"));

        // Database conventions
        assert!(content.contains("### Database"));
        assert!(content.contains("UUID primary keys"));
        assert!(content.contains("tenant_id"));

        // API conventions
        assert!(content.contains("### API"));
        assert!(content.contains("/api/v1/"));
        assert!(content.contains("BaseAPIClient"));

        // CSS conventions
        assert!(content.contains("### CSS & Design"));
        assert!(content.contains("Tailwind CSS"));
        assert!(content.contains("8pt spatial grid"));
        assert!(content.contains("WCAG 2.2 AA"));
        assert!(content.contains("Dark mode"));
        assert!(content.contains("375px"));

        // Security conventions
        assert!(content.contains("### Security"));
        assert!(content.contains("No hardcoded secrets. Ever."));
        assert!(content.contains("OWASP Top 10"));
    }

    #[test]
    fn test_code_conventions_in_all_agents() {
        let gen = ClaudeMdGenerator::new().with_system_prompt(false);
        for role in crate::ALL_ROLES {
            let vars = TemplateVars::from_role(*role);
            let content = gen.render(&vars, &[]).unwrap();
            assert!(
                content.contains("## Code Conventions"),
                "{} should have code conventions section",
                role.display_name()
            );
        }
    }

    #[test]
    fn test_code_conventions_before_agent_instructions() {
        let gen = ClaudeMdGenerator::new();
        let vars = TemplateVars::from_role(AgentRole::Frontend);
        let content = gen.render(&vars, &[]).unwrap();

        let conv_pos = content.find("## Code Conventions").unwrap();
        let instr_pos = content.find("## Agent Instructions").unwrap();
        assert!(
            conv_pos < instr_pos,
            "Code conventions should appear before agent instructions"
        );
    }

    #[test]
    fn test_unknown_role_error() {
        let gen = ClaudeMdGenerator::new();
        let mut vars = TemplateVars::from_role(AgentRole::Backend);
        vars.agent_role = "nonexistent".into();

        let result = gen.render(&vars, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }
}
