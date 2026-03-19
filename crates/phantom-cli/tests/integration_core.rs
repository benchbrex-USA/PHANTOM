//! Integration tests: core engine — task graph, message bus, self-healing, pipeline.

use phantom_core::{
    audit::{AuditAction, AuditLog},
    job_queue::{Job, JobQueue},
    message_bus::{Message, MessageBus, MessageKind},
    pipeline::{BuildPhase, BuildPipeline},
    self_healer::{HealingLayer, SelfHealer},
    task_graph::{Task, TaskGraph},
};

#[test]
fn test_task_graph_full_lifecycle() {
    let mut graph = TaskGraph::new();

    // Build a realistic task graph (Task::new takes name, description, agent_role)
    let setup_id = graph.add_task(Task::new("setup", "Setup infrastructure", "devops")).unwrap();
    let schema_id = graph.add_task(Task::new("schema", "Design database schema", "architect")).unwrap();

    let api_task = Task::new("api", "Build API endpoints", "backend")
        .depends_on(&setup_id)
        .depends_on(&schema_id);
    let api_id = graph.add_task(api_task).unwrap();

    let fe_task = Task::new("frontend", "Build frontend", "frontend")
        .depends_on(&schema_id);
    let fe_id = graph.add_task(fe_task).unwrap();

    let test_task = Task::new("test", "Run tests", "qa")
        .depends_on(&api_id)
        .depends_on(&fe_id);
    graph.add_task(test_task).unwrap();

    // Validate — should pass (no cycles)
    assert!(graph.validate().is_ok());

    // Parallel layers
    let layers = graph.parallel_layers().unwrap();
    assert!(layers.len() >= 2);

    // Ready tasks (no deps or all deps complete)
    let ready = graph.ready_tasks();
    assert!(ready.len() >= 2); // setup and schema are ready

    // Complete setup and schema
    graph.get_task_mut(&setup_id).unwrap().complete(None);
    graph.get_task_mut(&schema_id).unwrap().complete(None);

    // Now api and frontend should be ready
    let ready = graph.ready_tasks();
    assert_eq!(ready.len(), 2);
}

#[test]
fn test_task_graph_cycle_detection() {
    let mut graph = TaskGraph::new();

    let a_id = graph.add_task(Task::new("a", "Task A", "backend")).unwrap();
    let b_id = graph.add_task(Task::new("b", "Task B", "backend")).unwrap();
    let c_id = graph.add_task(Task::new("c", "Task C", "backend")).unwrap();

    // Create circular dependencies
    graph.get_task_mut(&a_id).unwrap().dependencies.push(c_id.clone());
    graph.get_task_mut(&b_id).unwrap().dependencies.push(a_id.clone());
    graph.get_task_mut(&c_id).unwrap().dependencies.push(b_id.clone());

    // Should detect the cycle
    assert!(graph.validate().is_err());
}

#[tokio::test]
async fn test_message_bus_agent_communication() {
    let bus = MessageBus::new(32);

    let _cto_inbox = bus.register_agent("cto").await.unwrap();
    let mut backend_inbox = bus.register_agent("backend").await.unwrap();

    // CTO sends a direct message to backend
    let msg = Message::new(
        "cto",
        "backend",
        MessageKind::TaskAssignment,
        serde_json::json!({"task_id": "api-build"}),
    );
    bus.send(msg).await.unwrap();

    // Backend should receive it
    let received = backend_inbox.recv().await;
    assert!(received.is_some());
    assert_eq!(received.unwrap().kind, MessageKind::TaskAssignment);
}

#[tokio::test]
async fn test_message_bus_broadcast() {
    let bus = MessageBus::new(32);

    let _cto = bus.register_agent("cto").await.unwrap();
    let mut backend = bus.register_agent("backend").await.unwrap();
    let mut frontend = bus.register_agent("frontend").await.unwrap();

    // Broadcast halt
    bus.halt_all("emergency").await.unwrap();

    // Both agents should receive it
    assert!(backend.recv_broadcast().await.is_some());
    assert!(frontend.recv_broadcast().await.is_some());
}

#[test]
fn test_self_healer_escalation() {
    let healer = SelfHealer::new();

    // Transient error → retry
    let result = healer.determine_layer(0, "connection timeout");
    assert_eq!(result, HealingLayer::Retry);

    // Provider unavailable → alternative
    let result = healer.determine_layer(0, "provider unavailable");
    assert_eq!(result, HealingLayer::Alternative);

    // Exhausted retries → escalate
    let result = healer.determine_layer(5, "persistent unknown error");
    assert_eq!(result, HealingLayer::Escalate);
}

#[test]
fn test_audit_log_integrity_chain() {
    let mut log = AuditLog::new();

    log.record("cto", AuditAction::AgentSpawned, "CTO started", serde_json::json!({}), None);
    log.record("architect", AuditAction::AgentSpawned, "Architect started", serde_json::json!({}), None);
    log.record("backend", AuditAction::TaskStarted, "Building API", serde_json::json!({"task": "api"}), Some("API_Expert/REST".into()));
    log.record("qa", AuditAction::TestsExecuted, "All tests pass", serde_json::json!({"passed": 42, "failed": 0}), None);
    log.record("security", AuditAction::SecurityAudit, "No vulnerabilities", serde_json::json!({"score": "A+"}), None);

    // Chain should be intact
    assert!(log.verify_integrity().is_ok());
    assert_eq!(log.len(), 5);

    // Export and verify
    let json = log.export_json().unwrap();
    assert!(json.contains("CTO started"));
    assert!(json.contains("API_Expert/REST"));
}

#[test]
fn test_build_pipeline_phase_progression() {
    let mut pipeline = BuildPipeline::new(Some("test_framework.md".into()));

    // Not started yet
    assert!(pipeline.current_phase.is_none());

    // Start the pipeline
    pipeline.start();
    assert_eq!(pipeline.current_phase, Some(BuildPhase::Ingest));

    // Progress through phases
    let next = pipeline.complete_current_phase();
    assert_eq!(next, Some(BuildPhase::Infrastructure));

    let next = pipeline.complete_current_phase();
    assert_eq!(next, Some(BuildPhase::Architecture));

    let next = pipeline.complete_current_phase();
    assert_eq!(next, Some(BuildPhase::Code));
}

#[test]
fn test_job_queue_priority_ordering() {
    let mut queue = JobQueue::new();

    queue.enqueue(Job::new("low-task", "backend", 10, serde_json::json!({"desc": "low"})));
    queue.enqueue(Job::new("high-task", "backend", 100, serde_json::json!({"desc": "high"})));
    queue.enqueue(Job::new("medium-task", "backend", 50, serde_json::json!({"desc": "medium"})));

    // Should come out in priority order (higher number = higher priority in BinaryHeap)
    let first = queue.dequeue().unwrap();
    assert_eq!(first.task_id, "high-task");
    assert_eq!(first.priority, 100);

    let second = queue.dequeue().unwrap();
    assert_eq!(second.task_id, "medium-task");
    assert_eq!(second.priority, 50);

    let third = queue.dequeue().unwrap();
    assert_eq!(third.task_id, "low-task");
    assert_eq!(third.priority, 10);
}
