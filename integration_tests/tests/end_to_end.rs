//! End-to-end integration tests for PHANTOM v2.
//!
//! 1. Ingest sample architecture MD → extract → generate plan
//! 2. Agent orchestration with lifecycle management
//! 3. P2P CRDT sync between 2+ simulated nodes
//! 4. Self-healing: inject failures at each layer, verify recovery
//! 5. Zero-footprint: verify no disk artifacts after session
//! 6. Doctor --full system readiness

use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════════════════════════

fn sample_architecture_md() -> &'static str {
    r#"# Phantom Test Project

## 1. Overview

A test project for integration testing.

## 2. Technology Stack

| Component | Technology | Provider |
|-----------|-----------|----------|
| Backend   | Rust      | Oracle   |
| Frontend  | React     | Vercel   |
| Database  | Postgres  | Supabase |
| Cache     | Redis     | Upstash  |

## 3. API Contracts

```rust
pub struct User {
    pub id: String,
    pub email: String,
}
```

### 3.1 Endpoints

- `GET /api/users` — list users
- `POST /api/users` — create user

## 4. Database Schema

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ DEFAULT now()
);
```

## 5. Infrastructure

- Primary compute: Oracle Cloud (2 VMs)
- Database: Supabase (500MB PG)
- Edge: Cloudflare Workers
- CI/CD: GitHub Actions

## 6. Security

- All secrets in environment variables
- TLS everywhere
- OWASP Top 10 compliance
"#
}

fn write_temp_file(name: &str, content: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("phantom-integ-{}", name));
    std::fs::write(&path, content).unwrap();
    path
}

fn cleanup(path: &Path) {
    let _ = std::fs::remove_file(path);
}

// ═══════════════════════════════════════════════════════════════════════════
//  1. End-to-End: Ingest → Extract → Plan
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_e2e_ingest_architecture_md() {
    use phantom_core::framework_ingestion::MarkdownParser;

    let parsed = MarkdownParser::parse(sample_architecture_md(), "test-project.md").unwrap();

    assert_eq!(parsed.title, "Phantom Test Project");
    assert!(!parsed.sections.is_empty());
    assert!(parsed.total_lines > 10);

    let headings: Vec<&str> = parsed.sections.iter().map(|s| s.heading.as_str()).collect();
    assert!(headings.iter().any(|h| h.contains("Technology Stack")));
    assert!(headings.iter().any(|h| h.contains("API Contracts")));
    assert!(headings.iter().any(|h| h.contains("Database Schema")));
}

#[test]
fn test_e2e_extract_tables_from_md() {
    use phantom_core::framework_ingestion::MarkdownParser;

    let parsed = MarkdownParser::parse(sample_architecture_md(), "test.md").unwrap();

    let tech_section = parsed
        .sections
        .iter()
        .find(|s| s.heading.contains("Technology Stack"));
    assert!(tech_section.is_some(), "Technology Stack section not found");

    let section = tech_section.unwrap();
    assert!(
        !section.tables.is_empty(),
        "no table found in Technology Stack"
    );

    let table = &section.tables[0];
    assert!(table.headers.iter().any(|h| h.contains("Component")));
    assert!(!table.rows.is_empty());
}

#[test]
fn test_e2e_extract_code_blocks() {
    use phantom_core::framework_ingestion::MarkdownParser;

    let parsed = MarkdownParser::parse(sample_architecture_md(), "test.md").unwrap();

    let code_blocks: Vec<_> = parsed
        .sections
        .iter()
        .flat_map(|s| s.code_blocks.iter())
        .collect();

    assert!(code_blocks.len() >= 2, "expected at least 2 code blocks");

    let has_rust = code_blocks
        .iter()
        .any(|cb| cb.language.as_deref() == Some("rust"));
    let has_sql = code_blocks
        .iter()
        .any(|cb| cb.language.as_deref() == Some("sql"));
    assert!(has_rust, "expected a rust code block");
    assert!(has_sql, "expected a sql code block");
}

#[test]
fn test_e2e_ingest_from_file() {
    use phantom_core::framework_ingestion::MarkdownParser;

    let path = write_temp_file("arch.md", sample_architecture_md());
    let parsed = MarkdownParser::parse_file(&path).unwrap();

    assert_eq!(parsed.title, "Phantom Test Project");
    assert!(parsed.sections.len() >= 5);

    cleanup(&path);
}

#[test]
fn test_e2e_component_extraction() {
    use phantom_core::framework_ingestion::{ComponentExtractor, MarkdownParser};

    let parsed = MarkdownParser::parse(sample_architecture_md(), "test.md").unwrap();
    let arch = ComponentExtractor::extract(&parsed).unwrap();

    assert!(!arch.components.is_empty(), "no components extracted");
    assert!(!arch.technologies.is_empty(), "no technologies extracted");
}

#[test]
fn test_e2e_full_pipeline_ingest_to_dag() {
    use phantom_core::framework_ingestion::{ComponentDag, ComponentExtractor, MarkdownParser};

    let parsed = MarkdownParser::parse(sample_architecture_md(), "test.md").unwrap();
    let arch = ComponentExtractor::extract(&parsed).unwrap();
    let dag = ComponentDag::build(&arch).unwrap();

    assert!(!dag.is_empty(), "DAG has no nodes");
    assert!(dag.len() >= arch.components.len());

    // Topological order should include all components
    let order = dag.topological_order();
    assert!(!order.is_empty());
}

#[test]
fn test_e2e_build_pipeline_phases() {
    use phantom_core::pipeline::BuildPhase;

    let phases = BuildPhase::all();
    assert_eq!(phases.len(), 8);
    assert_eq!(phases[0], BuildPhase::Ingest);
    assert_eq!(phases[7], BuildPhase::Deliver);

    assert_eq!(BuildPhase::Ingest.next(), Some(BuildPhase::Infrastructure));
    assert_eq!(BuildPhase::Deliver.next(), None);

    for phase in phases {
        assert!(phase.estimated_seconds() > 0);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. Agent Orchestration
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_agent_manager_spawn_full_team() {
    use phantom_core::agent_manager::AgentManager;

    let mut manager = AgentManager::new();
    let ids = manager.spawn_full_team().unwrap();

    assert_eq!(ids.len(), 8, "expected 8-agent team");

    // All agents should be idle
    for handle in manager.agents() {
        assert_eq!(handle.state, phantom_core::agent_manager::AgentState::Idle);
    }
}

#[test]
fn test_agent_manager_spawn_and_assign() {
    use phantom_ai::AgentRole;
    use phantom_core::agent_manager::{AgentManager, AgentState};

    let mut manager = AgentManager::new();
    let id = manager.spawn(AgentRole::Backend).unwrap();

    // Agent starts idle
    let handle = manager.get(&id).unwrap();
    assert_eq!(handle.state, AgentState::Idle);

    // Assign a task directly on the handle
    let handle = manager.get_mut(&id).unwrap();
    handle.assign_task("build-api");
    assert_eq!(handle.state, AgentState::Working);
    assert_eq!(handle.current_task.as_deref(), Some("build-api"));

    // Complete
    handle.complete_task();
    assert_eq!(handle.state, AgentState::Idle);
    assert_eq!(handle.tasks_completed, 1);
}

#[test]
fn test_agent_token_budget_enforcement() {
    use phantom_ai::AgentRole;
    use phantom_core::agent_manager::AgentHandle;

    let mut handle = AgentHandle::new("backend-1", AgentRole::Backend);
    handle.token_budget = 1000;

    handle.tokens_consumed = 999;
    assert!(!handle.is_over_budget());

    handle.tokens_consumed = 1001;
    assert!(handle.is_over_budget());
}

#[test]
fn test_agent_manager_stats() {
    use phantom_core::agent_manager::AgentManager;

    let mut manager = AgentManager::new();
    manager.spawn_full_team().unwrap();

    let stats = manager.stats();
    assert_eq!(stats.total, 8);
    assert_eq!(stats.idle, 8);
    assert_eq!(stats.working, 0);
}

#[tokio::test]
async fn test_message_bus_send_receive() {
    use phantom_core::message_bus::{Message, MessageBus, MessageKind};

    let bus = MessageBus::new(64);

    let mut cto_mailbox = bus.register_agent("cto-1").await.unwrap();
    let _backend_mailbox = bus.register_agent("backend-1").await.unwrap();

    // Send a message to cto
    let msg = Message::new(
        "backend-1",
        "cto-1",
        MessageKind::TaskCompleted,
        serde_json::json!({"task": "build-api", "status": "done"}),
    );
    bus.send(msg).await.unwrap();

    // CTO receives it
    let received = cto_mailbox.try_recv();
    assert!(received.is_some());
    let received = received.unwrap();
    assert_eq!(received.from, "backend-1");
    assert_eq!(received.kind, MessageKind::TaskCompleted);
}

#[tokio::test]
async fn test_message_bus_broadcast() {
    use phantom_core::message_bus::{Message, MessageBus, MessageKind};

    let bus = MessageBus::new(64);

    let mut agent1 = bus.register_agent("agent-1").await.unwrap();
    let mut agent2 = bus.register_agent("agent-2").await.unwrap();

    // Broadcast a halt message
    let msg = Message::broadcast(
        "system",
        MessageKind::Halt,
        serde_json::json!({"reason": "test halt"}),
    );
    bus.broadcast(msg).await.unwrap();

    // Both agents should receive the broadcast
    let r1 = agent1.try_recv_broadcast();
    let r2 = agent2.try_recv_broadcast();
    assert!(r1.is_some());
    assert!(r2.is_some());
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. P2P CRDT Sync Between 2+ Simulated Nodes
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_crdt_two_node_sync() {
    use phantom_net::CrdtSync;

    let mut node_a = CrdtSync::new();
    let mut node_b = CrdtSync::new();

    node_a.put_str("project", "phantom").unwrap();
    node_a.put_u64("version", 1).unwrap();

    // Sync A→B, B→A, A→B until converged
    for _ in 0..3 {
        if let Some(msg) = node_a.generate_sync_message("node-b") {
            node_b.receive_sync_message("node-a", &msg).unwrap();
        }
        if let Some(msg) = node_b.generate_sync_message("node-a") {
            node_a.receive_sync_message("node-b", &msg).unwrap();
        }
    }

    assert_eq!(node_b.get_str("project"), Some("phantom".into()));
    assert_eq!(node_b.get_i64("version"), Some(1));
}

#[test]
fn test_crdt_concurrent_edits_converge() {
    use phantom_net::CrdtSync;

    let mut node_a = CrdtSync::new();
    let mut node_b = CrdtSync::new();

    node_a.put_str("owner", "alice").unwrap();
    node_b.put_str("role", "backend").unwrap();

    for _ in 0..3 {
        if let Some(msg) = node_a.generate_sync_message("node-b") {
            node_b.receive_sync_message("node-a", &msg).unwrap();
        }
        if let Some(msg) = node_b.generate_sync_message("node-a") {
            node_a.receive_sync_message("node-b", &msg).unwrap();
        }
    }

    assert_eq!(node_a.get_str("owner"), Some("alice".into()));
    assert_eq!(node_a.get_str("role"), Some("backend".into()));
    assert_eq!(node_b.get_str("owner"), Some("alice".into()));
    assert_eq!(node_b.get_str("role"), Some("backend".into()));
}

#[test]
fn test_crdt_conflict_resolution_deterministic() {
    use phantom_net::CrdtSync;

    let mut node_a = CrdtSync::new();
    let mut node_b = CrdtSync::new();

    // Same key, different values — conflict
    node_a.put_str("status", "active").unwrap();
    node_b.put_str("status", "paused").unwrap();

    for _ in 0..3 {
        if let Some(msg) = node_a.generate_sync_message("node-b") {
            node_b.receive_sync_message("node-a", &msg).unwrap();
        }
        if let Some(msg) = node_b.generate_sync_message("node-a") {
            node_a.receive_sync_message("node-b", &msg).unwrap();
        }
    }

    // Both must converge to the SAME value (deterministic LWW)
    let a_val = node_a.get_str("status");
    let b_val = node_b.get_str("status");
    assert!(a_val.is_some());
    assert_eq!(a_val, b_val, "nodes diverged after sync");
}

#[test]
fn test_crdt_save_and_load() {
    use phantom_net::CrdtSync;

    let mut node = CrdtSync::new();
    node.put_str("project", "phantom").unwrap();
    node.put_u64("tasks", 42).unwrap();
    node.put_bool("healthy", true).unwrap();

    let bytes = node.save();
    assert!(!bytes.is_empty());

    let loaded = CrdtSync::load(&bytes).unwrap();
    assert_eq!(loaded.get_str("project"), Some("phantom".into()));
    assert_eq!(loaded.get_i64("tasks"), Some(42));
    assert_eq!(loaded.get_bool("healthy"), Some(true));
}

#[test]
fn test_crdt_three_node_mesh() {
    use phantom_net::CrdtSync;

    let mut a = CrdtSync::new();
    let mut b = CrdtSync::new();
    let mut c = CrdtSync::new();

    a.put_str("from_a", "hello").unwrap();
    b.put_str("from_b", "world").unwrap();
    c.put_str("from_c", "!").unwrap();

    // Full mesh sync
    for _ in 0..3 {
        {
            let (id_x, id_y, x, y) = ("a", "b", &mut a as &mut CrdtSync, &mut b as &mut CrdtSync);
            if let Some(msg) = x.generate_sync_message(id_y) {
                y.receive_sync_message(id_x, &msg).unwrap();
            }
            if let Some(msg) = y.generate_sync_message(id_x) {
                x.receive_sync_message(id_y, &msg).unwrap();
            }
        }
        // B↔C
        if let Some(msg) = b.generate_sync_message("c") {
            c.receive_sync_message("b", &msg).unwrap();
        }
        if let Some(msg) = c.generate_sync_message("b") {
            b.receive_sync_message("c", &msg).unwrap();
        }
        // A↔C
        if let Some(msg) = a.generate_sync_message("c") {
            c.receive_sync_message("a", &msg).unwrap();
        }
        if let Some(msg) = c.generate_sync_message("a") {
            a.receive_sync_message("c", &msg).unwrap();
        }
    }

    for node in [&a, &b, &c] {
        assert_eq!(node.get_str("from_a"), Some("hello".into()));
        assert_eq!(node.get_str("from_b"), Some("world".into()));
        assert_eq!(node.get_str("from_c"), Some("!".into()));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. Self-Healing: Inject Failures at Each Layer
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_healing_layer1_retry() {
    use phantom_core::self_healer::{HealingLayer, SelfHealer};

    let healer = SelfHealer::new();

    let layer = healer.determine_layer(0, "connection timed out");
    assert_eq!(layer, HealingLayer::Retry);

    let result = healer.create_retry_result(0, true);
    assert!(result.success);
    assert_eq!(result.layer, HealingLayer::Retry);
    assert_eq!(result.attempts, 1);
}

#[test]
fn test_healing_layer2_alternative() {
    use phantom_core::self_healer::{HealingLayer, SelfHealer};

    let healer = SelfHealer::new();

    let layer = healer.determine_layer(5, "provider unavailable");
    assert_eq!(layer, HealingLayer::Alternative);

    let result = healer.create_alternative_result("use backup provider", true);
    assert!(result.success);
    assert_eq!(result.layer, HealingLayer::Alternative);
}

#[test]
fn test_healing_layer3_decompose() {
    use phantom_core::self_healer::{HealingLayer, SelfHealer};

    let healer = SelfHealer::new();

    let layer = healer.determine_layer(6, "task too complex");
    assert_eq!(layer, HealingLayer::Decompose);

    let result = healer.create_decompose_result(vec!["sub-task-1".into(), "sub-task-2".into()]);
    assert_eq!(result.layer, HealingLayer::Decompose);
    assert_eq!(result.sub_tasks.len(), 2);
}

#[test]
fn test_healing_layer4_escalate() {
    use phantom_core::self_healer::{HealingConfig, HealingLayer, SelfHealer};

    let healer = SelfHealer::with_config(HealingConfig {
        max_retries: 3,
        ..Default::default()
    });

    let layer = healer.determine_layer(3, "unknown error in module");
    assert_eq!(layer, HealingLayer::Escalate);

    let result = healer.create_escalation_result("security-agent", true);
    assert!(result.success);
    assert_eq!(result.escalated_to, Some("security-agent".into()));
}

#[test]
fn test_healing_layer5_pause_and_alert() {
    use phantom_core::self_healer::{HealingLayer, SelfHealer};

    let healer = SelfHealer::new();
    assert!(HealingLayer::PauseAndAlert.next().is_none());

    let result = healer.create_pause_result("all recovery options exhausted");
    assert!(!result.success);
    assert!(result.owner_notified);
    assert_eq!(result.layer, HealingLayer::PauseAndAlert);
}

#[test]
fn test_healing_full_escalation_chain() {
    use phantom_core::self_healer::HealingLayer;

    let mut layer = HealingLayer::Retry;
    let expected = [
        HealingLayer::Alternative,
        HealingLayer::Decompose,
        HealingLayer::Escalate,
        HealingLayer::PauseAndAlert,
    ];

    for exp in expected {
        layer = layer.next().unwrap();
        assert_eq!(layer, exp);
    }
    assert!(layer.next().is_none());
}

#[test]
fn test_healing_exponential_backoff() {
    use phantom_core::self_healer::SelfHealer;

    let healer = SelfHealer::new();

    let d0 = healer.backoff_delay(0);
    let d1 = healer.backoff_delay(1);
    let d2 = healer.backoff_delay(2);

    assert!(d1 > d0);
    assert!(d2 > d1);
    assert!(healer.backoff_delay(100).as_millis() <= 30_000);
}

#[test]
fn test_healing_exhaustion_detection() {
    use phantom_core::self_healer::{HealingLayer, SelfHealer};

    let healer = SelfHealer::new();

    // Not exhausted with just retry
    let layers_tried = vec![HealingLayer::Retry];
    assert!(!healer.is_exhausted(1, &layers_tried));

    // Exhausted when all layers tried including PauseAndAlert
    let all_layers = vec![
        HealingLayer::Retry,
        HealingLayer::Alternative,
        HealingLayer::Decompose,
        HealingLayer::Escalate,
        HealingLayer::PauseAndAlert,
    ];
    assert!(healer.is_exhausted(10, &all_layers));
}

// ═══════════════════════════════════════════════════════════════════════════
//  5. Zero-Footprint: Verify No Disk Artifacts After Session
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_zero_footprint_session_cleanup() {
    use phantom_core::zero_footprint::RuntimeSession;

    let tmp = std::env::temp_dir().join("phantom-integ-zfp-test");
    std::fs::create_dir_all(&tmp).unwrap();

    let secret_file = tmp.join("session-key.tmp");
    let data_file = tmp.join("task-cache.json");
    std::fs::write(&secret_file, b"secret-key-material").unwrap();
    std::fs::write(&data_file, b"{\"tasks\": []}").unwrap();

    let mut session = RuntimeSession::bootstrap(std::slice::from_ref(&tmp), false).unwrap();
    session.track_temp_file(secret_file.clone());
    session.track_temp_file(data_file.clone());
    session
        .protect_secret(b"master-key-bytes-32!!!!!!!!!!!!!!", true)
        .ok();

    let results = session.teardown();

    assert!(!secret_file.exists(), "secret file not cleaned up");
    assert!(!data_file.exists(), "data file not cleaned up");
    assert!(results.iter().all(|r| r.success));
    assert_eq!(session.secrets.count(), 0);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_zero_footprint_disk_policy_enforcement() {
    use phantom_core::zero_footprint::{guarded_write, DiskPolicy};

    let mut policy = DiskPolicy::strict();
    let vault = std::env::temp_dir().join("phantom-integ-vault");
    std::fs::create_dir_all(&vault).unwrap();
    policy.allow_prefix(&vault);

    // Inside vault → allowed
    let allowed = vault.join("allowed.dat");
    assert!(guarded_write(&policy, &allowed, b"ok").is_ok());

    // Outside vault → blocked
    let blocked = std::env::temp_dir().join("phantom-integ-outside.dat");
    assert!(policy.validate_write(&blocked).is_err());

    let _ = std::fs::remove_dir_all(&vault);
}

#[test]
fn test_zero_footprint_secure_delete() {
    use phantom_core::zero_footprint::secure_delete;

    let path = std::env::temp_dir().join("phantom-integ-secure-del");
    std::fs::write(&path, b"sensitive data that must be wiped").unwrap();
    assert!(path.exists());

    secure_delete(&path).unwrap();
    assert!(!path.exists());
}

#[test]
fn test_zero_footprint_guard_drop_cleanup() {
    use phantom_core::zero_footprint::SessionGuard;

    let f1 = std::env::temp_dir().join("phantom-integ-guard-1");
    let f2 = std::env::temp_dir().join("phantom-integ-guard-2");
    std::fs::write(&f1, b"temp1").unwrap();
    std::fs::write(&f2, b"temp2").unwrap();

    {
        let guard = SessionGuard::new();
        guard.track_file(f1.clone());
        guard.track_file(f2.clone());
    } // dropped

    assert!(!f1.exists());
    assert!(!f2.exists());
}

#[test]
fn test_zero_footprint_startup_validator() {
    use phantom_core::zero_footprint::StartupValidator;

    let validator = StartupValidator::new();
    let report = validator.validate();
    assert!(report.scanned_paths > 0);
}

// ═══════════════════════════════════════════════════════════════════════════
//  6. Doctor --full System Readiness
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_doctor_full_report() {
    use phantom_infra::doctor::Doctor;

    let doctor = Doctor::new();
    let report = doctor.run_full();

    assert!(report.total > 0);
    assert_eq!(
        report.total,
        report.ok_count + report.missing_count + report.warning_count + report.error_count,
    );

    // Should include Runtime, Disk, Network, Crypto categories
    let categories: std::collections::HashSet<&str> =
        report.results.iter().map(|r| r.category.as_str()).collect();
    assert!(categories.contains("Runtime"), "missing Runtime category");
    assert!(categories.contains("Disk"), "missing Disk category");
}

#[test]
fn test_doctor_format_report() {
    use phantom_infra::doctor::{DoctorReport, DoctorResult};
    use phantom_infra::format_report;

    let report = DoctorReport::from_results(vec![
        DoctorResult::ok("Git", "System", Some("2.40".into())),
        DoctorResult::ok("Rust", "Runtime", Some("1.80".into())),
        DoctorResult::warning("Docker", "System", "optional"),
    ]);

    let output = format_report(&report);
    assert!(output.contains("PHANTOM DOCTOR"));
    assert!(output.contains("Git"));
    assert!(output.contains("2/3 OK"));
    assert!(output.contains("HEALTHY"));
}

#[test]
fn test_doctor_provider_cli_checks() {
    use phantom_infra::doctor::Doctor;

    let doctor = Doctor::new();
    let results = doctor.check_provider_clis();

    assert!(!results.is_empty());
    for result in &results {
        assert_eq!(result.category, "Provider CLI");
    }
}

#[test]
fn test_doctor_crypto_checks() {
    use phantom_infra::doctor::Doctor;

    let doctor = Doctor::new();
    let results = doctor.check_crypto_requirements();

    assert!(!results.is_empty());
    for result in &results {
        assert_eq!(result.category, "Crypto");
    }
}

#[test]
fn test_doctor_runtime_checks() {
    use phantom_infra::doctor::Doctor;

    let doctor = Doctor::new();
    let results = doctor.check_runtime_environment();

    assert!(!results.is_empty());

    // Should detect the OS
    let os_check = results.iter().find(|r| r.name == "Operating System");
    assert!(os_check.is_some());
    assert_eq!(os_check.unwrap().version.as_deref(), Some("macos"));
}

// ═══════════════════════════════════════════════════════════════════════════
//  Cross-cutting: Scheduler → Job Queue
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_scheduler_feeds_job_queue() {
    use phantom_core::beyond_human::{GitEventKind, ScheduleRule, SelfScheduler, TriggerKind};
    use phantom_core::job_queue::{Job, JobQueue, JobStatus};

    let mut scheduler = SelfScheduler::new();
    scheduler.add_rule(ScheduleRule {
        id: "post-commit-lint".into(),
        description: "lint after commit".into(),
        trigger: TriggerKind::GitEvent(GitEventKind::PostCommit),
        agent_role: "qa".into(),
        payload: serde_json::json!({"action": "lint"}),
        priority: 5,
        enabled: true,
    });

    scheduler.notify_git_event(GitEventKind::PostCommit);

    let mut queue = JobQueue::new();
    for fired in scheduler.drain_pending() {
        let job = Job::new(
            fired.rule_id,
            fired.agent_role,
            fired.priority,
            fired.payload,
        );
        queue.enqueue(job);
    }

    assert_eq!(queue.queued_count(), 1);
    let job = queue.dequeue().unwrap();
    assert_eq!(job.agent_role, "qa");

    queue.complete(&job.id);
    assert_eq!(queue.get(&job.id).unwrap().status, JobStatus::Completed);
}

// ═══════════════════════════════════════════════════════════════════════════
//  Cross-cutting: Cost Oracle
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_cost_oracle_session_tracking() {
    use phantom_core::beyond_human::{CostOracle, ModelTier};

    let mut oracle = CostOracle::new();
    oracle.set_session_budget(5.0);

    oracle
        .record(ModelTier::Sonnet, 2000, 1000, "cto-planning")
        .unwrap();
    oracle
        .record(ModelTier::Haiku, 5000, 2000, "backend-codegen")
        .unwrap();

    assert!(oracle.total_spend_dollars() > 0.0);
    assert!(oracle.remaining_dollars().unwrap() < 5.0);
    assert_eq!(oracle.call_count(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
//  Cross-cutting: Predictive Scanner
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_predictive_scanner_integration() {
    use phantom_core::beyond_human::PredictiveScanner;

    let dir = std::env::temp_dir().join("phantom-integ-scanner");
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("clean.rs"),
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("bad.rs"),
        "fn main() {\n    let x = foo().unwrap();\n    dbg!(x);\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("conflict.txt"),
        "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> branch\n",
    )
    .unwrap();

    let scanner = PredictiveScanner::new();
    let report = scanner.scan(&dir).unwrap();

    assert!(report.files_scanned >= 3);
    assert!(report.warnings.iter().any(|w| w.rule == "unwrap_usage"));
    assert!(report.warnings.iter().any(|w| w.rule == "debug_print"));
    assert!(report
        .warnings
        .iter()
        .any(|w| w.rule == "unresolved_conflict"));
    assert!(report.has_blockers());

    let _ = std::fs::remove_dir_all(&dir);
}
