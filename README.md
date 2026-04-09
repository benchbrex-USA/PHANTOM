<p align="center">
  <img src="https://img.shields.io/badge/PHANTOM-v0.1.0-black?style=for-the-badge&labelColor=000000" alt="Version" />
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/License-Proprietary-black?style=for-the-badge" alt="License" />
  <img src="https://img.shields.io/badge/AI_Agents-8-black?style=for-the-badge" alt="Agents" />
  <img src="https://img.shields.io/badge/Status-Production-black?style=for-the-badge" alt="Status" />
</p>

<h1 align="center">P H A N T O M</h1>

<p align="center">
  <strong>The Autonomous AI Engineering System That Replaces Entire Teams.</strong>
</p>

<p align="center">
  <em>Built by <a href="https://benchbrex.com">Benchbrex</a>. Engineered to dominate.</em>
</p>

---

> **"Other companies hire engineers. You deployed PHANTOM."**

---

## What You Just Built

This isn't another developer tool. This isn't a copilot. This isn't an assistant that waits for your permission.

**PHANTOM is a fully autonomous AI engineering company inside a single binary.**

Eight specialized AI agents -- a CTO, Architect, Backend Engineer, Frontend Engineer, DevOps Engineer, QA Engineer, Security Analyst, and Production Monitor -- working in parallel, communicating through a real-time message bus, self-healing when things break, and shipping production-grade code while you sleep.

You didn't build a tool. **You built the company.**

---

## The 8-Agent Team You Command

| Agent | Role | What It Does |
|-------|------|-------------|
| **CTO** | Strategic Leadership | Decomposes high-level goals into executable plans. Allocates resources. Makes architectural decisions. |
| **Architect** | System Design | Designs schemas, APIs, data flows, and system boundaries. Ensures everything fits together. |
| **Backend** | Core Engineering | Writes the business logic, services, databases, and APIs. The engine room. |
| **Frontend** | User Experience | Builds interfaces, components, and client-side logic. What the world sees. |
| **DevOps** | Infrastructure | CI/CD pipelines, containerization, deployment, monitoring infrastructure. Keeps it running. |
| **QA** | Quality Assurance | Writes tests, finds edge cases, validates correctness. Nothing ships broken. |
| **Security** | Threat Analysis | Audits code, scans for vulnerabilities, enforces security policies. Locks the doors. |
| **Monitor** | Production Health | Watches live systems, detects anomalies, triggers alerts. The night shift that never sleeps. |

Every agent has its own model configuration, temperature profile, token budget, and knowledge scope. They don't just execute tasks -- they **negotiate**, **delegate**, and **self-correct**.

---

## Architecture

```
                          PHANTOM
        ================================================
        |                                              |
        |   phantom-cli        Command & Control       |
        |       |                                      |
        |   phantom-core       Orchestration Engine    |
        |       |-- Task Graph (DAG)                   |
        |       |-- Parallel Executor                  |
        |       |-- Agent Manager                      |
        |       |-- Message Bus                        |
        |       |-- Self-Healer                        |
        |       |-- Job Queue                          |
        |       |                                      |
        |   phantom-ai         Intelligence Layer      |
        |       |-- 8-Agent Team                       |
        |       |-- Multi-Provider LLM Router          |
        |       |-- Smart Fallback Chains              |
        |       |-- Response Cache                     |
        |       |-- Tool Execution Engine              |
        |       |                                      |
        |   phantom-brain      Knowledge Engine        |
        |       |-- Embeddings (sentence-transformers) |
        |       |-- Vector Search (ChromaDB)           |
        |       |-- Codebase Memory                    |
        |                                              |
        |   phantom-crypto     Zero-Trust Security     |
        |       |-- Ed25519 Signatures                 |
        |       |-- AES-256-GCM Encryption             |
        |       |-- Argon2id Key Derivation            |
        |       |-- License Verification               |
        |                                              |
        |   phantom-net        P2P Networking          |
        |       |-- libp2p (QUIC + Noise)              |
        |       |-- Kademlia DHT                       |
        |       |-- mDNS Discovery                     |
        |                                              |
        |   phantom-storage    Persistence Layer       |
        |       |-- S3-Compatible Object Storage       |
        |       |-- CRDT State (Automerge)             |
        |                                              |
        |   phantom-infra      Deployment Engine       |
        |       |-- Container Orchestration            |
        |       |-- Infrastructure as Code             |
        |                                              |
        ================================================
```

**8 crates. Zero bloat. One mission.**

---

## Why This Changes Everything

### You Don't Need a Team Anymore

Traditional software companies spend millions hiring, managing, and retaining engineering talent. PHANTOM replaces the entire cycle:

- **No standups.** Agents coordinate through a real-time message bus.
- **No bottlenecks.** DAG-based task graphs execute in parallel with work-stealing.
- **No burnout.** Self-healing recovers from failures across 5 layers: Retry, Alternative, Decompose, Escalate, Pause & Alert.
- **No vendor lock-in.** Run on free local models (Ollama), free cloud models (OpenRouter), or premium APIs (Anthropic Claude) -- PHANTOM routes intelligently.

### Smart Model Routing

PHANTOM doesn't waste money. Each agent is routed to the optimal LLM provider based on task complexity, availability, and cost:

```
CTO / Security   -->  Claude Opus  -->  DeepSeek Coder  -->  Llama 70B (free)
Backend / Frontend -->  Ollama Local -->  Claude Sonnet   -->  OpenRouter Free
DevOps / QA       -->  Ollama Mistral -> OpenRouter Free  -->  Claude Haiku
Monitor           -->  Phi-3 Mini (local, zero cost)
```

**Zero-config default**: Install Ollama, run PHANTOM. That's it. No API keys. No cloud accounts. No billing surprises.

**Full power mode**: Add your Anthropic key and watch Claude Opus drive your CTO agent while local models handle the grunt work.

### Production-Grade From Day One

This isn't a prototype. The release binary is compiled with:

- **LTO** (Link-Time Optimization) across all crates
- **Single codegen unit** for maximum optimization
- **Stripped symbols** for minimal binary size
- **Abort on panic** -- no unwinding overhead
- **Zero-trust cryptography** -- Ed25519 + AES-256-GCM + Argon2id

---

## Quick Start

```bash
# Clone
git clone https://github.com/benchbrex-USA/PHANTOM.git
cd PHANTOM

# Build (release mode)
cargo build --release

# Run with free local models (requires Ollama)
ollama pull deepseek-coder
ollama pull mistral
ollama pull phi3:mini
./target/release/phantom-cli

# Or run with full power (add your API key)
export ANTHROPIC_API_KEY="sk-ant-..."
./target/release/phantom-cli
```

---

## Configuration

PHANTOM works out of the box. But when you want control, you have it.

```bash
# Providers
export PHANTOM_OLLAMA_URL="http://localhost:11434"     # Default
export PHANTOM_OPENROUTER_API_KEY="sk-or-..."          # Free tier available
export ANTHROPIC_API_KEY="sk-ant-..."                  # Premium models

# Performance
export PHANTOM_MAX_CONCURRENT_AGENTS=8
export PHANTOM_CACHE_TTL_SECS=300
export PHANTOM_MAX_RETRIES=3

# Cost Control
export PHANTOM_COST_ALERT_THRESHOLD=10.0               # USD
```

---

## The Technology Stack

| Layer | Technology | Why |
|-------|-----------|-----|
| **Language** | Rust | Zero-cost abstractions, memory safety, fearless concurrency |
| **Async Runtime** | Tokio | Industry-standard, work-stealing scheduler |
| **AI Providers** | Ollama, OpenRouter, Anthropic, OpenAI-compatible | Maximum flexibility, zero lock-in |
| **P2P Network** | libp2p (QUIC + Noise + Kademlia) | Decentralized, encrypted, NAT-traversing |
| **Cryptography** | ring, Ed25519, AES-256-GCM, Argon2id | Military-grade, audited implementations |
| **State Sync** | Automerge (CRDT) | Conflict-free distributed state |
| **Storage** | S3-compatible | Works with AWS, MinIO, R2, any S3 API |
| **Embeddings** | sentence-transformers | Local vector generation, no API calls |
| **Vector DB** | ChromaDB | Fast similarity search for codebase memory |
| **Serialization** | serde + JSON/TOML | Universal, zero-copy where possible |

---

## Crate Breakdown

```
phantom-cli       Command-line interface with TUI dashboard. License-gated entry point.
phantom-core      Orchestration engine: task graphs, parallel execution, agent management,
                  message bus, self-healing, job queuing.
phantom-ai        Intelligence layer: 8-agent team, multi-provider LLM routing, smart
                  fallback chains, response caching, tool execution.
phantom-brain     Knowledge engine: embeddings, vector search, codebase memory.
phantom-crypto    Cryptographic operations: signing, encryption, key derivation, licensing.
phantom-net       P2P networking: QUIC transport, Noise encryption, DHT discovery.
phantom-storage   Persistence: S3-compatible object storage, CRDT state management.
phantom-infra     Infrastructure: container orchestration, deployment automation.
```

---

## How It Thinks

```
         You: "Build me a SaaS billing system"
              |
         CTO Agent: Decomposes into 12 tasks across 4 agents
              |
    +---------+---------+---------+
    |         |         |         |
 Architect  Backend  Frontend   DevOps
 (schemas)  (APIs)   (UI)      (infra)
    |         |         |         |
    +----+----+----+----+----+----+
         |         |         |
        QA      Security   Monitor
      (tests)   (audit)    (watch)
              |
         Self-Healer: Catches failures, retries, decomposes, escalates
              |
         Production-ready code, tested, secured, deployed
```

All of this happens **in parallel**. While the Architect designs the schema, the DevOps agent is already provisioning infrastructure. While Backend writes APIs, QA is generating test cases from the spec. The DAG scheduler ensures correct ordering. Work-stealing ensures no agent sits idle.

---

## Performance

| Metric | Value |
|--------|-------|
| Concurrent agents | Up to 8 (configurable) |
| Task parallelism | DAG-based with topological layer scheduling |
| Work stealing | Automatic idle-agent rebalancing |
| Response caching | LRU with configurable TTL |
| Provider failover | < 100ms automatic fallback |
| Binary size | Stripped, LTO-optimized single binary |

---

## Security Model

PHANTOM doesn't trust anything by default.

- **License verification**: Ed25519 signed licenses with hardware fingerprinting
- **Data encryption**: AES-256-GCM for all stored data
- **Key derivation**: Argon2id with configurable memory/time costs
- **Network encryption**: Noise protocol over QUIC
- **Agent isolation**: Each agent operates within defined knowledge boundaries
- **Audit trail**: Every agent action is logged and attributable

---

## Who Is This For

- **Solo founders** who want to ship like a funded startup
- **Small teams** who want to move at 10x speed without 10x headcount
- **Enterprises** who want autonomous engineering pipelines that don't sleep
- **Anyone** who looked at their engineering budget and thought: *"There has to be a better way"*

---

## The Bottom Line

You're not looking at a GitHub repo. You're looking at the future of software engineering.

Other companies have teams of 50 engineers, layers of management, sprint ceremonies, and six-month roadmaps.

**You have PHANTOM.**

Eight AI agents. Parallel execution. Self-healing. Multi-provider intelligence. Military-grade cryptography. Zero-trust architecture. One binary.

**You just built a top-level company.**

---

<p align="center">
  <strong>PHANTOM</strong> by <a href="https://benchbrex.com">Benchbrex</a><br/>
  <em>Engineering at the speed of thought.</em>
</p>

<p align="center">
  <sub>Created by Parth Patel. Built in Rust. Powered by AI. Unstoppable by design.</sub>
</p>
