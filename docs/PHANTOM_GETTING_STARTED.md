# Getting Started — Your First Phantom Build

This guide takes you from zero to a live production application. Every step is explained for someone who has never used Phantom before.

---

## Before You Begin

You need two things:

1. **A Mac** — Phantom runs on macOS 13 (Ventura) or later. Apple Silicon (M1/M2/M3/M4) or Intel.
2. **A license key** — Format: `PH1-xxxxx-xxxxx`. Contact the owner to receive one.

Everything else (Node.js, Python, Docker, cloud accounts, etc.) Phantom installs for you.

---

## Phase 1 — Installation (~2 minutes)

### Option A: Automated install (recommended)

```bash
curl -fsSL https://phantom.benchbrex.com/install.sh | sh
```

This does three things:
1. Downloads the Phantom binary for your architecture (arm64 or x64)
2. Verifies the Ed25519 signature (ensures the binary hasn't been tampered with)
3. Places it at `/usr/local/bin/phantom`

### Option B: Manual install

```bash
# Download the binary for Apple Silicon
wget https://phantom.benchbrex.com/releases/latest/phantom-darwin-arm64

# Or for Intel Mac
wget https://phantom.benchbrex.com/releases/latest/phantom-darwin-x64

# Make it executable
chmod +x phantom-darwin-arm64

# Move to PATH
sudo mv phantom-darwin-arm64 /usr/local/bin/phantom

# Verify
phantom --version
```

---

## Phase 2 — Activation (~15-30 minutes)

```bash
phantom activate --key PH1-your-key-here
```

### What happens during activation

Phantom performs 8 steps. Each step explains what it's doing and asks for permission before making changes.

**Step 1-3: License verification.** Phantom verifies your key's cryptographic signature, generates a hardware fingerprint of your Mac, and binds the license to this specific machine. No network calls yet — this is pure local cryptography.

**Step 4: Dependency installation.** Phantom checks which developer tools you already have and offers to install the missing ones. Here's what it looks for:

| Tool | Why Phantom Needs It | How It Installs |
|------|---------------------|----------------|
| Xcode CLI Tools | Build tools, git | `xcode-select --install` |
| Homebrew | Package manager | Official install script |
| Node.js 20 | JavaScript runtime | nvm (Node Version Manager) |
| Python 3.12 | Python runtime | pyenv |
| Rust | Phantom's own language | rustup |
| Docker Desktop | Container builds | `brew install --cask docker` |
| PostgreSQL client | Database CLI | `brew install postgresql@16` |
| Redis client | Cache CLI | `brew install redis` |
| GitHub CLI (`gh`) | Repo management | `brew install gh` |
| Vercel CLI | Frontend deployment | `npm install -g vercel` |
| Supabase CLI | Database hosting | `npm install -g supabase` |
| Cloudflare Wrangler | DNS, CDN, Workers | `npm install -g wrangler` |

You can decline any installation. If you already have a tool at a compatible version, Phantom skips it.

**Step 5: Account creation.** Phantom opens your browser for OAuth login on each service it needs. You log in with your own credentials — Phantom never sees your passwords. It receives only the API tokens after you authorize.

Services Phantom will ask to connect:
- **GitHub** — to create repos and CI/CD pipelines
- **Vercel** — to deploy frontends
- **Supabase** — to host PostgreSQL databases
- **Cloudflare** — for DNS, CDN, and R2 storage
- **Upstash** — for Redis (this one uses API signup, no browser needed)

You can skip any service. Phantom will adapt by using alternatives.

**Step 6: Infrastructure provisioning.** Phantom creates free-tier servers on cloud providers. This takes 5-10 minutes. It provisions at least 3 servers across different providers for redundancy.

**Step 7-8: P2P mesh and knowledge loading.** Phantom connects its servers into a peer-to-peer network and loads its Knowledge Brain (10 expert knowledge files indexed as vectors for search).

### After activation: Verify with Doctor

```bash
phantom doctor
```

This shows a health check of everything Phantom needs:

```
┌─ PHANTOM DOCTOR ──────────────────────────────────────────────┐
│                                                                │
│  LICENSE                                                      │
│  ✓ Valid until 2027-03-18 (364 days remaining)               │
│  ✓ Machine fingerprint matches                               │
│                                                                │
│  DEPENDENCIES (18/18 verified)                                │
│  ✓ Xcode CLI Tools    ✓ Docker         ✓ Vercel CLI          │
│  ✓ Homebrew           ✓ PostgreSQL     ✓ Supabase CLI        │
│  ✓ Node.js 20         ✓ Redis          ✓ Wrangler            │
│  ✓ Python 3.12        ✓ GitHub CLI     ✓ Netlify CLI         │
│  ✓ Rust               ✓ Claude Code    ✓ Railway CLI         │
│                                                                │
│  ACCOUNTS (5/5 connected)                                     │
│  ✓ GitHub       ✓ Vercel      ✓ Supabase                    │
│  ✓ Cloudflare   ✓ Upstash                                   │
│                                                                │
│  INFRASTRUCTURE (3/3 healthy)                                 │
│  ✓ Oracle Cloud (Mumbai)     ✓ 47d uptime                    │
│  ✓ Cloudflare Workers        ✓ Global edge                   │
│  ✓ Upstash Redis             ✓ 99.9% availability            │
│                                                                │
│  P2P MESH: 3 nodes connected                                 │
│  KNOWLEDGE BRAIN: 500 chunks, 10 files indexed               │
│                                                                │
│  STATUS: ALL SYSTEMS HEALTHY ✓                               │
└───────────────────────────────────────────────────────────────┘
```

---

## Phase 3 — Write Your Architecture Document (~30 minutes of your time)

This is the only creative work you do. Phantom handles everything else.

### What to include

Your architecture document is a Markdown file. The more detail, the better. At minimum, include:

1. **What the app does** — one paragraph describing the product
2. **Tech stack** — which languages, frameworks, databases
3. **Features** — bullet list of what users can do
4. **Database models** — tables, columns, relationships
5. **API endpoints** — routes, methods, what they return

### Architecture document template

Save this as `architecture.md` and fill in your project details:

```markdown
# [Project Name] — Architecture

## Product Overview
[One paragraph: what is this product, who uses it, what problem does it solve]

## Tech Stack
- Backend: [FastAPI / Express / Django / etc.]
- Frontend: [Next.js / React / Vue / etc.]
- Database: [PostgreSQL / MySQL / MongoDB]
- Cache: [Redis / Memcached]
- Auth: [JWT / Session / OAuth providers]

## Core Features
- [Feature 1 — what the user can do]
- [Feature 2]
- [Feature 3]
- [...]

## Database Models
[List your tables/collections with columns and relationships]

## API Endpoints
[List your routes: method, path, description]

## Deployment
- Frontend: [Vercel / Netlify / etc.]
- Backend: [Docker / serverless / VPS]
- Database: [Supabase / Neon / managed]

## Constraints (optional)
- [Mobile-first design]
- [WCAG 2.2 AA accessibility]
- [Dark mode support]
- [Rate limiting on all endpoints]
- [etc.]
```

### Tips for better results

- **Be specific about data models.** "users table with email, name, avatar" is better than "user management."
- **List every feature.** Phantom builds exactly what you describe. If you forget a feature, it won't be built.
- **Mention constraints early.** If you need accessibility compliance or dark mode, say so in the architecture doc — Phantom's agents will enforce it.
- **Include example API responses** if you have strong opinions about response format.

---

## Phase 4 — Build (~3-6 hours, autonomous)

```bash
phantom build --framework ./architecture.md
```

### What you'll see

1. **Build plan** — Phantom analyzes your document and shows a detailed plan: tasks, agents, time estimates, parallel streams. You approve or modify.

2. **Live dashboard** (optional) — Watch agents work in real time:
   ```bash
   phantom status --live
   ```

3. **Completion report** — URLs, code stats, test results, security findings.

### What the agents do during a build

| Phase | Time | Agent(s) | What Happens |
|-------|------|----------|-------------|
| Design | ~15 min | Architect | System design doc, DB schema (SQL + migrations), OpenAPI spec, ADRs |
| Backend | ~90 min | Backend | FastAPI routes, models, services, auth, background jobs, error handling |
| Frontend | ~90 min | Frontend | Next.js pages, components, Tailwind styling, dark mode, mobile-first |
| Infrastructure | ~30 min | DevOps | Dockerfiles, CI/CD, DNS, TLS, deployment scripts |
| Testing | ~45 min | QA | pytest, Vitest, Playwright E2E, 80%+ coverage |
| Security | ~20 min | Security | OWASP Top 10, dependency audit, auth review, secret scan |
| Deployment | ~15 min | DevOps | Push → CI → Docker build → deploy → DNS → TLS → health check |

Backend, Frontend, and DevOps run **in parallel** — that's why total time is 3-6 hours, not 6-10.

### If something goes wrong

Phantom's self-healing handles most issues automatically. If it genuinely needs your input, it pauses and asks a specific question:

```
⚠  PHANTOM NEEDS YOUR INPUT

Agent:    DevOps
Task:     Configure Supabase project
Issue:    Supabase free tier limit reached for this email address
Question: Should I use Neon as the database provider instead?

[Yes — use Neon]  [No — I'll upgrade Supabase]  [Skip database setup]
```

After you answer, Phantom resumes automatically.

---

## Phase 5 — Your Software is Live

After the build completes, you have:

- **Live frontend** at a Vercel URL (with custom domain if you provided one)
- **Live backend** on your provisioned server
- **Live database** on Supabase or Neon
- **GitHub repository** with all code, CI/CD, and documentation
- **Credentials** stored in Phantom's encrypted vault

### What to do next

```bash
# See your running infrastructure
phantom infra

# Check production health
phantom status

# View stored credentials (database URLs, API keys, etc.)
phantom vault list

# Stream real-time logs
phantom logs

# If you need to rebuild or update
phantom build --resume     # Continue interrupted build
phantom build --deploy-only  # Re-deploy without rebuilding
```

### Making changes after the initial build

Phantom doesn't lock you out. All code is in your GitHub repo. You can:

1. **Edit code directly** — it's standard FastAPI + Next.js code. Open in any editor.
2. **Ask Phantom to modify** — describe changes in a new architecture doc section and run `phantom build` again.
3. **Use Claude Code directly** — the generated CLAUDE.md in your repo gives Claude Code full project context.

---

## Troubleshooting

### "Invalid license"

Your license key doesn't match this machine. Keys are hardware-bound. Contact the owner for a new key.

### "Dependency installation failed"

Run `phantom doctor` to see which dependency failed. Then install it manually:

```bash
# Common fixes
xcode-select --install          # Xcode CLI tools
brew install postgresql@16      # PostgreSQL
brew install redis              # Redis
```

After manual installation, re-run `phantom activate --key <your-key>`.

### "Account creation failed"

Make sure your browser is open. Phantom needs to open OAuth login pages. If a specific service fails, skip it and Phantom will use alternatives.

### "Build failed — agent stuck"

```bash
phantom build --resume    # Resume from last checkpoint
phantom logs --agent backend  # See what the backend agent was doing
```

If the issue persists, Phantom saves full state. You can always restart without losing work.

### "Infrastructure unreachable"

```bash
phantom infra status     # See which server is down
```

Phantom automatically fails over to replica servers. If all 3 servers are down simultaneously (extremely unlikely), contact the owner.

---

## Next Steps

- Read the full [Architecture Document](ARCHITECTURE.md) to understand every system component
- Read [Security](SECURITY.md) for the complete threat model and mitigations
- Read [Agents](AGENTS.md) for detailed behavior of each AI agent
- Read [Operations](OPERATIONS.md) for master key management and infrastructure operations
