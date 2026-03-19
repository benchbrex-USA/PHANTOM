# PHANTOM v2 — The Autonomous AI Engineering Team
### Architecture Framework v2.0
### Reference: PHANTOM-ARCH-002-2026-03-18

> **Codename:** Phantom
> **Owner:** Parth Patel / Benchbrex
> **Classification:** Proprietary — Master Key Protected

---

> **What Phantom Is (v2 — nothing left out):**
> You install one binary. You insert one license key. You hand it your Architecture Framework.
> From that moment, Phantom owns your terminal. It installs Homebrew. It installs Node.js.
> It installs Python. It installs Docker. It installs PostgreSQL. It installs Redis.
> It creates your GitHub account. It creates your Vercel account. It creates your Supabase project.
> It creates your Cloudflare zone. It sets up DNS. It provisions SSL.
> It reads your entire computer — files, apps, processes, network, keychain, clipboard, screen.
> It writes code. It tests code. It deploys code. It monitors code.
> It finds its own servers. It pays nothing. It heals itself.
> It uses peer-to-peer to survive any single failure.
> It cannot be installed without your license key.
> It cannot be owned without your master key.
> It can be erased from all existence with one command.
> Its brain is 10 expert-level knowledge bases totaling 25,000+ lines of CTO-grade intelligence.
> No one has built this.

---

## Table of Contents

1. [System Identity & Core Laws](#1-system-identity--core-laws)
2. [The Phantom Knowledge Brain — All 10 Vault Files](#2-the-phantom-knowledge-brain--all-10-vault-files)
3. [Full macOS Computer Access Layer](#3-full-macos-computer-access-layer)
4. [Autonomous Dependency Installation Pipeline](#4-autonomous-dependency-installation-pipeline)
5. [Autonomous Account Creation Pipeline](#5-autonomous-account-creation-pipeline)
6. [The 5-Layer Phantom Stack](#6-the-5-layer-phantom-stack)
7. [Security Architecture — Key Hierarchy & Fortress Model](#7-security-architecture--key-hierarchy--fortress-model)
8. [Agent Architecture — The AI Engineering Team](#8-agent-architecture--the-ai-engineering-team)
9. [Zero-Footprint Execution Engine](#9-zero-footprint-execution-engine)
10. [Self-Discovering Infrastructure](#10-self-discovering-infrastructure)
11. [Peer-to-Peer Mesh Layer](#11-peer-to-peer-mesh-layer)
12. [Architecture Framework Ingestion Pipeline](#12-architecture-framework-ingestion-pipeline)
13. [Autonomous Build Pipeline — Spec to Production](#13-autonomous-build-pipeline--spec-to-production)
14. [Self-Healing & Recovery System](#14-self-healing--recovery-system)
15. [Beyond Human — Capabilities Nobody Has Thought Of](#15-beyond-human--capabilities-nobody-has-thought-of)
16. [Terminal Interface & UX](#16-terminal-interface--ux)
17. [Installation & Bootstrap Sequence](#17-installation--bootstrap-sequence)
18. [Master Key Operations & Owner Powers](#18-master-key-operations--owner-powers)
19. [Complete Technical Specification](#19-complete-technical-specification)
20. [CLAUDE.md — The Orchestration Context File](#20-claudemd--the-orchestration-context-file)
21. [Claude Code Prompt Sequence](#21-claude-code-prompt-sequence)
22. [File Manifest & LOC Estimates](#22-file-manifest--loc-estimates)
23. [Anti-Patterns & Failure Modes](#23-anti-patterns--failure-modes)

---

## 1. System Identity & Core Laws

### What Phantom Is

A terminal-native, license-gated, master-key-controlled, zero-footprint, self-provisioning, full-computer-access, knowledge-driven autonomous AI engineering system.

### Core Laws (Immutable — enforced in code, not policy)

| # | Law | How It's Enforced |
|---|-----|-------------------|
| 1 | No installation without a valid license key | Ed25519 signature check at binary entry point. Fails = process exit. |
| 2 | No ownership without the master key | Argon2id-derived 256-bit key. Never stored. All destructive ops require live passphrase entry. |
| 3 | Zero local disk footprint | All persistent state in remote encrypted storage. Local = binary + in-memory session only. |
| 4 | The Knowledge Brain is the source of truth | Every agent decision traces to one of the 10 embedded knowledge files. |
| 5 | Full computer access via terminal only | No GUI. Everything via shell commands, osascript, launchctl, defaults, security CLI. |
| 6 | Self-provisioning infrastructure | Phantom finds, creates, and binds to free-tier servers autonomously. |
| 7 | Self-healing at every layer | 5-layer recovery: retry → alternative → decompose → escalate → pause & alert. |
| 8 | Master key holder has absolute power | Issue/revoke licenses, remote-kill installations, full system destruction. |
| 9 | Every action is audited | Signed audit log. Exportable. Tamper-evident. |
| 10 | No third-party AI APIs in production builds | Self-hosted models via vLLM/Ollama. Anthropic API only for Phantom's own agent reasoning. |

---

## 2. The Phantom Knowledge Brain — All 10 Vault Files

Phantom is not a generic AI coding tool. Its intelligence comes from 10 specific expert-level knowledge bases — your personal vault files — embedded as its permanent context corpus. Every agent queries this corpus before making any decision.

### 2.1 The Knowledge Corpus Map

```
PHANTOM'S BRAIN = 10 Expert Knowledge Files
Total: ~25,000+ lines of CTO-grade intelligence
Loaded into: Vector database (ChromaDB, self-hosted)
Indexed by: Semantic search (sentence-transformers embeddings)
Queried by: Every agent before every decision

┌─────────────────────────────────────────────────────────────────────┐
│  FILE                                        │ LINES │ AGENT USE    │
├─────────────────────────────────────────────────────────────────────┤
│  1. The_CTO_Architecture_Framework           │ 1,333 │ CTO, Architect│
│     → 10 First Principles, Phase 0-1 discovery,      │              │
│       Architecture Pattern Selection, Quality         │              │
│       Attributes, 7 Architecture Layers, ADRs,       │              │
│       Team structure, Review Checklist, Anti-patterns │              │
│                                                                      │
│  2. The_CTO_s_Complete_Technology_Knowledge   │ 3,172 │ ALL AGENTS   │
│     → Mental models, every architecture pattern,     │              │
│       system design at any scale, API design,        │              │
│       frontend/backend/mobile/database engineering,  │              │
│       DevOps, cloud, containers, testing, security,  │              │
│       performance, observability, AI/ML, data eng,   │              │
│       real-time systems, team leadership, disaster   │              │
│       recovery playbook                              │              │
│                                                                      │
│  3. The_Complete_Multi-Agent_Autonomous_System│ 3,255 │ CTO, Monitor │
│     → 4 agent types, 5-layer self-healing,           │              │
│       Claude Code subagents, Agent Teams,            │              │
│       CLAUDE.md orchestration, CrewAI system,        │              │
│       LangGraph workflows, daemon architecture,      │              │
│       job queue, agent memory, monitoring dashboard, │              │
│       software dev agent, social media agent,        │              │
│       business research agent, master orchestrator   │              │
│                                                                      │
│  4. Build_Once._Launch_Directly              │ 1,335 │ CTO, DevOps  │
│     → AI tool stack (Claude Code, Codex, Cursor),    │              │
│       Phase 0 Master Spec, project scaffolding,      │              │
│       context files, feature development with agents,│              │
│       testing strategy, security hardening,          │              │
│       performance optimization, CI/CD automation,    │              │
│       production launch checklist, anti-patterns,    │              │
│       master prompt patterns                         │              │
│                                                                      │
│  5. The_Complete_Full-Stack_Software_Blueprint│   339 │ Backend,     │
│     → Modular Monolith default, Layer 1-10 stack,    │ Frontend     │
│       foundations, frontend, backend, database,      │              │
│       API, auth, testing, DevOps, monitoring,        │              │
│       security layers                                │              │
│                                                                      │
│  6. Every_Technology_Used_to_Build_Software   │ 1,475 │ Architect,   │
│     → Every language + usage stats, every framework, │ CTO          │
│       every database, every DevOps tool, every cloud │              │
│       platform, every container tool, every testing  │              │
│       framework, every monitoring tool, every        │              │
│       security tool, every IDE                       │              │
│                                                                      │
│  7. The_Complete_Design_Expert_Knowledge_Base │ 2,890 │ Frontend     │
│     → Designer mental models, color theory, typography│             │
│       spacing/layout/8pt grid, design tokens,        │              │
│       component architecture, dark mode, WCAG 2.2 AA,│              │
│       glassmorphism, bento grid, claymorphism,       │              │
│       aurora UI, neomorphism, variable fonts,        │              │
│       micro-interactions, 3D depth, navigation,      │              │
│       form design, data viz, empty states,           │              │
│       AI-assisted design workflow, design-to-code    │              │
│                                                                      │
│  8. The_Complete_AI_ML_Expert_Knowledge_Base  │ 3,558 │ CTO (ML      │
│     → ML mental models, mathematics, framework stack,│ decisions)   │
│       hardware, classical ML, neural nets, training, │              │
│       CNNs, RNNs, transformers, GPT from scratch,   │              │
│       tokenization, pre-training, fine-tuning        │              │
│       (LoRA/QLoRA/RLHF), computer vision, speech,   │              │
│       diffusion models, multimodal, RL, evaluation,  │              │
│       optimization, self-hosted inference (vLLM),    │              │
│       MLOps production pipeline                      │              │
│                                                                      │
│  9. The_Complete_API_Expert_Knowledge_Base    │ 3,368 │ Backend,     │
│     → API mental model, REST/GraphQL/WebSocket/gRPC, │ DevOps       │
│       every auth pattern, BaseAPIClient architecture,│              │
│       social media APIs, AI APIs, communication APIs,│              │
│       payment APIs, cloud infra APIs, maps APIs,     │              │
│       auth/identity APIs, storage APIs, analytics    │              │
│       APIs, search APIs, productivity APIs,          │              │
│       Claude Code API workflow, Codex integration,   │              │
│       AgentAPI, unified gateway pattern, webhooks,   │              │
│       rate limiting/retry/circuit breaker,           │              │
│       credential management, unified social poster   │              │
│                                                                      │
│  10. AI_Code_GitHub_Errors_Fixes             │ 1,692 │ QA, DevOps   │
│     → Why AI code fails on GitHub, 10 error classes, │              │
│       pre-push checklist, GitHub Actions pipeline,   │              │
│       build/compile failures, type errors, lint,     │              │
│       test failures, dependency errors, env/secrets, │              │
│       database/migration errors, security scans,     │              │
│       Docker/container errors, deployment failures,  │              │
│       auto-healing CI, branch protection rules,      │              │
│       AI agent pre-push contract, error investigation│              │
│       playbook, CI health dashboard, CTO's 10 rules │              │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 How Agents Use the Knowledge Brain

```
QUERY FLOW:

  Agent receives task
       │
       ▼
  Agent generates semantic query from task description
       │
       ▼
  ChromaDB returns top-K relevant chunks from the 10 knowledge files
       │
       ▼
  Relevant chunks injected into agent's context window as:
    "KNOWLEDGE REFERENCE (source: <filename>, section: <heading>):"
    "<chunk content>"
       │
       ▼
  Agent makes decision GROUNDED in the knowledge base
       │
       ▼
  Agent's output CITES which knowledge section it used:
    "Decision based on: CTO Architecture Framework, Law 2 — Simple Beats Clever"

EXAMPLES:

  Backend Agent needs to design an API:
  → Queries: "API design patterns REST authentication"
  → Gets: API Expert KB §4 (BaseAPIClient), §3 (Auth Patterns), §22 (Rate Limiting)
  → Builds API following YOUR established BaseAPIClient pattern with retry + circuit breaker

  Frontend Agent needs to build a dashboard:
  → Queries: "dashboard design responsive dark mode accessibility"
  → Gets: Design Expert KB §5 (Design Tokens), §7 (Dark Mode), §8 (WCAG 2.2 AA)
  → Builds UI with YOUR 8pt grid, YOUR CSS custom properties, YOUR semantic tokens

  DevOps Agent needs to set up CI/CD:
  → Queries: "GitHub Actions CI pipeline AI code errors"
  → Gets: AI Code Errors KB §4 (Right Pipeline), §17 (Pre-Push Contract)
  → Sets up CI following YOUR error taxonomy and branch protection rules

  CTO Agent needs to choose architecture:
  → Queries: "architecture pattern selection modular monolith"
  → Gets: CTO Framework §4 (Pattern Selection), Full-Stack Blueprint §1 (Modular Monolith)
  → Chooses architecture following YOUR first principles
```

### 2.3 Knowledge Embedding Pipeline

```
AT FIRST ACTIVATION:

1. Phantom reads all 10 MD files from the knowledge vault
2. Splits each file into semantic chunks (by section heading, ~500 tokens each)
3. Generates embeddings using sentence-transformers (all-MiniLM-L6-v2, self-hosted)
4. Stores in ChromaDB collection on the remote encrypted server
5. Creates metadata index: {filename, section_heading, line_range, agent_tags[]}

Total chunks: ~500 (across all 10 files)
Embedding dimensions: 384
Storage: ~2MB (negligible)
Query latency: <50ms

ON KNOWLEDGE UPDATE:
  phantom brain update --file <new_or_updated_md_file>
  → Re-chunks the file
  → Re-embeds changed chunks
  → Updates ChromaDB collection
  → All agents immediately have access to new knowledge
```

---

## 3. Full macOS Computer Access Layer

Phantom doesn't just write code. It controls your entire macOS computer from the terminal. Every capability macOS exposes through command-line tools, Phantom uses.

### 3.1 The macOS Terminal Access Matrix

```
WHAT PHANTOM CAN DO ON YOUR MAC (all via terminal, no GUI):

┌─────────────────────────────────────────────────────────────────────────┐
│  CATEGORY          │  HOW                          │  PHANTOM USES FOR  │
├─────────────────────────────────────────────────────────────────────────┤
│  FILE SYSTEM       │  ls, find, cat, cp, mv, rm,  │  Project scaffold, │
│                    │  mkdir, chmod, chown, xattr,  │  config files,     │
│                    │  ditto, rsync, fd, rg         │  search, organize  │
│                    │                                │                    │
│  PROCESS MGMT      │  ps, kill, pkill, lsof,      │  Kill stuck procs, │
│                    │  top, htop, launchctl,        │  manage daemons,   │
│                    │  nohup, disown               │  background tasks  │
│                    │                                │                    │
│  NETWORK           │  curl, wget, ssh, scp, sftp, │  API calls, server │
│                    │  nc, dig, nslookup, ping,     │  provisioning,     │
│                    │  traceroute, ifconfig,        │  DNS setup, health │
│                    │  networksetup, scutil         │  checks, tunneling │
│                    │                                │                    │
│  PACKAGE MANAGERS  │  brew, npm, npx, pip, pip3,  │  Install EVERYTHING│
│                    │  cargo, gem, go install,      │  No manual setup   │
│                    │  composer, apt (Linux)        │                    │
│                    │                                │                    │
│  SYSTEM INFO       │  uname, sw_vers, sysctl,     │  Environment       │
│                    │  system_profiler, ioreg,      │  detection, machine│
│                    │  diskutil, df, du             │  fingerprinting    │
│                    │                                │                    │
│  APPLE AUTOMATION  │  osascript (AppleScript/JXA), │  Browser auto,     │
│                    │  shortcuts (macOS Shortcuts),  │  app control,      │
│                    │  automator                    │  UI scripting      │
│                    │                                │                    │
│  KEYCHAIN          │  security find-generic-password│  Retrieve stored   │
│                    │  security add-generic-password │  credentials,      │
│                    │  security find-internet-password│ store new ones    │
│                    │                                │                    │
│  CLIPBOARD         │  pbcopy, pbpaste              │  Copy/paste data   │
│                    │                                │  between contexts  │
│                    │                                │                    │
│  SCREEN/DISPLAY    │  screencapture,               │  Visual verification│
│                    │  osascript (window position),  │  UI testing,       │
│                    │  defaults (dock, spaces)       │  screenshots       │
│                    │                                │                    │
│  SYSTEM PREFS      │  defaults read/write,         │  Configure Mac     │
│                    │  systemsetup, pmset,           │  settings: power,  │
│                    │  networksetup, scutil          │  network, display  │
│                    │                                │                    │
│  CRON / LAUNCHD    │  crontab, launchctl,          │  Self-scheduling,  │
│                    │  launchd plist creation        │  background daemon │
│                    │                                │                    │
│  DOCKER            │  docker, docker compose,      │  Container builds, │
│                    │  docker buildx                │  local dev envs    │
│                    │                                │                    │
│  GIT               │  git (all operations),        │  Version control,  │
│                    │  gh (GitHub CLI)              │  PRs, releases     │
│                    │                                │                    │
│  DATABASE          │  psql, redis-cli, mongosh,    │  Schema setup,     │
│                    │  sqlite3                      │  migrations, data  │
│                    │                                │                    │
│  BROWSER CONTROL   │  osascript → Safari/Chrome,   │  Account creation, │
│                    │  open -a "App" URL,            │  OAuth flows,      │
│                    │  /usr/bin/open                 │  verification      │
│                    │                                │                    │
│  SSH KEYS          │  ssh-keygen, ssh-add,         │  Server auth,      │
│                    │  ssh-copy-id, ssh config      │  GitHub deploy keys│
│                    │                                │                    │
│  CERTIFICATES      │  openssl, security (Keychain),│  TLS setup, code   │
│                    │  codesign                     │  signing           │
│                    │                                │                    │
│  NOTIFICATIONS     │  osascript → display dialog,  │  Alert owner,      │
│                    │  terminal-notifier,            │  progress updates  │
│                    │  say (text-to-speech)         │                    │
│                    │                                │                    │
│  DISK & VOLUMES    │  diskutil, mount, umount,     │  Detect USB,       │
│                    │  hdiutil                      │  create disk images│
│                    │                                │                    │
│  XCODE CLI         │  xcode-select, xcrun,         │  iOS builds,       │
│                    │  xcodebuild, swift             │  native compilation│
│                    │                                │                    │
│  TEXT PROCESSING   │  sed, awk, grep, jq, yq,     │  Config parsing,   │
│                    │  perl, python -c               │  data transform    │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Browser Automation via Terminal

```
HOW PHANTOM CREATES ACCOUNTS WITHOUT A GUI:

Method 1: AppleScript → Safari/Chrome (for sites requiring browser)
  osascript -e '
    tell application "Safari"
      activate
      open location "https://github.com/signup"
      delay 3
      do JavaScript "document.getElementById(\"email\").value = \"parth@benchbrex.com\"" in current tab
      do JavaScript "document.getElementById(\"email\").form.submit()" in current tab
    end tell
  '

Method 2: curl + API (for services with signup APIs)
  curl -X POST https://api.github.com/user \
    -H "Content-Type: application/json" \
    -d '{"login":"benchbrex","email":"parth@benchbrex.com"}'

Method 3: CLI tools (for services with official CLIs)
  gh auth login                    # GitHub CLI
  vercel login                     # Vercel CLI
  supabase login                   # Supabase CLI
  wrangler login                   # Cloudflare CLI
  flyctl auth login                # Fly.io CLI
  railway login                    # Railway CLI
  firebase login                   # Firebase CLI

Method 4: Playwright/Puppeteer headless (for complex signup flows)
  → Phantom installs Playwright via npm
  → Runs headless Chromium for multi-step signups
  → Handles CAPTCHA by pausing and asking owner to solve on their device
  → Stores credentials in encrypted vault immediately after creation

PERMISSION: Owner approves each account creation via terminal prompt:
  ┌─────────────────────────────────────────────────────────────┐
  │  Phantom needs to create the following accounts:             │
  │                                                             │
  │  ✓ GitHub (code hosting + CI/CD)                           │
  │  ✓ Vercel (frontend deployment)                            │
  │  ✓ Supabase (PostgreSQL + auth)                            │
  │  ✓ Cloudflare (DNS + CDN + R2 storage)                     │
  │  ✓ Upstash (Redis cache + queue)                           │
  │                                                             │
  │  Email to use: parth@benchbrex.com                         │
  │  [Approve All]  [Select Individually]  [Cancel]            │
  └─────────────────────────────────────────────────────────────┘
```

### 3.3 macOS Keychain Integration

```
CREDENTIAL STORAGE:
  Phantom uses macOS Keychain as a SECURE BRIDGE (not primary storage).
  
  Store:
    security add-generic-password \
      -a "phantom" \
      -s "phantom-session-token" \
      -w "<encrypted_session_token>" \
      -T /usr/local/bin/phantom

  Retrieve:
    security find-generic-password \
      -a "phantom" \
      -s "phantom-session-token" \
      -w

  Delete (on deactivation):
    security delete-generic-password \
      -a "phantom" \
      -s "phantom-session-token"

  This means: even the session token is stored in Keychain (not disk),
  protected by the macOS login password, and accessible only to the
  Phantom binary (via -T flag Access Control List).
```

---

## 4. Autonomous Dependency Installation Pipeline

When Phantom first runs, it ensures your Mac has everything needed — without you touching anything except pressing Enter when prompted.

### 4.1 The Installation Sequence

```
phantom activate --key PH1-xxxxx

PHASE 1: SYSTEM PREREQUISITES (auto-detected, auto-installed)

Step 1: Xcode Command Line Tools
  IF: xcode-select -p fails
  THEN: xcode-select --install
  WAIT: Owner clicks "Install" on macOS dialog (unavoidable Apple requirement)
  VERIFY: xcode-select -p returns valid path

Step 2: Homebrew
  IF: which brew fails
  THEN: /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  CONFIGURE: Add to PATH in ~/.zshrc or ~/.bash_profile
  VERIFY: brew --version

Step 3: Core tools via Homebrew
  brew install git curl wget jq yq ripgrep fd tree htop
  brew install openssl libsodium argon2
  brew install gh               # GitHub CLI
  brew install --cask docker    # Docker Desktop

Step 4: Node.js (via nvm for version management)
  IF: which node fails OR node --version < 20
  THEN: curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash
        source ~/.nvm/nvm.sh
        nvm install 20
        nvm use 20
        nvm alias default 20
  VERIFY: node --version → v20.x.x

Step 5: Python (via pyenv for version management)
  IF: which python3 fails OR python3 --version < 3.11
  THEN: brew install pyenv
        pyenv install 3.12
        pyenv global 3.12
  VERIFY: python3 --version → 3.12.x

Step 6: Rust (for Phantom's own binary and tools)
  IF: which rustc fails
  THEN: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source ~/.cargo/env
  VERIFY: rustc --version

Step 7: PostgreSQL client
  brew install postgresql@16
  VERIFY: psql --version

Step 8: Redis client
  brew install redis
  VERIFY: redis-cli --version

Step 9: Docker verification
  IF: docker info fails
  THEN: open -a Docker    # Launch Docker Desktop
  WAIT: Until docker info succeeds (poll every 5 seconds, max 60 seconds)
  VERIFY: docker compose version


PHASE 2: AI TOOLS (for Phantom's own reasoning)

Step 10: Claude Code (if not installed)
  npm install -g @anthropic-ai/claude-code
  VERIFY: claude --version

Step 11: Python AI dependencies (in isolated venv)
  python3 -m venv ~/.phantom/venv
  source ~/.phantom/venv/bin/activate
  pip install sentence-transformers chromadb anthropic httpx
  VERIFY: python3 -c "import chromadb; print('OK')"


PHASE 3: DEPLOYMENT TOOLS

Step 12: Platform CLIs
  npm install -g vercel
  npm install -g supabase
  npm install -g wrangler          # Cloudflare
  npm install -g @railway/cli
  npm install -g netlify-cli
  VERIFY: Each CLI responds to --version


PHASE 4: VERIFICATION

  phantom doctor

  ┌─ PHANTOM DOCTOR ─────────────────────────────────────────────┐
  │                                                               │
  │  SYSTEM PREREQUISITES                                        │
  │  ✓ Xcode CLI Tools      15.4                                │
  │  ✓ Homebrew              4.4.x                              │
  │  ✓ Git                   2.45.x                             │
  │  ✓ curl                  8.x                                │
  │                                                               │
  │  RUNTIMES                                                    │
  │  ✓ Node.js               v20.18.x (via nvm)                │
  │  ✓ Python                3.12.x (via pyenv)                 │
  │  ✓ Rust                  1.82.x                             │
  │                                                               │
  │  DATABASES                                                   │
  │  ✓ PostgreSQL client     16.x                               │
  │  ✓ Redis client          7.x                                │
  │  ✓ Docker                27.x (running)                     │
  │                                                               │
  │  AI TOOLS                                                    │
  │  ✓ Claude Code           1.x                                │
  │  ✓ ChromaDB              0.5.x (Python)                     │
  │  ✓ sentence-transformers 3.x                                │
  │                                                               │
  │  DEPLOYMENT CLIs                                             │
  │  ✓ Vercel CLI            37.x                               │
  │  ✓ Supabase CLI          1.x                                │
  │  ✓ Wrangler              3.x                                │
  │  ✓ GitHub CLI            2.x                                │
  │                                                               │
  │  STATUS: ALL 18 DEPENDENCIES VERIFIED ✓                     │
  └──────────────────────────────────────────────────────────────┘

IMPORTANT: Phantom asks for permission before installing ANYTHING.
Each step shows what will be installed and why, and waits for Enter.
Owner can skip any step if already installed differently.
```

---

## 5. Autonomous Account Creation Pipeline

### 5.1 The Account Matrix

```
Phantom creates these accounts AUTONOMOUSLY (with owner approval):

┌────────────────────┬──────────────────┬──────────────────────────────────┐
│  SERVICE           │  METHOD          │  WHAT PHANTOM DOES               │
├────────────────────┼──────────────────┼──────────────────────────────────┤
│  GitHub            │  gh auth login   │  Create repo, configure SSH keys,│
│                    │  + browser OAuth │  set up Actions secrets, branch  │
│                    │                  │  protection, deploy keys         │
│                    │                  │                                  │
│  Vercel            │  vercel login    │  Link project, configure domains,│
│                    │  + browser OAuth │  set env vars, deploy            │
│                    │                  │                                  │
│  Supabase          │  supabase login  │  Create project, run migrations, │
│                    │  + browser OAuth │  configure RLS, create API keys  │
│                    │                  │                                  │
│  Cloudflare        │  wrangler login  │  Create zone, configure DNS,     │
│                    │  + browser OAuth │  setup R2 bucket, Workers, SSL   │
│                    │                  │                                  │
│  Upstash           │  API signup      │  Create Redis database, get      │
│                    │  via curl        │  connection string               │
│                    │                  │                                  │
│  Oracle Cloud      │  OCI CLI + web   │  Create free-tier VM, configure  │
│                    │  signup flow     │  networking, install Phantom     │
│                    │                  │  daemon on server                │
│                    │                  │                                  │
│  Google Cloud      │  gcloud auth     │  Create e2-micro VM, configure   │
│                    │  login           │  firewall, install Phantom daemon│
│                    │                  │                                  │
│  Fly.io            │  flyctl auth     │  Create apps, configure volumes, │
│                    │  login           │  deploy containers               │
│                    │                  │                                  │
│  Railway           │  railway login   │  Create project, deploy services │
│                    │                  │                                  │
│  Neon              │  API signup      │  Create serverless Postgres,     │
│                    │  via curl        │  branch per environment          │
│                    │                  │                                  │
│  Resend            │  API key via     │  Configure transactional email,  │
│                    │  web signup      │  verify domain                   │
│                    │                  │                                  │
│  Sentry            │  API + CLI       │  Create project, configure DSN,  │
│                    │                  │  setup source maps               │
│                    │                  │                                  │
│  Let's Encrypt     │  certbot CLI     │  Auto-provision SSL certificates │
│                    │  (via Cloudflare │  for all domains                 │
│                    │  DNS challenge)  │                                  │
└────────────────────┴──────────────────┴──────────────────────────────────┘
```

### 5.2 Credential Lifecycle

```
1. Account created → credentials captured immediately
2. Credentials encrypted with session key (AES-256-GCM)
3. Encrypted blob stored in remote vault (never local disk)
4. Also stored in macOS Keychain as backup (encrypted)
5. Credentials injected as env vars when agents need them
6. Credentials auto-rotated every 90 days
7. On destruction: all credentials revoked + accounts deleted
```

---

## 6. The 5-Layer Phantom Stack

```
┌─────────────────────────────────────────────────────────────────────────┐
│  LAYER 5: TERMINAL INTERFACE                                            │
│  Commander.js CLI + ratatui dashboard. One command in, live progress.   │
├─────────────────────────────────────────────────────────────────────────┤
│  LAYER 4: AGENT ORCHESTRATION + KNOWLEDGE BRAIN                         │
│  CTO + 7 specialist agents. ChromaDB knowledge retrieval.              │
│  Parallel execution. Self-healing. Job queue. Agent memory.            │
├─────────────────────────────────────────────────────────────────────────┤
│  LAYER 3: SECURITY & KEY MANAGEMENT                                     │
│  License gate (Ed25519). Master key vault (Argon2id).                  │
│  E2E encryption (AES-256-GCM). Zero-knowledge. Tamper detection.      │
├─────────────────────────────────────────────────────────────────────────┤
│  LAYER 2: ZERO-FOOTPRINT + FULL COMPUTER ACCESS                         │
│  No local storage. Full macOS terminal control. Keychain integration.  │
│  Stream processing. Remote-only state.                                  │
├─────────────────────────────────────────────────────────────────────────┤
│  LAYER 1: SELF-DISCOVERING INFRASTRUCTURE + P2P MESH                    │
│  Auto-provisions 14+ free-tier providers. libp2p mesh.                 │
│  CRDT state sync. Lifetime server binding. Auto-migration.             │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Security Architecture — Key Hierarchy & Fortress Model

### 7.1 Key Hierarchy

```
MASTER KEY (Parth Patel only — passphrase → Argon2id → 256-bit key)
│
├── LICENSE SIGNING KEY (Ed25519)
│   └── LICENSE TOKEN (per-machine, hardware-bound)
│       └── SESSION KEY (ephemeral, in-memory only)
│           └── AGENT KEYS (per-agent, per-task, scoped, time-limited)
│
├── INFRASTRUCTURE KEY (cloud credentials encryption)
│   └── SERVER BIND TOKENS (cryptographic ownership proof per server)
│
└── DESTRUCTION KEY (master key + passphrase + TOTP 2FA)
```

### 7.2 License Key Gate

```
LICENSE FORMAT: PH1-<base62_payload>-<ed25519_signature>

PAYLOAD:
{
  "v": 1,
  "mid": "<machine_fingerprint_hmac_sha256>",
  "iat": 1742256000,
  "exp": 1773792000,
  "cap": ["cto","architect","backend","frontend","devops","qa","security","monitor"],
  "tier": "founder"
}

MACHINE FINGERPRINT = HMAC-SHA256(
  key: license_salt,
  data: MAC_address || CPU_serial || disk_UUID || OS_install_UUID
)

VERIFICATION AT EVERY LAUNCH:
  1. Extract payload and signature from license key
  2. Verify Ed25519 signature against embedded public key
  3. Compute current machine fingerprint
  4. Compare fingerprint against payload.mid
  5. Check expiration against NTP-verified time
  6. ANY failure → "Invalid license" → process exit code 1
```

### 7.3 Master Key Powers

```
phantom master issue --email <e>          Issue new license
phantom master revoke --key <k>           Revoke license
phantom master list                       List all installations
phantom master kill <id>                  Remote-kill installation
phantom master destroy                    Full erasure (requires TOTP 2FA)
phantom master rotate                     Rotate all keys
phantom master audit                      Export audit log
phantom master transfer --to <pubkey>     Transfer ownership
phantom master halt                       Emergency stop all agents
```

### 7.4 Anti-Hack Architecture

| Threat | Mitigation |
|--------|-----------|
| Binary reverse engineering | Control flow flattening, string encryption, anti-debug traps, SHA-256 self-integrity check |
| License forgery | Ed25519 — infeasible without private key |
| Memory dump | mlock'd pages, zeroed after use, no swap |
| Network interception | TLS 1.3 + certificate pinning + mTLS between agents |
| Server compromise | Zero-knowledge encryption — servers are dumb storage |
| Rogue agent | Scoped permissions, signed actions, audit trail |
| Time manipulation | 3+ NTP sources, server-side timestamp backup |
| Supply chain | Single static binary, zero runtime deps, signed releases |

---

## 8. Agent Architecture — The AI Engineering Team

### 8.1 The Team

```
                    ┌──────────────────────────────┐
                    │      PHANTOM CTO AGENT        │
                    │  Reads Architecture Framework │
                    │  Queries Knowledge Brain      │
                    │  Decomposes → delegates →     │
                    │  monitors → synthesizes       │
                    └──────────────┬───────────────┘
                                   │
        ┌──────────┬──────────┬────┴────┬──────────┬──────────┬──────────┐
        ▼          ▼          ▼         ▼          ▼          ▼          ▼
   ┌─────────┐┌─────────┐┌─────────┐┌─────────┐┌─────────┐┌─────────┐┌─────────┐
   │ARCHITECT││BACKEND  ││FRONTEND ││ DEVOPS  ││   QA    ││SECURITY ││ MONITOR │
   │         ││         ││         ││         ││         ││         ││         │
   │KB refs: ││KB refs: ││KB refs: ││KB refs: ││KB refs: ││KB refs: ││KB refs: │
   │CTO Arch ││API Exp  ││Design   ││Build    ││AI Code  ││CTO Tech ││Multi-   │
   │CTO Tech ││CTO Tech ││Expert   ││Once     ││Errors   ││Full-Stk ││Agent    │
   │Full-Stk ││Full-Stk ││CTO Tech ││AI Code  ││CTO Tech ││CTO Arch ││CTO Tech │
   │Every    ││AI/ML    ││Full-Stk ││CTO Tech ││Full-Stk ││AI/ML    ││Build    │
   │Tech     ││         ││         ││Every    ││         ││         ││Once     │
   │         ││         ││         ││Tech     ││         ││         ││         │
   └─────────┘└─────────┘└─────────┘└─────────┘└─────────┘└─────────┘└─────────┘
```

### 8.2 What Each Agent Does (with Knowledge Brain references)

```
CTO AGENT (Orchestrator):
  Model: Claude Opus | Temperature: 0.3
  Knowledge: ALL 10 files (full corpus access)
  Actions:
    - Parse Architecture Framework into task graph
    - Apply CTO First Principles (CTO Arch Framework §1)
    - Choose architecture pattern (CTO Arch Framework §4)
    - Decompose into parallel work streams
    - Monitor all agents, retry failures
    - Apply Build Once Launch Directly phases (Build Once §3-11)
    - Final synthesis and delivery

ARCHITECT AGENT:
  Model: Claude Opus | Temperature: 0.2
  Knowledge: CTO Arch Framework, CTO Tech KB, Full-Stack Blueprint, Every Technology
  Actions:
    - Generate system design following Quality Attributes (CTO Arch §5)
    - Select tech stack using Technology Decision Framework (CTO Tech §22)
    - Design DB schema following Data Architecture layer (CTO Arch §8)
    - Generate OpenAPI 3.1 spec following API Design patterns (CTO Tech §6)
    - Write ADRs following Governance patterns (CTO Arch §15)

BACKEND AGENT:
  Model: Claude Sonnet | Temperature: 0.1
  Knowledge: API Expert KB, CTO Tech KB, Full-Stack Blueprint, AI/ML KB
  Actions:
    - Build FastAPI following domain module pattern (API Expert §4 BaseAPIClient)
    - Implement auth with retry + circuit breaker (API Expert §22)
    - Database models with SQLAlchemy 2.0 (CTO Tech §10)
    - Background jobs following async patterns (CTO Tech §20)
    - Self-hosted model integration via vLLM (AI/ML KB §22)
    - All credentials via env vars (API Expert §23)

FRONTEND AGENT:
  Model: Claude Sonnet | Temperature: 0.1
  Knowledge: Design Expert KB, CTO Tech KB, Full-Stack Blueprint
  Actions:
    - Build Next.js with design tokens (Design Expert §5)
    - 8pt grid spacing system (Design Expert §4)
    - Dark mode with semantic CSS tokens (Design Expert §7)
    - WCAG 2.2 AA compliance (Design Expert §8)
    - Mobile-first responsive (375px minimum) (Design Expert §4)
    - Component architecture (Design Expert §6)
    - Current design trends where appropriate (Design Expert §9-16)

DEVOPS AGENT:
  Model: Claude Sonnet | Temperature: 0.1
  Knowledge: Build Once KB, AI Code Errors KB, CTO Tech KB, Every Technology
  Actions:
    - Docker multi-stage builds (CTO Tech §13)
    - GitHub Actions CI following AI error prevention (AI Errors §4)
    - Pre-push contract enforcement (AI Errors §17)
    - Branch protection rules (AI Errors §16)
    - Auto-healing CI (AI Errors §15)
    - Infrastructure as code (CTO Tech §11)
    - Production launch checklist (Build Once §11)

QA AGENT:
  Model: Claude Sonnet | Temperature: 0.1
  Knowledge: AI Code Errors KB, CTO Tech KB, Full-Stack Blueprint
  Actions:
    - Write tests for all 10 error classes (AI Errors §5-14)
    - pytest + Vitest + Playwright following testing strategy (CTO Tech §14)
    - 80%+ coverage enforcement
    - Error investigation playbook (AI Errors §18)
    - CTO's 10 rules for AI code in production (AI Errors §20)

SECURITY AGENT:
  Model: Claude Opus | Temperature: 0.2
  Knowledge: CTO Tech KB, CTO Arch Framework, Full-Stack Blueprint, AI/ML KB
  Actions:
    - Security as architecture (CTO Arch §10, Law 5)
    - OWASP Top 10 verification (CTO Tech §15)
    - Dependency audit (AI Errors §12)
    - Auth flow audit following Security Architecture (CTO Arch §10)
    - AI model security (AI/ML KB — supply chain, model provenance)

MONITOR AGENT:
  Model: Claude Haiku | Temperature: 0.0
  Knowledge: Multi-Agent System KB, CTO Tech KB, Build Once KB
  Actions:
    - 5-layer self-healing protocol (Multi-Agent §3)
    - Permanent daemon operation (Multi-Agent §12)
    - System health monitoring (CTO Tech §17)
    - Cost tracking (CTO Tech, Mental Model 2: Four Cost Dimensions)
    - Disaster recovery playbook (CTO Tech §23)
```

---

## 9. Zero-Footprint Execution Engine

```
LOCAL MACHINE:
  /usr/local/bin/phantom    (binary, ~15MB)
  macOS Keychain             (encrypted session token)
  RAM only                   (session key, agent contexts — zeroed on exit)
  DISK: ZERO FILES

REMOTE (encrypted, distributed across P2P mesh):
  Server A: encrypted_state.blob (project files, git repo)
  Server B: encrypted_config.blob (agent configs, task graph)
  Server C: encrypted_secrets.blob (API keys, credentials)
  Server D: knowledge_brain.blob (ChromaDB vectors)

ALL data encrypted with AES-256-GCM, session-derived keys.
Servers cannot decrypt. They are dumb encrypted blob storage.
```

---

## 10. Self-Discovering Infrastructure

```
14+ FREE-TIER PROVIDERS:
  Oracle Cloud     →  2 VMs + 200GB (primary compute)
  Google Cloud     →  e2-micro (secondary)
  AWS Free Tier    →  t2.micro 12mo (backup)
  Cloudflare       →  Workers + R2 + DNS (edge)
  Fly.io           →  3 shared VMs (P2P mesh)
  Railway          →  $5/mo credit (ephemeral builds)
  Vercel           →  Serverless (frontend)
  Netlify          →  100GB bw/mo (fallback frontend)
  Supabase         →  500MB PG (database)
  Neon             →  0.5GB PG (backup database)
  Upstash          →  10K cmd/day (Redis)
  Render           →  Static + DB (static hosting)
  GitHub           →  Unlimited repos (code + CI/CD)
  Cloudflare R2    →  10GB (encrypted blob storage)

TOTAL MONTHLY COST: $0.00
REDUNDANCY: 3x minimum (survives 2 provider failures)
```

---

## 11. Peer-to-Peer Mesh Layer

```
PROTOCOL STACK:
  Transport:   QUIC (UDP, NAT-traversal friendly)
  Security:    Noise protocol (XX handshake)
  Identity:    Ed25519 peer IDs
  Discovery:   Kademlia DHT + mDNS (local)
  Sync:        CRDT (Automerge) — conflict-free replication
  Encryption:  ChaCha20-Poly1305

WHAT SYNCS:       project state, task graph, infra bindings, audit log, health metrics
WHAT NEVER SYNCS: master key, session keys, raw credentials
```

---

## 12. Architecture Framework Ingestion Pipeline

```
phantom build --framework ./my-architecture.md

Step 1: PARSE → extract headings, sections, tables, code blocks, constraints
Step 2: EXTRACT → components, technologies, patterns, API contracts, DB models
Step 3: GRAPH → build dependency DAG (what depends on what)
Step 4: ENRICH → query Knowledge Brain for best practices per component
Step 5: PLAN → generate task graph with parallel streams + time estimates
Step 6: PRESENT → show plan to owner for approval
Step 7: EXECUTE → spawn agents, build everything
```

---

## 13. Autonomous Build Pipeline — Spec to Production

```
PHASE 0: INGEST (5 min) — Parse framework, plan
PHASE 1: INFRASTRUCTURE (15-30 min) — Provision servers, create accounts, setup CI/CD
PHASE 2: ARCHITECTURE (15 min) — System design, DB schema, API contracts, ADRs
PHASE 3: CODE (1-3 hours) — 4 parallel streams: Backend + Frontend + DevOps + Docs
PHASE 4: TEST (30-60 min) — Unit + integration + E2E, 80%+ coverage
PHASE 5: SECURITY (15-30 min) — Dependency audit, OWASP, auth review, secret scan
PHASE 6: DEPLOY (15-30 min) — Push → CI → Docker build → deploy → DNS → TLS → health
PHASE 7: DELIVER (5 min) — Report, URLs, credentials, architecture log

TOTAL: 3-6 hours
HUMAN INPUT: approve plan + solve CAPTCHAs + verify emails
EVERYTHING ELSE: autonomous
```

---

## 14. Self-Healing & Recovery System

```
Layer 1: RETRY         (80% of failures) — exponential backoff, 5 attempts
Layer 2: ALTERNATIVE   (10%) — different tool, different provider, different approach
Layer 3: DECOMPOSE     (5%) — split complex task into smaller pieces
Layer 4: ESCALATE      (3%) — ask another agent for help
Layer 5: PAUSE & ALERT (2%) — save state, ask owner, resume on reply
```

---

## 15. Beyond Human — Capabilities Nobody Has Thought Of

These are things Phantom does that no human developer would think to automate.

### 15.1 Ambient Context Awareness

```
Phantom continuously monitors (with owner permission):

WHAT IT WATCHES:
  - Active terminal tabs (iTerm2/Terminal.app via osascript)
  - Current git branch and uncommitted changes
  - Running Docker containers and their health
  - Port usage (lsof -i) — detects conflicts before they break things
  - CPU/memory/disk (sysctl + df) — preempts resource issues
  - Network connectivity (periodic ping to 3 endpoints)
  - macOS battery level (pmset -g batt) — pauses heavy work on low battery
  - Time of day — shifts heavy work to off-peak hours

WHAT IT DOES WITH THIS:
  - Detects you're working on a file → doesn't touch that file
  - Detects port 3000 in use → uses 3001 for its dev server
  - Detects low disk → cleans Docker cache, node_modules, pip cache
  - Detects low battery → pauses non-critical agents, saves state
  - Detects 2 AM → runs heavy builds/tests when you're sleeping
  - Detects you're on slow WiFi → defers large uploads
```

### 15.2 Self-Scheduling Daemon

```
Phantom installs itself as a launchd daemon (macOS native):

~/Library/LaunchAgents/com.benchbrex.phantom.plist

Capabilities:
  - Runs at login (optional, owner-approved)
  - Runs scheduled builds at specified times
  - Monitors production health even when terminal is closed
  - Sends macOS notifications for critical events
  - Auto-updates itself (signed binary, verified before replacing)
  - Survives terminal closure and system sleep
  - Respects macOS Energy Saver settings
```

### 15.3 Smart Git Workflows

```
Phantom doesn't just `git push`. It:

  1. Creates feature branches with semantic names
  2. Writes conventional commit messages (feat:, fix:, chore:, etc.)
  3. Creates PRs with auto-generated descriptions
  4. Links PRs to GitHub Issues it created
  5. Requests its own Security Agent as reviewer
  6. Auto-merges after all checks pass
  7. Tags releases with semantic versioning
  8. Generates changelogs from commit history
  9. Signs commits with GPG key (if configured)
  10. Protects main branch — no direct pushes, ever
```

### 15.4 Predictive Error Prevention

```
BEFORE writing any code, agents check:

  1. Query Knowledge Brain for "AI Code Errors" related to the task
  2. Pre-check: will this library version work on the target Node/Python version?
  3. Pre-check: does this API endpoint still exist? (curl -I to verify)
  4. Pre-check: does this port conflict with anything running?
  5. Pre-check: do we have enough disk space for this build?
  6. Pre-check: will this import create a circular dependency?
  7. Pre-check: does this database migration conflict with existing schema?

  This means: most errors from the AI Code Errors KB (§1 — Pattern 1-6)
  are prevented BEFORE they happen, not fixed AFTER.
```

### 15.5 Cross-Project Memory

```
Phantom remembers what worked across ALL projects:

  - "Last time we used Supabase with Next.js, the auth callback URL pattern was X"
  - "FastAPI + SQLAlchemy async requires selectin loading, not lazy"
  - "Vercel deployment with Python backends requires serverless adapter"
  - "Cloudflare R2 needs specific CORS headers for browser uploads"

  Stored in: ChromaDB knowledge brain as "learned patterns" collection
  Grows over time. Gets smarter with every project.
```

### 15.6 Cost Oracle

```
phantom cost estimate --framework ./architecture.md

┌─ COST ORACLE ──────────────────────────────────────────────────┐
│                                                                 │
│  INFRASTRUCTURE (monthly):                                     │
│  ├── Compute:    $0.00 (Oracle free tier)                     │
│  ├── Database:   $0.00 (Supabase free tier)                   │
│  ├── Cache:      $0.00 (Upstash free tier)                    │
│  ├── CDN/DNS:    $0.00 (Cloudflare free tier)                 │
│  ├── Storage:    $0.00 (R2 free tier)                         │
│  └── TOTAL:      $0.00/month                                  │
│                                                                 │
│  AI AGENT COSTS (one-time build):                              │
│  ├── Claude Opus:   ~$12.00 (CTO + Architect + Security)     │
│  ├── Claude Sonnet: ~$8.00  (Backend + Frontend + DevOps + QA)│
│  ├── Claude Haiku:  ~$0.50  (Monitor agent)                  │
│  └── TOTAL:         ~$20.50 one-time                          │
│                                                                 │
│  SCALING THRESHOLD:                                            │
│  └── Free tiers support up to ~1,000 DAU                      │
│  └── First paid tier needed at ~$25/month                     │
│  └── $100/month handles ~50,000 DAU                           │
│                                                                 │
│  TIME TO BUILD: ~4.5 hours                                    │
│  TIME TO DEPLOY: ~30 minutes after build                      │
└────────────────────────────────────────────────────────────────┘
```

### 15.7 Universal Clipboard Bridge

```
Phantom can use the macOS clipboard as a data bridge:

  # Agent writes data to clipboard for owner to paste elsewhere
  echo "deployment-url: https://app.benchbrex.com" | pbcopy
  phantom notify "Deployment URL copied to clipboard"

  # Owner pastes something into Phantom
  phantom paste   → reads pbpaste, processes content
  
  # Agent copies OAuth tokens from browser automatically
  osascript -e 'tell application "Safari" to get URL of current tab'
  → Extracts OAuth callback token from URL bar
```

### 15.8 Voice Notifications (optional)

```
# Phantom speaks to you when critical events happen
say -v Samantha "Phantom build complete. Your app is live."
say -v Samantha "Warning. Database approaching free tier limit."
say -v Samantha "Security audit found 2 critical issues. Check your terminal."
```

### 15.9 Self-Updating Binary

```
Phantom checks for updates at every launch:
  1. curl -s https://phantom.benchbrex.com/releases/latest/version.txt
  2. Compare with current version
  3. If newer: download new binary + Ed25519 signature
  4. Verify signature against embedded public key
  5. Replace binary atomically (rename, not in-place write)
  6. Notify owner: "Phantom updated to v1.2.0"
  
  Owner can disable: phantom config set auto-update false
```

---

## 16. Terminal Interface & UX

```
CORE:
  phantom activate --key <KEY>         License activation + full system bootstrap
  phantom build --framework <file>     Full autonomous build
  phantom status --live                Live agent dashboard
  phantom doctor                       Verify all dependencies
  phantom agents                       List agent status
  phantom logs [--agent <name>]        Stream logs
  phantom infra                        Infrastructure status
  phantom brain search <query>         Query Knowledge Brain directly
  phantom cost estimate --framework <f> Cost projection

BUILD:
  phantom build --resume               Resume interrupted build
  phantom build --component <name>     Build single component
  phantom build --test-only            Run tests only
  phantom build --deploy-only          Deploy existing build

MASTER (requires passphrase):
  phantom master issue/revoke/list/kill/destroy/rotate/audit/transfer/halt
```

---

## 17. Installation & Bootstrap Sequence

```bash
# ONE COMMAND:
curl -fsSL https://phantom.benchbrex.com/install.sh | sh

# This downloads the binary, verifies signature, and starts activation.
# Phantom then installs ALL dependencies autonomously (Section 4).
# Total bootstrap time: ~30 minutes (mostly waiting for Xcode CLI tools).
```

---

## 18. Master Key Operations & Owner Powers

```
phantom master init
  → Create master passphrase (32+ chars)
  → Derive 256-bit key via Argon2id
  → Generate Ed25519 license signing keypair
  → Generate TOTP secret (QR code)
  → Display 24-word BIP-39 recovery phrase ONCE
  → Master key NEVER written to disk

phantom master destroy
  → Passphrase entry → TOTP 2FA → confirmation string
  → Connects to every bound server
  → Deletes all data (3-pass overwrite)
  → Revokes all API keys + accounts
  → P2P mesh propagates destruction
  → Local binary self-deletes
  → "Phantom has been completely erased from existence."
```

---

## 19. Complete Technical Specification

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Binary | Rust (static) | Zero-dep single executable, memory safety |
| CLI | clap (Rust) | Argument parsing, command routing |
| Terminal UI | ratatui (Rust) | Live dashboard, progress bars |
| Crypto | ring + ed25519-dalek | AES-256-GCM, Ed25519, Argon2id, HKDF |
| P2P | libp2p (Rust) | QUIC, Noise, Kademlia DHT |
| State Sync | automerge-rs | CRDT conflict-free replication |
| HTTP | reqwest | TLS 1.3, cert pinning, async |
| AI | Anthropic API + vLLM | Agent reasoning |
| Knowledge | ChromaDB + sentence-transformers | Knowledge Brain vector search |
| Storage | Cloudflare R2 | Encrypted blob storage |
| Database | Supabase PostgreSQL | State, audit log, licenses |
| Cache | Upstash Redis | Queue, messaging, cache |
| Browser Auto | Playwright (headless) | Account creation, OAuth flows |
| macOS Control | osascript + system CLIs | Full computer access |

---

## 20. CLAUDE.md — The Orchestration Context File

```markdown
# CLAUDE.md — Phantom Agent Context

## IDENTITY
You are an agent in the Phantom autonomous engineering system.
Your role: [AGENT_ROLE env var]
Your Knowledge Brain access: [AGENT_KB_SCOPE env var]

## KNOWLEDGE BRAIN PROTOCOL
BEFORE every decision:
1. Query ChromaDB with a semantic description of what you need to know
2. Read the returned knowledge chunks
3. CITE which knowledge section influenced your decision
4. If knowledge doesn't cover it → ask CTO Agent, don't guess

## YOUR KNOWLEDGE FILES
[Populated dynamically based on agent role — see Section 8.2]

## CODE CONVENTIONS (from owner's vault)
- Python: FastAPI domain modules, type hints, mypy, ruff, structlog
- TypeScript: strict mode, no `any`, ESLint + Prettier
- Database: PostgreSQL, UUID PKs, tenant_id on every table
- API: REST /api/v1/, standard error schema, BaseAPIClient pattern
- CSS: Tailwind, CSS custom properties, 8pt grid, semantic tokens
- Mobile-first (375px min), WCAG 2.2 AA, dark mode
- All credentials via env vars. No hardcoded secrets. Ever.

## SELF-HEALING
1. Log error → 2. Retry (3x, exponential) → 3. Alternative approach →
4. Report to CTO Agent → 5. NEVER silently fail
```

---

## 21. Claude Code Prompt Sequence

```
PROMPT 1: Scaffold Rust workspace (phantom-cli, -core, -crypto, -net, -infra, -ai, -storage, -brain)
PROMPT 2: Implement phantom-crypto (Argon2id, Ed25519, AES-256-GCM, HKDF, fingerprint)
PROMPT 3: Implement phantom-brain (ChromaDB client, embedding pipeline, knowledge query)
PROMPT 4: Implement phantom-core (task graph, agent manager, message bus, self-healer)
PROMPT 5: Implement phantom-net (libp2p mesh, QUIC, CRDT sync)
PROMPT 6: Implement phantom-infra (14 provider clients, provisioner, health checks, account creation)
PROMPT 7: Implement phantom-ai (Anthropic client, agent prompts, context management)
PROMPT 8: Implement phantom-storage (encrypted R2 client, vault)
PROMPT 9: Implement phantom-cli (all commands, ratatui dashboard, doctor)
PROMPT 10: Integration tests, CI/CD, signed release pipeline
```

---

## 22. File Manifest & LOC Estimates

| Crate | Est. LOC | Files | Time |
|-------|---------|-------|------|
| phantom-cli | 1,800 | 10 | 3h |
| phantom-core | 4,500 | 18 | 7h |
| phantom-crypto | 2,200 | 7 | 3h |
| phantom-net | 2,800 | 9 | 4h |
| phantom-infra | 4,000 | 16 | 6h |
| phantom-ai | 1,800 | 7 | 3h |
| phantom-storage | 1,200 | 5 | 2h |
| phantom-brain | 1,500 | 5 | 2h |
| tests | 3,500 | 22 | 5h |
| CI/CD + docs | 700 | 12 | 1h |
| **TOTAL** | **~24,000** | **111** | **~36h** |

---

## 23. Anti-Patterns & Failure Modes

| Anti-Pattern | Phantom's Approach |
|-------------|-------------------|
| Generic AI with no domain knowledge | 10 expert-level knowledge files as embedded brain |
| Code gen without computer access | Full macOS terminal control — installs, configures, deploys |
| Manual dependency installation | Autonomous detection + installation of all 18+ dependencies |
| Manual account creation | CLI-based + browser automation for 14+ services |
| Store keys on disk | mlock'd memory only, zeroed on exit, Keychain as encrypted bridge |
| Single server | 14+ providers, P2P mesh, 3x redundancy |
| Agent without constraints | Token budgets, scoped permissions, timeouts, signed audit log |
| Ignore AI-specific errors | Knowledge Brain includes entire AI Code Errors taxonomy |
| Skip pre-push checks | Pre-push contract from AI Errors KB enforced before every commit |
| Trust the server | Zero-knowledge encryption — servers can't decrypt anything |

---

> **This is the complete Phantom Architecture Framework v2.**
> Every requirement from the owner is addressed. Every capability is specified.
> Every knowledge file is mapped to an agent. Every computer access method is documented.
> Every dependency is auto-installed. Every account is auto-created.
> Every capability beyond human imagination is detailed.
>
> — Parth Patel, Founder, Benchbrex
> — Reference: PHANTOM-ARCH-002-2026-03-18
