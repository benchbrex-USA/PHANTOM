//! Build pipeline integration tests — architecture parsing, component extraction,
//! task graph construction, topological sort, parallel layers.

use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════════════════
//  Sample Architecture Markdown
// ═══════════════════════════════════════════════════════════════════════════

fn sample_architecture_md() -> &'static str {
    r#"# SaaS Dashboard Platform

## 1. Product Overview

A cloud-native SaaS analytics dashboard that provides real-time insights
for enterprise customers. The platform ingests event streams, processes
them through a pipeline, and renders interactive dashboards.

Key goals:
- Sub-second query latency
- Multi-tenant data isolation
- 99.99% uptime SLA

## 2. Technology Stack

| Component   | Technology   | Provider     |
|-------------|-------------|--------------|
| Backend     | Rust (Axum) | Self-hosted  |
| Frontend    | React + TS  | Vercel       |
| Database    | PostgreSQL  | Supabase     |
| Cache       | Redis       | Upstash      |
| Queue       | Kafka       | Confluent    |
| Storage     | S3          | AWS          |

## 3. Core Features

### 3.1 Real-time Event Ingestion

Accept events via REST and WebSocket APIs.
Validate schema, enrich with metadata, and push to Kafka.

```rust
pub struct Event {
    pub id: String,
    pub tenant_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

### 3.2 Analytics Processing Pipeline

Consume events from Kafka, aggregate metrics, and store
in PostgreSQL materialized views for fast dashboard queries.

### 3.3 Interactive Dashboard UI

React-based SPA with:
- Drag-and-drop widget builder
- Real-time chart updates via WebSocket
- Role-based access control (RBAC)

## 4. Database Models

### 4.1 Tenant Model

```sql
CREATE TABLE tenants (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    plan TEXT NOT NULL DEFAULT 'free',
    created_at TIMESTAMPTZ DEFAULT now()
);
```

### 4.2 Event Model

```sql
CREATE TABLE events (
    id UUID PRIMARY KEY,
    tenant_id UUID REFERENCES tenants(id),
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX idx_events_tenant ON events(tenant_id, created_at);
```

## 5. API Endpoints

### 5.1 Events API

- `POST /api/v1/events` — ingest a single event
- `POST /api/v1/events/batch` — ingest a batch of events
- `GET /api/v1/events/:id` — get event by ID

### 5.2 Dashboard API

- `GET /api/v1/dashboards` — list dashboards for tenant
- `POST /api/v1/dashboards` — create a new dashboard

## 6. Infrastructure

- Primary compute: AWS ECS Fargate (3 services)
- Database: Supabase (dedicated Postgres)
- Cache: Upstash Redis (global)
- CDN: Cloudflare (static assets)
- CI/CD: GitHub Actions → Docker → ECS

## 7. Security

- All secrets in AWS Secrets Manager
- mTLS between services
- OWASP Top 10 compliance
- SOC2 Type II audit trail
"#
}

fn write_temp_file(name: &str, content: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("phantom-integ-bp-{}", name));
    std::fs::write(&path, content).unwrap();
    path
}

fn cleanup(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
}

// ═══════════════════════════════════════════════════════════════════════════
//  1. Parse architecture markdown
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_architecture_sections() {
    use phantom_core::framework_ingestion::MarkdownParser;

    let parsed = MarkdownParser::parse(sample_architecture_md(), "dashboard.md").unwrap();

    assert_eq!(parsed.title, "SaaS Dashboard Platform");
    assert!(
        parsed.sections.len() >= 7,
        "expected at least 7 top-level sections"
    );
    assert!(parsed.total_lines > 50);
}

#[test]
fn test_parse_extracts_technology_table() {
    use phantom_core::framework_ingestion::MarkdownParser;

    let parsed = MarkdownParser::parse(sample_architecture_md(), "dashboard.md").unwrap();

    let tech_section = parsed
        .sections
        .iter()
        .find(|s| s.heading.contains("Technology Stack"));
    assert!(
        tech_section.is_some(),
        "Technology Stack section must be found"
    );

    let section = tech_section.unwrap();
    assert!(
        !section.tables.is_empty(),
        "must have a table in Technology Stack"
    );

    let table = &section.tables[0];
    assert!(table.headers.iter().any(|h| h.contains("Component")));
    assert!(table.headers.iter().any(|h| h.contains("Technology")));
    assert!(table.rows.len() >= 6, "expected at least 6 technology rows");
}

#[test]
fn test_parse_extracts_code_blocks() {
    use phantom_core::framework_ingestion::MarkdownParser;

    let parsed = MarkdownParser::parse(sample_architecture_md(), "dashboard.md").unwrap();

    let code_blocks: Vec<_> = parsed
        .sections
        .iter()
        .flat_map(|s| s.code_blocks.iter())
        .collect();

    assert!(
        code_blocks.len() >= 3,
        "expected at least 3 code blocks (1 rust + 2 sql)"
    );

    let has_rust = code_blocks
        .iter()
        .any(|cb| cb.language.as_deref() == Some("rust"));
    let has_sql = code_blocks
        .iter()
        .any(|cb| cb.language.as_deref() == Some("sql"));
    assert!(has_rust, "expected a rust code block");
    assert!(has_sql, "expected sql code blocks");
}

#[test]
fn test_parse_from_file() {
    use phantom_core::framework_ingestion::MarkdownParser;

    let path = write_temp_file("arch.md", sample_architecture_md());
    let parsed = MarkdownParser::parse_file(&path).unwrap();

    assert_eq!(parsed.title, "SaaS Dashboard Platform");
    assert!(parsed.sections.len() >= 7);

    cleanup(&path);
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. Component extraction — ArchitectureSpec fields populated
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_component_extraction_populates_fields() {
    use phantom_core::framework_ingestion::{ComponentExtractor, MarkdownParser};

    let parsed = MarkdownParser::parse(sample_architecture_md(), "dashboard.md").unwrap();
    let arch = ComponentExtractor::extract(&parsed).unwrap();

    assert!(!arch.components.is_empty(), "must extract components");
    assert!(!arch.technologies.is_empty(), "must extract technologies");
    assert!(!arch.project_name.is_empty(), "project name must be set");
}

#[test]
fn test_component_dag_construction() {
    use phantom_core::framework_ingestion::{ComponentDag, ComponentExtractor, MarkdownParser};

    let parsed = MarkdownParser::parse(sample_architecture_md(), "dashboard.md").unwrap();
    let arch = ComponentExtractor::extract(&parsed).unwrap();
    let dag = ComponentDag::build(&arch).unwrap();

    assert!(!dag.is_empty(), "DAG must have nodes");
    assert!(dag.len() >= arch.components.len());

    let order = dag.topological_order();
    assert!(!order.is_empty(), "topological order must be non-empty");
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. TaskGraph — add tasks with dependencies, topological sort
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_task_graph_add_and_topo_sort() {
    use phantom_core::task_graph::{Task, TaskGraph};

    let mut graph = TaskGraph::new();

    let t1 = Task::new("setup-infra", "Provision infrastructure", "devops").with_estimate(120);
    let t1_id = t1.id.clone();
    graph.add_task(t1).unwrap();

    let t2 = Task::new("build-api", "Build REST API", "backend")
        .depends_on(&t1_id)
        .with_estimate(300);
    let t2_id = t2.id.clone();
    graph.add_task(t2).unwrap();

    let t3 = Task::new("build-ui", "Build dashboard UI", "frontend")
        .depends_on(&t1_id)
        .with_estimate(240);
    let t3_id = t3.id.clone();
    graph.add_task(t3).unwrap();

    let t4 = Task::new("integration-tests", "Run integration tests", "qa")
        .depends_on(&t2_id)
        .depends_on(&t3_id)
        .with_estimate(180);
    let t4_id = t4.id.clone();
    graph.add_task(t4).unwrap();

    let t5 = Task::new("deploy", "Deploy to production", "devops")
        .depends_on(&t4_id)
        .with_estimate(60);
    graph.add_task(t5).unwrap();

    // Graph should be valid (no cycles, no missing deps)
    assert!(graph.validate().is_ok());

    // Topological sort: infra before api/ui, api+ui before tests, tests before deploy
    let order = graph.topological_order().unwrap();
    assert_eq!(order.len(), 5);

    let pos = |id: &str| order.iter().position(|x| x == id).unwrap();
    assert!(pos(&t1_id) < pos(&t2_id), "infra must come before api");
    assert!(pos(&t1_id) < pos(&t3_id), "infra must come before ui");
    assert!(pos(&t2_id) < pos(&t4_id), "api must come before tests");
    assert!(pos(&t3_id) < pos(&t4_id), "ui must come before tests");
    assert!(
        pos(&t4_id) < pos(order.last().unwrap()),
        "tests must come before deploy"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. Parallel layers — tasks at same depth can run concurrently
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_task_graph_parallel_layers() {
    use phantom_core::task_graph::{Task, TaskGraph};

    let mut graph = TaskGraph::new();

    // Layer 0: architect
    let t_arch = Task::new("architect", "Design architecture", "architect");
    let arch_id = t_arch.id.clone();
    graph.add_task(t_arch).unwrap();

    // Layer 1: backend + frontend (parallel, both depend on architect)
    let t_backend = Task::new("backend", "Build backend", "backend").depends_on(&arch_id);
    let backend_id = t_backend.id.clone();
    graph.add_task(t_backend).unwrap();

    let t_frontend = Task::new("frontend", "Build frontend", "frontend").depends_on(&arch_id);
    let frontend_id = t_frontend.id.clone();
    graph.add_task(t_frontend).unwrap();

    // Layer 2: QA (depends on both backend and frontend)
    let t_qa = Task::new("qa", "Run QA", "qa")
        .depends_on(&backend_id)
        .depends_on(&frontend_id);
    graph.add_task(t_qa).unwrap();

    let layers = graph.parallel_layers().unwrap();

    // Should have 3 layers
    assert_eq!(layers.len(), 3, "expected 3 parallel layers");
    assert_eq!(layers[0].len(), 1, "layer 0: architect only");
    assert_eq!(
        layers[1].len(),
        2,
        "layer 1: backend + frontend in parallel"
    );
    assert_eq!(layers[2].len(), 1, "layer 2: QA after both");
}

#[test]
fn test_task_graph_cycle_detection() {
    use phantom_core::task_graph::{Task, TaskGraph};

    let mut graph = TaskGraph::new();

    let mut t1 = Task::new("t1", "Task 1", "cto");
    let mut t2 = Task::new("t2", "Task 2", "cto");
    let t1_id = t1.id.clone();
    let t2_id = t2.id.clone();

    t1.dependencies.push(t2_id.clone());
    t2.dependencies.push(t1_id.clone());

    graph.add_task(t1).unwrap();
    graph.add_task(t2).unwrap();

    assert!(graph.validate().is_err(), "cycle must be detected");
    assert!(
        graph.topological_order().is_err(),
        "topo sort must fail on cycle"
    );
}

#[test]
fn test_task_graph_stats() {
    use phantom_core::task_graph::{Task, TaskGraph, TaskStatus};

    let mut graph = TaskGraph::new();

    let t1 = Task::new("t1", "Task 1", "cto").with_estimate(60);
    let t1_id = t1.id.clone();
    graph.add_task(t1).unwrap();

    let t2 = Task::new("t2", "Task 2", "backend").with_estimate(120);
    graph.add_task(t2).unwrap();

    let t3 = Task::new("t3", "Task 3", "frontend").with_estimate(90);
    graph.add_task(t3).unwrap();

    // Complete one task
    graph.get_task_mut(&t1_id).unwrap().complete(None);

    let stats = graph.stats();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.completed, 1);
    assert_eq!(stats.pending, 2);
    assert_eq!(stats.total_estimated_seconds, 270);
}

#[test]
fn test_task_lifecycle_complete_flow() {
    use phantom_core::task_graph::{Task, TaskStatus};

    let mut task = Task::new("test-task", "A test task", "backend");
    assert_eq!(task.status, TaskStatus::Pending);

    task.start();
    assert_eq!(task.status, TaskStatus::Running);
    assert!(task.started_at.is_some());

    task.fail("temporary error");
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.error.as_deref(), Some("temporary error"));

    assert!(task.can_retry());
    task.retry();
    assert_eq!(task.status, TaskStatus::Retrying);
    assert_eq!(task.retry_count, 1);

    task.complete(Some(serde_json::json!({"result": "success"})));
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.completed_at.is_some());
    assert!(task.output.is_some());
}
