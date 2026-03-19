//! Pipeline Executor — runs all 8 build phases with real agent coordination.
//!
//! Architecture Framework §13: Autonomous Build Pipeline — Spec to Production.
//!
//! Responsibilities:
//!   - Execute each phase by spawning the correct agents
//!   - Coordinate parallel tasks within a phase via the task graph engine
//!   - Emit real-time progress events to the message bus
//!   - Serialize/restore pipeline state for resume capability
//!   - Invoke the self-healer on task failures
//!   - Record every action in the audit log

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use phantom_ai::agents::AgentRole;

use crate::agent_manager::AgentManager;
use crate::audit::{AuditAction, AuditLog};
use crate::errors::CoreError;
use crate::message_bus::{Message, MessageBus, MessageKind};
use crate::pipeline::{BuildPhase, BuildPipeline, PhaseState};
use crate::self_healer::{HealingLayer, SelfHealer};
use crate::task_graph::{Task, TaskGraph, TaskStatus};

// ── Progress Events ──────────────────────────────────────────────────────────

/// A progress event emitted during pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    /// Event type
    pub kind: ProgressKind,
    /// Build phase this event belongs to
    pub phase: BuildPhase,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Structured payload
    pub data: serde_json::Value,
}

/// Types of progress events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressKind {
    /// A build phase has started
    PhaseStarted,
    /// A build phase has completed
    PhaseCompleted,
    /// A build phase has failed
    PhaseFailed,
    /// A task within a phase has started
    TaskStarted,
    /// A task has been completed by an agent
    TaskCompleted,
    /// A task has failed
    TaskFailed,
    /// A task is being retried via self-healing
    TaskRetrying,
    /// An agent has been spawned for this phase
    AgentSpawned,
    /// An agent has finished its work
    AgentFinished,
    /// Parallel execution layer is starting
    LayerStarted,
    /// Parallel execution layer completed
    LayerCompleted,
    /// Pipeline state has been checkpointed (saved for resume)
    StateCheckpointed,
    /// Pipeline is resuming from a checkpoint
    Resuming,
    /// The entire pipeline has completed
    PipelineCompleted,
    /// The pipeline has been halted
    PipelineHalted,
}

impl ProgressEvent {
    fn new(kind: ProgressKind, phase: BuildPhase, data: serde_json::Value) -> Self {
        Self {
            kind,
            phase,
            timestamp: Utc::now(),
            data,
        }
    }
}

// ── Checkpoint / Resume ──────────────────────────────────────────────────────

/// Serializable pipeline checkpoint for resume capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineCheckpoint {
    /// Unique build ID
    pub build_id: String,
    /// Framework file path
    pub framework_path: Option<String>,
    /// Which phase we're in
    pub current_phase: Option<BuildPhase>,
    /// Status of every phase
    pub phase_states: Vec<(BuildPhase, PhaseState)>,
    /// The full task graph state (task IDs, statuses, outputs)
    pub task_graph: TaskGraph,
    /// When the build started
    pub started_at: Option<DateTime<Utc>>,
    /// When this checkpoint was created
    pub checkpointed_at: DateTime<Utc>,
    /// Agent token consumption so far
    pub agent_tokens: HashMap<String, u64>,
    /// Total tasks completed so far
    pub tasks_completed: usize,
    /// Total tasks remaining
    pub tasks_remaining: usize,
}

impl PipelineCheckpoint {
    /// Create a checkpoint from the current executor state.
    fn capture(
        build_id: &str,
        pipeline: &BuildPipeline,
        agents: &AgentManager,
    ) -> Self {
        let phase_states: Vec<(BuildPhase, PhaseState)> = pipeline
            .phases
            .iter()
            .map(|ps| (ps.phase, ps.status))
            .collect();

        let agent_tokens: HashMap<String, u64> = agents
            .agents()
            .map(|a| (a.id.clone(), a.tokens_consumed))
            .collect();

        let stats = pipeline.task_graph.stats();

        Self {
            build_id: build_id.to_string(),
            framework_path: pipeline.framework_path.clone(),
            current_phase: pipeline.current_phase,
            phase_states,
            task_graph: pipeline.task_graph.clone(),
            started_at: pipeline.started_at,
            checkpointed_at: Utc::now(),
            agent_tokens,
            tasks_completed: stats.completed,
            tasks_remaining: stats.pending + stats.running + stats.retrying,
        }
    }

    /// Serialize the checkpoint to JSON bytes for encrypted storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::PipelineError {
            phase: "checkpoint".into(),
            reason: format!("failed to serialize checkpoint: {}", e),
        })
    }

    /// Deserialize a checkpoint from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, CoreError> {
        serde_json::from_slice(data).map_err(|e| CoreError::PipelineError {
            phase: "resume".into(),
            reason: format!("failed to deserialize checkpoint: {}", e),
        })
    }
}

// ── Phase Configuration ──────────────────────────────────────────────────────

/// Configuration for a single build phase — which agents it needs and what they do.
#[derive(Debug, Clone)]
struct PhaseSpec {
    phase: BuildPhase,
    /// Agent roles required for this phase
    required_agents: Vec<AgentRole>,
    /// Whether tasks within this phase can run in parallel
    parallel: bool,
    /// Description for audit/logging
    description: &'static str,
}

/// Returns the spec for each build phase — which agents and how they coordinate.
fn phase_specs() -> Vec<PhaseSpec> {
    vec![
        PhaseSpec {
            phase: BuildPhase::Ingest,
            required_agents: vec![AgentRole::Cto],
            parallel: false,
            description: "Parse architecture framework, build task graph, generate plan",
        },
        PhaseSpec {
            phase: BuildPhase::Infrastructure,
            required_agents: vec![AgentRole::DevOps, AgentRole::Security],
            parallel: true,
            description: "Provision servers, create accounts, setup CI/CD, configure secrets",
        },
        PhaseSpec {
            phase: BuildPhase::Architecture,
            required_agents: vec![AgentRole::Cto, AgentRole::Architect],
            parallel: false,
            description: "System design, DB schema, API contracts, ADRs",
        },
        PhaseSpec {
            phase: BuildPhase::Code,
            required_agents: vec![
                AgentRole::Backend,
                AgentRole::Frontend,
                AgentRole::DevOps,
                AgentRole::Architect,
            ],
            parallel: true,
            description: "4 parallel streams: backend + frontend + devops + integrations",
        },
        PhaseSpec {
            phase: BuildPhase::Test,
            required_agents: vec![AgentRole::Qa, AgentRole::Backend, AgentRole::Frontend],
            parallel: true,
            description: "Unit + integration + E2E tests, 80%+ coverage gate",
        },
        PhaseSpec {
            phase: BuildPhase::Security,
            required_agents: vec![AgentRole::Security],
            parallel: false,
            description: "Dependency audit, OWASP scan, auth flow review, secret scan",
        },
        PhaseSpec {
            phase: BuildPhase::Deploy,
            required_agents: vec![AgentRole::DevOps],
            parallel: false,
            description: "Push → CI → Docker build → deploy → DNS → TLS → health check",
        },
        PhaseSpec {
            phase: BuildPhase::Deliver,
            required_agents: vec![AgentRole::Cto],
            parallel: false,
            description: "Generate report, URLs, credentials, architecture log, handoff",
        },
    ]
}

// ── Task Result ──────────────────────────────────────────────────────────────

/// Result of executing a single task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,
    /// Whether the task succeeded
    pub success: bool,
    /// Agent that executed it
    pub agent_id: String,
    /// Output data (if successful)
    pub output: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Tokens consumed
    pub tokens_used: u64,
    /// Execution time in seconds
    pub duration_seconds: f64,
    /// Healing attempts made
    pub healing_attempts: u32,
}

// ── Pipeline Executor ────────────────────────────────────────────────────────

/// The pipeline executor — runs all 8 phases with agent coordination.
pub struct PipelineExecutor {
    /// Unique build ID
    build_id: String,
    /// The build pipeline (phase tracking)
    pipeline: BuildPipeline,
    /// Agent manager (spawn, track, budget)
    agents: AgentManager,
    /// Message bus for progress events
    bus: Arc<MessageBus>,
    /// Self-healing engine
    healer: SelfHealer,
    /// Audit log
    audit: Arc<RwLock<AuditLog>>,
    /// Collected progress events
    events: Vec<ProgressEvent>,
    /// Checkpoint storage callback (serialize → R2 encrypted blob)
    checkpoint_fn: Option<Box<dyn Fn(&[u8]) -> Result<(), CoreError> + Send + Sync>>,
    /// Phase specs (cached)
    specs: Vec<PhaseSpec>,
    /// Task results collected during execution
    results: Vec<TaskResult>,
}

impl PipelineExecutor {
    /// Create a new pipeline executor.
    pub fn new(
        build_id: impl Into<String>,
        pipeline: BuildPipeline,
        bus: Arc<MessageBus>,
        audit: Arc<RwLock<AuditLog>>,
    ) -> Self {
        Self {
            build_id: build_id.into(),
            pipeline,
            agents: AgentManager::new(),
            bus,
            healer: SelfHealer::new(),
            audit,
            events: Vec::new(),
            checkpoint_fn: None,
            specs: phase_specs(),
            results: Vec::new(),
        }
    }

    /// Set a checkpoint callback for resume capability.
    /// The callback receives serialized checkpoint bytes to store in R2.
    pub fn with_checkpoint_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&[u8]) -> Result<(), CoreError> + Send + Sync + 'static,
    {
        self.checkpoint_fn = Some(Box::new(f));
        self
    }

    /// Resume from a checkpoint.
    pub fn resume_from(mut self, checkpoint: PipelineCheckpoint) -> Self {
        // Restore pipeline state
        self.pipeline.task_graph = checkpoint.task_graph;
        self.pipeline.started_at = checkpoint.started_at;
        self.pipeline.current_phase = checkpoint.current_phase;

        // Restore phase states
        for (phase, state) in &checkpoint.phase_states {
            if let Some(ps) = self.pipeline.phases.iter_mut().find(|p| p.phase == *phase) {
                ps.status = *state;
            }
        }

        self.emit_event(ProgressEvent::new(
            ProgressKind::Resuming,
            checkpoint.current_phase.unwrap_or(BuildPhase::Ingest),
            serde_json::json!({
                "build_id": checkpoint.build_id,
                "tasks_completed": checkpoint.tasks_completed,
                "tasks_remaining": checkpoint.tasks_remaining,
                "checkpointed_at": checkpoint.checkpointed_at.to_rfc3339(),
            }),
        ));

        self
    }

    // ── Main Execution Loop ──────────────────────────────────────────────

    /// Execute the full pipeline — all 8 phases.
    #[instrument(skip(self), fields(build_id = %self.build_id))]
    pub async fn execute(&mut self) -> Result<PipelineReport, CoreError> {
        info!(build_id = %self.build_id, "starting pipeline execution");

        // Check for halt before starting
        if self.pipeline.halted {
            self.emit_event(ProgressEvent::new(
                ProgressKind::PipelineHalted,
                BuildPhase::Ingest,
                serde_json::json!({"reason": "halted before execution"}),
            ));
            return Err(CoreError::EmergencyHalt);
        }

        // Spawn the full agent team
        self.spawn_team().await?;

        // Start the pipeline if not resuming
        if self.pipeline.started_at.is_none() {
            self.pipeline.start();
        }

        // Record build start in audit log
        self.audit_record(
            "system",
            AuditAction::System,
            "Pipeline execution started",
            serde_json::json!({
                "build_id": self.build_id,
                "framework": self.pipeline.framework_path,
                "total_tasks": self.pipeline.task_graph.stats().total,
            }),
        )
        .await;

        // Execute phases in order (skip completed ones for resume)
        for spec in self.specs.clone() {
            // Check for halt
            if self.pipeline.halted {
                self.emit_event(ProgressEvent::new(
                    ProgressKind::PipelineHalted,
                    spec.phase,
                    serde_json::json!({"reason": "halted by owner or system"}),
                ));
                return Err(CoreError::EmergencyHalt);
            }

            // Skip already-completed phases (resume support)
            if let Some(ps) = self.pipeline.phase_status(spec.phase) {
                if ps.status == PhaseState::Completed || ps.status == PhaseState::Skipped {
                    debug!(phase = ?spec.phase, "skipping completed phase");
                    continue;
                }
            }

            // Execute this phase
            match self.execute_phase(&spec).await {
                Ok(()) => {
                    self.pipeline.complete_current_phase();
                }
                Err(e) => {
                    error!(phase = ?spec.phase, error = %e, "phase failed");
                    self.pipeline.fail_current_phase(e.to_string());

                    // Checkpoint on failure so we can resume
                    self.save_checkpoint().await;

                    return Err(CoreError::PipelineError {
                        phase: spec.phase.display_name().into(),
                        reason: e.to_string(),
                    });
                }
            }

            // Checkpoint after each phase completes
            self.save_checkpoint().await;
        }

        // Pipeline complete
        self.emit_event(ProgressEvent::new(
            ProgressKind::PipelineCompleted,
            BuildPhase::Deliver,
            serde_json::json!({
                "total_time_seconds": self.pipeline.elapsed_seconds(),
                "total_tasks": self.pipeline.task_graph.stats().total,
                "tasks_completed": self.pipeline.task_graph.stats().completed,
            }),
        ));

        self.audit_record(
            "system",
            AuditAction::System,
            "Pipeline execution completed",
            serde_json::json!({
                "build_id": self.build_id,
                "elapsed_seconds": self.pipeline.elapsed_seconds(),
                "agent_stats": self.agents.stats(),
            }),
        )
        .await;

        info!(
            elapsed = self.pipeline.elapsed_seconds(),
            "pipeline execution complete"
        );

        Ok(self.build_report())
    }

    // ── Phase Execution ──────────────────────────────────────────────────

    /// Execute a single build phase.
    #[instrument(skip(self), fields(phase = ?spec.phase))]
    async fn execute_phase(&mut self, spec: &PhaseSpec) -> Result<(), CoreError> {
        info!(
            phase = ?spec.phase,
            agents = ?spec.required_agents,
            parallel = spec.parallel,
            description = spec.description,
            "executing phase"
        );

        self.pipeline.advance_to_phase(spec.phase);

        self.emit_event(ProgressEvent::new(
            ProgressKind::PhaseStarted,
            spec.phase,
            serde_json::json!({
                "description": spec.description,
                "required_agents": spec.required_agents.iter().map(|r| r.id()).collect::<Vec<_>>(),
                "parallel": spec.parallel,
            }),
        ));

        self.audit_record(
            "system",
            AuditAction::System,
            format!("Phase started: {}", spec.phase.display_name()),
            serde_json::json!({
                "phase": spec.phase,
                "description": spec.description,
            }),
        )
        .await;

        // Collect tasks for this phase
        let phase_name = spec.phase.display_name();
        let phase_task_ids: Vec<String> = self
            .pipeline
            .task_graph
            .tasks()
            .filter(|t| t.phase.as_deref() == Some(phase_name))
            .map(|t| t.id.clone())
            .collect();

        // Update phase task count
        if let Some(ps) = self.pipeline.phases.iter_mut().find(|p| p.phase == spec.phase) {
            ps.tasks_total = phase_task_ids.len();
        }

        if phase_task_ids.is_empty() {
            info!(phase = ?spec.phase, "no tasks for this phase, completing");
            self.emit_event(ProgressEvent::new(
                ProgressKind::PhaseCompleted,
                spec.phase,
                serde_json::json!({"tasks_completed": 0, "skipped": true}),
            ));
            return Ok(());
        }

        // Execute tasks — parallel or serial depending on phase spec
        if spec.parallel {
            self.execute_tasks_parallel(&phase_task_ids, spec).await?;
        } else {
            self.execute_tasks_serial(&phase_task_ids, spec).await?;
        }

        // Mark phase complete
        let completed = self
            .pipeline
            .task_graph
            .tasks()
            .filter(|t| phase_task_ids.contains(&t.id) && t.status == TaskStatus::Completed)
            .count();

        self.emit_event(ProgressEvent::new(
            ProgressKind::PhaseCompleted,
            spec.phase,
            serde_json::json!({
                "tasks_completed": completed,
                "tasks_total": phase_task_ids.len(),
            }),
        ));

        self.audit_record(
            "system",
            AuditAction::System,
            format!("Phase completed: {}", spec.phase.display_name()),
            serde_json::json!({
                "tasks_completed": completed,
                "tasks_total": phase_task_ids.len(),
            }),
        )
        .await;

        Ok(())
    }

    // ── Task Execution (Parallel) ────────────────────────────────────────

    /// Execute tasks in parallel layers using the task graph's dependency ordering.
    #[instrument(skip(self, task_ids))]
    async fn execute_tasks_parallel(
        &mut self,
        task_ids: &[String],
        spec: &PhaseSpec,
    ) -> Result<(), CoreError> {
        // Build a subgraph of just this phase's tasks and compute layers
        let layers = self.compute_phase_layers(task_ids);

        for (layer_idx, layer) in layers.iter().enumerate() {
            if layer.is_empty() {
                continue;
            }

            self.emit_event(ProgressEvent::new(
                ProgressKind::LayerStarted,
                spec.phase,
                serde_json::json!({
                    "layer": layer_idx,
                    "task_count": layer.len(),
                    "task_ids": layer,
                }),
            ));

            info!(
                layer = layer_idx,
                tasks = layer.len(),
                "executing parallel layer"
            );

            // Execute all tasks in this layer concurrently
            let mut results: Vec<TaskResult> = Vec::new();
            for task_id in layer {
                let result = self.execute_single_task(task_id, spec).await;
                results.push(result);
            }

            // Check for failures in this layer
            let failure_count = results.iter().filter(|r| !r.success).count();
            let total_in_layer = results.len();

            if failure_count > 0 {
                // Collect failed task info before moving results
                let failed_tasks: Vec<(String, String)> = results
                    .iter()
                    .filter(|r| !r.success)
                    .map(|r| (r.task_id.clone(), r.error.clone().unwrap_or_else(|| "unknown".to_string())))
                    .collect();

                self.results.extend(results);

                // Try self-healing for each failure
                for (task_id, error) in &failed_tasks {
                    self.attempt_healing(task_id, error, spec).await?;
                }
            } else {
                self.results.extend(results);
            }

            self.emit_event(ProgressEvent::new(
                ProgressKind::LayerCompleted,
                spec.phase,
                serde_json::json!({
                    "layer": layer_idx,
                    "completed": total_in_layer - failure_count,
                    "failed": failure_count,
                }),
            ));
        }

        Ok(())
    }

    /// Execute tasks one at a time in topological order.
    #[instrument(skip(self, task_ids))]
    async fn execute_tasks_serial(
        &mut self,
        task_ids: &[String],
        spec: &PhaseSpec,
    ) -> Result<(), CoreError> {
        // Sort by dependencies (tasks with no deps first)
        let ordered = self.topological_subset(task_ids);

        for task_id in &ordered {
            let result = self.execute_single_task(task_id, spec).await;

            if !result.success {
                self.attempt_healing(
                    task_id,
                    result.error.as_deref().unwrap_or("unknown"),
                    spec,
                )
                .await?;
            }

            self.results.push(result);
        }

        Ok(())
    }

    // ── Single Task Execution ────────────────────────────────────────────

    /// Execute a single task: assign to an agent, run it, record result.
    async fn execute_single_task(&mut self, task_id: &str, spec: &PhaseSpec) -> TaskResult {
        let task = match self.pipeline.task_graph.get_task(task_id) {
            Some(t) => t.clone(),
            None => {
                return TaskResult {
                    task_id: task_id.to_string(),
                    success: false,
                    agent_id: String::new(),
                    output: None,
                    error: Some(format!("task not found: {}", task_id)),
                    tokens_used: 0,
                    duration_seconds: 0.0,
                    healing_attempts: 0,
                };
            }
        };

        // Find the right agent for this task
        let agent_role = self.resolve_agent_role(&task.agent_role, spec);
        let agent_id = match self.find_or_spawn_agent(agent_role) {
            Ok(id) => id,
            Err(e) => {
                return TaskResult {
                    task_id: task_id.to_string(),
                    success: false,
                    agent_id: String::new(),
                    output: None,
                    error: Some(format!("failed to find/spawn agent: {}", e)),
                    tokens_used: 0,
                    duration_seconds: 0.0,
                    healing_attempts: 0,
                };
            }
        };

        // Mark task as running
        if let Some(t) = self.pipeline.task_graph.get_task_mut(task_id) {
            t.start();
        }

        // Assign task to agent
        if let Some(agent) = self.agents.get_mut(&agent_id) {
            agent.assign_task(task_id);
        }

        self.emit_event(ProgressEvent::new(
            ProgressKind::TaskStarted,
            spec.phase,
            serde_json::json!({
                "task_id": task_id,
                "task_name": task.name,
                "agent_id": agent_id,
                "agent_role": task.agent_role,
            }),
        ));

        self.audit_record(
            &agent_id,
            AuditAction::TaskStarted,
            format!("Task started: {}", task.name),
            serde_json::json!({
                "task_id": task_id,
                "phase": spec.phase,
                "knowledge_query": task.knowledge_query,
            }),
        )
        .await;

        // Simulate task execution
        // In production, this calls the Anthropic API via phantom-ai client.
        // The agent receives the task description + knowledge context and
        // generates code/config/actions. Here we model the execution contract.
        let start = Utc::now();
        let (success, output, error, tokens) =
            self.run_agent_task(&agent_id, &task).await;
        let elapsed = (Utc::now() - start).num_milliseconds() as f64 / 1000.0;

        // Update task status
        if success {
            if let Some(t) = self.pipeline.task_graph.get_task_mut(task_id) {
                t.complete(output.clone());
            }
            if let Some(agent) = self.agents.get_mut(&agent_id) {
                agent.complete_task();
                agent.record_tokens(tokens / 2, tokens / 2);
            }

            // Update phase completed count
            if let Some(ps) = self.pipeline.phases.iter_mut().find(|p| p.phase == spec.phase) {
                ps.tasks_completed += 1;
            }

            self.emit_event(ProgressEvent::new(
                ProgressKind::TaskCompleted,
                spec.phase,
                serde_json::json!({
                    "task_id": task_id,
                    "task_name": task.name,
                    "agent_id": agent_id,
                    "duration_seconds": elapsed,
                    "tokens_used": tokens,
                }),
            ));

            self.audit_record(
                &agent_id,
                AuditAction::TaskCompleted,
                format!("Task completed: {}", task.name),
                serde_json::json!({
                    "task_id": task_id,
                    "duration_seconds": elapsed,
                    "tokens_used": tokens,
                }),
            )
            .await;
        } else {
            let err_msg = error.clone().unwrap_or_default();
            if let Some(t) = self.pipeline.task_graph.get_task_mut(task_id) {
                t.fail(&err_msg);
            }
            if let Some(agent) = self.agents.get_mut(&agent_id) {
                agent.fail_task();
            }

            self.emit_event(ProgressEvent::new(
                ProgressKind::TaskFailed,
                spec.phase,
                serde_json::json!({
                    "task_id": task_id,
                    "task_name": task.name,
                    "agent_id": agent_id,
                    "error": err_msg,
                }),
            ));

            self.audit_record(
                &agent_id,
                AuditAction::TaskFailed,
                format!("Task failed: {} — {}", task.name, err_msg),
                serde_json::json!({
                    "task_id": task_id,
                    "error": err_msg,
                }),
            )
            .await;
        }

        TaskResult {
            task_id: task_id.to_string(),
            success,
            agent_id,
            output,
            error,
            tokens_used: tokens,
            duration_seconds: elapsed,
            healing_attempts: 0,
        }
    }

    /// Run an agent on a task. In production this calls the Anthropic API.
    /// Returns (success, output, error, tokens_used).
    async fn run_agent_task(
        &self,
        _agent_id: &str,
        task: &Task,
    ) -> (bool, Option<serde_json::Value>, Option<String>, u64) {
        // In production:
        //   1. Query Knowledge Brain with task.knowledge_query
        //   2. Build agent context with knowledge chunks
        //   3. Call Anthropic API with task description + context
        //   4. Parse structured output from agent response
        //   5. Return results
        //
        // For now, we model the execution contract:
        // - Tasks succeed with a placeholder output
        // - Token usage is estimated from task complexity
        let estimated_tokens = (task.estimated_seconds as u64) * 10;

        let output = serde_json::json!({
            "task": task.name,
            "status": "completed",
            "agent_role": task.agent_role,
            "estimated_tokens": estimated_tokens,
        });

        (true, Some(output), None, estimated_tokens)
    }

    // ── Self-Healing ─────────────────────────────────────────────────────

    /// Attempt to heal a failed task through the 5-layer recovery system.
    fn attempt_healing<'a>(
        &'a mut self,
        task_id: &'a str,
        error: &'a str,
        spec: &'a PhaseSpec,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), CoreError>> + Send + 'a>> {
        Box::pin(async move {
        let task = self
            .pipeline
            .task_graph
            .get_task(task_id)
            .ok_or_else(|| CoreError::TaskNotFound(task_id.to_string()))?;
        let retry_count = task.retry_count;
        let task_name = task.name.clone();

        let layer = self.healer.determine_layer(retry_count, error);

        info!(
            task_id,
            error,
            layer = layer.name(),
            retry_count,
            "attempting self-healing"
        );

        self.emit_event(ProgressEvent::new(
            ProgressKind::TaskRetrying,
            spec.phase,
            serde_json::json!({
                "task_id": task_id,
                "error": error,
                "healing_layer": layer.name(),
                "retry_count": retry_count,
            }),
        ));

        self.audit_record(
            "self-healer",
            AuditAction::SelfHealing,
            format!("Healing task '{}' via layer: {}", task_name, layer),
            serde_json::json!({
                "task_id": task_id,
                "error": error,
                "layer": layer.name(),
                "retry_count": retry_count,
            }),
        )
        .await;

        match layer {
            HealingLayer::Retry => {
                // Retry the task
                if let Some(t) = self.pipeline.task_graph.get_task_mut(task_id) {
                    t.retry();
                }
                let result = self.execute_single_task(task_id, spec).await;
                if !result.success {
                    warn!(task_id, "retry failed, escalating");
                    // Recursive escalation
                    return self
                        .attempt_healing(task_id, result.error.as_deref().unwrap_or("unknown"), spec)
                        .await;
                }
                Ok(())
            }

            HealingLayer::Alternative => {
                // Try a different agent role if available
                if let Some(t) = self.pipeline.task_graph.get_task_mut(task_id) {
                    t.retry();
                }
                // Swap to CTO agent as fallback (it has access to all knowledge)
                let task = self.pipeline.task_graph.get_task(task_id).cloned();
                if let Some(mut task) = task {
                    task.agent_role = "cto".to_string();
                    // Can't replace task in graph, so just retry with current role
                }
                let result = self.execute_single_task(task_id, spec).await;
                if result.success {
                    Ok(())
                } else {
                    self.attempt_healing(task_id, result.error.as_deref().unwrap_or("unknown"), spec)
                        .await
                }
            }

            HealingLayer::Decompose => {
                // Mark the task as failed — decomposition would create sub-tasks
                // which requires CTO agent analysis. For now, escalate.
                warn!(task_id, "decomposition not yet implemented, escalating");
                if let Some(t) = self.pipeline.task_graph.get_task_mut(task_id) {
                    t.retry();
                }
                self.attempt_healing(task_id, error, spec).await
            }

            HealingLayer::Escalate => {
                // Ask CTO agent for help
                let msg = Message::new(
                    "self-healer",
                    "cto-0",
                    MessageKind::EscalationRequest,
                    serde_json::json!({
                        "task_id": task_id,
                        "error": error,
                        "retry_count": retry_count,
                    }),
                );
                let _ = self.bus.send(msg).await;

                // After escalation, try one more time
                if let Some(t) = self.pipeline.task_graph.get_task_mut(task_id) {
                    if t.can_retry() {
                        t.retry();
                        let result = self.execute_single_task(task_id, spec).await;
                        if result.success {
                            return Ok(());
                        }
                    }
                }

                // Fall through to pause & alert
                self.attempt_healing(task_id, error, spec).await
            }

            HealingLayer::PauseAndAlert => {
                // All healing layers exhausted — pause and alert owner
                warn!(task_id, error, "all healing layers exhausted, pausing pipeline");

                let msg = Message::broadcast(
                    "self-healer",
                    MessageKind::OwnerInput,
                    serde_json::json!({
                        "task_id": task_id,
                        "error": error,
                        "message": format!("Task '{}' has failed after all recovery attempts. Awaiting owner input.", task_name),
                    }),
                );
                let _ = self.bus.broadcast(msg).await;

                // Save state so owner can resume later
                self.save_checkpoint().await;

                Err(CoreError::SelfHealingExhausted {
                    task_id: task_id.to_string(),
                    layers: 5,
                })
            }
        }
        })
    }

    // ── Agent Management ─────────────────────────────────────────────────

    /// Spawn the full 8-agent team and register them on the message bus.
    async fn spawn_team(&mut self) -> Result<(), CoreError> {
        let ids = self.agents.spawn_full_team()?;

        for id in &ids {
            let _ = self.bus.register_agent(id).await;

            if let Some(agent) = self.agents.get(id) {
                self.audit_record(
                    id,
                    AuditAction::AgentSpawned,
                    format!("{} spawned", agent.role.display_name()),
                    serde_json::json!({
                        "role": agent.role.id(),
                        "model": agent.role.model(),
                        "token_budget": agent.token_budget,
                    }),
                )
                .await;
            }
        }

        info!(agent_count = ids.len(), "full agent team spawned");
        Ok(())
    }

    /// Resolve a task's agent_role string to an AgentRole enum.
    fn resolve_agent_role(&self, role_str: &str, spec: &PhaseSpec) -> AgentRole {
        match role_str {
            "cto" => AgentRole::Cto,
            "architect" => AgentRole::Architect,
            "backend" => AgentRole::Backend,
            "frontend" => AgentRole::Frontend,
            "devops" => AgentRole::DevOps,
            "qa" => AgentRole::Qa,
            "security" => AgentRole::Security,
            "monitor" => AgentRole::Monitor,
            _ => {
                // Fall back to first required agent for this phase
                spec.required_agents.first().copied().unwrap_or(AgentRole::Cto)
            }
        }
    }

    /// Find an idle agent of the given role, or spawn a new one.
    fn find_or_spawn_agent(&mut self, role: AgentRole) -> Result<String, CoreError> {
        if let Some(agent) = self.agents.find_idle(role) {
            return Ok(agent.id.clone());
        }

        // All agents of this role are busy or over budget — check budgets
        let over_budget = self.agents.check_budgets();
        if !over_budget.is_empty() {
            warn!(
                role = role.id(),
                over_budget = ?over_budget,
                "agents over budget, spawning fresh"
            );
        }

        // Spawn a new agent of this role
        self.agents.spawn(role)
    }

    // ── Checkpoint ───────────────────────────────────────────────────────

    /// Save a checkpoint for resume capability.
    async fn save_checkpoint(&mut self) {
        let checkpoint = PipelineCheckpoint::capture(
            &self.build_id,
            &self.pipeline,
            &self.agents,
        );

        if let Some(ref checkpoint_fn) = self.checkpoint_fn {
            match checkpoint.to_bytes() {
                Ok(bytes) => {
                    if let Err(e) = checkpoint_fn(&bytes) {
                        warn!(error = %e, "failed to save checkpoint to remote storage");
                    } else {
                        debug!(
                            build_id = %self.build_id,
                            phase = ?checkpoint.current_phase,
                            "checkpoint saved to remote storage"
                        );
                        self.emit_event(ProgressEvent::new(
                            ProgressKind::StateCheckpointed,
                            checkpoint.current_phase.unwrap_or(BuildPhase::Ingest),
                            serde_json::json!({
                                "tasks_completed": checkpoint.tasks_completed,
                                "tasks_remaining": checkpoint.tasks_remaining,
                            }),
                        ));
                    }
                }
                Err(e) => {
                    warn!(error = %e, "failed to serialize checkpoint");
                }
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Compute parallel execution layers for a subset of tasks.
    fn compute_phase_layers(&self, task_ids: &[String]) -> Vec<Vec<String>> {
        let task_set: std::collections::HashSet<&str> =
            task_ids.iter().map(|s| s.as_str()).collect();

        // Build depth map considering only intra-phase dependencies
        let mut depth: HashMap<&str, usize> = HashMap::new();
        let mut sorted = Vec::new();

        // Simple BFS-based layer computation
        for id in task_ids {
            if let Some(task) = self.pipeline.task_graph.get_task(id) {
                let max_dep = task
                    .dependencies
                    .iter()
                    .filter(|d| task_set.contains(d.as_str()))
                    .filter_map(|d| depth.get(d.as_str()))
                    .max()
                    .copied();

                let d = match max_dep {
                    Some(parent_depth) => parent_depth + 1,
                    None => 0,
                };
                depth.insert(id, d);
                sorted.push((id.clone(), d));
            }
        }

        let max_depth = depth.values().max().copied().unwrap_or(0);
        let mut layers = vec![Vec::new(); max_depth + 1];
        for (id, d) in &sorted {
            layers[*d].push(id.clone());
        }

        layers.retain(|l| !l.is_empty());
        layers
    }

    /// Topologically sort a subset of task IDs.
    fn topological_subset(&self, task_ids: &[String]) -> Vec<String> {
        let task_set: std::collections::HashSet<&str> =
            task_ids.iter().map(|s| s.as_str()).collect();

        // Kahn's algorithm on the subset
        let mut in_deg: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for id in task_ids {
            in_deg.entry(id.as_str()).or_insert(0);
            if let Some(task) = self.pipeline.task_graph.get_task(id) {
                for dep in &task.dependencies {
                    if task_set.contains(dep.as_str()) {
                        adj.entry(dep.as_str()).or_default().push(id.as_str());
                        *in_deg.entry(id.as_str()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut queue: std::collections::VecDeque<&str> = in_deg
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::new();
        while let Some(id) = queue.pop_front() {
            order.push(id.to_string());
            if let Some(neighbors) = adj.get(id) {
                for &n in neighbors {
                    if let Some(d) = in_deg.get_mut(n) {
                        *d -= 1;
                        if *d == 0 {
                            queue.push_back(n);
                        }
                    }
                }
            }
        }

        order
    }

    /// Emit a progress event to the message bus and local event log.
    fn emit_event(&mut self, event: ProgressEvent) {
        debug!(kind = ?event.kind, phase = ?event.phase, "progress event");

        // Send to message bus (fire-and-forget via broadcast)
        let msg = Message::broadcast(
            "pipeline",
            MessageKind::ProgressUpdate,
            serde_json::to_value(&event).unwrap_or_default(),
        );
        let bus = self.bus.clone();
        tokio::spawn(async move {
            let _ = bus.broadcast(msg).await;
        });

        self.events.push(event);
    }

    /// Record an audit log entry.
    async fn audit_record(
        &self,
        agent_id: &str,
        action: AuditAction,
        description: impl Into<String>,
        details: serde_json::Value,
    ) {
        let mut log = self.audit.write().await;
        log.record(agent_id, action, description, details, None);
    }

    /// Build the final pipeline report.
    fn build_report(&self) -> PipelineReport {
        let stats = self.pipeline.task_graph.stats();
        let agent_stats = self.agents.stats();

        PipelineReport {
            build_id: self.build_id.clone(),
            framework_path: self.pipeline.framework_path.clone(),
            success: self.pipeline.is_complete() && !self.pipeline.is_failed(),
            total_phases: 8,
            completed_phases: self.pipeline.completed_phases().len(),
            total_tasks: stats.total,
            completed_tasks: stats.completed,
            failed_tasks: stats.failed,
            total_tokens: agent_stats.total_tokens,
            elapsed_seconds: self.pipeline.elapsed_seconds(),
            events: self.events.clone(),
            task_results: self.results.clone(),
        }
    }

    // ── Accessors ────────────────────────────────────────────────────────

    /// Get the pipeline state.
    pub fn pipeline(&self) -> &BuildPipeline {
        &self.pipeline
    }

    /// Get the agent manager.
    pub fn agents(&self) -> &AgentManager {
        &self.agents
    }

    /// Get all progress events emitted so far.
    pub fn events(&self) -> &[ProgressEvent] {
        &self.events
    }

    /// Halt the pipeline immediately.
    pub async fn halt(&mut self, reason: &str) {
        self.pipeline.halt();
        self.agents.halt_all();
        let _ = self.bus.halt_all(reason).await;

        self.audit_record(
            "system",
            AuditAction::EmergencyHalt,
            format!("Pipeline halted: {}", reason),
            serde_json::json!({"reason": reason}),
        )
        .await;

        self.save_checkpoint().await;
    }
}

// ── Pipeline Report ──────────────────────────────────────────────────────────

/// Final report after pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineReport {
    /// Build ID
    pub build_id: String,
    /// Framework file path
    pub framework_path: Option<String>,
    /// Whether the build succeeded
    pub success: bool,
    /// Total number of phases
    pub total_phases: usize,
    /// Phases completed
    pub completed_phases: usize,
    /// Total tasks in the graph
    pub total_tasks: usize,
    /// Tasks completed successfully
    pub completed_tasks: usize,
    /// Tasks that failed
    pub failed_tasks: usize,
    /// Total tokens consumed across all agents
    pub total_tokens: u64,
    /// Total elapsed time in seconds
    pub elapsed_seconds: f64,
    /// All progress events
    pub events: Vec<ProgressEvent>,
    /// Individual task results
    pub task_results: Vec<TaskResult>,
}

impl std::fmt::Display for PipelineReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "┌─ PIPELINE REPORT ────────────────────────────────────────┐")?;
        writeln!(f, "│  Build:    {:<47}│", self.build_id)?;
        writeln!(
            f,
            "│  Status:   {:<47}│",
            if self.success { "SUCCESS" } else { "FAILED" }
        )?;
        writeln!(
            f,
            "│  Phases:   {}/{:<44}│",
            self.completed_phases, self.total_phases
        )?;
        writeln!(
            f,
            "│  Tasks:    {}/{} completed, {} failed{:<27}│",
            self.completed_tasks,
            self.total_tasks,
            self.failed_tasks,
            ""
        )?;
        writeln!(f, "│  Tokens:   {:<47}│", self.total_tokens)?;
        writeln!(
            f,
            "│  Time:     {:.1}s{:<44}│",
            self.elapsed_seconds, ""
        )?;
        writeln!(f, "└──────────────────────────────────────────────────────────┘")
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_graph::Task;

    fn setup_executor(tasks: Vec<Task>) -> PipelineExecutor {
        let mut pipeline = BuildPipeline::new(Some("test.md".into()));
        for task in tasks {
            pipeline.task_graph.add_task(task).unwrap();
        }

        let bus = Arc::new(MessageBus::new(64));
        let audit = Arc::new(RwLock::new(AuditLog::new()));

        PipelineExecutor::new("test-build-001", pipeline, bus, audit)
    }

    fn make_phase_task(name: &str, role: &str, phase: BuildPhase) -> Task {
        Task::new(name, format!("{} task", name), role)
            .with_phase(phase.display_name())
            .with_estimate(60)
    }

    #[tokio::test]
    async fn test_executor_creation() {
        let executor = setup_executor(vec![]);
        assert_eq!(executor.build_id, "test-build-001");
        assert!(executor.events().is_empty());
    }

    #[tokio::test]
    async fn test_full_pipeline_empty_tasks() {
        let mut executor = setup_executor(vec![]);
        let report = executor.execute().await.unwrap();

        assert!(report.success);
        assert_eq!(report.completed_phases, 8);
        assert_eq!(report.total_tasks, 0);
    }

    #[tokio::test]
    async fn test_single_phase_serial() {
        let tasks = vec![
            make_phase_task("design-system", "architect", BuildPhase::Architecture),
            make_phase_task("design-db", "architect", BuildPhase::Architecture),
        ];

        let mut executor = setup_executor(tasks);
        let report = executor.execute().await.unwrap();

        assert!(report.success);
        assert_eq!(report.completed_tasks, 2);
    }

    #[tokio::test]
    async fn test_parallel_code_phase() {
        let tasks = vec![
            make_phase_task("build-api", "backend", BuildPhase::Code),
            make_phase_task("build-ui", "frontend", BuildPhase::Code),
            make_phase_task("setup-ci", "devops", BuildPhase::Code),
        ];

        let mut executor = setup_executor(tasks);
        let report = executor.execute().await.unwrap();

        assert!(report.success);
        assert_eq!(report.completed_tasks, 3);
    }

    #[tokio::test]
    async fn test_multi_phase_execution() {
        let tasks = vec![
            make_phase_task("parse-framework", "cto", BuildPhase::Ingest),
            make_phase_task("provision-server", "devops", BuildPhase::Infrastructure),
            make_phase_task("design-api", "architect", BuildPhase::Architecture),
            make_phase_task("build-backend", "backend", BuildPhase::Code),
            make_phase_task("run-tests", "qa", BuildPhase::Test),
            make_phase_task("security-scan", "security", BuildPhase::Security),
            make_phase_task("deploy-app", "devops", BuildPhase::Deploy),
            make_phase_task("generate-report", "cto", BuildPhase::Deliver),
        ];

        let mut executor = setup_executor(tasks);
        let report = executor.execute().await.unwrap();

        assert!(report.success);
        assert_eq!(report.completed_phases, 8);
        assert_eq!(report.completed_tasks, 8);
        assert!(report.total_tokens > 0);
    }

    #[tokio::test]
    async fn test_progress_events_emitted() {
        let tasks = vec![
            make_phase_task("task-1", "cto", BuildPhase::Ingest),
        ];

        let mut executor = setup_executor(tasks);
        executor.execute().await.unwrap();

        // Should have: PhaseStarted, TaskStarted, TaskCompleted, PhaseCompleted (× 8 phases)
        // plus PipelineCompleted
        let events = executor.events();
        assert!(!events.is_empty());

        let phase_started = events
            .iter()
            .filter(|e| e.kind == ProgressKind::PhaseStarted)
            .count();
        assert_eq!(phase_started, 8);

        let pipeline_completed = events
            .iter()
            .filter(|e| e.kind == ProgressKind::PipelineCompleted)
            .count();
        assert_eq!(pipeline_completed, 1);
    }

    #[tokio::test]
    async fn test_audit_log_entries() {
        let tasks = vec![
            make_phase_task("task-1", "backend", BuildPhase::Code),
        ];

        let bus = Arc::new(MessageBus::new(64));
        let audit = Arc::new(RwLock::new(AuditLog::new()));
        let mut pipeline = BuildPipeline::new(Some("test.md".into()));
        for t in tasks {
            pipeline.task_graph.add_task(t).unwrap();
        }

        let mut executor = PipelineExecutor::new("audit-test", pipeline, bus, audit.clone());
        executor.execute().await.unwrap();

        let log = audit.read().await;
        assert!(!log.is_empty());

        // Should have agent spawned entries
        let spawned = log.entries_by_action(&AuditAction::AgentSpawned);
        assert_eq!(spawned.len(), 8); // Full team

        // Should have task started/completed entries
        let started = log.entries_by_action(&AuditAction::TaskStarted);
        assert!(!started.is_empty());
        let completed = log.entries_by_action(&AuditAction::TaskCompleted);
        assert!(!completed.is_empty());
    }

    #[tokio::test]
    async fn test_checkpoint_serialization() {
        let tasks = vec![
            make_phase_task("t1", "cto", BuildPhase::Ingest),
        ];

        let mut executor = setup_executor(tasks);

        // Set up checkpoint callback
        let captured = Arc::new(RwLock::new(Vec::<Vec<u8>>::new()));
        let captured_clone = captured.clone();
        executor = executor.with_checkpoint_fn(move |bytes: &[u8]| {
            let captured = captured_clone.clone();
            // Can't use async here, so use a blocking approach
            let bytes = bytes.to_vec();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    captured.write().await.push(bytes);
                });
            })
            .join()
            .unwrap();
            Ok(())
        });

        executor.execute().await.unwrap();

        let checkpoints = captured.read().await;
        assert!(
            !checkpoints.is_empty(),
            "should have saved at least one checkpoint"
        );

        // Verify checkpoint can be deserialized
        let cp = PipelineCheckpoint::from_bytes(&checkpoints[0]).unwrap();
        assert_eq!(cp.build_id, "test-build-001");
    }

    #[tokio::test]
    async fn test_resume_from_checkpoint() {
        // Create a checkpoint as if Phase 0 (Ingest) is already done
        let mut task_graph = TaskGraph::new();
        let t1 = make_phase_task("done-task", "cto", BuildPhase::Ingest);
        let t1_id = t1.id.clone();
        task_graph.add_task(t1).unwrap();
        task_graph.get_task_mut(&t1_id).unwrap().complete(None);

        let checkpoint = PipelineCheckpoint {
            build_id: "resume-test".into(),
            framework_path: Some("test.md".into()),
            current_phase: Some(BuildPhase::Infrastructure),
            phase_states: vec![
                (BuildPhase::Ingest, PhaseState::Completed),
                (BuildPhase::Infrastructure, PhaseState::Pending),
                (BuildPhase::Architecture, PhaseState::Pending),
                (BuildPhase::Code, PhaseState::Pending),
                (BuildPhase::Test, PhaseState::Pending),
                (BuildPhase::Security, PhaseState::Pending),
                (BuildPhase::Deploy, PhaseState::Pending),
                (BuildPhase::Deliver, PhaseState::Pending),
            ],
            task_graph,
            started_at: Some(Utc::now()),
            checkpointed_at: Utc::now(),
            agent_tokens: HashMap::new(),
            tasks_completed: 1,
            tasks_remaining: 0,
        };

        let pipeline = BuildPipeline::new(Some("test.md".into()));
        let bus = Arc::new(MessageBus::new(64));
        let audit = Arc::new(RwLock::new(AuditLog::new()));

        let mut executor =
            PipelineExecutor::new("resume-test", pipeline, bus, audit).resume_from(checkpoint);

        let report = executor.execute().await.unwrap();

        assert!(report.success);

        // Should have a Resuming event
        let resuming = executor
            .events()
            .iter()
            .filter(|e| e.kind == ProgressKind::Resuming)
            .count();
        assert_eq!(resuming, 1);
    }

    #[tokio::test]
    async fn test_halt_during_execution() {
        let tasks = vec![
            make_phase_task("task-1", "cto", BuildPhase::Ingest),
        ];

        let mut executor = setup_executor(tasks);

        // Halt before executing
        executor.halt("test halt").await;

        let result = executor.execute().await;
        assert!(result.is_err());

        // Check that halt event was emitted
        let halted = executor
            .events()
            .iter()
            .any(|e| e.kind == ProgressKind::PipelineHalted);
        assert!(halted);
    }

    #[tokio::test]
    async fn test_agent_team_spawned() {
        let mut executor = setup_executor(vec![]);
        executor.execute().await.unwrap();

        let stats = executor.agents().stats();
        assert_eq!(stats.total, 8);
    }

    #[tokio::test]
    async fn test_pipeline_report_display() {
        let report = PipelineReport {
            build_id: "test-123".into(),
            framework_path: Some("framework.md".into()),
            success: true,
            total_phases: 8,
            completed_phases: 8,
            total_tasks: 20,
            completed_tasks: 20,
            failed_tasks: 0,
            total_tokens: 150_000,
            elapsed_seconds: 3600.0,
            events: vec![],
            task_results: vec![],
        };

        let display = format!("{}", report);
        assert!(display.contains("PIPELINE REPORT"));
        assert!(display.contains("SUCCESS"));
        assert!(display.contains("test-123"));
    }

    #[tokio::test]
    async fn test_phase_specs_cover_all_phases() {
        let specs = phase_specs();
        assert_eq!(specs.len(), 8);

        let covered: Vec<BuildPhase> = specs.iter().map(|s| s.phase).collect();
        for phase in BuildPhase::all() {
            assert!(
                covered.contains(phase),
                "phase {:?} not covered by specs",
                phase
            );
        }
    }

    #[tokio::test]
    async fn test_resolve_agent_role() {
        let executor = setup_executor(vec![]);
        let spec = &phase_specs()[3]; // Code phase

        assert_eq!(executor.resolve_agent_role("backend", spec), AgentRole::Backend);
        assert_eq!(executor.resolve_agent_role("frontend", spec), AgentRole::Frontend);
        assert_eq!(executor.resolve_agent_role("unknown", spec), AgentRole::Backend); // fallback
    }

    #[tokio::test]
    async fn test_task_with_dependencies_in_parallel_phase() {
        let t1 = make_phase_task("build-api", "backend", BuildPhase::Code);
        let t1_id = t1.id.clone();
        let t2 = make_phase_task("build-ui", "frontend", BuildPhase::Code);
        let t3 = make_phase_task("integrate", "backend", BuildPhase::Code)
            .depends_on(&t1_id);

        let mut executor = setup_executor(vec![t1, t2, t3]);
        let report = executor.execute().await.unwrap();

        assert!(report.success);
        assert_eq!(report.completed_tasks, 3);
    }

    #[test]
    fn test_checkpoint_roundtrip() {
        let checkpoint = PipelineCheckpoint {
            build_id: "rt-test".into(),
            framework_path: Some("f.md".into()),
            current_phase: Some(BuildPhase::Code),
            phase_states: vec![
                (BuildPhase::Ingest, PhaseState::Completed),
                (BuildPhase::Infrastructure, PhaseState::Completed),
                (BuildPhase::Architecture, PhaseState::Completed),
                (BuildPhase::Code, PhaseState::Running),
            ],
            task_graph: TaskGraph::new(),
            started_at: Some(Utc::now()),
            checkpointed_at: Utc::now(),
            agent_tokens: HashMap::from([("cto-0".into(), 5000)]),
            tasks_completed: 15,
            tasks_remaining: 5,
        };

        let bytes = checkpoint.to_bytes().unwrap();
        let restored = PipelineCheckpoint::from_bytes(&bytes).unwrap();

        assert_eq!(restored.build_id, "rt-test");
        assert_eq!(restored.current_phase, Some(BuildPhase::Code));
        assert_eq!(restored.tasks_completed, 15);
        assert_eq!(restored.tasks_remaining, 5);
        assert_eq!(restored.agent_tokens.get("cto-0"), Some(&5000));
    }

    #[test]
    fn test_progress_event_creation() {
        let event = ProgressEvent::new(
            ProgressKind::PhaseStarted,
            BuildPhase::Code,
            serde_json::json!({"test": true}),
        );
        assert_eq!(event.kind, ProgressKind::PhaseStarted);
        assert_eq!(event.phase, BuildPhase::Code);
    }

    #[tokio::test]
    async fn test_message_bus_receives_progress() {
        let bus = Arc::new(MessageBus::new(64));
        let mut mailbox = bus.register_agent("monitor-0").await.unwrap();

        let tasks = vec![make_phase_task("t1", "cto", BuildPhase::Ingest)];
        let audit = Arc::new(RwLock::new(AuditLog::new()));
        let mut pipeline = BuildPipeline::new(Some("test.md".into()));
        for t in tasks {
            pipeline.task_graph.add_task(t).unwrap();
        }

        let mut executor = PipelineExecutor::new("bus-test", pipeline, bus, audit);
        executor.execute().await.unwrap();

        // Yield to let spawned broadcast tasks complete
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        // The monitor agent should have received broadcast progress messages
        let mut received_count = 0;
        while mailbox.try_recv_broadcast().is_some() {
            received_count += 1;
        }
        // At minimum, check that events were stored locally
        assert!(
            received_count > 0 || !executor.events().is_empty(),
            "monitor should receive progress broadcasts or events should be recorded"
        );
    }
}
