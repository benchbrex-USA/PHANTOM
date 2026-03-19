//! Architecture Framework Ingestion Pipeline (§12).
//!
//! Reads a Markdown Architecture Framework, extracts structured components,
//! builds a dependency DAG, enriches each node via the Knowledge Brain,
//! generates a prioritized execution plan, and presents it for owner approval.
//!
//! Pipeline:
//!   Step 1: PARSE   → headings, sections, tables, code blocks, constraints
//!   Step 2: EXTRACT → components, technologies, patterns, API contracts, DB models
//!   Step 3: GRAPH   → dependency DAG (what depends on what)
//!   Step 4: ENRICH  → query Knowledge Brain for best practices per component
//!   Step 5: PLAN    → execution plan with parallel streams + time estimates
//!   Step 6: PRESENT → show plan to owner for approval
//!   Step 7: EXECUTE → spawn agents, build everything (handed off to pipeline.rs)

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use phantom_brain::knowledge::{KnowledgeChunk, KnowledgeQuery};
use phantom_brain::KnowledgeBrain;

use crate::errors::CoreError;
use crate::pipeline::BuildPhase;
use crate::task_graph::{Task, TaskGraph};

// ── Markdown Parser ──────────────────────────────────────────────────────────

/// A parsed section from a Markdown file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownSection {
    /// Section heading text (without `#` prefix)
    pub heading: String,
    /// Heading depth (1 = `#`, 2 = `##`, etc.)
    pub depth: u8,
    /// Section number string (e.g. "3.1", "7.2.1")
    pub number: Option<String>,
    /// Raw body text (everything between this heading and the next)
    pub body: String,
    /// Tables extracted from the section body
    pub tables: Vec<MarkdownTable>,
    /// Fenced code blocks extracted from the body
    pub code_blocks: Vec<CodeBlock>,
    /// Line range in source file (start, end) — 1-indexed
    pub line_range: (usize, usize),
}

/// A parsed Markdown table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownTable {
    /// Column headers
    pub headers: Vec<String>,
    /// Rows, each a Vec of cell values
    pub rows: Vec<Vec<String>>,
}

/// A fenced code block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    /// Language hint after the opening fence (e.g. `rust`, `bash`)
    pub language: Option<String>,
    /// The code content
    pub content: String,
}

/// Complete parse result of a Markdown Architecture Framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFramework {
    /// Original file path
    pub source_path: String,
    /// Document title (first H1)
    pub title: String,
    /// All parsed sections in document order
    pub sections: Vec<MarkdownSection>,
    /// Total line count
    pub total_lines: usize,
}

/// Parses a Markdown architecture framework into structured sections.
pub struct MarkdownParser;

impl MarkdownParser {
    /// Parse a markdown file from disk.
    #[instrument(skip_all, fields(path = %path.as_ref().display()))]
    pub fn parse_file(path: impl AsRef<Path>) -> Result<ParsedFramework, CoreError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| CoreError::PipelineError {
            phase: "ingest".into(),
            reason: format!("failed to read framework file {}: {}", path.display(), e),
        })?;

        info!(path = %path.display(), "parsing architecture framework");
        Self::parse(&content, &path.display().to_string())
    }

    /// Parse raw markdown content.
    pub fn parse(content: &str, source_path: &str) -> Result<ParsedFramework, CoreError> {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        if lines.is_empty() {
            return Err(CoreError::PipelineError {
                phase: "ingest".into(),
                reason: "framework file is empty".into(),
            });
        }

        let mut sections: Vec<MarkdownSection> = Vec::new();
        let mut title = String::new();

        // Find all heading positions
        let mut heading_positions: Vec<(usize, u8, String)> = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if let Some((depth, heading_text)) = Self::parse_heading(trimmed) {
                heading_positions.push((i, depth, heading_text));
            }
        }

        if heading_positions.is_empty() {
            return Err(CoreError::PipelineError {
                phase: "ingest".into(),
                reason: "no headings found in framework file".into(),
            });
        }

        // Extract title from first H1
        if let Some((_, 1, ref h)) = heading_positions.first() {
            title = h.clone();
        }

        // Build sections from heading pairs
        for (idx, &(line_start, depth, ref heading)) in heading_positions.iter().enumerate() {
            let line_end = if idx + 1 < heading_positions.len() {
                heading_positions[idx + 1].0
            } else {
                total_lines
            };

            // Body is everything between this heading and the next
            let body_lines = &lines[line_start + 1..line_end];
            let body = body_lines.join("\n");

            // Extract section number from heading (e.g. "3.1" from "3.1 Foo Bar")
            let number = Self::extract_section_number(heading);

            // Parse tables and code blocks from body
            let tables = Self::extract_tables(&body);
            let code_blocks = Self::extract_code_blocks(&body);

            sections.push(MarkdownSection {
                heading: heading.clone(),
                depth,
                number,
                body,
                tables,
                code_blocks,
                line_range: (line_start + 1, line_end),
            });
        }

        info!(
            sections = sections.len(),
            total_lines,
            "framework parsed successfully"
        );

        Ok(ParsedFramework {
            source_path: source_path.to_string(),
            title,
            sections,
            total_lines,
        })
    }

    /// Parse a heading line, returning (depth, text).
    fn parse_heading(line: &str) -> Option<(u8, String)> {
        if !line.starts_with('#') {
            return None;
        }
        let depth = line.chars().take_while(|&c| c == '#').count();
        if depth == 0 || depth > 6 {
            return None;
        }
        let text = line[depth..].trim().to_string();
        if text.is_empty() {
            return None;
        }
        Some((depth as u8, text))
    }

    /// Extract a section number from heading text (e.g. "3.1" from "3.1 The Stack").
    fn extract_section_number(heading: &str) -> Option<String> {
        let mut chars = heading.chars().peekable();
        let mut num = String::new();

        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || c == '.' {
                num.push(c);
                chars.next();
            } else {
                break;
            }
        }

        // Must start and end with a digit, must contain at least one char
        let trimmed = num.trim_end_matches('.');
        if trimmed.is_empty() || !trimmed.chars().next().unwrap().is_ascii_digit() {
            return None;
        }

        Some(trimmed.to_string())
    }

    /// Extract Markdown tables from body text.
    fn extract_tables(body: &str) -> Vec<MarkdownTable> {
        let mut tables = Vec::new();
        let lines: Vec<&str> = body.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            // A table starts with a `|` line, followed by a separator line `|---|`
            if lines[i].trim().starts_with('|') && i + 1 < lines.len() {
                let separator_line = lines[i + 1].trim();
                if separator_line.starts_with('|') && separator_line.contains("---") {
                    // Parse header row
                    let headers = Self::parse_table_row(lines[i]);

                    // Skip separator
                    let mut rows = Vec::new();
                    let mut j = i + 2;
                    while j < lines.len() && lines[j].trim().starts_with('|') {
                        let row = Self::parse_table_row(lines[j]);
                        if !row.is_empty() {
                            rows.push(row);
                        }
                        j += 1;
                    }

                    if !headers.is_empty() {
                        tables.push(MarkdownTable { headers, rows });
                    }

                    i = j;
                    continue;
                }
            }
            i += 1;
        }

        tables
    }

    /// Parse a single table row into cell values.
    fn parse_table_row(line: &str) -> Vec<String> {
        line.split('|')
            .map(|cell| cell.trim().to_string())
            .filter(|cell| !cell.is_empty())
            .collect()
    }

    /// Extract fenced code blocks from body text.
    fn extract_code_blocks(body: &str) -> Vec<CodeBlock> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = body.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let trimmed = lines[i].trim();
            if trimmed.starts_with("```") {
                let language = {
                    let lang = trimmed.trim_start_matches('`').trim();
                    if lang.is_empty() {
                        None
                    } else {
                        Some(lang.to_string())
                    }
                };

                let mut content_lines = Vec::new();
                i += 1;
                while i < lines.len() && !lines[i].trim().starts_with("```") {
                    content_lines.push(lines[i]);
                    i += 1;
                }

                blocks.push(CodeBlock {
                    language,
                    content: content_lines.join("\n"),
                });
            }
            i += 1;
        }

        blocks
    }
}

// ── Component Extraction ─────────────────────────────────────────────────────

/// The type of architectural component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    /// A system module or crate (e.g. phantom-crypto, phantom-net)
    Module,
    /// An infrastructure/deployment concern (e.g. database, CDN, CI/CD)
    Infrastructure,
    /// A security layer (e.g. encryption, license gate, audit)
    Security,
    /// An agent in the AI team (e.g. CTO agent, Backend agent)
    Agent,
    /// An API or protocol (e.g. REST endpoints, P2P mesh, CRDT sync)
    Protocol,
    /// A build pipeline phase
    PipelinePhase,
    /// A CLI command or user-facing feature
    Feature,
    /// A knowledge base or data source
    Knowledge,
}

impl fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module => write!(f, "module"),
            Self::Infrastructure => write!(f, "infrastructure"),
            Self::Security => write!(f, "security"),
            Self::Agent => write!(f, "agent"),
            Self::Protocol => write!(f, "protocol"),
            Self::PipelinePhase => write!(f, "pipeline_phase"),
            Self::Feature => write!(f, "feature"),
            Self::Knowledge => write!(f, "knowledge"),
        }
    }
}

/// A technology mentioned in the framework (language, library, service).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Technology {
    /// Technology name (e.g. "Rust", "libp2p", "ChromaDB")
    pub name: String,
    /// Category (e.g. "language", "library", "database", "service")
    pub category: String,
    /// Purpose in the system
    pub purpose: String,
}

/// A constraint or invariant extracted from the framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// The constraint text
    pub rule: String,
    /// Which section it came from
    pub source_section: String,
    /// Severity: "must", "should", "may"
    pub severity: String,
}

/// An extracted architectural component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    /// Unique identifier (slug form, e.g. "phantom-crypto", "license-gate")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this component does
    pub description: String,
    /// Type of component
    pub kind: ComponentKind,
    /// Which agent role is responsible for building this
    pub agent_role: String,
    /// Which build phase this belongs to
    pub build_phase: BuildPhase,
    /// Technologies used by this component
    pub technologies: Vec<Technology>,
    /// Constraints that apply to this component
    pub constraints: Vec<Constraint>,
    /// IDs of components this depends on (must be built first)
    pub depends_on: Vec<String>,
    /// Estimated LOC
    pub estimated_loc: u32,
    /// Source section in the framework
    pub source_section: String,
    /// Knowledge queries to run for enrichment
    pub knowledge_queries: Vec<String>,
}

/// The complete extraction result from a parsed framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedArchitecture {
    /// Project name / title
    pub project_name: String,
    /// All extracted components
    pub components: Vec<Component>,
    /// Global technologies list (deduplicated)
    pub technologies: Vec<Technology>,
    /// Global constraints / core laws
    pub constraints: Vec<Constraint>,
    /// Total estimated LOC
    pub total_estimated_loc: u32,
}

/// Extracts structured components from a parsed framework.
pub struct ComponentExtractor;

impl ComponentExtractor {
    /// Extract all components from a parsed framework.
    #[instrument(skip_all)]
    pub fn extract(framework: &ParsedFramework) -> Result<ExtractedArchitecture, CoreError> {
        let mut components = Vec::new();
        let mut global_technologies = Vec::new();
        let mut global_constraints = Vec::new();

        // Extract core laws / constraints from §1
        for section in &framework.sections {
            if Self::section_matches(section, &["core laws", "immutable"]) {
                global_constraints.extend(Self::extract_constraints(section));
            }
        }

        // Extract technical specification table (§19)
        for section in &framework.sections {
            if Self::section_matches(section, &["technical specification", "technology"]) {
                global_technologies.extend(Self::extract_technologies_from_section(section));
            }
        }

        // §2: Knowledge Brain
        if let Some(section) = Self::find_section(framework, &["knowledge brain", "vault files"]) {
            components.push(Component {
                id: "knowledge-brain".into(),
                name: "Knowledge Brain".into(),
                description: "10 expert knowledge files embedded as vector database with semantic search".into(),
                kind: ComponentKind::Knowledge,
                agent_role: "cto".into(),
                build_phase: BuildPhase::Infrastructure,
                technologies: vec![
                    Technology { name: "ChromaDB".into(), category: "database".into(), purpose: "Vector storage for knowledge chunks".into() },
                    Technology { name: "sentence-transformers".into(), category: "library".into(), purpose: "Embedding generation (all-MiniLM-L6-v2)".into() },
                ],
                constraints: vec![],
                depends_on: vec![],
                estimated_loc: 1500,
                source_section: section.heading.clone(),
                knowledge_queries: vec![
                    "knowledge brain vector database embedding pipeline".into(),
                    "ChromaDB semantic search agent knowledge".into(),
                ],
            });
        }

        // §3: macOS Computer Access Layer
        if let Some(section) = Self::find_section(framework, &["macos", "computer access"]) {
            components.push(Component {
                id: "macos-access-layer".into(),
                name: "macOS Computer Access Layer".into(),
                description: "Full terminal control of macOS: filesystem, processes, network, keychain, clipboard, browser automation".into(),
                kind: ComponentKind::Module,
                agent_role: "devops".into(),
                build_phase: BuildPhase::Infrastructure,
                technologies: vec![
                    Technology { name: "osascript".into(), category: "tool".into(), purpose: "AppleScript/JXA automation".into() },
                    Technology { name: "launchctl".into(), category: "tool".into(), purpose: "Daemon management".into() },
                    Technology { name: "security".into(), category: "tool".into(), purpose: "Keychain access".into() },
                ],
                constraints: vec![Constraint {
                    rule: "Full computer access via terminal only — no GUI".into(),
                    source_section: section.heading.clone(),
                    severity: "must".into(),
                }],
                depends_on: vec![],
                estimated_loc: 800,
                source_section: section.heading.clone(),
                knowledge_queries: vec!["macOS terminal automation osascript keychain launchctl".into()],
            });
        }

        // §4: Autonomous Dependency Installation
        if let Some(section) = Self::find_section(framework, &["dependency installation", "autonomous dependency"]) {
            components.push(Component {
                id: "dependency-installer".into(),
                name: "Autonomous Dependency Installation Pipeline".into(),
                description: "Auto-detect and install all 18+ dependencies with owner approval".into(),
                kind: ComponentKind::Infrastructure,
                agent_role: "devops".into(),
                build_phase: BuildPhase::Infrastructure,
                technologies: vec![
                    Technology { name: "Homebrew".into(), category: "package_manager".into(), purpose: "macOS package management".into() },
                    Technology { name: "nvm".into(), category: "version_manager".into(), purpose: "Node.js version management".into() },
                    Technology { name: "pyenv".into(), category: "version_manager".into(), purpose: "Python version management".into() },
                ],
                constraints: vec![Constraint {
                    rule: "Ask permission before installing anything".into(),
                    source_section: section.heading.clone(),
                    severity: "must".into(),
                }],
                depends_on: vec!["macos-access-layer".into()],
                estimated_loc: 600,
                source_section: section.heading.clone(),
                knowledge_queries: vec!["dependency installation automation homebrew nvm pyenv".into()],
            });
        }

        // §5: Autonomous Account Creation
        if let Some(section) = Self::find_section(framework, &["account creation", "account matrix"]) {
            components.push(Component {
                id: "account-manager".into(),
                name: "Autonomous Account Creation Pipeline".into(),
                description: "Create and manage accounts on 14+ services with credential lifecycle".into(),
                kind: ComponentKind::Infrastructure,
                agent_role: "devops".into(),
                build_phase: BuildPhase::Infrastructure,
                technologies: vec![
                    Technology { name: "Playwright".into(), category: "automation".into(), purpose: "Headless browser for complex signup flows".into() },
                ],
                constraints: vec![Constraint {
                    rule: "Owner approves each account creation via terminal prompt".into(),
                    source_section: section.heading.clone(),
                    severity: "must".into(),
                }],
                depends_on: vec!["dependency-installer".into(), "credential-vault".into()],
                estimated_loc: 600,
                source_section: section.heading.clone(),
                knowledge_queries: vec!["account creation OAuth CLI automation credential management".into()],
            });
        }

        // §7: Security Architecture
        Self::extract_security_components(framework, &mut components);

        // §8: Agent Architecture
        Self::extract_agent_components(framework, &mut components);

        // §9: Zero-Footprint Execution Engine
        if let Some(section) = Self::find_section(framework, &["zero-footprint", "execution engine"]) {
            components.push(Component {
                id: "zero-footprint-engine".into(),
                name: "Zero-Footprint Execution Engine".into(),
                description: "No local disk storage — all state in remote encrypted blobs, RAM-only session".into(),
                kind: ComponentKind::Module,
                agent_role: "backend".into(),
                build_phase: BuildPhase::Architecture,
                technologies: vec![
                    Technology { name: "AES-256-GCM".into(), category: "crypto".into(), purpose: "Encrypt all remote data".into() },
                    Technology { name: "Cloudflare R2".into(), category: "storage".into(), purpose: "Encrypted blob storage".into() },
                ],
                constraints: vec![Constraint {
                    rule: "Zero local disk footprint — binary + in-memory session only".into(),
                    source_section: section.heading.clone(),
                    severity: "must".into(),
                }],
                depends_on: vec!["crypto-foundation".into(), "remote-storage".into()],
                estimated_loc: 400,
                source_section: section.heading.clone(),
                knowledge_queries: vec!["zero-footprint encrypted remote storage session management".into()],
            });
        }

        // §10: Self-Discovering Infrastructure
        if let Some(section) = Self::find_section(framework, &["self-discovering", "infrastructure"]) {
            components.push(Component {
                id: "infra-discovery".into(),
                name: "Self-Discovering Infrastructure".into(),
                description: "Auto-provision and bind to 14+ free-tier cloud providers".into(),
                kind: ComponentKind::Infrastructure,
                agent_role: "devops".into(),
                build_phase: BuildPhase::Infrastructure,
                technologies: vec![],
                constraints: vec![Constraint {
                    rule: "Total monthly cost: $0.00 — free tiers only".into(),
                    source_section: section.heading.clone(),
                    severity: "must".into(),
                }],
                depends_on: vec!["account-manager".into()],
                estimated_loc: 800,
                source_section: section.heading.clone(),
                knowledge_queries: vec![
                    "free-tier cloud providers Oracle Google AWS provisioning".into(),
                    "infrastructure discovery health checks quota monitoring".into(),
                ],
            });
        }

        // §11: P2P Mesh Layer
        if let Some(section) = Self::find_section(framework, &["peer-to-peer", "p2p mesh"]) {
            components.push(Component {
                id: "p2p-mesh".into(),
                name: "Peer-to-Peer Mesh Layer".into(),
                description: "libp2p mesh with QUIC transport, Noise handshake, Kademlia DHT, CRDT state sync".into(),
                kind: ComponentKind::Protocol,
                agent_role: "backend".into(),
                build_phase: BuildPhase::Architecture,
                technologies: vec![
                    Technology { name: "libp2p".into(), category: "networking".into(), purpose: "P2P transport and discovery".into() },
                    Technology { name: "Automerge".into(), category: "library".into(), purpose: "CRDT conflict-free replication".into() },
                    Technology { name: "QUIC".into(), category: "protocol".into(), purpose: "UDP transport with NAT traversal".into() },
                ],
                constraints: vec![Constraint {
                    rule: "Master key, session keys, raw credentials NEVER sync over P2P".into(),
                    source_section: section.heading.clone(),
                    severity: "must".into(),
                }],
                depends_on: vec!["crypto-foundation".into()],
                estimated_loc: 1700,
                source_section: section.heading.clone(),
                knowledge_queries: vec!["P2P mesh libp2p QUIC Kademlia CRDT Automerge state sync".into()],
            });
        }

        // §12: Framework Ingestion Pipeline (this module itself)
        components.push(Component {
            id: "framework-ingestion".into(),
            name: "Architecture Framework Ingestion Pipeline".into(),
            description: "Parse markdown → extract components → build DAG → enrich via KB → plan → approve".into(),
            kind: ComponentKind::PipelinePhase,
            agent_role: "cto".into(),
            build_phase: BuildPhase::Ingest,
            technologies: vec![],
            constraints: vec![],
            depends_on: vec!["knowledge-brain".into()],
            estimated_loc: 1500,
            source_section: "Architecture Framework Ingestion Pipeline".into(),
            knowledge_queries: vec!["architecture framework parsing dependency graph task planning".into()],
        });

        // §13: Build Pipeline
        if let Some(section) = Self::find_section(framework, &["autonomous build pipeline", "spec to production"]) {
            components.push(Component {
                id: "build-pipeline".into(),
                name: "Autonomous Build Pipeline".into(),
                description: "8-phase pipeline: Ingest → Infra → Arch → Code → Test → Security → Deploy → Deliver".into(),
                kind: ComponentKind::PipelinePhase,
                agent_role: "cto".into(),
                build_phase: BuildPhase::Ingest,
                technologies: vec![],
                constraints: vec![Constraint {
                    rule: "Total build time: 3-6 hours. Human input: approve plan + solve CAPTCHAs + verify emails".into(),
                    source_section: section.heading.clone(),
                    severity: "should".into(),
                }],
                depends_on: vec!["framework-ingestion".into(), "agent-orchestration".into()],
                estimated_loc: 500,
                source_section: section.heading.clone(),
                knowledge_queries: vec!["autonomous build pipeline phases parallel execution".into()],
            });
        }

        // §14: Self-Healing
        if let Some(section) = Self::find_section(framework, &["self-healing", "recovery system"]) {
            components.push(Component {
                id: "self-healing".into(),
                name: "Self-Healing & Recovery System".into(),
                description: "5-layer recovery: retry → alternative → decompose → escalate → pause & alert".into(),
                kind: ComponentKind::Module,
                agent_role: "monitor".into(),
                build_phase: BuildPhase::Architecture,
                technologies: vec![],
                constraints: vec![],
                depends_on: vec!["agent-orchestration".into()],
                estimated_loc: 450,
                source_section: section.heading.clone(),
                knowledge_queries: vec!["self-healing recovery retry exponential backoff error handling".into()],
            });
        }

        // §15: Beyond Human capabilities
        Self::extract_beyond_human_components(framework, &mut components);

        // §16: Terminal Interface
        if let Some(section) = Self::find_section(framework, &["terminal interface", "ux"]) {
            components.push(Component {
                id: "terminal-ui".into(),
                name: "Terminal Interface & UX".into(),
                description: "CLI commands + ratatui live dashboard".into(),
                kind: ComponentKind::Feature,
                agent_role: "frontend".into(),
                build_phase: BuildPhase::Code,
                technologies: vec![
                    Technology { name: "clap".into(), category: "library".into(), purpose: "CLI argument parsing".into() },
                    Technology { name: "ratatui".into(), category: "library".into(), purpose: "Terminal UI dashboard".into() },
                ],
                constraints: vec![],
                depends_on: vec!["build-pipeline".into()],
                estimated_loc: 1800,
                source_section: section.heading.clone(),
                knowledge_queries: vec!["terminal CLI interface ratatui dashboard progress".into()],
            });
        }

        // §17: Installation & Bootstrap
        if let Some(section) = Self::find_section(framework, &["installation", "bootstrap"]) {
            components.push(Component {
                id: "installer".into(),
                name: "Installation & Bootstrap Sequence".into(),
                description: "curl install script → download signed binary → activate".into(),
                kind: ComponentKind::Feature,
                agent_role: "devops".into(),
                build_phase: BuildPhase::Deploy,
                technologies: vec![],
                constraints: vec![],
                depends_on: vec!["crypto-foundation".into()],
                estimated_loc: 300,
                source_section: section.heading.clone(),
                knowledge_queries: vec!["installation script binary distribution signed release".into()],
            });
        }

        // Add remote storage component (implicit dependency)
        components.push(Component {
            id: "remote-storage".into(),
            name: "Encrypted Remote Storage".into(),
            description: "AES-256-GCM encrypted R2/S3-compatible blob storage with vault".into(),
            kind: ComponentKind::Module,
            agent_role: "backend".into(),
            build_phase: BuildPhase::Architecture,
            technologies: vec![
                Technology { name: "aws-sdk-s3".into(), category: "sdk".into(), purpose: "S3-compatible client for R2".into() },
            ],
            constraints: vec![Constraint {
                rule: "Servers are dumb encrypted blob storage — zero-knowledge".into(),
                source_section: "Zero-Footprint Execution Engine".into(),
                severity: "must".into(),
            }],
            depends_on: vec!["crypto-foundation".into()],
            estimated_loc: 1200,
            source_section: "Zero-Footprint Execution Engine".into(),
            knowledge_queries: vec!["encrypted remote storage S3 R2 zero-knowledge vault".into()],
        });

        let total_estimated_loc = components.iter().map(|c| c.estimated_loc).sum();

        info!(
            components = components.len(),
            technologies = global_technologies.len(),
            constraints = global_constraints.len(),
            total_estimated_loc,
            "component extraction complete"
        );

        Ok(ExtractedArchitecture {
            project_name: framework.title.clone(),
            components,
            technologies: global_technologies,
            constraints: global_constraints,
            total_estimated_loc,
        })
    }

    /// Extract security components from §7.
    fn extract_security_components(framework: &ParsedFramework, components: &mut Vec<Component>) {
        // Crypto foundation (always present)
        components.push(Component {
            id: "crypto-foundation".into(),
            name: "Cryptographic Foundation".into(),
            description: "Ed25519 signatures, Argon2id key derivation, AES-256-GCM encryption, HKDF sub-keys, machine fingerprinting".into(),
            kind: ComponentKind::Security,
            agent_role: "security".into(),
            build_phase: BuildPhase::Architecture,
            technologies: vec![
                Technology { name: "ring".into(), category: "crypto".into(), purpose: "Core cryptographic primitives".into() },
                Technology { name: "ed25519-dalek".into(), category: "crypto".into(), purpose: "Ed25519 signing".into() },
                Technology { name: "argon2".into(), category: "crypto".into(), purpose: "Password-based key derivation".into() },
                Technology { name: "aes-gcm".into(), category: "crypto".into(), purpose: "Authenticated encryption".into() },
            ],
            constraints: vec![
                Constraint { rule: "Master key NEVER written to disk".into(), source_section: "Security Architecture".into(), severity: "must".into() },
                Constraint { rule: "All key material zeroized after use".into(), source_section: "Security Architecture".into(), severity: "must".into() },
            ],
            depends_on: vec![],
            estimated_loc: 700,
            source_section: "Security Architecture".into(),
            knowledge_queries: vec!["cryptographic foundation Ed25519 Argon2id AES-256-GCM HKDF".into()],
        });

        // License gate
        if Self::find_section(framework, &["license", "key gate"]).is_some() {
            components.push(Component {
                id: "license-gate".into(),
                name: "License Key Gate".into(),
                description: "Ed25519-signed license tokens with machine binding and expiration".into(),
                kind: ComponentKind::Security,
                agent_role: "security".into(),
                build_phase: BuildPhase::Architecture,
                technologies: vec![],
                constraints: vec![Constraint {
                    rule: "No installation without a valid license key — fails = process exit".into(),
                    source_section: "License Key Gate".into(),
                    severity: "must".into(),
                }],
                depends_on: vec!["crypto-foundation".into()],
                estimated_loc: 300,
                source_section: "License Key Gate".into(),
                knowledge_queries: vec!["license key Ed25519 signature machine fingerprint binding".into()],
            });
        }

        // Credential vault
        components.push(Component {
            id: "credential-vault".into(),
            name: "Credential Vault".into(),
            description: "Encrypted credential storage with TTL, rotation, macOS Keychain bridge".into(),
            kind: ComponentKind::Security,
            agent_role: "security".into(),
            build_phase: BuildPhase::Architecture,
            technologies: vec![],
            constraints: vec![Constraint {
                rule: "Credentials auto-rotated every 90 days".into(),
                source_section: "Credential Lifecycle".into(),
                severity: "should".into(),
            }],
            depends_on: vec!["crypto-foundation".into(), "remote-storage".into()],
            estimated_loc: 400,
            source_section: "Credential Lifecycle".into(),
            knowledge_queries: vec!["credential vault encrypted storage rotation keychain".into()],
        });

        // Audit log
        components.push(Component {
            id: "audit-log".into(),
            name: "Tamper-Evident Audit Log".into(),
            description: "SHA-256 hash-chain audit log, signed entries, exportable".into(),
            kind: ComponentKind::Security,
            agent_role: "security".into(),
            build_phase: BuildPhase::Architecture,
            technologies: vec![],
            constraints: vec![Constraint {
                rule: "Every action is audited — tamper-evident, signed".into(),
                source_section: "Core Laws".into(),
                severity: "must".into(),
            }],
            depends_on: vec!["crypto-foundation".into()],
            estimated_loc: 450,
            source_section: "Core Laws".into(),
            knowledge_queries: vec!["audit log tamper-evident hash chain signed entries".into()],
        });
    }

    /// Extract agent components from §8.
    fn extract_agent_components(_framework: &ParsedFramework, components: &mut Vec<Component>) {
        let agent_specs: Vec<(&str, &str, &str, &str, u32)> = vec![
            ("agent-cto", "CTO Agent", "cto", "Orchestrator — parse framework, decompose tasks, monitor agents", 500),
            ("agent-architect", "Architect Agent", "architect", "System design, tech stack selection, DB schema, API contracts, ADRs", 400),
            ("agent-backend", "Backend Agent", "backend", "FastAPI, auth, database models, background jobs, model integration", 500),
            ("agent-frontend", "Frontend Agent", "frontend", "Next.js, design tokens, 8pt grid, dark mode, WCAG 2.2 AA, responsive", 500),
            ("agent-devops", "DevOps Agent", "devops", "Docker, GitHub Actions, CI/CD, branch protection, IaC", 400),
            ("agent-qa", "QA Agent", "qa", "Tests for all error classes, pytest + Vitest + Playwright, 80%+ coverage", 400),
            ("agent-security", "Security Agent", "security", "OWASP Top 10, dependency audit, auth flow audit, AI model security", 300),
            ("agent-monitor", "Monitor Agent", "monitor", "5-layer self-healing, daemon operation, health monitoring, cost tracking", 300),
        ];

        // Agent orchestration (the manager that coordinates all agents)
        components.push(Component {
            id: "agent-orchestration".into(),
            name: "Agent Orchestration Layer".into(),
            description: "Spawn, coordinate, and monitor the 8-agent AI engineering team".into(),
            kind: ComponentKind::Module,
            agent_role: "cto".into(),
            build_phase: BuildPhase::Architecture,
            technologies: vec![
                Technology { name: "Anthropic API".into(), category: "api".into(), purpose: "Agent reasoning via Claude".into() },
            ],
            constraints: vec![Constraint {
                rule: "Token budgets, scoped permissions, timeouts, signed audit log per agent".into(),
                source_section: "Agent Architecture".into(),
                severity: "must".into(),
            }],
            depends_on: vec![
                "crypto-foundation".into(),
                "knowledge-brain".into(),
                "audit-log".into(),
            ],
            estimated_loc: 1200,
            source_section: "Agent Architecture".into(),
            knowledge_queries: vec!["multi-agent orchestration task delegation parallel execution token budget".into()],
        });

        // Individual agent definitions as components
        for (id, name, role, desc, loc) in agent_specs {
            components.push(Component {
                id: id.into(),
                name: name.into(),
                description: desc.into(),
                kind: ComponentKind::Agent,
                agent_role: role.into(),
                build_phase: BuildPhase::Code,
                technologies: vec![],
                constraints: vec![],
                depends_on: vec!["agent-orchestration".into()],
                estimated_loc: loc,
                source_section: "Agent Architecture".into(),
                knowledge_queries: vec![format!("{} agent role responsibilities knowledge scope", role)],
            });
        }
    }

    /// Extract "Beyond Human" feature components from §15.
    fn extract_beyond_human_components(_framework: &ParsedFramework, components: &mut Vec<Component>) {
        let features: Vec<(&str, &str, &str, &str, u32, Vec<&str>)> = vec![
            (
                "ambient-awareness",
                "Ambient Context Awareness",
                "Continuously monitor terminal, git, docker, ports, CPU, battery, time",
                "monitor",
                500,
                vec!["agent-monitor".into()],
            ),
            (
                "self-scheduling-daemon",
                "Self-Scheduling Daemon",
                "launchd daemon — runs at login, scheduled builds, survives terminal closure",
                "devops",
                300,
                vec!["macos-access-layer".into()],
            ),
            (
                "smart-git",
                "Smart Git Workflows",
                "Semantic branches, conventional commits, auto PRs, auto merge, changelogs",
                "devops",
                400,
                vec!["agent-devops".into()],
            ),
            (
                "predictive-errors",
                "Predictive Error Prevention",
                "Pre-check library versions, API endpoints, ports, disk, circular deps, migrations",
                "qa",
                300,
                vec!["knowledge-brain".into()],
            ),
            (
                "cross-project-memory",
                "Cross-Project Memory",
                "Remember what worked across all projects, stored as learned patterns in ChromaDB",
                "cto",
                200,
                vec!["knowledge-brain".into()],
            ),
            (
                "cost-oracle",
                "Cost Oracle",
                "Estimate infrastructure and AI costs per framework before building",
                "cto",
                200,
                vec!["infra-discovery".into()],
            ),
        ];

        for (id, name, desc, role, loc, deps) in features {
            components.push(Component {
                id: id.into(),
                name: name.into(),
                description: desc.into(),
                kind: ComponentKind::Feature,
                agent_role: role.into(),
                build_phase: BuildPhase::Code,
                technologies: vec![],
                constraints: vec![],
                depends_on: deps.into_iter().map(String::from).collect(),
                estimated_loc: loc,
                source_section: "Beyond Human".into(),
                knowledge_queries: vec![format!("{} automation monitoring", name)],
            });
        }
    }

    /// Check if a section heading matches any of the given keywords (case-insensitive).
    fn section_matches(section: &MarkdownSection, keywords: &[&str]) -> bool {
        let lower = section.heading.to_lowercase();
        keywords.iter().any(|kw| lower.contains(kw))
    }

    /// Find the first section whose heading matches any of the given keywords.
    fn find_section<'a>(
        framework: &'a ParsedFramework,
        keywords: &[&str],
    ) -> Option<&'a MarkdownSection> {
        framework
            .sections
            .iter()
            .find(|s| Self::section_matches(s, keywords))
    }

    /// Extract technologies from a section's tables.
    fn extract_technologies_from_section(section: &MarkdownSection) -> Vec<Technology> {
        let mut techs = Vec::new();

        for table in &section.tables {
            // Look for tables with columns like "Technology", "Purpose" or "Layer", "Technology"
            let name_col = table.headers.iter().position(|h| {
                let lower = h.to_lowercase();
                lower.contains("technology") || lower.contains("tool") || lower.contains("name")
            });
            let purpose_col = table.headers.iter().position(|h| {
                let lower = h.to_lowercase();
                lower.contains("purpose") || lower.contains("description") || lower.contains("use")
            });
            let category_col = table.headers.iter().position(|h| {
                let lower = h.to_lowercase();
                lower.contains("layer") || lower.contains("category") || lower.contains("type")
            });

            if let Some(name_idx) = name_col {
                for row in &table.rows {
                    if let Some(name) = row.get(name_idx) {
                        if name.trim().is_empty() {
                            continue;
                        }
                        techs.push(Technology {
                            name: name.trim().to_string(),
                            category: category_col
                                .and_then(|i| row.get(i))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_default(),
                            purpose: purpose_col
                                .and_then(|i| row.get(i))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_default(),
                        });
                    }
                }
            }
        }

        techs
    }

    /// Extract constraints from a section's tables and body.
    fn extract_constraints(section: &MarkdownSection) -> Vec<Constraint> {
        let mut constraints = Vec::new();

        for table in &section.tables {
            // Look for a "Law" or "Rule" column
            let rule_col = table.headers.iter().position(|h| {
                let lower = h.to_lowercase();
                lower.contains("law") || lower.contains("rule") || lower.contains("constraint")
            });
            let enforcement_col = table.headers.iter().position(|h| {
                let lower = h.to_lowercase();
                lower.contains("enforce") || lower.contains("how") || lower.contains("mitigation")
            });

            if let Some(rule_idx) = rule_col {
                for row in &table.rows {
                    if let Some(rule_text) = row.get(rule_idx) {
                        if rule_text.trim().is_empty() {
                            continue;
                        }
                        let mut full_rule = rule_text.trim().to_string();
                        if let Some(enforce_idx) = enforcement_col {
                            if let Some(how) = row.get(enforce_idx) {
                                if !how.trim().is_empty() {
                                    full_rule = format!("{} [Enforced: {}]", full_rule, how.trim());
                                }
                            }
                        }
                        constraints.push(Constraint {
                            rule: full_rule,
                            source_section: section.heading.clone(),
                            severity: "must".into(),
                        });
                    }
                }
            }
        }

        constraints
    }
}

// ── Dependency DAG Builder ───────────────────────────────────────────────────

/// A node in the dependency DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    /// Component ID
    pub component_id: String,
    /// Component name
    pub name: String,
    /// Agent role
    pub agent_role: String,
    /// Build phase
    pub build_phase: BuildPhase,
    /// IDs this node depends on
    pub depends_on: Vec<String>,
    /// Estimated LOC
    pub estimated_loc: u32,
    /// Knowledge enrichment (populated after enrichment step)
    pub knowledge_context: Vec<KnowledgeChunk>,
    /// Generated task ID (populated during plan generation)
    pub task_id: Option<String>,
}

/// The component dependency DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDag {
    /// All nodes, indexed by component ID
    pub nodes: HashMap<String, DagNode>,
    /// Insertion order for deterministic iteration
    order: Vec<String>,
    /// Parallel execution layers (computed from topological sort)
    pub layers: Vec<Vec<String>>,
    /// Total estimated LOC
    pub total_estimated_loc: u32,
}

impl ComponentDag {
    /// Build a DAG from extracted components.
    #[instrument(skip_all)]
    pub fn build(architecture: &ExtractedArchitecture) -> Result<Self, CoreError> {
        let mut nodes = HashMap::new();
        let mut order = Vec::new();

        // Insert all components as nodes
        for component in &architecture.components {
            let node = DagNode {
                component_id: component.id.clone(),
                name: component.name.clone(),
                agent_role: component.agent_role.clone(),
                build_phase: component.build_phase,
                depends_on: component.depends_on.clone(),
                estimated_loc: component.estimated_loc,
                knowledge_context: Vec::new(),
                task_id: None,
            };
            order.push(component.id.clone());
            nodes.insert(component.id.clone(), node);
        }

        // Validate: all dependency references exist, remove dangling ones
        let all_ids: HashSet<String> = nodes.keys().cloned().collect();
        for node in nodes.values_mut() {
            let before = node.depends_on.len();
            node.depends_on.retain(|dep| all_ids.contains(dep));
            if node.depends_on.len() < before {
                warn!(
                    component = %node.component_id,
                    removed = before - node.depends_on.len(),
                    "removed references to missing dependency components"
                );
            }
        }

        // Detect cycles
        Self::detect_cycles(&nodes)?;

        // Compute parallel layers
        let layers = Self::compute_layers(&nodes, &order)?;

        let total_estimated_loc = nodes.values().map(|n| n.estimated_loc).sum();

        info!(
            nodes = nodes.len(),
            layers = layers.len(),
            total_estimated_loc,
            "dependency DAG built successfully"
        );

        Ok(Self {
            nodes,
            order,
            layers,
            total_estimated_loc,
        })
    }

    /// Detect cycles using Kahn's algorithm.
    fn detect_cycles(nodes: &HashMap<String, DagNode>) -> Result<(), CoreError> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in nodes.values() {
            in_degree.entry(&node.component_id).or_insert(0);
            for dep in &node.depends_on {
                adj.entry(dep.as_str())
                    .or_default()
                    .push(&node.component_id);
                *in_degree.entry(&node.component_id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut visited = 0;
        while let Some(id) = queue.pop_front() {
            visited += 1;
            if let Some(neighbors) = adj.get(id) {
                for &n in neighbors {
                    if let Some(d) = in_degree.get_mut(n) {
                        *d -= 1;
                        if *d == 0 {
                            queue.push_back(n);
                        }
                    }
                }
            }
        }

        if visited != nodes.len() {
            let cycle_nodes: Vec<String> = in_degree
                .iter()
                .filter(|(_, &d)| d > 0)
                .map(|(&id, _)| id.to_string())
                .collect();
            return Err(CoreError::DependencyCycle(cycle_nodes.join(", ")));
        }

        Ok(())
    }

    /// Compute parallel execution layers via topological depth.
    fn compute_layers(
        nodes: &HashMap<String, DagNode>,
        order: &[String],
    ) -> Result<Vec<Vec<String>>, CoreError> {
        // Topological sort first
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for node in nodes.values() {
            in_degree.entry(&node.component_id).or_insert(0);
            for dep in &node.depends_on {
                adj.entry(dep.as_str())
                    .or_default()
                    .push(&node.component_id);
                *in_degree.entry(&node.component_id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut topo_order = Vec::new();
        while let Some(id) = queue.pop_front() {
            topo_order.push(id.to_string());
            if let Some(neighbors) = adj.get(id) {
                for &n in neighbors {
                    if let Some(d) = in_degree.get_mut(n) {
                        *d -= 1;
                        if *d == 0 {
                            queue.push_back(n);
                        }
                    }
                }
            }
        }

        // Compute depth for each node
        let mut depth: HashMap<&str, usize> = HashMap::new();
        for id in &topo_order {
            let node = &nodes[id];
            let max_dep_depth = node
                .depends_on
                .iter()
                .filter_map(|dep| depth.get(dep.as_str()))
                .max()
                .copied()
                .unwrap_or(0);

            let d = if node.depends_on.is_empty() {
                0
            } else {
                max_dep_depth + 1
            };
            depth.insert(id, d);
        }

        // Group by depth
        let max_depth = depth.values().max().copied().unwrap_or(0);
        let mut layers: Vec<Vec<String>> = vec![Vec::new(); max_depth + 1];
        // Use insertion order within each layer for determinism
        for id in order {
            if let Some(&d) = depth.get(id.as_str()) {
                layers[d].push(id.clone());
            }
        }

        // Remove empty layers
        layers.retain(|l| !l.is_empty());

        Ok(layers)
    }

    /// Get topological ordering of component IDs.
    pub fn topological_order(&self) -> Vec<String> {
        self.layers.iter().flat_map(|l| l.iter().cloned()).collect()
    }

    /// Get the node for a component.
    pub fn get(&self, id: &str) -> Option<&DagNode> {
        self.nodes.get(id)
    }

    /// Get a mutable node for a component.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut DagNode> {
        self.nodes.get_mut(id)
    }

    /// Number of components in the DAG.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Iterate over nodes in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &DagNode> {
        self.order.iter().filter_map(|id| self.nodes.get(id))
    }
}

// ── Knowledge Enrichment ─────────────────────────────────────────────────────

/// Enrichment metadata attached to a DAG node after querying the Knowledge Brain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentResult {
    /// Component ID that was enriched
    pub component_id: String,
    /// Knowledge chunks retrieved
    pub chunks: Vec<KnowledgeChunk>,
    /// Generated best-practice summary from knowledge
    pub best_practices: Vec<String>,
    /// Suggested technologies from knowledge (may add to component's tech list)
    pub suggested_technologies: Vec<String>,
    /// Relevant knowledge file references (for citation)
    pub citations: Vec<String>,
}

/// Enriches DAG nodes by querying the Knowledge Brain for each component.
pub struct KnowledgeEnricher;

impl KnowledgeEnricher {
    /// Enrich all components in a DAG with knowledge from the brain.
    ///
    /// For each component, queries the Knowledge Brain with the component's
    /// knowledge queries and attaches the results to the DAG node.
    #[instrument(skip_all)]
    pub async fn enrich(
        dag: &mut ComponentDag,
        architecture: &ExtractedArchitecture,
        brain: &KnowledgeBrain,
    ) -> Result<Vec<EnrichmentResult>, CoreError> {
        let mut results = Vec::new();

        // Build a map of component ID → knowledge queries
        let queries_by_id: HashMap<String, Vec<String>> = architecture
            .components
            .iter()
            .map(|c| (c.id.clone(), c.knowledge_queries.clone()))
            .collect();

        for (component_id, queries) in &queries_by_id {
            let node = match dag.get(component_id) {
                Some(n) => n,
                None => continue,
            };

            let agent_role = node.agent_role.clone();
            let mut all_chunks: Vec<KnowledgeChunk> = Vec::new();

            for query_text in queries {
                let query = KnowledgeQuery::new(query_text.as_str())
                    .with_agent_role(&agent_role)
                    .with_top_k(3)
                    .with_min_score(0.4);

                match brain.query(&query).await {
                    Ok(chunks) => {
                        debug!(
                            component = %component_id,
                            query = %query_text,
                            results = chunks.len(),
                            "knowledge query returned results"
                        );
                        all_chunks.extend(chunks);
                    }
                    Err(e) => {
                        warn!(
                            component = %component_id,
                            query = %query_text,
                            error = %e,
                            "knowledge query failed, continuing without enrichment"
                        );
                    }
                }
            }

            // Deduplicate chunks by source_file + section
            let mut seen = HashSet::new();
            all_chunks.retain(|chunk| {
                let key = format!("{}::{}", chunk.source_file, chunk.section);
                seen.insert(key)
            });

            // Sort by score descending
            all_chunks.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

            // Extract best practices and citations
            let best_practices: Vec<String> = all_chunks
                .iter()
                .take(5)
                .map(|c| {
                    // Take first meaningful line as a practice summary
                    c.content
                        .lines()
                        .find(|l| !l.trim().is_empty() && l.len() > 10)
                        .unwrap_or(&c.section)
                        .trim()
                        .to_string()
                })
                .collect();

            let citations: Vec<String> = all_chunks
                .iter()
                .take(5)
                .map(|c| format!("{} §{}", c.source_file, c.section))
                .collect();

            let suggested_technologies: Vec<String> = Vec::new();

            let enrichment = EnrichmentResult {
                component_id: component_id.clone(),
                chunks: all_chunks.clone(),
                best_practices,
                suggested_technologies,
                citations,
            };

            // Attach enrichment to the DAG node
            if let Some(node) = dag.get_mut(component_id) {
                node.knowledge_context = all_chunks;
            }

            results.push(enrichment);
        }

        info!(
            enriched = results.len(),
            "knowledge enrichment complete"
        );

        Ok(results)
    }

    /// Enrich without a live Knowledge Brain (offline mode).
    /// Generates placeholder enrichment so the pipeline can proceed.
    pub fn enrich_offline(
        _dag: &mut ComponentDag,
        architecture: &ExtractedArchitecture,
    ) -> Vec<EnrichmentResult> {
        let mut results = Vec::new();

        for component in &architecture.components {
            let citations: Vec<String> = component
                .knowledge_queries
                .iter()
                .map(|q| format!("(offline) query: {}", q))
                .collect();

            results.push(EnrichmentResult {
                component_id: component.id.clone(),
                chunks: Vec::new(),
                best_practices: vec![format!(
                    "Build {} following established patterns from the Knowledge Brain",
                    component.name
                )],
                suggested_technologies: Vec::new(),
                citations,
            });
        }

        info!(
            enriched = results.len(),
            "offline enrichment generated (no Knowledge Brain)"
        );

        results
    }
}

// ── Execution Plan ───────────────────────────────────────────────────────────

/// A work stream — a set of tasks that can execute in parallel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkStream {
    /// Stream index (0-based, maps to DAG layer)
    pub index: usize,
    /// Components in this stream (can run in parallel)
    pub components: Vec<String>,
    /// Agent roles involved
    pub agent_roles: Vec<String>,
    /// Estimated total LOC for this stream
    pub estimated_loc: u32,
    /// Estimated time in seconds (based on ~100 LOC/min for AI agents)
    pub estimated_seconds: u32,
    /// Which build phase(s) this stream covers
    pub build_phases: Vec<BuildPhase>,
}

/// The complete execution plan, ready for owner approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Project name
    pub project_name: String,
    /// Work streams in dependency order
    pub streams: Vec<WorkStream>,
    /// Total components
    pub total_components: usize,
    /// Total estimated LOC
    pub total_estimated_loc: u32,
    /// Total estimated time in seconds
    pub total_estimated_seconds: u32,
    /// Global constraints that apply to the entire build
    pub constraints: Vec<Constraint>,
    /// All enrichment results for reference
    pub enrichments: Vec<EnrichmentResult>,
    /// Whether the owner has approved this plan
    pub approved: bool,
    /// The TaskGraph generated from this plan (populated on approval)
    pub task_graph: Option<TaskGraph>,
}

/// Generates an execution plan from the enriched DAG.
pub struct PlanGenerator;

impl PlanGenerator {
    /// Generate an execution plan from an enriched DAG.
    #[instrument(skip_all)]
    pub fn generate(
        dag: &ComponentDag,
        architecture: &ExtractedArchitecture,
        enrichments: Vec<EnrichmentResult>,
    ) -> Result<ExecutionPlan, CoreError> {
        let mut streams = Vec::new();

        for (layer_idx, layer) in dag.layers.iter().enumerate() {
            let mut stream_loc: u32 = 0;
            let mut agent_roles: HashSet<String> = HashSet::new();
            let mut build_phases: HashSet<BuildPhase> = HashSet::new();

            for component_id in layer {
                if let Some(node) = dag.get(component_id) {
                    stream_loc += node.estimated_loc;
                    agent_roles.insert(node.agent_role.clone());
                    build_phases.insert(node.build_phase);
                }
            }

            // Estimate time: ~100 LOC/min for AI agents, with parallel execution discount
            let parallel_factor = layer.len().max(1) as u32;
            let serial_seconds = (stream_loc as f64 / 100.0 * 60.0) as u32;
            let estimated_seconds = serial_seconds / parallel_factor;

            let mut sorted_roles: Vec<String> = agent_roles.into_iter().collect();
            sorted_roles.sort();
            let mut sorted_phases: Vec<BuildPhase> = build_phases.into_iter().collect();
            sorted_phases.sort_by_key(|p| p.estimated_seconds());

            streams.push(WorkStream {
                index: layer_idx,
                components: layer.clone(),
                agent_roles: sorted_roles,
                estimated_loc: stream_loc,
                estimated_seconds,
                build_phases: sorted_phases,
            });
        }

        let total_estimated_seconds = streams.iter().map(|s| s.estimated_seconds).sum();
        let total_estimated_loc = dag.total_estimated_loc;

        info!(
            streams = streams.len(),
            total_estimated_loc,
            total_estimated_seconds,
            "execution plan generated"
        );

        Ok(ExecutionPlan {
            project_name: architecture.project_name.clone(),
            streams,
            total_components: dag.len(),
            total_estimated_loc,
            total_estimated_seconds,
            constraints: architecture.constraints.clone(),
            enrichments,
            approved: false,
            task_graph: None,
        })
    }

    /// Convert an approved execution plan into a TaskGraph for the build pipeline.
    pub fn to_task_graph(plan: &ExecutionPlan, dag: &ComponentDag) -> Result<TaskGraph, CoreError> {
        let mut graph = TaskGraph::new();
        let mut component_to_task_id: HashMap<String, String> = HashMap::new();

        // Create tasks in topological order
        for component_id in dag.topological_order() {
            let node = match dag.get(&component_id) {
                Some(n) => n,
                None => continue,
            };

            // Build knowledge query hint from enrichment citations
            let knowledge_hint = plan
                .enrichments
                .iter()
                .find(|e| e.component_id == component_id)
                .map(|e| e.citations.join("; "))
                .unwrap_or_default();

            let mut task = Task::new(
                &node.name,
                format!(
                    "Build component '{}' ({} est. LOC). Agent: {}",
                    node.name, node.estimated_loc, node.agent_role
                ),
                &node.agent_role,
            )
            .with_estimate(Self::loc_to_seconds(node.estimated_loc))
            .with_phase(node.build_phase.display_name());

            if !knowledge_hint.is_empty() {
                task = task.with_knowledge_query(knowledge_hint);
            }

            // Wire dependencies using the component → task ID mapping
            for dep_id in &node.depends_on {
                if let Some(task_id) = component_to_task_id.get(dep_id) {
                    task = task.depends_on(task_id);
                }
            }

            let task_id = graph.add_task(task)?;
            component_to_task_id.insert(component_id.clone(), task_id);
        }

        // Validate the generated graph
        graph.validate()?;

        info!(
            tasks = graph.len(),
            "task graph generated from execution plan"
        );

        Ok(graph)
    }

    /// Rough LOC → seconds estimate: ~100 LOC/min for AI-assisted development.
    fn loc_to_seconds(loc: u32) -> u32 {
        ((loc as f64 / 100.0) * 60.0) as u32
    }
}

impl ExecutionPlan {
    /// Mark the plan as approved by the owner.
    pub fn approve(&mut self) {
        self.approved = true;
    }

    /// Format the plan as a human-readable summary for owner review.
    pub fn display_summary(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!(
            "┌─ EXECUTION PLAN ─────────────────────────────────────────────────┐\n"
        ));
        out.push_str(&format!(
            "│  Project: {:<54}│\n",
            truncate_str(&self.project_name, 54)
        ));
        out.push_str(&format!(
            "│  Components: {:<51}│\n",
            self.total_components
        ));
        out.push_str(&format!(
            "│  Estimated LOC: {:<48}│\n",
            format!("~{}", self.total_estimated_loc)
        ));
        out.push_str(&format!(
            "│  Estimated Time: {:<47}│\n",
            format_duration(self.total_estimated_seconds)
        ));
        out.push_str(
            "├──────────────────────────────────────────────────────────────────┤\n",
        );

        for stream in &self.streams {
            out.push_str(&format!(
                "│  Stream {} ({} parallel) — {} — ~{}          │\n",
                stream.index,
                stream.components.len(),
                format_duration(stream.estimated_seconds),
                stream.estimated_loc,
            ));

            for comp_id in &stream.components {
                let role = self
                    .enrichments
                    .iter()
                    .find(|e| &e.component_id == comp_id)
                    .map(|_| "")
                    .unwrap_or("");
                out.push_str(&format!(
                    "│    ├── {:<56}│\n",
                    truncate_str(comp_id, 56)
                ));
                let _ = role; // Suppress unused warning
            }
        }

        if !self.constraints.is_empty() {
            out.push_str(
                "├──────────────────────────────────────────────────────────────────┤\n",
            );
            out.push_str(
                "│  CONSTRAINTS:                                                    │\n",
            );
            for (i, c) in self.constraints.iter().take(5).enumerate() {
                out.push_str(&format!(
                    "│  {}. {:<60}│\n",
                    i + 1,
                    truncate_str(&c.rule, 60)
                ));
            }
            if self.constraints.len() > 5 {
                out.push_str(&format!(
                    "│  ... and {} more                                                │\n",
                    self.constraints.len() - 5
                ));
            }
        }

        out.push_str(
            "└──────────────────────────────────────────────────────────────────┘\n",
        );

        out
    }
}

// ── Full Ingestion Pipeline ──────────────────────────────────────────────────

/// Result of the full ingestion pipeline.
#[derive(Debug)]
pub struct IngestionResult {
    /// The parsed framework
    pub framework: ParsedFramework,
    /// Extracted architecture
    pub architecture: ExtractedArchitecture,
    /// Component dependency DAG
    pub dag: ComponentDag,
    /// Generated execution plan
    pub plan: ExecutionPlan,
}

/// The full Architecture Framework Ingestion Pipeline.
///
/// Orchestrates: Parse → Extract → DAG → Enrich → Plan → Present.
pub struct IngestionPipeline;

impl IngestionPipeline {
    /// Run the full ingestion pipeline on a framework file.
    ///
    /// Steps 1-6 from Architecture Framework §12:
    ///   1. PARSE   → extract headings, sections, tables, code blocks
    ///   2. EXTRACT → components, technologies, patterns, constraints
    ///   3. GRAPH   → build dependency DAG
    ///   4. ENRICH  → query Knowledge Brain for best practices
    ///   5. PLAN    → generate execution plan with time estimates
    ///   6. PRESENT → return plan for owner approval
    ///
    /// Step 7 (EXECUTE) is handled by the build pipeline.
    #[instrument(skip(brain), fields(path = %path.as_ref().display()))]
    pub async fn run(
        path: impl AsRef<Path>,
        brain: Option<&KnowledgeBrain>,
    ) -> Result<IngestionResult, CoreError> {
        let path = path.as_ref();
        info!("starting Architecture Framework ingestion");

        // Step 1: PARSE
        info!("step 1/6: parsing markdown framework");
        let framework = MarkdownParser::parse_file(path)?;
        info!(
            sections = framework.sections.len(),
            lines = framework.total_lines,
            "parsed {} sections from {} lines",
            framework.sections.len(),
            framework.total_lines
        );

        // Step 2: EXTRACT
        info!("step 2/6: extracting architectural components");
        let architecture = ComponentExtractor::extract(&framework)?;
        info!(
            components = architecture.components.len(),
            technologies = architecture.technologies.len(),
            constraints = architecture.constraints.len(),
            "extracted {} components, {} technologies, {} constraints",
            architecture.components.len(),
            architecture.technologies.len(),
            architecture.constraints.len()
        );

        // Step 3: GRAPH
        info!("step 3/6: building dependency DAG");
        let mut dag = ComponentDag::build(&architecture)?;
        info!(
            nodes = dag.len(),
            layers = dag.layers.len(),
            "built DAG with {} nodes in {} parallel layers",
            dag.len(),
            dag.layers.len()
        );

        // Step 4: ENRICH
        info!("step 4/6: enriching components via Knowledge Brain");
        let enrichments = if let Some(brain) = brain {
            match KnowledgeEnricher::enrich(&mut dag, &architecture, brain).await {
                Ok(results) => results,
                Err(e) => {
                    warn!(
                        error = %e,
                        "Knowledge Brain enrichment failed, falling back to offline mode"
                    );
                    KnowledgeEnricher::enrich_offline(&mut dag, &architecture)
                }
            }
        } else {
            info!("no Knowledge Brain available, using offline enrichment");
            KnowledgeEnricher::enrich_offline(&mut dag, &architecture)
        };

        // Step 5: PLAN
        info!("step 5/6: generating execution plan");
        let plan = PlanGenerator::generate(&dag, &architecture, enrichments)?;

        // Step 6: PRESENT (caller is responsible for displaying and getting approval)
        info!(
            streams = plan.streams.len(),
            total_loc = plan.total_estimated_loc,
            total_time = format_duration(plan.total_estimated_seconds),
            "execution plan ready for owner approval"
        );

        Ok(IngestionResult {
            framework,
            architecture,
            dag,
            plan,
        })
    }

    /// Run the ingestion pipeline synchronously (no Knowledge Brain enrichment).
    pub fn run_sync(path: impl AsRef<Path>) -> Result<IngestionResult, CoreError> {
        let path = path.as_ref();
        info!("starting synchronous framework ingestion (offline mode)");

        let framework = MarkdownParser::parse_file(path)?;
        let architecture = ComponentExtractor::extract(&framework)?;
        let mut dag = ComponentDag::build(&architecture)?;
        let enrichments = KnowledgeEnricher::enrich_offline(&mut dag, &architecture);
        let plan = PlanGenerator::generate(&dag, &architecture, enrichments)?;

        Ok(IngestionResult {
            framework,
            architecture,
            dag,
            plan,
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Format seconds into a human-readable duration string.
fn format_duration(seconds: u32) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}

/// Truncate a string to fit within a display width.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_FRAMEWORK: &str = r#"# Test Project — Architecture Framework

## 1. System Identity & Core Laws

### Core Laws (Immutable — enforced in code, not policy)

| # | Law | How It's Enforced |
|---|-----|-------------------|
| 1 | No installation without a valid license key | Ed25519 signature check |
| 2 | Zero local disk footprint | All state in remote encrypted storage |

## 2. The Knowledge Brain — All 10 Vault Files

The Knowledge Brain stores expert-level intelligence.

```
QUERY FLOW:
  Agent receives task → queries ChromaDB → gets knowledge chunks
```

## 3. Full macOS Computer Access Layer

Phantom controls your macOS terminal.

| CATEGORY | HOW | PURPOSE |
|----------|-----|---------|
| FILE SYSTEM | ls, find, cp | Project scaffold |
| NETWORK | curl, ssh | API calls |

## 7. Security Architecture — Key Hierarchy

### 7.2 License Key Gate

License format: PH1-<payload>-<signature>

## 8. Agent Architecture — The AI Engineering Team

The CTO agent leads 7 specialist agents.

## 9. Zero-Footprint Execution Engine

No local storage. RAM only.

## 10. Self-Discovering Infrastructure

14+ free-tier providers.

## 11. Peer-to-Peer Mesh Layer

libp2p mesh with QUIC transport.

## 12. Architecture Framework Ingestion Pipeline

Parse → Extract → Graph → Enrich → Plan → Approve → Execute.

## 13. Autonomous Build Pipeline — Spec to Production

8 phases from ingest to deliver.

## 14. Self-Healing & Recovery System

5-layer recovery.

## 15. Beyond Human — Capabilities

### 15.1 Ambient Context Awareness

Monitor everything.

## 16. Terminal Interface & UX

CLI + ratatui dashboard.

## 17. Installation & Bootstrap Sequence

One command install.

## 19. Complete Technical Specification

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Binary | Rust (static) | Zero-dep executable |
| CLI | clap (Rust) | Argument parsing |
| Crypto | ring + ed25519-dalek | AES-256-GCM, Ed25519 |
"#;

    #[test]
    fn test_parse_headings() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        assert_eq!(framework.title, "Test Project — Architecture Framework");
        assert!(!framework.sections.is_empty());
        assert!(framework.sections.len() >= 10);
    }

    #[test]
    fn test_parse_heading_depths() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();

        let h1_count = framework.sections.iter().filter(|s| s.depth == 1).count();
        let h2_count = framework.sections.iter().filter(|s| s.depth == 2).count();
        let h3_count = framework.sections.iter().filter(|s| s.depth == 3).count();

        assert_eq!(h1_count, 1);
        assert!(h2_count >= 10);
        assert!(h3_count >= 2);
    }

    #[test]
    fn test_extract_section_number() {
        assert_eq!(
            MarkdownParser::extract_section_number("3.1 The Stack"),
            Some("3.1".into())
        );
        assert_eq!(
            MarkdownParser::extract_section_number("15.2 Feature"),
            Some("15.2".into())
        );
        assert_eq!(
            MarkdownParser::extract_section_number("No number here"),
            None
        );
        assert_eq!(
            MarkdownParser::extract_section_number("1. System Identity"),
            Some("1".into())
        );
    }

    #[test]
    fn test_extract_tables() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();

        let tables: Vec<&MarkdownTable> = framework
            .sections
            .iter()
            .flat_map(|s| &s.tables)
            .collect();

        assert!(
            tables.len() >= 2,
            "expected at least 2 tables, found {}",
            tables.len()
        );

        // Core laws table should have "Law" header
        let law_table = tables
            .iter()
            .find(|t| t.headers.iter().any(|h| h.contains("Law")));
        assert!(law_table.is_some(), "expected to find a table with 'Law' header");
    }

    #[test]
    fn test_extract_code_blocks() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();

        let code_blocks: Vec<&CodeBlock> = framework
            .sections
            .iter()
            .flat_map(|s| &s.code_blocks)
            .collect();

        assert!(!code_blocks.is_empty(), "expected at least one code block");
    }

    #[test]
    fn test_component_extraction() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();

        assert!(!arch.components.is_empty());
        assert!(arch.total_estimated_loc > 0);

        // Should find key components
        let ids: Vec<&str> = arch.components.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&"knowledge-brain"), "missing knowledge-brain");
        assert!(ids.contains(&"crypto-foundation"), "missing crypto-foundation");
        assert!(ids.contains(&"agent-orchestration"), "missing agent-orchestration");
        assert!(ids.contains(&"p2p-mesh"), "missing p2p-mesh");
    }

    #[test]
    fn test_component_kinds() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();

        let kinds: HashSet<ComponentKind> = arch.components.iter().map(|c| c.kind).collect();
        assert!(kinds.contains(&ComponentKind::Module));
        assert!(kinds.contains(&ComponentKind::Security));
        assert!(kinds.contains(&ComponentKind::Agent));
        assert!(kinds.contains(&ComponentKind::Knowledge));
    }

    #[test]
    fn test_dag_build() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();
        let dag = ComponentDag::build(&arch).unwrap();

        assert!(!dag.is_empty());
        assert!(!dag.layers.is_empty());

        // First layer should have no dependencies (roots like crypto-foundation)
        for component_id in &dag.layers[0] {
            let node = dag.get(component_id).unwrap();
            assert!(
                node.depends_on.is_empty(),
                "root component {} has dependencies: {:?}",
                component_id,
                node.depends_on
            );
        }
    }

    #[test]
    fn test_dag_no_cycles() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();
        let dag = ComponentDag::build(&arch).unwrap();

        // topological_order should succeed (no cycles)
        let order = dag.topological_order();
        assert_eq!(order.len(), dag.len());
    }

    #[test]
    fn test_dag_dependency_ordering() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();
        let dag = ComponentDag::build(&arch).unwrap();

        let order = dag.topological_order();
        let position: HashMap<&str, usize> = order
            .iter()
            .enumerate()
            .map(|(i, id)| (id.as_str(), i))
            .collect();

        // For every node, all its deps must appear before it
        for node in dag.iter() {
            let node_pos = position[node.component_id.as_str()];
            for dep in &node.depends_on {
                let dep_pos = position[dep.as_str()];
                assert!(
                    dep_pos < node_pos,
                    "{} (pos {}) depends on {} (pos {}) but {} comes first",
                    node.component_id,
                    node_pos,
                    dep,
                    dep_pos,
                    node.component_id
                );
            }
        }
    }

    #[test]
    fn test_offline_enrichment() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();
        let mut dag = ComponentDag::build(&arch).unwrap();
        let enrichments = KnowledgeEnricher::enrich_offline(&mut dag, &arch);

        assert_eq!(enrichments.len(), arch.components.len());
        for enrichment in &enrichments {
            assert!(!enrichment.best_practices.is_empty());
        }
    }

    #[test]
    fn test_plan_generation() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();
        let mut dag = ComponentDag::build(&arch).unwrap();
        let enrichments = KnowledgeEnricher::enrich_offline(&mut dag, &arch);
        let plan = PlanGenerator::generate(&dag, &arch, enrichments).unwrap();

        assert!(!plan.streams.is_empty());
        assert!(plan.total_estimated_loc > 0);
        assert!(plan.total_estimated_seconds > 0);
        assert_eq!(plan.total_components, dag.len());
        assert!(!plan.approved);
    }

    #[test]
    fn test_plan_to_task_graph() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();
        let mut dag = ComponentDag::build(&arch).unwrap();
        let enrichments = KnowledgeEnricher::enrich_offline(&mut dag, &arch);
        let plan = PlanGenerator::generate(&dag, &arch, enrichments).unwrap();

        let graph = PlanGenerator::to_task_graph(&plan, &dag).unwrap();

        assert_eq!(graph.len(), dag.len());
        assert!(!graph.is_complete()); // all tasks should be pending
        assert!(!graph.ready_tasks().is_empty()); // root tasks should be ready
    }

    #[test]
    fn test_plan_display_summary() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();
        let mut dag = ComponentDag::build(&arch).unwrap();
        let enrichments = KnowledgeEnricher::enrich_offline(&mut dag, &arch);
        let plan = PlanGenerator::generate(&dag, &arch, enrichments).unwrap();

        let summary = plan.display_summary();
        assert!(summary.contains("EXECUTION PLAN"));
        assert!(summary.contains("Stream"));
        assert!(summary.contains("Components:"));
    }

    #[test]
    fn test_plan_approval() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();
        let mut dag = ComponentDag::build(&arch).unwrap();
        let enrichments = KnowledgeEnricher::enrich_offline(&mut dag, &arch);
        let mut plan = PlanGenerator::generate(&dag, &arch, enrichments).unwrap();

        assert!(!plan.approved);
        plan.approve();
        assert!(plan.approved);
    }

    #[test]
    fn test_sync_pipeline() {
        // Write a temp file
        let dir = std::env::temp_dir().join("phantom_test_ingestion");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_framework.md");
        std::fs::write(&path, SAMPLE_FRAMEWORK).unwrap();

        let result = IngestionPipeline::run_sync(&path).unwrap();

        assert!(!result.framework.sections.is_empty());
        assert!(!result.architecture.components.is_empty());
        assert!(!result.dag.is_empty());
        assert!(!result.plan.streams.is_empty());

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3661), "1h 1m");
        assert_eq!(format_duration(7200), "2h 0m");
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn test_empty_framework() {
        let result = MarkdownParser::parse("", "test.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_no_headings() {
        let result = MarkdownParser::parse("Just some text with no headings.", "test.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_table_parsing_edge_cases() {
        let body = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n\nSome text.\n\n| X |\n|---|\n| y |";
        let tables = MarkdownParser::extract_tables(body);
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].headers, vec!["A", "B"]);
        assert_eq!(tables[0].rows.len(), 2);
        assert_eq!(tables[1].headers, vec!["X"]);
    }

    #[test]
    fn test_agent_components_present() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();

        let agent_components: Vec<&Component> = arch
            .components
            .iter()
            .filter(|c| c.kind == ComponentKind::Agent)
            .collect();

        assert_eq!(agent_components.len(), 8, "should have all 8 agents");

        let roles: HashSet<&str> = agent_components.iter().map(|c| c.agent_role.as_str()).collect();
        assert!(roles.contains("cto"));
        assert!(roles.contains("backend"));
        assert!(roles.contains("frontend"));
        assert!(roles.contains("devops"));
        assert!(roles.contains("qa"));
        assert!(roles.contains("security"));
        assert!(roles.contains("monitor"));
        assert!(roles.contains("architect"));
    }

    #[test]
    fn test_constraint_extraction() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();

        assert!(
            !arch.constraints.is_empty(),
            "should extract constraints from core laws table"
        );
    }

    #[test]
    fn test_technology_extraction() {
        let framework = MarkdownParser::parse(SAMPLE_FRAMEWORK, "test.md").unwrap();
        let arch = ComponentExtractor::extract(&framework).unwrap();

        let tech_names: Vec<&str> = arch.technologies.iter().map(|t| t.name.as_str()).collect();
        assert!(
            tech_names.iter().any(|t| t.contains("Rust")),
            "should extract Rust from tech spec table, got: {:?}",
            tech_names
        );
    }
}
