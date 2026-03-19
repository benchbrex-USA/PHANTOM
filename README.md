# Phantom

**Autonomous AI engineering system.** Phantom takes an Architecture Framework (a structured markdown spec) and autonomously builds, tests, secures, deploys, and delivers a complete software project — using an 8-agent team powered by Claude.

Built in Rust. License-gated. Zero local footprint. Every decision knowledge-driven. Every action audited.

---

## Core Principles

1. **No installation without a valid license key** — Ed25519-signed, bound to machine fingerprint
2. **No ownership without the master key** — Argon2id-derived from passphrase, never stored
3. **Zero local disk footprint** — all state encrypted (AES-256-GCM) and stored remotely in R2
4. **Knowledge Brain is the source of truth** — every agent decision queries ChromaDB before acting
5. **Every action is audited** — tamper-evident SHA-256 hash chain
6. **Self-provisioning infrastructure** — autonomous discovery across 14+ free-tier cloud providers
7. **Self-healing at every layer** — 5-layer recovery: retry → alternative → decompose → escalate → pause
8. **Master key holder has absolute power** — halt, kill, rotate, destroy
9. **Master key never leaves memory** — derived on demand, zeroized on drop

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                       phantom-cli                           │
│             License gate · TUI dashboard · Commands         │
├──────────┬──────────┬──────────┬──────────┬─────────────────┤
│  core    │   net    │  infra   │    ai    │     brain       │
│ TaskDAG  │  QUIC    │ 14+ prov │ Anthropic│   ChromaDB      │
│ Agents   │  CRDT    │ Accounts │ 8 agents │   Embeddings    │
│ Healing  │  mDNS    │ Health   │ Prompts  │   Chunking      │
│ Audit    │  DHT     │ Doctor   │ Context  │   Search        │
├──────────┴──────────┴──────────┼──────────┴─────────────────┤
│           phantom-storage      │       phantom-crypto       │
│     R2 client · Vault · State  │  Ed25519 · AES · Argon2id  │
└────────────────────────────────┴────────────────────────────┘
```

### Crates

| Crate | Purpose |
|-------|---------|
| **phantom-cli** | Binary entry point — license activation, build orchestration, agent monitoring, master key operations, TUI dashboard |
| **phantom-core** | Orchestration engine — task graph (DAG with topological ordering), agent lifecycle management, inter-agent message bus, 5-layer self-healing, tamper-evident audit log |
| **phantom-crypto** | Cryptographic foundation — Ed25519 license signing/verification, Argon2id master key derivation (256 MB, 4 iterations), AES-256-GCM authenticated encryption, HKDF sub-key derivation, machine fingerprinting |
| **phantom-net** | P2P mesh networking — QUIC transport with Noise XX handshake, Kademlia DHT + mDNS peer discovery, Automerge CRDT state synchronization, gossipsub messaging |
| **phantom-infra** | Infrastructure management — 14+ cloud provider clients (Oracle, GCP, AWS, Cloudflare, Fly.io, Vercel, Supabase, etc.), autonomous account management, resource provisioning with failover, system health checks |
| **phantom-ai** | AI integration — Anthropic API client with retry/backoff, 8-agent team configuration (CTO, Architect, Backend, Frontend, DevOps, QA, Security, Monitor), per-agent token budgets, context window management, role-specific system prompts |
| **phantom-storage** | Encrypted remote storage — Cloudflare R2 client (S3-compatible), credential vault with TTL and rotation, remote state management with compare-and-swap, zero-knowledge design |
| **phantom-brain** | Knowledge Brain — ChromaDB vector database integration, markdown chunking pipeline, semantic search with `all-MiniLM-L6-v2` embeddings (384-dim), role-scoped knowledge injection |

### Dependency Layers

```
Layer 0 (Foundation)    phantom-crypto
Layer 1 (Storage/Net)   phantom-storage, phantom-net
Layer 2 (AI/Knowledge)  phantom-ai, phantom-brain
Layer 3 (Infra)         phantom-infra
Layer 4 (Core)          phantom-core
Layer 5 (CLI)           phantom-cli
```

---

## Agent Team

Phantom deploys an 8-agent team, each with a dedicated role, model, token budget, and knowledge scope:

| Agent | Model | Token Budget | Role |
|-------|-------|-------------|------|
| **CTO** | claude-opus-4-6 | 500K | Orchestrates the build — parses Architecture Framework, creates task graph, resolves conflicts, makes architectural decisions |
| **Architect** | claude-opus-4-6 | 300K | System design — database schemas, API contracts, architecture decision records |
| **Backend** | claude-sonnet-4-6 | 200K | Server-side code generation — APIs, business logic, database layers |
| **Frontend** | claude-sonnet-4-6 | 200K | UI/UX implementation — components, styling, client-side logic |
| **DevOps** | claude-sonnet-4-6 | 100K | Infrastructure automation — CI/CD pipelines, Docker, deployment configs |
| **QA** | claude-sonnet-4-6 | 100K | Testing — unit, integration, E2E tests targeting 80%+ coverage |
| **Security** | claude-opus-4-6 | 100K | Security audits — dependency scanning, OWASP checks, threat modeling, secret detection |
| **Monitor** | claude-haiku-4-5 | 50K | Health observer — tracks agent performance, resource usage, system metrics |

Every agent queries the Knowledge Brain before making decisions and must cite which knowledge section influenced the outcome.

---

## Build Pipeline

Phantom executes an 8-phase build pipeline:

| Phase | Name | Duration | Description |
|-------|------|----------|-------------|
| 0 | **Ingest** | ~5 min | Parse Architecture Framework, create task graph, validate requirements |
| 1 | **Infrastructure** | 15–30 min | Provision servers, create cloud accounts, setup CI/CD |
| 2 | **Architecture** | ~15 min | Design system, define schemas, draft API contracts and ADRs |
| 3 | **Code** | 1–3 hrs | Parallel code generation — Backend + Frontend + DevOps + Docs |
| 4 | **Test** | 30–60 min | Unit + integration + E2E tests, enforce 80%+ coverage |
| 5 | **Security** | 15–30 min | Dependency audit, OWASP scan, secret detection |
| 6 | **Deploy** | 15–30 min | CI → Docker → deploy → DNS → TLS → health checks |
| 7 | **Deliver** | ~5 min | Generate report with URLs, credentials, and architecture log |

---

## Self-Healing

When a task fails, Phantom escalates through 5 recovery layers:

| Layer | Success Rate | Strategy |
|-------|-------------|----------|
| Retry | ~80% | Exponential backoff (1s → 30s), up to 5 attempts |
| Alternative | ~10% | Try a different tool, provider, or approach |
| Decompose | ~5% | Break the task into smaller sub-tasks |
| Escalate | ~3% | Route to a specialist agent for help |
| Pause & Alert | ~2% | Save state, notify owner, resume on reply |

---

## Cryptography

| Primitive | Usage |
|-----------|-------|
| **Ed25519** | License key signing and verification |
| **Argon2id** (256 MB / 4 iter / 4 parallel) | Master key derivation from passphrase |
| **AES-256-GCM** | All data-at-rest encryption (R2 blobs, vault entries, state) |
| **HKDF-SHA256** | Sub-key derivation (session, infrastructure, storage, license, agent-scoped) |
| **HMAC-SHA256** | Machine fingerprinting for license binding |
| **SHA-256** | Tamper-evident audit chain |

The master key is never stored — it is derived in memory from a user-supplied passphrase and zeroized on drop. Sub-keys are deterministically derived via HKDF for different operational contexts.

---

## Infrastructure Providers

Phantom auto-discovers and provisions across 14+ free-tier cloud providers:

| Provider | Resources |
|----------|-----------|
| Oracle Cloud | 2 VMs + 200 GB (primary compute) |
| Google Cloud | e2-micro instance |
| AWS Free Tier | t2.micro (12 months) |
| Cloudflare | Workers, R2 (10 GB), DNS |
| Fly.io | 3 shared VMs |
| Railway | $5/mo credit |
| Render | Free tier hosting |
| Vercel | Frontend hosting |
| Netlify | Frontend hosting |
| Supabase | PostgreSQL (500 MB) |
| Neon | PostgreSQL (0.5 GB) |
| Upstash | Redis (10K cmd/day) |
| GitHub | Unlimited repos, Actions CI |

Provider selection is automatic with failover — if one provider's quota is exhausted, Phantom migrates to the next available.

---

## P2P Networking

Phantom nodes form a mesh network for state synchronization:

| Layer | Technology |
|-------|-----------|
| Transport | QUIC (UDP, NAT-traversal friendly) |
| Security | Noise XX handshake (authenticated encryption) |
| Identity | Ed25519 ephemeral peer keys |
| Discovery | Kademlia DHT (wide-area) + mDNS (local) |
| Sync | Automerge CRDT (conflict-free state replication) |
| Messaging | Gossipsub (`phantom-crdt-sync`, `phantom-heartbeat`) |

State that syncs: project state, task graph, infra bindings, audit log, health metrics.
State that never syncs: master key, session keys, raw credentials.

---

## CLI Reference

```
phantom activate --key <PH1-...>       License activation (required first)
phantom build --framework <path.md>    Autonomous build from Architecture Framework
phantom build --resume                 Resume interrupted build
phantom build --component <name>       Build single component
phantom status [--live]                Agent dashboard
phantom doctor                         Verify dependencies and system health
phantom agents                         List agent status and token usage
phantom logs [--agent <name>]          Stream real-time logs
phantom infra                          Show provisioned infrastructure
phantom brain search <query>           Semantic knowledge search
phantom brain update --file <path>     Update knowledge file
phantom cost estimate --framework <p>  Estimate project build cost
phantom master init                    First-time master key setup
phantom master issue --email <email>   Issue new license
phantom master revoke --key <key>      Revoke a license
phantom master list                    List all installations
phantom master kill <id>               Remote-kill an installation
phantom master destroy                 Full system destruction (requires TOTP)
phantom master rotate                  Rotate all cryptographic keys
phantom master audit                   Export tamper-evident audit log
phantom master transfer --to <email>   Transfer ownership
phantom master halt                    Emergency stop all agents
```

---

## License Format

```
PH1-<base64url_payload>-<base64url_signature>
```

Payload (JSON):
```json
{
  "v": 1,
  "mid": "<machine_fingerprint_hex>",
  "iat": 1710000000,
  "exp": 1741536000,
  "cap": ["cto", "architect", "backend", "frontend", "devops", "qa", "security", "monitor"],
  "tier": "founder"
}
```

Licenses are Ed25519-signed and bound to a specific machine's fingerprint. Capabilities control which agents can be spawned.

---

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | Yes | API key for Claude agent calls |
| `RUST_LOG` | No | Log level filter (default: `phantom=info`) |
| `CLOUDFLARE_API_TOKEN` | For R2 | Cloudflare API access |
| `AWS_ACCESS_KEY_ID` | For AWS | AWS credentials |
| `AWS_SECRET_ACCESS_KEY` | For AWS | AWS credentials |
| `GITHUB_TOKEN` | For GitHub | GitHub API access |

Additional provider credentials are managed through the credential vault after activation.

---

## Building from Source

```bash
# Prerequisites: Rust 1.75+, cargo
git clone https://github.com/benchbrex-USA/BenchBrex-PHANTOM.git
cd BenchBrex-PHANTOM
cargo build --release

# Binary at target/release/phantom-cli
```

Release profile: LTO enabled, single codegen unit, symbols stripped, abort on panic.

---

## License

Proprietary. All rights reserved.

Copyright (c) 2024–2026 Parth Patel / BenchBrex
