# Phantom Agent System

## Overview

Phantom uses 8 specialized AI agents orchestrated by a central coordinator. Each agent has scoped permissions, a dedicated knowledge domain, and operates under the principle of least privilege.

## Agent Roles

### 1. Architect Agent

- **Role**: Parses Architecture Framework documents, generates task graphs, determines build order
- **Permissions**: `READ_CODE | READ_NET | READ_BRAIN`
- **Knowledge**: Architecture patterns, framework parsing, dependency resolution
- **Inputs**: Architecture Framework markdown files
- **Outputs**: `ArchitectureSpec`, `TaskGraph` with topological ordering

### 2. Frontend Agent

- **Role**: Generates and modifies frontend code (React, Next.js, Vue, Svelte)
- **Permissions**: `READ_CODE | WRITE_CODE | EXECUTE_CMD | READ_NET`
- **Knowledge**: Component libraries, CSS frameworks, accessibility patterns, SSR/SSG
- **Inputs**: Component specs from task graph, design tokens
- **Outputs**: Frontend source files, component tests

### 3. Backend Agent

- **Role**: Generates and modifies backend code (API routes, middleware, database schemas)
- **Permissions**: `READ_CODE | WRITE_CODE | EXECUTE_CMD | READ_NET | READ_CREDENTIALS`
- **Knowledge**: REST/GraphQL patterns, ORM usage, auth middleware, rate limiting
- **Inputs**: API specs from task graph, database schema requirements
- **Outputs**: Backend source files, migration scripts, API tests

### 4. Database Agent

- **Role**: Manages database schemas, migrations, and queries
- **Permissions**: `READ_CODE | WRITE_CODE | EXECUTE_CMD | READ_CREDENTIALS`
- **Knowledge**: SQL dialects, migration patterns, indexing strategies, connection pooling
- **Inputs**: Data model specs, relationship definitions
- **Outputs**: Migration files, seed data, query optimizations

### 5. DevOps Agent

- **Role**: Manages infrastructure provisioning, CI/CD, and deployment
- **Permissions**: `READ_CODE | WRITE_CODE | EXECUTE_CMD | READ_NET | WRITE_NET | READ_CREDENTIALS | MANAGE_INFRA`
- **Knowledge**: Cloud provider APIs, container orchestration, DNS, TLS certificates
- **Inputs**: Deployment targets, infrastructure requirements
- **Outputs**: Infrastructure configs, deployment scripts, health checks

### 6. Testing Agent

- **Role**: Generates and runs tests across the entire project
- **Permissions**: `READ_CODE | WRITE_CODE | EXECUTE_CMD | READ_NET`
- **Knowledge**: Testing frameworks, coverage analysis, property-based testing, mocking
- **Inputs**: Source files, API contracts, component specs
- **Outputs**: Unit tests, integration tests, E2E tests, coverage reports

### 7. Security Agent

- **Role**: Audits code for vulnerabilities, manages secrets, enforces security policies
- **Permissions**: `READ_CODE | READ_CREDENTIALS | MANAGE_KEYS`
- **Knowledge**: OWASP Top 10, dependency CVEs, secret scanning, CSP policies
- **Inputs**: Source files, dependency manifests, infrastructure configs
- **Outputs**: Security audit reports, remediation patches, secret rotation

### 8. Monitor Agent

- **Role**: Monitors deployed services, tracks costs, manages alerts
- **Permissions**: `READ_NET | READ_CREDENTIALS | MANAGE_INFRA`
- **Knowledge**: Observability patterns, cost optimization, alerting thresholds
- **Inputs**: Deployed service endpoints, cost budgets
- **Outputs**: Health reports, cost summaries, alert configurations

## Permission System

Permissions are 16-bit flags assigned per agent role:

| Bit | Permission | Description |
|-----|-----------|-------------|
| 0 | `READ_CODE` | Read source files |
| 1 | `WRITE_CODE` | Create/modify source files |
| 2 | `EXECUTE_CMD` | Run shell commands |
| 3 | `READ_NET` | Make outbound HTTP requests |
| 4 | `WRITE_NET` | Listen on network ports |
| 5 | `READ_CREDENTIALS` | Access encrypted credentials |
| 6 | `WRITE_CREDENTIALS` | Store new credentials |
| 7 | `MANAGE_KEYS` | Derive/rotate encryption keys |
| 8 | `MANAGE_INFRA` | Provision/destroy infrastructure |
| 9 | `MANAGE_AGENTS` | Start/stop other agents |
| 10 | `READ_BRAIN` | Query Knowledge Brain |
| 11 | `WRITE_BRAIN` | Update Knowledge Brain |
| 12 | `DEPLOY` | Trigger deployments |
| 13 | `ROLLBACK` | Revert deployments |
| 14 | `AUDIT` | Read audit logs |
| 15 | `ADMIN` | Full system access |

## Orchestration

The build coordinator (`phantom-core`) manages agent lifecycle:

1. **Task Graph Generation**: Architect agent parses framework → produces DAG
2. **Topological Scheduling**: Tasks dispatched in dependency order
3. **Agent Assignment**: Each task routed to the appropriate agent based on task type
4. **Parallel Execution**: Independent tasks run concurrently across agents
5. **Result Aggregation**: Outputs collected, validated, merged into project
6. **Error Recovery**: Failed tasks retried with context; persistent failures escalated

## Agent Communication

- Agents communicate via in-process async channels (tokio mpsc)
- Each agent receives an `AgentKey` with scoped permissions
- Agent keys are derived from the session key with agent+task scoping
- No agent can escalate its own permissions
