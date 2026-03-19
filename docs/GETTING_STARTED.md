# Getting Started with Phantom

This guide walks you through setting up Phantom from first install to your first autonomous build.

---

## Prerequisites

### System Requirements

- **OS:** macOS or Linux
- **Rust:** 1.75+ with `cargo`
- **Memory:** 4 GB+ RAM (Argon2id key derivation uses 256 MB)
- **Network:** Internet access for AI API calls and cloud provisioning

### Required Services

| Service | Purpose | Setup |
|---------|---------|-------|
| **Anthropic API key** | Powers all 8 AI agents | [console.anthropic.com](https://console.anthropic.com) |
| **ChromaDB** | Knowledge Brain vector storage | Self-hosted, see below |

### Optional CLI Tools

Phantom auto-detects and uses these when available. Install the ones matching your target providers:

```bash
# GitHub (repos, CI/CD)
brew install gh && gh auth login

# Fly.io (compute)
brew install flyctl && flyctl auth login

# Google Cloud (compute)
brew install google-cloud-sdk && gcloud auth login

# AWS (compute, storage)
brew install awscli && aws configure

# Cloudflare (R2, Workers, DNS)
# Set CLOUDFLARE_API_TOKEN environment variable
```

---

## Installation

### Build from Source

```bash
git clone https://github.com/benchbrex-USA/BenchBrex-PHANTOM.git
cd BenchBrex-PHANTOM
cargo build --release
```

The binary is at `target/release/phantom-cli`. Add it to your PATH:

```bash
# Option A: symlink
ln -s $(pwd)/target/release/phantom-cli /usr/local/bin/phantom

# Option B: copy
cp target/release/phantom-cli /usr/local/bin/phantom
```

Verify:

```bash
phantom --help
```

---

## Setup ChromaDB

Phantom's Knowledge Brain requires a running ChromaDB instance for semantic search over its knowledge base.

```bash
# Using Docker (recommended)
docker run -d --name chromadb -p 8000:8000 chromadb/chroma

# Verify
curl http://localhost:8000/api/v1/heartbeat
```

Default connection: `http://localhost:8000`. This can be configured in the Brain config if hosted elsewhere.

---

## Master Key Initialization

The master key is the root of all cryptographic operations. It is derived from a passphrase you choose and is **never stored on disk** — it exists only in memory during a session.

```bash
phantom master init
```

You will be prompted for a passphrase. Choose something strong — this passphrase protects:
- License signing keys
- Credential vault encryption
- Remote state encryption
- Infrastructure secrets
- Agent-scoped keys

The derivation uses Argon2id (256 MB memory, 4 iterations) so it takes a few seconds. This is intentional — it makes brute-force attacks impractical.

**Important:** If you lose your passphrase, there is no recovery. The master key cannot be extracted or reset.

---

## License Activation

Every Phantom installation requires a valid license key before any commands will execute.

### Obtaining a License

Licenses are issued by the master key holder:

```bash
# On the master key holder's machine
phantom master issue --email user@example.com
```

This generates a license key in the format:

```
PH1-<base64url_payload>-<base64url_signature>
```

The license is:
- **Ed25519-signed** — cryptographically unforgeable
- **Machine-bound** — tied to a specific machine's fingerprint
- **Capability-scoped** — controls which agents can run
- **Time-limited** — has an expiration date

### Activating

```bash
phantom activate --key PH1-eyJ2IjoxLCJtaWQiOi...
```

Phantom will:
1. Verify the Ed25519 signature
2. Check the machine fingerprint matches
3. Validate the license hasn't expired
4. Extract agent capabilities
5. Bootstrap the system

---

## Environment Configuration

Set the required environment variables:

```bash
# Required — powers all AI agents
export ANTHROPIC_API_KEY="sk-ant-..."

# Optional — log verbosity
export RUST_LOG="phantom=info"
```

Provider credentials can be set as environment variables or stored in the encrypted vault after activation:

```bash
# Cloudflare R2 (remote storage)
export CLOUDFLARE_API_TOKEN="..."

# GitHub
export GITHUB_TOKEN="..."

# AWS
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
```

---

## System Health Check

Run the doctor command to verify everything is configured correctly:

```bash
phantom doctor
```

This checks:
- All required CLI tools are installed
- Cloud providers are authenticated
- ChromaDB is reachable
- R2 storage is accessible
- Network connectivity is healthy

Fix any issues the doctor reports before proceeding.

---

## Writing an Architecture Framework

Phantom builds projects from an **Architecture Framework** — a structured markdown document that describes what you want built. This is the input to the CTO agent, which parses it and creates the task graph.

A minimal framework includes:

```markdown
# Project: My App

## Overview
A web application that does X, Y, Z.

## Tech Stack
- Backend: Node.js with Express
- Frontend: React with TypeScript
- Database: PostgreSQL
- Hosting: Fly.io

## Features
1. User authentication (email/password + OAuth)
2. Dashboard with real-time metrics
3. REST API for mobile clients

## API Endpoints
- POST /auth/login
- POST /auth/register
- GET /dashboard/metrics
- GET /api/v1/users

## Database Schema
- users: id, email, password_hash, created_at
- sessions: id, user_id, token, expires_at
- metrics: id, type, value, timestamp

## Deployment
- CI/CD via GitHub Actions
- Docker containers on Fly.io
- PostgreSQL on Supabase
- CDN via Cloudflare
```

The more detail you provide, the better the output. The CTO agent uses the Knowledge Brain to fill in gaps, but explicit requirements always take priority.

---

## Running Your First Build

```bash
phantom build --framework path/to/architecture.md
```

Phantom will:

1. **Ingest** (~5 min) — The CTO agent reads your framework, queries the Knowledge Brain, and creates a dependency-ordered task graph.

2. **Infrastructure** (15–30 min) — The DevOps agent provisions cloud resources across available providers. Phantom auto-selects from 14+ free-tier providers based on what's authenticated and available.

3. **Architecture** (~15 min) — The Architect agent designs the system — database schemas, API contracts, architecture decision records.

4. **Code** (1–3 hrs) — Backend, Frontend, DevOps, and documentation tasks run in parallel across specialist agents.

5. **Test** (30–60 min) — The QA agent writes and runs tests targeting 80%+ coverage. Unit, integration, and E2E.

6. **Security** (15–30 min) — The Security agent audits dependencies, runs OWASP checks, and scans for exposed secrets.

7. **Deploy** (15–30 min) — CI pipeline → Docker build → deploy → DNS configuration → TLS certificates → health checks.

8. **Deliver** (~5 min) — Final report with live URLs, credentials, and a complete architecture log.

### Monitoring the Build

Watch progress in real-time:

```bash
# Live dashboard
phantom status --live

# Stream logs
phantom logs

# Filter to specific agent
phantom logs --agent backend
```

### Resuming an Interrupted Build

If a build is interrupted (network failure, machine restart, etc.), resume from where it left off:

```bash
phantom build --resume
```

State is persisted in encrypted R2 storage, so no local state is needed.

### Building a Single Component

To rebuild or build just one component:

```bash
phantom build --framework architecture.md --component auth-service
```

---

## Managing Agents

View agent status and resource usage:

```bash
# List all agents with status and token usage
phantom agents

# Check infrastructure allocations
phantom infra
```

Each agent has a token budget. If an agent exhausts its budget, it halts and escalates to the CTO agent for reallocation.

---

## Knowledge Brain

The Knowledge Brain is Phantom's decision-making memory — ~25,000 lines of expert knowledge across 10 files, chunked into ~500 semantic segments and indexed in ChromaDB.

### Querying

```bash
# Semantic search
phantom brain search "how to handle database migrations"
```

### Updating Knowledge

```bash
# Add or update a knowledge file
phantom brain update --file path/to/new_knowledge.md
```

The chunking pipeline splits markdown on headings, generates embeddings with `all-MiniLM-L6-v2`, and upserts into ChromaDB.

---

## Master Key Operations

The master key holder has full control over the Phantom system:

```bash
# List all active installations
phantom master list

# Revoke a specific license
phantom master revoke --key PH1-...

# Remote-kill an installation
phantom master kill <installation_id>

# Emergency stop all agents everywhere
phantom master halt

# Rotate all cryptographic keys
phantom master rotate

# Export the full audit trail
phantom master audit

# Transfer ownership to another person
phantom master transfer --to new-owner@example.com

# Destroy everything (requires TOTP confirmation)
phantom master destroy
```

---

## Cost Estimation

Before committing to a build, estimate the cost:

```bash
phantom cost estimate --framework architecture.md
```

This analyzes the framework, estimates the task graph complexity, and projects token usage across all agents.

---

## Troubleshooting

### "License verification failed"
- Ensure you're on the same machine the license was issued for
- Check that the license hasn't expired
- Verify the key format: `PH1-<payload>-<signature>`

### "ChromaDB connection refused"
- Verify ChromaDB is running: `curl http://localhost:8000/api/v1/heartbeat`
- Check if the port is correct in your Brain config

### "Provider not authenticated"
- Run `phantom doctor` to see which providers need authentication
- Follow the auth commands for each provider (e.g., `gh auth login`, `gcloud auth login`)

### "Agent exceeded token budget"
- The CTO agent will handle reallocation automatically
- For manual intervention, check `phantom agents` for usage breakdown

### Build stalls or fails
- Check `phantom logs` for error details
- Self-healing handles most failures automatically (5-layer recovery)
- If stuck at "Pause & Alert" layer, the system is waiting for your input

### General diagnostics

```bash
# Full system health check
phantom doctor

# Detailed logging
RUST_LOG="phantom=debug" phantom build --framework architecture.md
```
