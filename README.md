<div align="center">

# PHANTOM

### The Autonomous AI Engineering Team That Lives in Your Terminal

**One command. One architecture document. Full-stack production software — built, tested, deployed.**

[![CI](https://github.com/benchbrex-USA/BenchBrex-PHANTOM/actions/workflows/ci.yml/badge.svg)](https://github.com/benchbrex-USA/BenchBrex-PHANTOM/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/benchbrex-USA/BenchBrex-PHANTOM?label=release&color=blue)](https://github.com/benchbrex-USA/BenchBrex-PHANTOM/releases/tag/v0.1.0)
[![Rust](https://img.shields.io/badge/rust-100%25-orange?logo=rust)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-880%20passing-brightgreen)](https://github.com/benchbrex-USA/BenchBrex-PHANTOM/actions)
[![Binary](https://img.shields.io/badge/binary-4.2%20MB-blue)](https://github.com/benchbrex-USA/BenchBrex-PHANTOM/releases)
[![Platform](https://img.shields.io/badge/platform-macOS-lightgrey?logo=apple)](https://www.apple.com/macos/)
[![License](https://img.shields.io/badge/license-proprietary-red)](LICENSE)

**Live Integrations:**

[![Supabase](https://img.shields.io/badge/Supabase-LIVE-3ECF8E?logo=supabase&logoColor=white)](https://supabase.com)
[![Vercel](https://img.shields.io/badge/Vercel-LIVE-000000?logo=vercel&logoColor=white)](https://vercel.com)
[![Cloudflare](https://img.shields.io/badge/Cloudflare-Connected-F38020?logo=cloudflare&logoColor=white)](https://cloudflare.com)
[![GitHub](https://img.shields.io/badge/GitHub-Connected-181717?logo=github&logoColor=white)](https://github.com)

[Install](#install) · [Quick Start](#quick-start) · [How It Works](#how-it-works) · [Commands](#cli-reference) · [Docs](docs/)

</div>

---

## What is Phantom?

Phantom is a terminal-native AI system that builds complete production software from a single markdown file. You describe your app — database tables, API endpoints, frontend pages, auth rules — and Phantom's **8 AI agents** design, code, test, secure, and deploy it. Autonomously.

```
You write this:                           Phantom builds this:
┌──────────────────────────┐              ┌──────────────────────────┐
│ architecture.md          │              │ ✓ Live frontend (Vercel) │
│                          │    phantom   │ ✓ Live backend (Docker)  │
│ - 5 database tables      │───build───→  │ ✓ PostgreSQL (Supabase)  │
│ - 20 API endpoints       │              │ ✓ Redis cache (Upstash)  │
│ - 8 frontend pages       │              │ ✓ CI/CD (GitHub Actions) │
│ - Auth + RBAC            │              │ ✓ Tests (80%+ coverage)  │
│ - Dark mode, mobile-first│              │ ✓ Security audit (OWASP) │
└──────────────────────────┘              └──────────────────────────┘
                                           Total infra cost: $0/month
```

**Not a code assistant.** Phantom is a complete engineering organization — CTO, Architect, Backend, Frontend, DevOps, QA, Security, and Monitor agents — working in parallel, communicating through a signed message bus, self-healing through 5 recovery layers, and deploying to free-tier infrastructure.

---

## Install

```bash
curl -fsSL https://phantom.benchbrex.com/install.sh | sh
```

Downloads the 4.2 MB binary, verifies SHA-256 checksum, installs to `/usr/local/bin/phantom`.

<details>
<summary>Manual install / build from source</summary>

```bash
# Apple Silicon (M1/M2/M3/M4)
wget https://phantom.benchbrex.com/releases/latest/phantom-darwin-arm64
chmod +x phantom-darwin-arm64 && sudo mv phantom-darwin-arm64 /usr/local/bin/phantom

# Intel Mac
wget https://phantom.benchbrex.com/releases/latest/phantom-darwin-x64
chmod +x phantom-darwin-x64 && sudo mv phantom-darwin-x64 /usr/local/bin/phantom

# Build from source (Rust 1.75+)
git clone https://github.com/benchbrex-USA/BenchBrex-PHANTOM.git
cd BenchBrex-PHANTOM && cargo build --release
# Binary at target/release/phantom
```

</details>

---

## Quick Start

```bash
# 1. Activate with your license key
phantom activate --key PH1-your-key-here

# 2. Set your Anthropic API key (powers the AI agents)
export ANTHROPIC_API_KEY=sk-ant-your-key-here
# Get one at https://console.anthropic.com

# 3. Check everything is working
phantom doctor

# 4. Write your architecture document (see below)
vim my-app.md

# 5. Build (Phantom does everything from here)
phantom build --framework ./my-app.md

# 6. Watch agents work in real-time
phantom dashboard
```

**Result:** live frontend + backend + database + CI/CD + tests + security audit. Deployed. $0/month.

---

## How It Works

### The 8-Agent Team

| Agent | Model | Budget | Role |
|-------|-------|--------|------|
| **CTO** | Claude Opus | 500K | Plans, delegates, monitors, synthesizes |
| **Architect** | Claude Opus | 300K | DB schemas, API contracts, ADRs |
| **Backend** | Claude Sonnet | 200K | FastAPI, auth, business logic, background jobs |
| **Frontend** | Claude Sonnet | 200K | Next.js, Tailwind, a11y, dark mode, mobile-first |
| **DevOps** | Claude Sonnet | 100K | Docker, CI/CD, DNS, TLS, deployment |
| **QA** | Claude Sonnet | 100K | pytest, Vitest, Playwright, 80%+ coverage |
| **Security** | Claude Opus | 100K | OWASP, dependency scan, auth review, secrets |
| **Monitor** | Claude Haiku | 50K | Health, auto-healing, cost tracking, alerts |

Every agent queries the **Knowledge Brain** (10 files, 25,000+ lines, vector-indexed) before every decision.

### The 8-Phase Build Pipeline

| Phase | Duration | Agent(s) | What Happens |
|-------|----------|----------|--------------|
| 0. Ingest | ~5 min | CTO | Parse architecture doc, build task graph |
| 1. Infrastructure | 15–30 min | DevOps | Provision servers, cloud accounts, CI/CD |
| 2. Architecture | ~15 min | Architect | System design, DB schema, API contracts |
| 3. Code | 1–3 hrs | Backend + Frontend + DevOps | Parallel code generation |
| 4. Test | 30–60 min | QA | Unit + integration + E2E, 80%+ coverage |
| 5. Security | 15–30 min | Security | Dependency audit, OWASP scan |
| 6. Deploy | 15–30 min | DevOps | Docker → deploy → DNS → TLS → health check |
| 7. Deliver | ~5 min | CTO | Report with live URLs and credentials |

### Self-Healing (5 layers)

| Layer | Rate | Strategy |
|-------|------|----------|
| Retry | ~80% | Exponential backoff, up to 5 attempts |
| Alternative | ~10% | Different tool, provider, or approach |
| Decompose | ~5% | Break task into smaller pieces |
| Escalate | ~3% | Route to another agent |
| Pause | ~2% | Save state, ask owner |

Builds complete autonomously **98%+ of the time**.

---

## Writing Your Architecture

Your architecture document has **9 sections**. This is the only thing you write:

| Section | Agent | What Gets Built |
|---------|-------|-----------------|
| 1. Product Overview | CTO | Repo, README, scope |
| 2. Tech Stack | Architect | Dependencies, framework config |
| 3. Database Models | Architect | SQL migrations, ORM models, RLS |
| 4. API Endpoints | Backend | Routes, middleware, validation |
| 5. Frontend Pages | Frontend | Pages, components, navigation |
| 6. Auth & Permissions | Backend + Security | JWT, OAuth, RBAC |
| 7. Background Jobs | Backend | Workers, queues, schedulers |
| 8. Deployment | DevOps | Docker, CI/CD, DNS, TLS |
| 9. Constraints | ALL agents | Enforced across every file |

<details>
<summary>Example: Task Management App</summary>

```markdown
# TaskFlow — Architecture Framework

## 1. Product Overview
Project management for freelancers with time tracking and invoicing.

## 2. Tech Stack
- Backend: FastAPI (Python 3.12+)
- Frontend: Next.js 14 (TypeScript, Tailwind, shadcn/ui)
- Database: PostgreSQL (Supabase)
- Auth: JWT + Google OAuth

## 3. Database Models
### users
| Column | Type | Constraints |
|--------|------|------------|
| id | UUID | PK |
| email | VARCHAR(255) | UNIQUE, NOT NULL |
| name | VARCHAR(100) | NOT NULL |
| role | ENUM('admin','member') | DEFAULT 'member' |

### projects
| Column | Type | Constraints |
|--------|------|------------|
| id | UUID | PK |
| name | VARCHAR(200) | NOT NULL |
| owner_id | UUID | FK → users.id |

### tasks
| Column | Type | Constraints |
|--------|------|------------|
| id | UUID | PK |
| project_id | UUID | FK → projects.id |
| title | VARCHAR(300) | NOT NULL |
| status | ENUM('todo','in_progress','done') | DEFAULT 'todo' |
| assignee_id | UUID | FK → users.id, NULLABLE |

## 4. API Endpoints
| Method | Path | Description | Auth |
|--------|------|-------------|------|
| POST | /api/v1/auth/login | Login | Public |
| POST | /api/v1/auth/register | Register | Public |
| GET | /api/v1/projects | List projects | Bearer |
| POST | /api/v1/projects | Create project | Bearer |
| GET | /api/v1/projects/:id/tasks | List tasks | Bearer |
| POST | /api/v1/projects/:id/tasks | Create task | Bearer |

## 5. Frontend Pages
| Route | Page | Auth |
|-------|------|------|
| / | Landing | No |
| /dashboard | Dashboard | Yes |
| /projects | Project list | Yes |
| /projects/:id | Kanban board | Yes |

## 6. Auth & Permissions
admin: everything. member: CRUD own data.

## 7. Background Jobs
daily_digest: 8 AM UTC, email task summary.

## 8. Deployment
Frontend: Vercel. Backend: Docker. Database: Supabase.

## 9. Constraints
Dark mode. Mobile-first. 80%+ test coverage. WCAG 2.2 AA.
```

</details>

Then build:

```bash
phantom build --framework ./taskflow.md --dry-run   # Preview plan
phantom build --framework ./taskflow.md              # Build everything
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  phantom-cli                                                 │
│  License gate · TUI dashboard · 12 commands                  │
├──────────┬──────────┬──────────┬──────────┬─────────────────┤
│  core    │  net     │  infra   │  ai      │  brain          │
│  TaskDAG │  QUIC    │  14+prov │  Claude  │  ChromaDB       │
│  Agents  │  CRDT    │  Supabase│  8 agents│  Embeddings     │
│  Healing │  mDNS    │  Vercel  │  Prompts │  Chunking       │
│  Audit   │  DHT     │  Doctor  │  Context │  Search         │
├──────────┴──────────┴──────────┼──────────┴─────────────────┤
│  phantom-storage               │  phantom-crypto             │
│  R2 client · Vault · State     │  Ed25519 · AES · Argon2id   │
└────────────────────────────────┴────────────────────────────┘
```

| Crate | Purpose |
|-------|---------|
| **phantom-cli** | Binary — license gate, build orchestration, TUI dashboard, master key ops |
| **phantom-core** | Task graph DAG, agent lifecycle, message bus, 5-layer self-healing, audit log |
| **phantom-crypto** | Ed25519, Argon2id (256 MB), AES-256-GCM, HKDF, machine fingerprinting |
| **phantom-net** | libp2p — QUIC, Noise encryption, Kademlia DHT, Automerge CRDT sync |
| **phantom-infra** | 14+ providers, Supabase (LIVE), Vercel (LIVE), provisioner, health |
| **phantom-ai** | Anthropic client, 8 agent prompts, context management, token budgets |
| **phantom-storage** | Encrypted R2/S3 client, credential vault, zero-knowledge blobs |
| **phantom-brain** | Markdown chunker, sentence-transformers embeddings, semantic search |

---

## Security

```
Master Key (passphrase → Argon2id → 256-bit, never stored)
├── License Signing Key (Ed25519)
│   └── License Token (per-machine, hardware-bound)
│       └── Session Key (ephemeral, RAM only, zeroed on exit)
│           └── Agent Keys (per-agent, per-task, scoped)
├── Infrastructure Key (encrypts cloud credentials)
│   └── Server Bind Tokens (ownership proof)
└── Destruction Key (master + TOTP 2FA)
```

| Primitive | Usage |
|-----------|-------|
| Ed25519 | License signing/verification |
| Argon2id (256 MB, 4 iter, 8 parallel) | Master key derivation |
| AES-256-GCM | All data-at-rest encryption |
| HKDF-SHA256 | Sub-key derivation |
| HMAC-SHA256 | Machine fingerprinting |
| SHA-256 | Tamper-evident audit chain |

**Zero-footprint:** nothing on disk. Binary + macOS Keychain + RAM. All persistent state encrypted on remote servers that cannot decrypt.

---

## Infrastructure ($0/month)

| Provider | Resources | Status |
|----------|-----------|--------|
| Oracle Cloud | 2 VMs + 200 GB | Primary compute |
| Google Cloud | e2-micro | Replica |
| Cloudflare | Workers + R2 (10 GB) + DNS | CDN + storage |
| **Supabase** | PostgreSQL (500 MB) | ✅ LIVE |
| **Vercel** | Serverless + edge | ✅ LIVE |
| Upstash | Redis (10K cmd/day) | Cache |
| Neon | PostgreSQL (0.5 GB) | Backup DB |
| Fly.io | 3 shared VMs | P2P mesh |
| GitHub | Unlimited repos + Actions | Code + CI |

**Redundancy: 3x** — survives 2 simultaneous provider failures.

---

## CLI Reference

### Core

```
phantom activate --key <PH1-...>              License activation
phantom build --framework <file.md>           Full autonomous build
phantom build --framework <f> --dry-run       Preview build plan
phantom build --resume                        Resume interrupted build
phantom dashboard                             Live TUI dashboard
phantom status                                System overview
phantom doctor                                Dependency + health check
```

### Information

```
phantom agents                                Agent status and config
phantom infra                                 Infrastructure health
phantom logs [--agent <name>]                 Stream real-time logs
phantom cost estimate --framework <file>      Cost projection
```

### Knowledge Brain

```
phantom brain status                          Corpus stats
phantom brain search "<query>"                Semantic search
phantom brain update --file <path.md>         Re-index knowledge file
```

### Master Key Operations

```
phantom master init                           First-time key setup
phantom master issue --email <e>              Issue license
phantom master revoke --key <PH1-...>         Revoke license
phantom master list                           List installations
phantom master destroy                        Full erasure (passphrase + TOTP)
phantom master rotate                         Rotate all keys
phantom master audit                          Export audit log
phantom master halt                           Emergency stop
```

---

## Knowledge Brain

10 expert-level knowledge files, 25,000+ lines, vector-indexed:

| # | File | Lines | Agents |
|---|------|-------|--------|
| 1 | CTO Architecture Framework | 1,333 | CTO, Architect |
| 2 | CTO Technology Knowledge Base | 3,172 | All |
| 3 | Multi-Agent Autonomous System | 3,255 | CTO, Monitor |
| 4 | Build Once, Launch Directly | 1,335 | CTO, DevOps |
| 5 | Full-Stack Software Blueprint | 339 | Backend, Frontend |
| 6 | Every Technology in Software | 1,475 | Architect |
| 7 | Design Expert Knowledge Base | 2,890 | Frontend |
| 8 | AI & ML Expert Knowledge Base | 3,558 | CTO |
| 9 | API Expert Knowledge Base | 3,368 | Backend, DevOps |
| 10 | AI Code Errors & Fixes | 1,692 | QA, DevOps |

---

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | **Yes** | Claude API key ([console.anthropic.com](https://console.anthropic.com)) |
| `RUST_LOG` | No | Log level (default: `phantom=info`) |

Provider credentials managed automatically through encrypted vault after activation.

---

## Project Structure

```
BenchBrex-PHANTOM/
├── .github/workflows/        CI + Release pipelines
├── crates/
│   ├── phantom-ai/           Anthropic client, prompts, context
│   ├── phantom-brain/        Knowledge Brain, embeddings, search
│   ├── phantom-cli/          Binary, commands, TUI dashboard
│   ├── phantom-core/         Task graph, agents, healing, audit
│   ├── phantom-crypto/       Ed25519, AES, Argon2id, HKDF
│   ├── phantom-infra/        Providers, Supabase, Vercel, provisioner
│   ├── phantom-net/          libp2p, QUIC, CRDT sync
│   └── phantom-storage/      Encrypted R2, vault
├── docs/                     Getting Started, Security, Agents, Ops
├── integration_tests/        Cross-crate tests
├── site/                     phantom.benchbrex.com
├── Cargo.toml                Workspace config
└── install.sh                One-line installer
```

---

## Stats

```
Language:       Rust 100%
Crates:         8
Tests:          880 passing
Binary:         4.2 MB (static, no runtime deps)
Clippy:         0 warnings
Infra cost:     $0/month
Integrations:   Supabase ✅ Vercel ✅ Cloudflare ✅ GitHub ✅
Release:        v0.1.0 (arm64 + x64 + SHA-256 checksums)
```

---

## Documentation

| Doc | Description |
|-----|-------------|
| [Getting Started](docs/GETTING_STARTED.md) | Install → activate → build → deploy |
| [Security](docs/SECURITY.md) | Key hierarchy, threat model, encryption |
| [Agents](docs/AGENTS.md) | 8 agents, knowledge mapping, permissions |
| [Operations](docs/OPERATIONS.md) | Master key, licenses, infrastructure |
| [Architecture v2](docs/PHANTOM_ARCHITECTURE_FRAMEWORK_v2.md) | Full system spec (1,311 lines) |
| [Execution Plan](docs/PHANTOM_MULTI_AGENT_EXECUTION_PLAN.md) | Build prompts + parallel map |

---

<div align="center">

**Built by [Parth Patel](https://linkedin.com/in/parthpatel) / [Benchbrex](https://benchbrex.com)** · India + USA

[phantom.benchbrex.com](https://phantom.benchbrex.com) · Proprietary License · Copyright © 2024–2026

</div>
