<div align="center">

# PHANTOM

### The Autonomous AI Engineering Team That Lives in Your Terminal

**One command. One architecture document. Full-stack production software — built, tested, deployed.**

[![CI](https://github.com/benchbrex-USA/BenchBrex-PHANTOM/actions/workflows/ci.yml/badge.svg)](https://github.com/benchbrex-USA/BenchBrex-PHANTOM/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-100%25-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-proprietary-red)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS-blue)](https://www.apple.com/macos/)

</div>

---

## What is Phantom?

Phantom is a **terminal-native autonomous AI engineering system** that operates as a complete software development team from your macOS terminal.

You give it one Architecture Framework document. Phantom reads it, then:

- **Installs** every dependency your project needs (Homebrew, Node.js, Python, Docker, PostgreSQL, Redis — everything)
- **Creates** accounts on GitHub, Vercel, Supabase, Cloudflare, and 10+ other services
- **Designs** your system architecture, database schema, and API contracts
- **Writes** production-quality backend (FastAPI), frontend (Next.js), and infrastructure code
- **Tests** everything (unit, integration, E2E) with 80%+ coverage
- **Audits** security (OWASP Top 10, dependency vulnerabilities, auth flows)
- **Deploys** to production on free-tier cloud infrastructure ($0/month)
- **Monitors** and self-heals in production — 24/7

All of this happens autonomously. You approve the plan, then sleep. You wake up. Your software is live.

---

## How It Works — The 60-Second Version

```
┌─────────────────────────────────────────────────────────────────────────┐
│  YOU                                                                     │
│  $ phantom build --framework ./my-architecture.md                       │
└───────────────────────────────────┬─────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  PHANTOM CTO AGENT                                                       │
│  Reads your architecture. Decomposes into tasks. Delegates to agents.   │
└────────────┬──────────┬──────────┬──────────┬──────────┬────────────────┘
             │          │          │          │          │
             ▼          ▼          ▼          ▼          ▼
         Architect   Backend   Frontend    DevOps      QA
          Agent       Agent     Agent      Agent     Agent
             │          │          │          │          │
             └──────────┴──────────┴──────────┴──────────┘
                                    │
                                    ▼
                          Security Agent (audit)
                                    │
                                    ▼
                          Monitor Agent (24/7)
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │  YOUR SOFTWARE IS LIVE         │
                    │  https://your-app.vercel.app   │
                    └───────────────────────────────┘
```

---

## Table of Contents

- [Quick Start](#quick-start)
- [The Complete First-Time Workflow](#the-complete-first-time-workflow)
- [What Phantom Does (Feature Map)](#what-phantom-does)
- [Architecture Overview](#architecture-overview)
- [The 8 Agents](#the-8-agents)
- [Security Model](#security-model)
- [CLI Reference](#cli-reference)
- [How the Knowledge Brain Works](#how-the-knowledge-brain-works)
- [Self-Healing System](#self-healing-system)
- [Infrastructure — $0/month](#infrastructure--0month)
- [Peer-to-Peer Mesh](#peer-to-peer-mesh)
- [Project Structure](#project-structure)
- [Development](#development)
- [FAQ](#faq)

---

## Quick Start

```bash
# 1. Download Phantom (macOS only)
curl -fsSL https://phantom.benchbrex.com/install.sh | sh

# 2. Activate with your license key
phantom activate --key PH1-your-license-key-here

# 3. Give it an architecture document and watch it build
phantom build --framework ./my-architecture.md
```

That's it. Phantom handles everything else.

---

## The Complete First-Time Workflow

If you've never used Phantom before, this section walks you through every step from zero to a live production application.

### Step 1 — Get Your License Key

Phantom requires a license key to run. Each key is cryptographically bound to your machine and cannot be transferred.

```
License format: PH1-<payload>-<signature>
```

Contact the owner (Parth Patel / Benchbrex) to receive a license key.

### Step 2 — Install Phantom

```bash
curl -fsSL https://phantom.benchbrex.com/install.sh | sh
```

This downloads a single static binary (~15MB) to `/usr/local/bin/phantom` and verifies its Ed25519 signature. No runtime dependencies. No npm. No pip. Just one binary.

### Step 3 — Activate

```bash
phantom activate --key PH1-xxxxx-xxxxx
```

**What happens when you activate:**

```
Step 1/8  Verifying license signature ............... ✓
Step 2/8  Generating machine fingerprint ............ ✓
Step 3/8  Binding license to this machine ........... ✓
Step 4/8  Installing dependencies ................... 
          → Xcode CLI Tools: ✓ (already installed)
          → Homebrew: ✓ (already installed)
          → Node.js 20 (via nvm): installing... ✓
          → Python 3.12 (via pyenv): installing... ✓
          → Docker Desktop: ✓ (already installed)
          → PostgreSQL client: installing... ✓
          → Redis client: installing... ✓
          → GitHub CLI: installing... ✓
          → Vercel CLI: installing... ✓
          → Supabase CLI: installing... ✓
          → Cloudflare Wrangler: installing... ✓
Step 5/8  Creating service accounts .................
          → GitHub: [open browser for OAuth] ✓
          → Vercel: [open browser for OAuth] ✓
          → Supabase: [open browser for OAuth] ✓
          → Cloudflare: [open browser for OAuth] ✓
          → Upstash Redis: ✓ (API signup)
Step 6/8  Provisioning infrastructure ...............
          → Oracle Cloud (Mumbai): provisioning free-tier VM... ✓
          → Cloudflare Workers: configuring edge relay... ✓
          → Upstash Redis: creating database... ✓
Step 7/8  Initializing P2P mesh ..................... ✓ (3 nodes)
Step 8/8  Loading Knowledge Brain ................... ✓ (500 chunks indexed)

╔════════════════════════════════════════════════════════════════╗
║  PHANTOM ACTIVATED                                             ║
║                                                                ║
║  Instance:    ph-inst-a7f2b9c1                                ║
║  Servers:     3 bound (Oracle, Cloudflare, Upstash)           ║
║  P2P Mesh:    3 nodes, all healthy                            ║
║  Knowledge:   10 expert files, 500 chunks indexed             ║
║  Status:      READY                                           ║
║                                                                ║
║  Next: phantom build --framework <your-architecture.md>       ║
╚════════════════════════════════════════════════════════════════╝
```

Phantom asks permission before installing anything. You can skip any step if you already have a tool installed.

### Step 4 — Write Your Architecture Framework

This is the document that tells Phantom what to build. It's a Markdown file describing your system.

**Minimal example** (`my-app-architecture.md`):

```markdown
# My SaaS App — Architecture

## Overview
A project management tool for small teams. Users can create projects,
assign tasks, set deadlines, and track progress.

## Tech Stack
- Backend: FastAPI (Python 3.12)
- Frontend: Next.js 14 (TypeScript, Tailwind CSS)
- Database: PostgreSQL
- Cache: Redis
- Auth: JWT + Google OAuth

## Core Features
- User registration and login (email + Google)
- Create/edit/delete projects
- Create/edit/delete tasks within projects
- Assign tasks to team members
- Dashboard with task stats and deadlines
- Email notifications for overdue tasks

## Database Models
- users (id, email, name, avatar_url, created_at)
- projects (id, name, description, owner_id, created_at)
- tasks (id, title, description, project_id, assignee_id, 
         status, priority, due_date, created_at)
- project_members (project_id, user_id, role)

## API Endpoints
- POST /api/v1/auth/register
- POST /api/v1/auth/login
- GET /api/v1/projects
- POST /api/v1/projects
- GET /api/v1/projects/:id/tasks
- POST /api/v1/projects/:id/tasks
- PATCH /api/v1/tasks/:id
- DELETE /api/v1/tasks/:id

## Deployment
- Frontend: Vercel
- Backend: Docker on free-tier cloud VM
- Database: Supabase (free tier)
```

The more detail you provide, the better the output. But even a minimal spec like this gives Phantom enough to build a working application.

### Step 5 — Build

```bash
phantom build --framework ./my-app-architecture.md
```

**What happens when you build:**

```
phantom build --framework ./my-app-architecture.md

┌─ BUILD PLAN ──────────────────────────────────────────────────────┐
│                                                                    │
│  Project:          My SaaS App                                    │
│  Architecture:     Modular Monolith                               │
│  Components:       Backend (FastAPI) + Frontend (Next.js)         │
│  Database:         PostgreSQL (Supabase)                          │
│  Cache:            Redis (Upstash)                                │
│                                                                    │
│  TASK BREAKDOWN                                                   │
│  ├── Phase 1: System Design .............. ~15 min (Architect)    │
│  ├── Phase 2: Backend .................... ~90 min (Backend)      │
│  ├── Phase 3: Frontend ................... ~90 min (Frontend)     │
│  ├── Phase 4: Infrastructure ............. ~30 min (DevOps)       │
│  ├── Phase 5: Testing .................... ~45 min (QA)           │
│  ├── Phase 6: Security Audit ............. ~20 min (Security)     │
│  └── Phase 7: Deploy + Verify ............ ~15 min (DevOps)      │
│                                                                    │
│  Parallel Streams:   3 (Backend + Frontend + DevOps)              │
│  Estimated Time:     ~3.5 hours                                   │
│  Estimated LOC:      ~8,000                                       │
│  Infrastructure:     $0/month (free tiers)                        │
│                                                                    │
│  [Approve & Start]  [Modify Plan]  [Cancel]                      │
└───────────────────────────────────────────────────────────────────┘

> Approve & Start
```

After approval, Phantom works autonomously. You can watch progress in real time:

```bash
phantom status --live
```

### Step 6 — Receive Your Software

When the build completes (typically 3-6 hours depending on complexity):

```
╔════════════════════════════════════════════════════════════════╗
║  BUILD COMPLETE                                                ║
║                                                                ║
║  Frontend:   https://my-saas-app.vercel.app                   ║
║  Backend:    https://api.my-saas-app.com                      ║
║  API Docs:   https://api.my-saas-app.com/docs                ║
║  GitHub:     https://github.com/yourname/my-saas-app          ║
║                                                                ║
║  Code:       8,234 lines across 47 files                      ║
║  Tests:      124 tests, 87% coverage                          ║
║  Security:   0 critical, 0 high, 2 low (informational)        ║
║                                                                ║
║  Credentials saved to Phantom vault (encrypted)               ║
║  Run: phantom vault list  to see stored credentials           ║
╚════════════════════════════════════════════════════════════════╝
```

Your application is live. In production. With tests, CI/CD, monitoring, and documentation.

---

## What Phantom Does

| Capability | What It Means |
|-----------|---------------|
| **Full computer access** | Phantom controls your Mac via terminal — filesystem, processes, network, Keychain, clipboard, system preferences, Docker, git, databases, browsers (via AppleScript) |
| **Autonomous dependency installation** | Detects missing tools and installs them — Homebrew, Node.js, Python, Rust, Docker, PostgreSQL, Redis, and 10+ deployment CLIs |
| **Autonomous account creation** | Creates accounts on GitHub, Vercel, Supabase, Cloudflare, Upstash, and more — via CLI OAuth or API signup |
| **Knowledge-driven decisions** | Every agent decision is grounded in 10 expert-level knowledge bases (25,000+ lines of CTO-grade intelligence) |
| **Parallel agent execution** | 8 specialized agents work simultaneously — 14x faster than sequential development |
| **Self-healing** | 5-layer recovery system: retry → alternative approach → decompose → escalate → pause & alert |
| **Zero local footprint** | All persistent state lives on remote encrypted servers — your Mac stores only the binary |
| **$0 infrastructure** | Auto-provisions across 14+ free-tier cloud providers — Oracle, GCP, Cloudflare, Supabase, Upstash, Vercel, etc. |
| **P2P resilience** | Peer-to-peer mesh with CRDT sync — survives any 2 provider failures simultaneously |
| **License-gated** | Ed25519 cryptographic license bound to your machine hardware — impossible to forge or transfer |
| **Master key control** | Owner holds absolute power — issue/revoke licenses, remote-kill, full system destruction with one command |

---

## Architecture Overview

Phantom is a Rust workspace with 8 crates:

```
phantom/
├── crates/
│   ├── phantom-cli/        Binary — CLI entry point, commands, TUI dashboard
│   ├── phantom-core/       Library — agent orchestration, task graph, message bus
│   ├── phantom-crypto/     Library — Ed25519, AES-256-GCM, Argon2id, HKDF
│   ├── phantom-net/        Library — libp2p P2P mesh, QUIC transport, CRDT sync
│   ├── phantom-infra/      Library — 14 cloud provider clients, provisioning
│   ├── phantom-ai/         Library — Anthropic API client, agent prompts
│   ├── phantom-storage/    Library — encrypted R2/S3 client, credential vault
│   └── phantom-brain/      Library — ChromaDB client, knowledge vector search
├── .github/workflows/      CI/CD pipelines
├── Cargo.toml              Workspace root
└── Cargo.lock
```

### The 5-Layer Stack

```
┌─────────────────────────────────────────────────────────────────┐
│  Layer 5: Terminal Interface (CLI + TUI dashboard)               │
├─────────────────────────────────────────────────────────────────┤
│  Layer 4: Agent Orchestration + Knowledge Brain                  │
├─────────────────────────────────────────────────────────────────┤
│  Layer 3: Security & Key Management                              │
├─────────────────────────────────────────────────────────────────┤
│  Layer 2: Zero-Footprint + Full Computer Access                  │
├─────────────────────────────────────────────────────────────────┤
│  Layer 1: Self-Discovering Infrastructure + P2P Mesh             │
└─────────────────────────────────────────────────────────────────┘
```

---

## The 8 Agents

| Agent | Role | Model | Key Responsibility |
|-------|------|-------|-------------------|
| **CTO** | Orchestrator | Claude Opus | Reads architecture, decomposes tasks, delegates, monitors, synthesizes |
| **Architect** | System Designer | Claude Opus | System design, DB schema, API contracts, ADRs |
| **Backend** | Code Generator | Claude Sonnet | FastAPI routes, models, services, auth, background jobs |
| **Frontend** | Code Generator | Claude Sonnet | Next.js pages, components, design tokens, accessibility |
| **DevOps** | Infrastructure | Claude Sonnet | Docker, CI/CD, DNS, TLS, deployment scripts |
| **QA** | Testing | Claude Sonnet | Unit tests, integration tests, E2E (Playwright), coverage |
| **Security** | Auditor | Claude Opus | OWASP Top 10, dependency audit, auth review, secret detection |
| **Monitor** | Guardian | Claude Haiku | Health checks, auto-healing, cost tracking, alerts |

Agents run in parallel where possible. The CTO Agent coordinates everything.

---

## Security Model

### Key Hierarchy

```
Master Key (owner's passphrase → Argon2id → 256-bit key)
├── License Signing Key (Ed25519 keypair)
│   └── License Token (per machine, hardware-bound)
│       └── Session Key (ephemeral, in-memory only)
│           └── Agent Keys (per-agent, per-task, scoped)
├── Infrastructure Key (cloud credentials)
│   └── Server Bind Tokens (per-server proof of ownership)
└── Destruction Key (master key + TOTP 2FA)
```

### Anti-Tamper Measures

| Threat | Mitigation |
|--------|-----------|
| Binary reverse engineering | Control flow flattening, string encryption, self-integrity check |
| License forgery | Ed25519 signatures — computationally infeasible without private key |
| Memory dump | Keys in mlock'd pages (no swap), zeroed on drop |
| Network interception | TLS 1.3 + certificate pinning + mutual TLS between agents |
| Server compromise | Zero-knowledge encryption — servers store only encrypted blobs |
| Rogue agent | Scoped permissions, signed actions, full audit trail |

### Master Key Powers

The master key holder (owner only) can:

```bash
phantom master issue     # Issue new license keys
phantom master revoke    # Revoke any license
phantom master list      # List all installations
phantom master kill      # Remote-kill any installation
phantom master destroy   # Full system erasure (requires TOTP 2FA)
phantom master rotate    # Rotate all keys and credentials
phantom master audit     # Export complete audit log
```

---

## CLI Reference

### Core Commands

```bash
phantom activate --key <KEY>          # Activate Phantom on this machine
phantom build --framework <file>      # Build from architecture document
phantom status [--live]               # Show agent/task/infra status
phantom doctor                        # Verify all dependencies
phantom agents                        # List active agents
phantom logs [--agent <name>]         # Stream logs
phantom infra                         # Infrastructure status
phantom pause                         # Pause all agents
phantom resume                        # Resume paused agents
```

### Build Commands

```bash
phantom build --framework <file>      # Full autonomous build
phantom build --resume                # Resume interrupted build
phantom build --component <name>      # Build single component
phantom build --test-only             # Run tests without building
phantom build --deploy-only           # Deploy existing build
phantom build --dry-run               # Show plan, don't execute
```

### Knowledge Brain Commands

```bash
phantom brain search <query>          # Query the knowledge base directly
phantom brain update --file <md>      # Add/update a knowledge file
phantom brain status                  # Show corpus stats
```

### Infrastructure Commands

```bash
phantom infra status                  # Show all bound infrastructure
phantom infra provision               # Manually trigger provisioning
phantom infra migrate                 # Migrate to different provider
phantom infra cost                    # Cost breakdown
phantom infra backup                  # Force immediate backup
```

### Master Commands (require passphrase)

```bash
phantom master init                   # One-time master key setup
phantom master issue --email <e>      # Issue license key
phantom master revoke --key <k>       # Revoke license key
phantom master list                   # List all installations
phantom master kill <id>              # Remote-kill installation
phantom master destroy                # Full system destruction
phantom master rotate                 # Rotate all keys
phantom master audit                  # Export audit log
```

---

## How the Knowledge Brain Works

Phantom's intelligence comes from 10 expert-level knowledge bases embedded as its permanent context corpus. These aren't generic AI instructions — they're 25,000+ lines of CTO-grade engineering knowledge covering every domain.

| # | Knowledge File | Lines | Used By |
|---|---------------|-------|---------|
| 1 | CTO Architecture Framework | 1,333 | CTO, Architect |
| 2 | CTO Technology Knowledge Base | 3,172 | All agents |
| 3 | Multi-Agent Autonomous System | 3,255 | CTO, Monitor |
| 4 | Build Once, Launch Directly | 1,335 | CTO, DevOps |
| 5 | Full-Stack Software Blueprint | 339 | Backend, Frontend |
| 6 | Every Technology in Software | 1,475 | Architect, CTO |
| 7 | Design Expert Knowledge Base | 2,890 | Frontend |
| 8 | AI & ML Expert Knowledge Base | 3,558 | CTO (ML decisions) |
| 9 | API Expert Knowledge Base | 3,368 | Backend, DevOps |
| 10 | AI Code GitHub Errors & Fixes | 1,692 | QA, DevOps |

**How agents use it:** Before every decision, an agent generates a semantic query, retrieves relevant chunks from ChromaDB (vector search), and cites which knowledge section influenced the decision. This means Phantom doesn't hallucinate patterns — it follows documented, battle-tested engineering practices.

---

## Self-Healing System

When something fails, Phantom doesn't stop. It recovers through 5 layers:

```
Layer 1: RETRY                 → 80% of failures
  Exponential backoff (1s → 2s → 4s → 8s → 16s), 5 attempts max

Layer 2: ALTERNATIVE APPROACH  → 10% of failures
  npm fails → try yarn. API v2 down → fall back to v1.
  PostgreSQL full → migrate to Neon.

Layer 3: DECOMPOSE             → 5% of failures
  Task too complex → CTO Agent splits into smaller subtasks

Layer 4: ESCALATE INTERNALLY   → 3% of failures
  Agent needs info from another domain → CTO spawns helper agent

Layer 5: PAUSE & ALERT         → 2% of failures
  Requires human authority → save state, ask owner, resume on reply
```

---

## Infrastructure — $0/month

Phantom auto-provisions across 14+ free-tier cloud providers:

| Provider | Free Tier | Phantom Uses For |
|---------|-----------|-----------------|
| Oracle Cloud | 2 VMs + 200GB | Primary compute + storage |
| Google Cloud | e2-micro | Secondary compute |
| Cloudflare Workers | 100K req/day | Edge relay + DNS + CDN |
| Cloudflare R2 | 10GB | Encrypted blob storage |
| Supabase | 500MB PostgreSQL | Primary database |
| Neon | 0.5GB PostgreSQL | Backup database |
| Upstash | 10K cmd/day | Redis cache + job queue |
| Vercel | Serverless | Frontend deployment |
| GitHub | Unlimited | Code storage + CI/CD |
| Fly.io | 3 shared VMs | P2P mesh nodes |

**Total monthly cost: $0.00.** Minimum 3 providers active for redundancy.

---

## Peer-to-Peer Mesh

Phantom distributes state across multiple servers using a P2P mesh:

```
Protocol Stack:
  Transport:   QUIC (UDP, NAT-traversal friendly)
  Security:    Noise protocol (XX handshake)
  Identity:    Ed25519 peer IDs
  Discovery:   Kademlia DHT + mDNS (local network)
  Sync:        CRDT (Automerge) — conflict-free state replication
  Encryption:  ChaCha20-Poly1305
```

**What syncs:** project state, task graph, infrastructure bindings, audit log, health metrics.

**What never syncs:** master key, session keys, raw credentials.

---

## Project Structure

```
BenchBrex-PHANTOM/
├── crates/
│   ├── phantom-cli/             # Binary crate — the executable
│   │   └── src/
│   │       ├── main.rs          # Entry point, clap command routing
│   │       ├── commands/        # activate, build, status, master, etc.
│   │       └── ui/              # ratatui TUI dashboard
│   ├── phantom-core/            # Orchestration engine
│   │   └── src/
│   │       ├── task_graph.rs    # DAG task dependency resolution
│   │       ├── agent_manager.rs # Spawn, monitor, kill agents
│   │       ├── message_bus.rs   # Pub/sub inter-agent messaging
│   │       ├── self_healer.rs   # 5-layer recovery system
│   │       └── agents/          # CTO, Architect, Backend, etc.
│   ├── phantom-crypto/          # Cryptographic primitives
│   │   └── src/
│   │       ├── master_key.rs    # Argon2id key derivation
│   │       ├── license.rs       # Ed25519 license sign/verify
│   │       ├── session.rs       # Ephemeral session keys
│   │       ├── encryption.rs    # AES-256-GCM encrypt/decrypt
│   │       └── fingerprint.rs   # Machine hardware fingerprint
│   ├── phantom-net/             # P2P networking
│   │   └── src/
│   │       ├── p2p.rs           # libp2p mesh setup
│   │       ├── discovery.rs     # Kademlia DHT + mDNS
│   │       └── sync.rs          # CRDT state replication
│   ├── phantom-infra/           # Cloud infrastructure
│   │   └── src/
│   │       ├── providers/       # Oracle, GCP, Cloudflare, etc.
│   │       ├── provisioner.rs   # Multi-provider orchestration
│   │       ├── installer.rs     # macOS dependency installation
│   │       └── accounts.rs      # Autonomous account creation
│   ├── phantom-ai/              # AI agent interface
│   │   └── src/
│   │       ├── client.rs        # Anthropic API client
│   │       ├── prompts.rs       # Agent role prompt templates
│   │       └── context.rs       # Context window management
│   ├── phantom-storage/         # Encrypted remote storage
│   │   └── src/
│   │       ├── encrypted_store.rs  # AES-256-GCM blob storage
│   │       ├── r2.rs            # Cloudflare R2 client
│   │       └── vault.rs         # Credential vault
│   └── phantom-brain/           # Knowledge retrieval
│       └── src/
│           ├── embeddings.rs    # Sentence-transformers client
│           ├── knowledge.rs     # ChromaDB vector search
│           └── chunker.rs       # Markdown semantic chunking
├── docs/
│   ├── ARCHITECTURE.md          # Full architecture specification
│   ├── SECURITY.md              # Threat model and mitigations
│   ├── AGENTS.md                # Agent behavior specification
│   └── OPERATIONS.md            # Master key and infra operations
├── .github/workflows/
│   ├── ci.yml                   # cargo fmt + clippy + test
│   └── release.yml              # Build signed binaries
├── Cargo.toml                   # Workspace root
├── Cargo.lock
└── README.md                    # You are here
```

---

## Development

### Prerequisites

- Rust stable (via [rustup](https://rustup.rs))
- macOS 13+ (Phantom is macOS-native)

### Build

```bash
git clone https://github.com/benchbrex-USA/BenchBrex-PHANTOM.git
cd BenchBrex-PHANTOM

# Check compilation
cargo check --workspace

# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace --deny warnings

# Build release binary
cargo build --release -p phantom-cli
```

### CI

Every push to `main` and every PR runs:
- `cargo fmt --check` — formatting
- `cargo clippy --deny warnings` — linting
- `cargo test --workspace` — all tests

---

## FAQ

### Who is this for?

Founders, solo developers, and small teams who want to ship production software without hiring a 10-person engineering team. You provide the what (architecture); Phantom provides the how (code, tests, infra, deployment).

### What makes this different from Cursor / Copilot / Claude Code alone?

Those tools help you write code. Phantom is a **complete engineering team** — it doesn't just write code, it also installs your dev environment, creates your cloud accounts, provisions servers, designs your system, tests everything, audits security, deploys to production, and monitors health. All autonomously, all from one command.

### Can it access my existing codebase?

Yes. Point `--framework` at any architecture document, and Phantom reads the project context. It can also work with existing codebases — modifying, extending, and refactoring code that's already written.

### What if the build fails halfway through?

Run `phantom build --resume`. Phantom saves state at every checkpoint. It picks up exactly where it left off.

### Is my code safe?

All state is encrypted with AES-256-GCM using keys that exist only in your machine's memory. Remote servers store encrypted blobs they cannot decrypt. The master key is derived from your passphrase via Argon2id and is never written to disk.

### Can someone else use my license key?

No. License keys are cryptographically bound to your machine's hardware fingerprint (MAC address + CPU serial + disk UUID). They don't work on any other machine.

### What if I want to delete everything Phantom ever created?

```bash
phantom master destroy
```

This requires your passphrase + TOTP 2FA, then erases all data across every server, revokes all API keys, deletes all cloud resources, and self-deletes the binary. Complete erasure.

### What tech stack does Phantom default to?

Based on the owner's preferences: FastAPI (Python), Next.js 14 (TypeScript, Tailwind, shadcn/ui), PostgreSQL, Redis, Docker, GitHub Actions. But Phantom follows whatever tech stack is specified in your architecture document.

---

<div align="center">

**Built by [Benchbrex](https://benchbrex.com)**

Phantom is proprietary software. License required.

</div>
