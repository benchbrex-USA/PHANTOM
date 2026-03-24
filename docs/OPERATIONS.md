# Phantom Operations Guide

## Initial Setup

### 1. Master Key Initialization

```bash
phantom master init
```

- Prompts for a passphrase (minimum 12 characters)
- Derives MasterKey via Argon2id
- Generates Ed25519 signing keypair
- Stores encrypted keypair in Cloudflare R2
- Outputs the public key fingerprint

**Important**: The passphrase is never stored. If lost, the master key cannot be recovered.

### 2. License Issuance

```bash
phantom master issue \
  --org "your-org" \
  --tier pro \
  --caps "build,deploy,monitor" \
  --days 365
```

- Creates a signed license token: `PH1-<base62_payload>-<base62_signature>`
- Each license includes a unique installation ID (`iid`)
- Capabilities control which features are available

License tiers: `free`, `pro`, `team`, `enterprise`

### 3. Activation

```bash
phantom activate --key PH1-<your-license-key>
```

- Verifies signature against embedded public key
- Checks expiry and capabilities
- Derives a SessionKey for this activation
- Bootstraps dependencies (runs `phantom doctor` internally)
- Provisions required infrastructure

## Dependency Management

### Doctor Check

```bash
phantom doctor
```

Checks 18+ dependencies across categories:

| Category | Dependencies |
|----------|-------------|
| System Prerequisites | Xcode CLI Tools, Homebrew, Git, curl, jq, ripgrep |
| Runtimes | Node.js 20, Python 3.12, Rust |
| Databases | PostgreSQL client, Redis client, Docker |
| Deployment CLIs | GitHub CLI, Vercel CLI, Supabase CLI, Wrangler, Fly CLI |
| AI Tools | sentence-transformers |

Required dependencies are installed automatically; optional ones prompt for confirmation.

## Infrastructure Operations

### Provider Status

```bash
phantom infra
```

Shows status of all configured providers:
- **Supabase**: Database projects, API endpoints, connection health
- **Vercel**: Deployed projects, domains, environment variables
- **Cloudflare**: R2 buckets, Workers, DNS zones
- **Fly.io**: Running machines, regions, scaling config
- **Neon**: Database branches, endpoints
- **Oracle Cloud**: Compute instances, VCNs

### Infrastructure Provisioning

Infrastructure is provisioned automatically during `phantom build` based on the Architecture Framework. The provisioner:

1. Reads infrastructure requirements from the framework
2. Selects optimal free-tier providers
3. Creates resources (databases, hosting, DNS)
4. Stores connection details in encrypted Vault
5. Configures environment variables on deployment targets

## Build Operations

### Full Build

```bash
phantom build --framework docs/my_architecture.md
```

1. Architect agent parses the framework document
2. Generates a task graph with dependency ordering
3. Dispatches tasks to specialized agents
4. Agents generate code, tests, and infrastructure
5. Testing agent validates all outputs
6. DevOps agent deploys to configured targets

### Resume Interrupted Build

```bash
phantom build --resume
```

Picks up from the last completed task in the graph.

### Single Component

```bash
phantom build --component auth-service
```

Builds only the specified component and its dependencies.

## Monitoring

### Agent Status

```bash
phantom agents
```

Lists all agents with their current state, assigned tasks, and resource usage.

### Live Dashboard

```bash
phantom status --live
```

Opens the TUI dashboard (ratatui) showing:
- Agent activity in real-time
- Build progress and task completion
- Infrastructure health
- Cost tracking

### Logs

```bash
phantom logs
```

Streams agent logs with filtering by agent, severity, and time range.

## Master Key Operations

### License Management

```bash
# List all installations
phantom master list

# Revoke a specific license
phantom master revoke --iid <installation-id>

# Remote-kill an installation
phantom master kill --iid <installation-id>
```

### Key Rotation

```bash
phantom master rotate
```

Rotates all derived keys. Active sessions re-derive on next operation.

### Emergency Operations

```bash
# Stop all running agents immediately
phantom master halt

# Full system destruction (requires TOTP 2FA)
phantom master destroy
```

`destroy` permanently deletes:
- All encrypted state in R2
- All provisioned infrastructure
- All issued licenses
- The master keypair itself

This operation is irreversible and requires TOTP confirmation.

### Audit Log

```bash
phantom master audit
```

Exports a complete audit trail of all master key operations, license issuances, revocations, and infrastructure changes.

## Cost Management

```bash
phantom cost
```

Shows estimated and actual costs across all providers, with alerts when approaching free-tier limits.
