# PHANTOM — Multi-Agent Build Execution Plan
## Exact Claude Code Prompts in Sequence
### Reference: PHANTOM-EXEC-001-2026-03-19

---

## Pre-Flight Setup (you do this manually, once)

```bash
# 1. Create project directory
mkdir -p ~/phantom && cd ~/phantom

# 2. Copy the CLAUDE.md into project root
cp /path/to/CLAUDE.md ./CLAUDE.md

# 3. Copy the Architecture Framework
cp /path/to/PHANTOM_ARCHITECTURE_FRAMEWORK_v2.md ./docs/PHANTOM_ARCHITECTURE_FRAMEWORK_v2.md

# 4. Start Claude Code
claude
```

---

## PROMPT 1 — Scaffold (Sequential, ~5 min)

```
Scaffold the complete Phantom Rust workspace. Create:

1. Root Cargo.toml as a workspace with members:
   phantom-cli, phantom-core, phantom-crypto, phantom-net,
   phantom-infra, phantom-ai, phantom-storage, phantom-brain

2. rust-toolchain.toml targeting stable channel, edition 2024

3. Each crate with its own Cargo.toml containing:
   - Appropriate dependencies (see CLAUDE.md for which libraries each crate uses)
   - phantom-crypto and phantom-storage as workspace dependencies where needed
   - [dev-dependencies] for tests

4. Each crate with src/lib.rs (libraries) or src/main.rs (phantom-cli) 
   containing module declarations and top-level doc comments referencing 
   the architecture section

5. .github/workflows/ci.yml with:
   - cargo fmt --check
   - cargo clippy --deny warnings
   - cargo test --workspace
   - Runs on push to main and PRs

6. Root .gitignore for Rust projects

7. docs/ directory with the architecture framework

8. README.md explaining the project

Verify the workspace compiles: cargo check --workspace
```

---

## PROMPT 2 — phantom-crypto (Sequential, ~20 min, everything depends on this)

```
Build phantom-crypto completely. This is the foundation — every other crate depends on it.

Implement these modules in phantom-crypto/src/:

1. master_key.rs
   - MasterKey struct that derives a 256-bit key from a passphrase via Argon2id
   - Parameters: memory=256MB, iterations=4, parallelism=8
   - Implements Zeroize + Drop (key zeroed when struct drops)
   - derive_subkey(context: &str) -> [u8; 32] via HKDF-SHA512

2. license.rs
   - LicensePayload struct (version, machine_id, issued_at, expires_at, capabilities, tier)
   - sign_license(payload, signing_key) -> LicenseToken using Ed25519
   - verify_license(token, public_key) -> Result<LicensePayload>
   - LicenseToken is base62-encoded: PH1-<payload>-<signature>

3. fingerprint.rs
   - generate_fingerprint() -> [u8; 32]
   - HMAC-SHA256 of: MAC address + CPU info + disk UUID + OS UUID
   - Platform-specific: use sysinfo crate for cross-platform system info
   - Falls back gracefully if any component unavailable

4. session.rs
   - SessionKey struct derived from license token + current timestamp via HKDF
   - Ephemeral — never serialized, implements Zeroize + Drop
   - derive_agent_key(agent_name: &str, task_id: &str) -> AgentKey
   - AgentKey has scoped permissions bitmap

5. encryption.rs
   - encrypt(plaintext, key, aad) -> EncryptedBlob using AES-256-GCM
   - decrypt(blob, key, aad) -> plaintext
   - EncryptedBlob = nonce || ciphertext || tag (serializable)
   - Also: encrypt_stream / decrypt_stream for large data

6. lib.rs — re-export all public types

Write comprehensive tests for every function.
Use test vectors from RFC 8032 (Ed25519) and RFC 7714 (AES-GCM) where applicable.
Run cargo test -p phantom-crypto and cargo clippy -p phantom-crypto before reporting done.
```

---

## PROMPT 3 — phantom-storage (Sequential, ~15 min, depends on crypto)

```
Build phantom-storage completely. Depends on phantom-crypto for encryption.

Implement in phantom-storage/src/:

1. encrypted_store.rs
   - EncryptedStore trait with: put(key, plaintext), get(key) -> plaintext, delete(key), list(prefix)
   - All data encrypted via phantom-crypto before storage
   - Associated data (AAD) = key path (prevents blob swapping)

2. r2.rs
   - R2Client implementing EncryptedStore for Cloudflare R2
   - Uses reqwest with S3-compatible API (R2 is S3-compatible)
   - Endpoints, access key, secret from config
   - Retry with exponential backoff (3 attempts)
   - Streaming upload/download for large blobs

3. vault.rs
   - CredentialVault — specialized encrypted store for API keys and secrets
   - store_credential(service, key, value) — encrypts and stores
   - get_credential(service, key) -> String
   - rotate_credential(service, key, new_value) — atomic update
   - list_services() -> Vec<String>
   - All operations logged to audit trail

4. lib.rs — re-export, define EncryptedStore trait

Tests for every function. Mock the R2 client with an in-memory backend for tests.
Run cargo test -p phantom-storage and cargo clippy -p phantom-storage.
```

---

## PROMPT 4 — PARALLEL BATCH: 4 crates simultaneously (~30 min)

```
Build 4 crates simultaneously using parallel subagents. Each subagent works on ONE crate.
DO NOT let subagents conflict on files — each stays in its own crate directory.

=== SUBAGENT 1: phantom-core ===
Build phantom-core/src/:

1. task_graph.rs
   - TaskGraph: directed acyclic graph of build tasks
   - Task struct: id, name, agent, dependencies[], status, priority
   - add_task(), add_dependency(), topological_sort()
   - get_ready_tasks() -> tasks with all dependencies complete
   - mark_complete(task_id), mark_failed(task_id, error)
   - Cycle detection on add_dependency

2. agent_manager.rs
   - AgentManager: spawn, track, and kill agent processes
   - Agent struct: id, name, role, status, pid, started_at, task_id
   - spawn_agent(role, task) -> AgentHandle
   - kill_agent(id), get_status(id), list_agents()
   - AgentHandle has send_message() and wait_completion()

3. message_bus.rs
   - MessageBus: in-process pub/sub for inter-agent messaging
   - publish(topic, message), subscribe(topic) -> Receiver
   - Message struct: id, from, to, type, payload, timestamp, signature
   - Async channels (tokio::sync::broadcast)

4. self_healer.rs
   - SelfHealer: 5-layer recovery engine
   - Layer 1: RetryWithBackoff (1s, 2s, 4s, 8s, 16s)
   - Layer 2: AlternativeApproach (try different strategy)
   - Layer 3: Decompose (split task into subtasks)
   - Layer 4: EscalateInternally (ask another agent)
   - Layer 5: PauseAndAlert (save state, notify owner)
   - Each layer returns HealResult::Healed | HealResult::Escalate

5. architecture_parser.rs
   - parse_markdown(content: &str) -> TaskGraph
   - Extract: components, technologies, patterns, constraints
   - Build dependency graph from extracted entities
   - Estimate LOC and time per task

Tests for each module. Run cargo test -p phantom-core && cargo clippy -p phantom-core.


=== SUBAGENT 2: phantom-net ===
Build phantom-net/src/:

1. p2p.rs
   - PhantomMesh: libp2p Swarm configuration
   - Ed25519 peer identity from phantom-crypto
   - Noise protocol for encryption
   - QUIC transport (quic feature of libp2p)
   - Identify protocol for peer info exchange

2. discovery.rs
   - Kademlia DHT for peer discovery
   - mDNS for local network discovery
   - Bootstrap from hardcoded relay nodes (Cloudflare Workers)
   - Peer cache with TTL

3. sync.rs
   - CRDTSync: Automerge-based state replication
   - sync_state(peer_id, local_doc, remote_doc) -> merged_doc
   - Conflict resolution: last-writer-wins for non-CRDT data
   - Event emitter for state change notifications

4. relay.rs
   - CloudflareRelay: fallback transport via Cloudflare Workers
   - HTTP/WebSocket relay for peers behind restrictive NATs
   - Encrypted tunneling via phantom-crypto session keys

Tests with mock network. Run cargo test -p phantom-net && cargo clippy -p phantom-net.


=== SUBAGENT 3: phantom-ai ===
Build phantom-ai/src/:

1. client.rs
   - AnthropicClient: HTTP client for Anthropic Messages API
   - send_message(model, system, messages, tools) -> Response
   - Streaming support (SSE parsing)
   - Retry with exponential backoff
   - Token counting and budget enforcement
   - Rate limit handling (429 detection + backoff)

2. prompts.rs
   - AgentPrompt struct: system_prompt, role, constraints, knowledge_refs
   - Predefined prompts for each agent role (CTO, Backend, Frontend, etc.)
   - Template rendering with variable injection
   - Context window budgeting (reserve space for knowledge chunks)

3. context.rs
   - ContextManager: manages what goes into each agent's context window
   - add_knowledge(chunks: Vec<String>) — from ChromaDB results
   - add_task(description: &str)
   - add_history(messages: Vec<Message>)
   - truncate_to_fit(max_tokens: usize) — intelligent truncation
   - Priority: task > knowledge > history

4. tools.rs
   - ToolDefinition struct matching Anthropic tool-use schema
   - Predefined tools: file_write, file_read, shell_exec, http_request
   - Tool result parsing
   - Permission scoping per agent role

Tests with mock HTTP server. Run cargo test -p phantom-ai && cargo clippy -p phantom-ai.


=== SUBAGENT 4: phantom-brain ===
Build phantom-brain/src/:

1. embeddings.rs
   - EmbeddingClient: HTTP client for sentence-transformers server
   - embed(texts: Vec<String>) -> Vec<Vec<f32>>
   - Batch embedding (up to 32 texts per call)
   - Local fallback: call Python subprocess if HTTP server unavailable

2. knowledge.rs
   - KnowledgeStore: ChromaDB client for knowledge retrieval
   - ingest_markdown(filename, content) — chunk by heading, embed, store
   - query(text, top_k, agent_tags) -> Vec<KnowledgeChunk>
   - KnowledgeChunk: text, source_file, section_heading, score
   - update_file(filename, new_content) — re-embed changed chunks

3. chunker.rs
   - chunk_markdown(content) -> Vec<Chunk>
   - Split by headings (##, ###)
   - Each chunk ~500 tokens with overlap
   - Preserve code blocks intact
   - Tag chunks with metadata: filename, heading, line_range

4. lib.rs — re-export, KnowledgeBrain facade that combines all modules

Tests with in-memory vector store mock. 
Run cargo test -p phantom-brain && cargo clippy -p phantom-brain.


=== END PARALLEL BATCH ===
After all 4 subagents report done, verify: cargo check --workspace
```

---

## PROMPT 5 — phantom-infra (Sequential, ~25 min, depends on crypto + storage)

```
Build phantom-infra completely. This is the self-provisioning infrastructure layer.

Implement in phantom-infra/src/:

1. providers/mod.rs — CloudProvider trait:
   - create_account(email, name) -> Result<AccountCredentials>
   - provision_server(spec) -> Result<ServerInfo>
   - health_check(server) -> Result<HealthStatus>
   - destroy_server(server_id) -> Result<()>
   - get_free_tier_limits() -> FreeTierLimits

2. providers/oracle.rs — Oracle Cloud free tier (2 OCPU, 12GB, 200GB)
3. providers/gcp.rs — Google Cloud e2-micro free tier
4. providers/cloudflare.rs — Workers, R2, DNS via Wrangler API
5. providers/fly.rs — Fly.io free tier (3 shared VMs)
6. providers/vercel.rs — Vercel serverless deployment
7. providers/supabase.rs — Supabase PostgreSQL + Auth
8. providers/upstash.rs — Upstash Redis
9. providers/github.rs — GitHub repos, Actions, secrets, deploy keys
10. providers/neon.rs — Neon serverless Postgres
11. providers/railway.rs — Railway deployment

Each provider implements CloudProvider trait.
Use reqwest for API calls. All credentials via phantom-storage vault.

12. provisioner.rs
    - InfraProvisioner: orchestrates multi-provider setup
    - discover_available_providers() -> ranked list by free tier value
    - provision_primary() + provision_replica() + provision_edge()
    - Minimum 3 providers for redundancy
    - bind_server(server, instance_token) — cryptographic binding

13. health.rs
    - InfraHealthMonitor: continuous health checking
    - check_all_servers() — HTTP + TCP health probes
    - detect_failure(server) — missed heartbeat threshold
    - trigger_failover(failed_server) — promote replica, provision replacement

14. accounts.rs
    - AccountCreator: autonomous account creation
    - create_all_required_accounts(email, services[]) -> Results
    - Uses CLI tools where available (gh, vercel, supabase, wrangler)
    - Falls back to API signup
    - Stores all credentials in vault immediately

15. installer.rs
    - DependencyInstaller: autonomous macOS dependency installation
    - detect_missing() -> Vec<Dependency>
    - install_all(missing) — Homebrew, nvm, pyenv, Rust, Docker, CLIs
    - doctor() -> SystemReport — verify all 18+ dependencies

Tests with mock providers. Run cargo test -p phantom-infra && cargo clippy -p phantom-infra.
```

---

## PROMPT 6 — phantom-cli (Sequential, ~25 min, depends on ALL libraries)

```
Build phantom-cli — the integration layer and user-facing CLI.

Implement in phantom-cli/src/:

1. main.rs
   - clap App with subcommands: activate, build, status, doctor, agents, 
     logs, infra, brain, cost, pause, resume, destroy, master
   - License verification at startup (before any command except --help and --version)
   - Graceful error handling with anyhow

2. commands/activate.rs
   - phantom activate --key <LICENSE_KEY>
   - Verify license signature (phantom-crypto)
   - Generate machine fingerprint
   - Run DependencyInstaller (phantom-infra)
   - Run AccountCreator (phantom-infra)
   - Provision infrastructure (phantom-infra)
   - Initialize P2P mesh (phantom-net)
   - Ingest knowledge files into ChromaDB (phantom-brain)
   - Display activation summary

3. commands/build.rs
   - phantom build --framework <file>
   - Parse architecture framework (phantom-core)
   - Enrich with knowledge brain queries (phantom-brain)
   - Generate task graph and build plan
   - Present plan for owner approval (interactive prompt)
   - Spawn agents per task graph (phantom-core + phantom-ai)
   - Monitor progress, retry failures
   - Deploy when complete (phantom-infra)
   - Deliver final report

4. commands/status.rs
   - phantom status [--live]
   - Live TUI dashboard using ratatui
   - Shows: agents, tasks, infrastructure, progress, ETA
   - Auto-refreshes every second
   - Keyboard shortcuts: q=quit, p=pause, r=resume, l=logs

5. commands/doctor.rs
   - phantom doctor
   - Check all 18+ dependencies
   - Check license validity
   - Check infrastructure health
   - Check P2P mesh connectivity
   - Check knowledge brain status
   - Color-coded output: green=OK, yellow=warning, red=missing

6. commands/master.rs
   - All master key operations (issue, revoke, list, kill, destroy, rotate, audit)
   - Passphrase prompt (rpassword crate — no echo)
   - TOTP verification for destructive operations
   - Confirmation prompts for irreversible actions

7. commands/brain.rs
   - phantom brain search <query> — direct knowledge brain query
   - phantom brain update --file <md_file> — update knowledge corpus
   - phantom brain status — show corpus stats

8. ui/dashboard.rs
   - ratatui-based live dashboard
   - Agent status table, progress bars, log stream
   - Infrastructure health panel
   - Responsive to terminal size

Run cargo build -p phantom-cli (full binary). Test all commands.
Run cargo clippy -p phantom-cli.
```

---

## PROMPT 7 — PARALLEL: Integration Tests + CI/CD (~15 min)

```
Using 3 parallel subagents, complete the test suite and CI/CD:

=== SUBAGENT 1: Integration Tests ===
Create tests/ directory at workspace root with:

1. tests/crypto_integration.rs
   - Full key hierarchy: master → license → session → agent key
   - Round-trip: generate license → sign → verify → extract payload
   - Encrypt → store → retrieve → decrypt

2. tests/build_pipeline.rs
   - Parse a sample architecture.md → generate task graph
   - Verify task dependency ordering
   - Verify parallel task identification

3. tests/knowledge_brain.rs
   - Ingest sample MD file → query → verify relevant results returned
   - Update file → re-query → verify updated results

Run: cargo test --workspace


=== SUBAGENT 2: CI/CD Pipeline ===
Update .github/workflows/ci.yml:
- Matrix build: ubuntu-latest + macos-latest
- Cache: cargo registry + target dir
- Jobs: fmt, clippy, test, build-release
- Security: cargo audit
- Release workflow: build signed binaries for darwin-arm64, darwin-x64, linux-x64

Create .github/workflows/release.yml:
- Triggered by git tag v*
- Cross-compile with cargo-zigbuild or cross
- Sign binaries with Ed25519
- Create GitHub release with artifacts


=== SUBAGENT 3: Documentation ===
Create/update:
- README.md — project overview, installation, quick start
- docs/SECURITY.md — threat model, key hierarchy, anti-hack measures
- docs/AGENTS.md — agent roles, knowledge mapping, communication protocol
- docs/OPERATIONS.md — master key setup, license management, infrastructure ops
- CONTRIBUTING.md — dev setup, code conventions, PR process

All 3 subagents work in parallel. Report when all complete.
```

---

## PROMPT 8 — Final Review & Polish (Sequential, ~10 min)

```
Run a final review of the entire Phantom codebase using 2 parallel subagents:

=== SUBAGENT 1: Security Review ===
Review ALL Rust code for:
- Any use of unwrap() in library code (must be ? or expect with message)
- Any hardcoded secrets or credentials
- Any unsafe blocks (must be documented if present)
- Proper Zeroize implementation on all key types
- No key material in Debug/Display trait implementations
- TLS certificate pinning in all HTTP clients
- Rate limiting on all external API calls

Report: list every issue with file:line and severity.


=== SUBAGENT 2: Quality Review ===
Review ALL Rust code for:
- Missing doc comments on public items
- Missing error handling (look for todo!, unimplemented!, panic!)
- Clippy warnings (run cargo clippy --workspace --deny warnings)
- Test coverage gaps (any module without corresponding test)
- Architecture doc references in module comments
- Consistent naming conventions

Report: list every issue with file:line and severity.

After both subagents report: fix ALL critical and high severity issues.
Then run the full test suite one final time: cargo test --workspace
Report the final status.
```

---

## Build Time Estimates

| Phase | Prompt | Mode | Est. Time | Crates |
|-------|--------|------|-----------|--------|
| 1 | Scaffold | Sequential | 5 min | workspace |
| 2 | Crypto | Sequential | 20 min | phantom-crypto |
| 3 | Storage | Sequential | 15 min | phantom-storage |
| 4 | Core + Net + AI + Brain | **4 PARALLEL** | 30 min | 4 crates |
| 5 | Infra | Sequential | 25 min | phantom-infra |
| 6 | CLI | Sequential | 25 min | phantom-cli |
| 7 | Tests + CI + Docs | **3 PARALLEL** | 15 min | cross-crate |
| 8 | Review + Polish | **2 PARALLEL** | 10 min | workspace |
| **TOTAL** | | | **~2.5 hours** | **8 crates** |

**Without multi-agent:** ~36 hours (sequential)
**With multi-agent:** ~2.5 hours (parallel subagents)
**Speedup:** ~14x

---

## Execution Map

```
TIME  ─────────────────────────────────────────────────────────────────▶

0:00  ┌─────────────────┐
      │  P1: Scaffold   │ (sequential)
0:05  └────────┬────────┘
              │
      ┌───────▼────────┐
      │  P2: Crypto    │ (sequential — everything depends on this)
0:25  └────────┬────────┘
              │
      ┌───────▼────────┐
      │  P3: Storage   │ (sequential — multiple crates depend on this)
0:40  └────────┬────────┘
              │
      ┌───────▼────────┬───────────────┬───────────────┬──────────────┐
      │  P4a: Core     │  P4b: Net     │  P4c: AI      │  P4d: Brain  │
      │  (parallel)    │  (parallel)   │  (parallel)   │  (parallel)  │
1:10  └───────┬────────┴───────┬───────┴───────┬───────┴──────┬───────┘
              │                │               │              │
              └────────────────┼───────────────┴──────────────┘
                               │
      ┌────────────────────────▼───────────────────────────┐
      │  P5: Infra                                          │ (sequential)
1:35  └────────────────────────┬───────────────────────────┘
                               │
      ┌────────────────────────▼───────────────────────────┐
      │  P6: CLI                                            │ (sequential)
2:00  └────────────────────────┬───────────────────────────┘
                               │
      ┌────────────────────────▼──────┬──────────────┬──────────────┐
      │  P7a: Integration Tests       │  P7b: CI/CD  │  P7c: Docs   │
      │  (parallel)                   │  (parallel)  │  (parallel)  │
2:15  └───────────────┬───────────────┴──────┬───────┴──────┬───────┘
                      │                      │              │
                      └──────────────────────┼──────────────┘
                                             │
      ┌──────────────────────────────────────▼──────────────────────┐
      │  P8a: Security Review (parallel) │ P8b: Quality Review     │
2:25  └──────────────────────────────────┴─────────────────────────┘
      │
2:30  ✅ PHANTOM BUILD COMPLETE
```

---

## Post-Build Verification

After all 8 prompts complete, run manually:

```bash
# Full workspace build
cargo build --workspace --release

# Full test suite
cargo test --workspace

# Clippy clean
cargo clippy --workspace --deny warnings

# Binary size check
ls -lh target/release/phantom-cli

# Smoke test
./target/release/phantom-cli --version
./target/release/phantom-cli doctor --dry-run
```

Expected outcomes:
- Binary size: ~15-25MB (static, no runtime deps)
- Test count: ~80-120 tests
- Clippy warnings: 0
- Build time (release): ~2-3 minutes
