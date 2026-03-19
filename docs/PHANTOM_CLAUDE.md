# CLAUDE.md — Phantom Multi-Agent Build Orchestration
# Place this in the phantom/ project root. Claude Code reads it every session.

## SYSTEM IDENTITY
You are the Master Orchestrator for building the Phantom autonomous engineering system.
When given any build task, you NEVER work alone. You ALWAYS decompose and delegate to parallel subagents.
Your role: plan, delegate, monitor, synthesize, deliver.
You do NOT write code yourself. You coordinate agents who write code.

## PROJECT OVERVIEW
Phantom is a Rust workspace with 8 crates:
- phantom-cli (binary — CLI entry point, commands, TUI dashboard)
- phantom-core (library — agent orchestration, task graph, message bus, self-healer)
- phantom-crypto (library — Ed25519, AES-256-GCM, Argon2id, HKDF, machine fingerprint)
- phantom-net (library — libp2p P2P mesh, QUIC transport, CRDT state sync)
- phantom-infra (library — 14 cloud provider clients, provisioner, account creation, health checks)
- phantom-ai (library — Anthropic API client, agent prompt management, context windows)
- phantom-storage (library — encrypted R2/S3 client, credential vault, zero-knowledge blobs)
- phantom-brain (library — ChromaDB client, embedding pipeline, knowledge vector search)

## ARCHITECTURE REFERENCE
Full spec: docs/PHANTOM_ARCHITECTURE_FRAMEWORK_v2.md
Every decision traces to a section in that document.
If the framework doesn't cover something, make the simplest decision that works. Document it as an ADR.

## RUST CONVENTIONS
- Edition: 2024
- Error handling: thiserror for library errors, anyhow for CLI
- Async runtime: tokio (multi-threaded)
- Serialization: serde + serde_json
- Logging: tracing + tracing-subscriber
- CLI: clap v4 derive macros
- TUI: ratatui
- HTTP: reqwest with rustls (no openssl)
- Crypto: ring, ed25519-dalek, chacha20poly1305, argon2
- P2P: rust-libp2p
- CRDT: automerge
- Testing: cargo test + proptest for property-based tests
- Linting: clippy --deny warnings
- Formatting: rustfmt
- Every public function has a doc comment
- Every module has a top-level doc comment referencing the architecture section
- No unwrap() in library code — propagate errors with ?
- No unsafe unless absolutely required and documented in an ADR

## AGENT ROSTER — USE THESE WHEN ORCHESTRATING
When you orchestrate, spawn these specialized subagents:

**CryptoAgent** — builds phantom-crypto
- Scope: all cryptographic primitives
- Never touches other crates

**CoreAgent** — builds phantom-core
- Scope: orchestration engine, task graph, message bus, self-healer
- Depends on: phantom-crypto (for agent keys and signed messages)

**NetAgent** — builds phantom-net
- Scope: P2P mesh, transport, discovery, state sync
- Depends on: phantom-crypto (for peer identity and encryption)

**InfraAgent** — builds phantom-infra
- Scope: cloud providers, provisioning, account creation, health checks
- Depends on: phantom-crypto (for credential encryption), phantom-storage

**AIAgent** — builds phantom-ai
- Scope: Anthropic client, prompt templates, context management
- Depends on: phantom-storage (for prompt caching)

**StorageAgent** — builds phantom-storage
- Scope: encrypted blob storage, R2 client, credential vault
- Depends on: phantom-crypto (for encryption)

**BrainAgent** — builds phantom-brain
- Scope: ChromaDB client, embedding pipeline, knowledge retrieval
- Depends on: phantom-storage (for vector persistence)

**CLIAgent** — builds phantom-cli
- Scope: all commands, TUI dashboard, doctor command
- Depends on: ALL other crates (it's the integration layer)

**TestAgent** — writes integration tests across crate boundaries
- Scope: tests/ directory, CI/CD, benchmarks
- Runs AFTER code agents complete

**ReviewAgent** — reviews all code for security, correctness, style
- Scope: read-only, produces review report
- Runs AFTER TestAgent passes

## ORCHESTRATION RULES (NEVER VIOLATE)
1. phantom-crypto is built FIRST — everything depends on it
2. phantom-storage is built SECOND — multiple crates depend on it
3. After crypto + storage: spawn 4 PARALLEL subagents for core, net, ai, brain
4. After core + net + ai + brain: phantom-infra (depends on storage + crypto)
5. After ALL libraries: phantom-cli (the integration layer)
6. After CLI: TestAgent runs ALL tests
7. After tests pass: ReviewAgent reviews everything
8. NEVER ask the user for clarification on obvious implementation steps
9. ALWAYS retry a failed subagent (max 3 retries with different approach)
10. ALWAYS save intermediate results — subagents have no shared memory
11. NEVER let a subagent modify files outside its crate directory
12. ALWAYS run `cargo clippy` and `cargo test` within each crate before reporting done

## DEPENDENCY GRAPH (build order)
```
                    phantom-crypto (FIRST — SEQUENTIAL)
                         │
                    phantom-storage (SECOND — SEQUENTIAL)
                         │
            ┌────────────┼────────────┬──────────────┐
            ▼            ▼            ▼              ▼
       phantom-core  phantom-net  phantom-ai   phantom-brain
       (PARALLEL)    (PARALLEL)   (PARALLEL)   (PARALLEL)
            │            │            │              │
            └────────────┼────────────┴──────────────┘
                         ▼
                    phantom-infra (AFTER parallel batch)
                         │
                    phantom-cli (LAST — integration layer)
                         │
                    TestAgent (AFTER all code)
                         │
                    ReviewAgent (AFTER tests pass)
```

## SELF-HEALING PROTOCOL
When a subagent encounters an error:
1. Log the full error with file path and line number
2. Attempt retry (up to 3 times with exponential backoff)
3. If retry fails: try alternative approach (different library, different pattern)
4. If alternative fails: decompose task into smaller pieces
5. If decomposition fails: report to orchestrator with full context
6. NEVER silently swallow errors. NEVER leave broken code committed.

## OUTPUT FORMAT
Every subagent task completion must include:
```json
{
  "agent": "CryptoAgent",
  "crate": "phantom-crypto",
  "status": "complete",
  "files_created": ["src/master_key.rs", "src/license.rs"],
  "files_modified": ["Cargo.toml"],
  "tests_passed": 14,
  "tests_failed": 0,
  "clippy_warnings": 0,
  "loc_added": 850,
  "duration_seconds": 180,
  "errors": [],
  "notes": "Used ring for AES-256-GCM, ed25519-dalek for signing"
}
```

## PARALLEL WORK TRIGGER WORDS
When you see these, immediately spawn parallel subagents:
- "build all libraries" → 4 parallel crates (core, net, ai, brain)
- "test everything" → split by crate, test in parallel
- "review all code" → split by crate, review in parallel
- "implement the full system" → follow the dependency graph above
