//! Parallel task executor — runs task graphs with maximum concurrency.
//!
//! Uses `TaskGraph::parallel_layers()` for layer-based parallelism and
//! work-stealing for eager execution of tasks whose dependencies resolve early.
//!
//! Integrates with the agent manager, message bus, self-healer, and metrics.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

use phantom_ai::agents::AgentRole;

use crate::agent_manager::AgentManager;
use crate::errors::CoreError;
use crate::message_bus::{Message, MessageBus, MessageKind};
use crate::self_healer::{HealingLayer, SelfHealer};
use crate::task_graph::{TaskGraph, TaskStatus};

// ── Parallel Executor ───────────────────────────────────────────────────────

/// Production-grade parallel task executor.
///
/// Executes a task graph with maximum parallelism by running independent tasks
/// concurrently. Uses layer-based scheduling with work-stealing: when an agent
/// finishes early, it pulls ready tasks from later layers.
pub struct ParallelExecutor {
    task_graph: Arc<RwLock<TaskGraph>>,
    agent_manager: Arc<RwLock<AgentManager>>,
    message_bus: Arc<MessageBus>,
    self_healer: Arc<SelfHealer>,
    metrics: Arc<RwLock<ExecutionMetrics>>,
}

impl ParallelExecutor {
    /// Create a new parallel executor.
    pub fn new(
        task_graph: TaskGraph,
        agent_manager: AgentManager,
        message_bus: MessageBus,
        self_healer: SelfHealer,
    ) -> Self {
        Self {
            task_graph: Arc::new(RwLock::new(task_graph)),
            agent_manager: Arc::new(RwLock::new(agent_manager)),
            message_bus: Arc::new(message_bus),
            self_healer: Arc::new(self_healer),
            metrics: Arc::new(RwLock::new(ExecutionMetrics::default())),
        }
    }

    /// Create from pre-wrapped Arc references (for embedding in a larger system).
    pub fn from_shared(
        task_graph: Arc<RwLock<TaskGraph>>,
        agent_manager: Arc<RwLock<AgentManager>>,
        message_bus: Arc<MessageBus>,
        self_healer: Arc<SelfHealer>,
    ) -> Self {
        Self {
            task_graph,
            agent_manager,
            message_bus,
            self_healer,
            metrics: Arc::new(RwLock::new(ExecutionMetrics::default())),
        }
    }

    /// Execute the full task graph with maximum parallelism.
    ///
    /// Processes layers in order. Within each layer, all tasks execute concurrently
    /// via `tokio::JoinSet`. After each task completes, a work-stealing pass checks
    /// for tasks from later layers whose dependencies are already satisfied.
    pub async fn execute_graph(&self) -> Result<ExecutionReport, CoreError> {
        let wall_start = Instant::now();

        // Validate the graph before execution
        {
            let graph = self.task_graph.read().await;
            graph.validate()?;
            if graph.is_empty() {
                return Ok(ExecutionReport {
                    total_tasks: 0,
                    completed: 0,
                    failed: 0,
                    cancelled: 0,
                    wall_time_secs: 0.0,
                    sequential_estimate_secs: 0.0,
                    parallel_efficiency: 1.0,
                    metrics: ExecutionMetrics::default(),
                });
            }
        }

        // Get layer schedule
        let layers = {
            let graph = self.task_graph.read().await;
            graph.parallel_layers()?
        };

        let total_layers = layers.len();
        info!(layers = total_layers, "starting parallel graph execution");

        self.emit_progress("graph_started", serde_json::json!({
            "total_layers": total_layers,
        }))
        .await;

        for (layer_idx, layer_task_ids) in layers.iter().enumerate() {
            // Check if the agent manager has been halted
            {
                let mgr = self.agent_manager.read().await;
                if mgr.is_halted() {
                    warn!("agent manager halted, cancelling remaining tasks");
                    self.cancel_remaining().await;
                    break;
                }
            }

            info!(
                layer = layer_idx,
                tasks = layer_task_ids.len(),
                "executing layer"
            );

            self.emit_progress("layer_started", serde_json::json!({
                "layer": layer_idx,
                "task_count": layer_task_ids.len(),
            }))
            .await;

            self.execute_layer(layer_task_ids).await?;

            // Work-stealing pass: after completing a layer, check if any tasks
            // from later layers now have all deps satisfied
            let stolen = self.work_steal_pass().await?;
            if stolen > 0 {
                info!(
                    layer = layer_idx,
                    stolen_tasks = stolen,
                    "work-stealing picked up tasks from later layers"
                );
            }

            self.emit_progress("layer_completed", serde_json::json!({
                "layer": layer_idx,
                "stolen_tasks": stolen,
            }))
            .await;

            // Propagate failures so downstream tasks get blocked
            {
                let mut graph = self.task_graph.write().await;
                graph.propagate_failures();
            }

            // Check for terminal failure
            {
                let graph = self.task_graph.read().await;
                if graph.has_terminal_failure() {
                    error!("terminal failure detected, halting execution");
                    break;
                }
            }
        }

        let wall_time = wall_start.elapsed().as_secs_f64();

        let report = {
            let graph = self.task_graph.read().await;
            let stats = graph.stats();
            let metrics = self.metrics.read().await;
            let sequential_estimate = stats.total_estimated_seconds as f64;
            let parallel_efficiency = if wall_time > 0.0 && sequential_estimate > 0.0 {
                (sequential_estimate / wall_time).min(stats.total as f64)
            } else {
                1.0
            };

            ExecutionReport {
                total_tasks: stats.total,
                completed: stats.completed,
                failed: stats.failed,
                cancelled: stats.cancelled,
                wall_time_secs: wall_time,
                sequential_estimate_secs: sequential_estimate,
                parallel_efficiency,
                metrics: metrics.clone(),
            }
        };

        info!(
            completed = report.completed,
            failed = report.failed,
            wall_time = format!("{:.2}s", report.wall_time_secs),
            efficiency = format!("{:.1}x", report.parallel_efficiency),
            "graph execution complete"
        );

        self.emit_progress("graph_completed", serde_json::json!({
            "completed": report.completed,
            "failed": report.failed,
            "wall_time_secs": report.wall_time_secs,
            "parallel_efficiency": report.parallel_efficiency,
        }))
        .await;

        Ok(report)
    }

    /// Execute all tasks in a single layer concurrently.
    async fn execute_layer(&self, task_ids: &[String]) -> Result<(), CoreError> {
        let mut join_set = JoinSet::new();

        for task_id in task_ids {
            // Skip tasks that are not pending/retrying (may have been stolen or cancelled)
            let should_run = {
                let graph = self.task_graph.read().await;
                graph
                    .get_task(task_id)
                    .map(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::Retrying)
                    .unwrap_or(false)
            };

            if !should_run {
                debug!(task_id = %task_id, "skipping non-pending task in layer");
                continue;
            }

            let task_id = task_id.clone();
            let graph = Arc::clone(&self.task_graph);
            let agent_mgr = Arc::clone(&self.agent_manager);
            let msg_bus = Arc::clone(&self.message_bus);
            let healer = Arc::clone(&self.self_healer);
            let metrics = Arc::clone(&self.metrics);

            join_set.spawn(async move {
                Self::execute_task_inner(
                    &task_id, &graph, &agent_mgr, &msg_bus, &healer, &metrics,
                )
                .await
            });
        }

        // Collect results — we don't fail the whole layer on individual task failure
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    warn!(error = %e, "task execution returned error");
                }
                Err(join_err) => {
                    error!(error = %join_err, "task panicked");
                }
            }
        }

        Ok(())
    }

    /// Execute a single task: resolve agent, run, record metrics.
    async fn execute_task_inner(
        task_id: &str,
        graph: &Arc<RwLock<TaskGraph>>,
        agent_mgr: &Arc<RwLock<AgentManager>>,
        msg_bus: &Arc<MessageBus>,
        healer: &Arc<SelfHealer>,
        metrics: &Arc<RwLock<ExecutionMetrics>>,
    ) -> Result<(), CoreError> {
        let task_start = Instant::now();

        // Extract task metadata
        let (task_name, agent_role_str, task_description) = {
            let mut g = graph.write().await;
            let task = g
                .get_task_mut(task_id)
                .ok_or_else(|| CoreError::TaskNotFound(task_id.to_string()))?;
            task.start();
            (
                task.name.clone(),
                task.agent_role.clone(),
                task.description.clone(),
            )
        };

        info!(
            task_id = %task_id,
            task_name = %task_name,
            agent_role = %agent_role_str,
            "executing task"
        );

        // Resolve agent role
        let agent_role = resolve_role(&agent_role_str);

        // Find or use an idle agent
        let agent_id = {
            let mut mgr = agent_mgr.write().await;
            if let Some(handle) = mgr.find_idle(agent_role) {
                let id = handle.id.clone();
                if let Some(h) = mgr.get_mut(&id) {
                    h.assign_task(task_id);
                }
                id
            } else {
                // Spawn a new agent if none idle
                let id = mgr.spawn(agent_role)?;
                if let Some(h) = mgr.get_mut(&id) {
                    h.assign_task(task_id);
                }
                id
            }
        };

        // Emit task started event
        let _ = msg_bus
            .broadcast(Message::broadcast(
                &agent_id,
                MessageKind::ProgressUpdate,
                serde_json::json!({
                    "event": "task_started",
                    "task_id": task_id,
                    "task_name": task_name,
                    "agent_id": agent_id,
                }),
            ))
            .await;

        // ── Placeholder for LLM call ────────────────────────────────────────
        // In production this would:
        //   1. Build prompt from task description + knowledge context
        //   2. Route to the correct provider via ModelRouter
        //   3. Stream the response and collect output
        //   4. Parse tool calls and execute them
        //
        // For now we simulate success with a placeholder.
        let execution_result: Result<serde_json::Value, String> = {
            debug!(
                task_id = %task_id,
                description = %task_description,
                "placeholder: would invoke LLM here"
            );
            Ok(serde_json::json!({
                "status": "placeholder_complete",
                "agent": agent_id,
                "description": task_description,
            }))
        };

        // Simulated token usage for metrics
        let input_tokens: u64 = 500;
        let output_tokens: u64 = 200;

        match execution_result {
            Ok(output) => {
                // Mark task completed
                {
                    let mut g = graph.write().await;
                    if let Some(task) = g.get_task_mut(task_id) {
                        task.complete(Some(output));
                    }
                }

                // Update agent state
                {
                    let mut mgr = agent_mgr.write().await;
                    if let Some(h) = mgr.get_mut(&agent_id) {
                        h.record_tokens(input_tokens, output_tokens);
                        h.complete_task();
                    }
                }

                // Record metrics
                {
                    let elapsed = task_start.elapsed().as_secs_f64();
                    let mut m = metrics.write().await;
                    m.total_tasks_executed += 1;
                    *m.tasks_per_role.entry(agent_role_str.clone()).or_insert(0) += 1;
                    m.task_latencies.push(elapsed);
                    *m.tokens_per_provider
                        .entry("placeholder".to_string())
                        .or_insert(0) += input_tokens + output_tokens;
                }

                info!(
                    task_id = %task_id,
                    elapsed = format!("{:.2}s", task_start.elapsed().as_secs_f64()),
                    "task completed"
                );

                // Emit completion event
                let _ = msg_bus
                    .broadcast(Message::broadcast(
                        &agent_id,
                        MessageKind::TaskCompleted,
                        serde_json::json!({
                            "task_id": task_id,
                            "task_name": task_name,
                            "agent_id": agent_id,
                            "elapsed_secs": task_start.elapsed().as_secs_f64(),
                        }),
                    ))
                    .await;
            }
            Err(err) => {
                warn!(task_id = %task_id, error = %err, "task failed");

                // Attempt self-healing
                let healed = Self::attempt_healing(task_id, &err, graph, healer).await;

                if healed {
                    info!(task_id = %task_id, "task recovered via self-healing");
                } else {
                    // Mark as failed
                    {
                        let mut g = graph.write().await;
                        if let Some(task) = g.get_task_mut(task_id) {
                            task.fail(&err);
                        }
                    }

                    // Record failure metrics
                    {
                        let mut m = metrics.write().await;
                        *m.errors_per_provider
                            .entry("placeholder".to_string())
                            .or_insert(0) += 1;
                    }
                }

                // Update agent state
                {
                    let mut mgr = agent_mgr.write().await;
                    if let Some(h) = mgr.get_mut(&agent_id) {
                        h.fail_task();
                    }
                }

                // Emit failure event
                let _ = msg_bus
                    .broadcast(Message::broadcast(
                        &agent_id,
                        MessageKind::TaskFailed,
                        serde_json::json!({
                            "task_id": task_id,
                            "task_name": task_name,
                            "error": err,
                        }),
                    ))
                    .await;
            }
        }

        Ok(())
    }

    /// Attempt self-healing for a failed task.
    async fn attempt_healing(
        task_id: &str,
        error: &str,
        graph: &Arc<RwLock<TaskGraph>>,
        healer: &Arc<SelfHealer>,
    ) -> bool {
        let (retry_count, can_retry) = {
            let g = graph.read().await;
            match g.get_task(task_id) {
                Some(t) => (t.retry_count, t.can_retry()),
                None => return false,
            }
        };

        if !can_retry {
            return false;
        }

        let layer = healer.determine_layer(retry_count, error);

        match layer {
            HealingLayer::Retry => {
                let delay = healer.backoff_delay(retry_count);
                debug!(
                    task_id = %task_id,
                    attempt = retry_count + 1,
                    delay_ms = delay.as_millis(),
                    "scheduling retry"
                );
                tokio::time::sleep(delay).await;

                {
                    let mut g = graph.write().await;
                    if let Some(task) = g.get_task_mut(task_id) {
                        task.retry();
                    }
                }
                true
            }
            _ => {
                // Layers beyond Retry need more sophisticated handling
                // (alternative providers, task decomposition, escalation).
                // For now, mark for retry and let the next execution pass handle it.
                debug!(
                    task_id = %task_id,
                    layer = %layer,
                    "healing layer not yet implemented, falling back to retry"
                );
                {
                    let mut g = graph.write().await;
                    if let Some(task) = g.get_task_mut(task_id) {
                        task.retry();
                    }
                }
                true
            }
        }
    }

    /// Work-stealing pass: find tasks from later layers whose deps are all satisfied.
    async fn work_steal_pass(&self) -> Result<usize, CoreError> {
        let ready_ids: Vec<String> = {
            let graph = self.task_graph.read().await;
            graph
                .ready_tasks()
                .iter()
                .map(|t| t.id.clone())
                .collect()
        };

        if ready_ids.is_empty() {
            return Ok(0);
        }

        let count = ready_ids.len();
        self.execute_layer(&ready_ids).await?;
        Ok(count)
    }

    /// Cancel all remaining non-completed tasks.
    async fn cancel_remaining(&self) {
        let mut graph = self.task_graph.write().await;
        graph.cancel_all();
    }

    /// Emit a progress event via the message bus.
    async fn emit_progress(&self, event: &str, data: serde_json::Value) {
        let msg = Message::broadcast("parallel_executor", MessageKind::ProgressUpdate, {
            let mut payload = data;
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("event".to_string(), serde_json::json!(event));
                obj.insert("timestamp".to_string(), serde_json::json!(Utc::now().to_rfc3339()));
            }
            payload
        });
        let _ = self.message_bus.broadcast(msg).await;
    }

    /// Get a snapshot of current execution metrics.
    pub async fn metrics(&self) -> ExecutionMetrics {
        self.metrics.read().await.clone()
    }

    /// Get a reference to the underlying task graph.
    pub fn task_graph(&self) -> &Arc<RwLock<TaskGraph>> {
        &self.task_graph
    }

    /// Get a reference to the agent manager.
    pub fn agent_manager(&self) -> &Arc<RwLock<AgentManager>> {
        &self.agent_manager
    }
}

// ── Execution Metrics ───────────────────────────────────────────────────────

/// Comprehensive execution metrics collected during graph execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Total tasks executed (including retries)
    pub total_tasks_executed: usize,
    /// Tasks completed per agent role
    pub tasks_per_role: HashMap<String, usize>,
    /// Individual task latencies in seconds
    pub task_latencies: Vec<f64>,
    /// Token usage per provider
    pub tokens_per_provider: HashMap<String, u64>,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Errors per provider
    pub errors_per_provider: HashMap<String, u64>,
}

impl ExecutionMetrics {
    /// Average task latency in seconds.
    pub fn avg_latency_secs(&self) -> f64 {
        if self.task_latencies.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.task_latencies.iter().sum();
        sum / self.task_latencies.len() as f64
    }

    /// P95 task latency in seconds.
    pub fn p95_latency_secs(&self) -> f64 {
        if self.task_latencies.is_empty() {
            return 0.0;
        }
        let mut sorted = self.task_latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((sorted.len() as f64 * 0.95) as usize).min(sorted.len() - 1);
        sorted[idx]
    }

    /// Total tokens consumed across all providers.
    pub fn total_tokens(&self) -> u64 {
        self.tokens_per_provider.values().sum()
    }

    /// Cache hit rate (0.0 to 1.0).
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / total as f64
    }

    /// Error rate per provider (0.0 to 1.0).
    pub fn error_rate(&self, provider: &str) -> f64 {
        let errors = self
            .errors_per_provider
            .get(provider)
            .copied()
            .unwrap_or(0) as f64;
        if self.total_tasks_executed == 0 {
            return 0.0;
        }
        errors / self.total_tasks_executed as f64
    }

    /// Total error count across all providers.
    pub fn total_errors(&self) -> u64 {
        self.errors_per_provider.values().sum()
    }
}

// ── Execution Report ────────────────────────────────────────────────────────

/// Final report produced after graph execution completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    /// Total tasks in the graph
    pub total_tasks: usize,
    /// Tasks that completed successfully
    pub completed: usize,
    /// Tasks that failed permanently
    pub failed: usize,
    /// Tasks that were cancelled
    pub cancelled: usize,
    /// Actual wall-clock time in seconds
    pub wall_time_secs: f64,
    /// Estimated sequential execution time in seconds
    pub sequential_estimate_secs: f64,
    /// Parallel efficiency: sequential_estimate / wall_time (higher = better)
    pub parallel_efficiency: f64,
    /// Detailed execution metrics
    pub metrics: ExecutionMetrics,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Resolve a string agent role to the `AgentRole` enum.
/// Falls back to `AgentRole::Backend` for unknown roles.
fn resolve_role(role_str: &str) -> AgentRole {
    match role_str.to_lowercase().as_str() {
        "cto" => AgentRole::Cto,
        "architect" => AgentRole::Architect,
        "backend" => AgentRole::Backend,
        "frontend" => AgentRole::Frontend,
        "devops" => AgentRole::DevOps,
        "qa" => AgentRole::Qa,
        "security" => AgentRole::Security,
        "monitor" => AgentRole::Monitor,
        _ => {
            warn!(role = %role_str, "unknown agent role, defaulting to Backend");
            AgentRole::Backend
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_graph::Task;

    fn make_task(name: &str, role: &str) -> Task {
        Task::new(name, format!("{} description", name), role)
    }

    fn setup_executor(graph: TaskGraph) -> ParallelExecutor {
        let agent_mgr = AgentManager::new();
        let msg_bus = MessageBus::new(64);
        let healer = SelfHealer::new();
        ParallelExecutor::new(graph, agent_mgr, msg_bus, healer)
    }

    #[tokio::test]
    async fn test_empty_graph() {
        let executor = setup_executor(TaskGraph::new());
        let report = executor.execute_graph().await.expect("should succeed");
        assert_eq!(report.total_tasks, 0);
        assert_eq!(report.completed, 0);
        assert_eq!(report.parallel_efficiency, 1.0);
    }

    #[tokio::test]
    async fn test_single_task() {
        let mut graph = TaskGraph::new();
        graph
            .add_task(make_task("setup", "backend").with_estimate(10))
            .expect("add task");

        let executor = setup_executor(graph);
        let report = executor.execute_graph().await.expect("should succeed");

        assert_eq!(report.total_tasks, 1);
        assert_eq!(report.completed, 1);
        assert_eq!(report.failed, 0);
    }

    #[tokio::test]
    async fn test_parallel_tasks() {
        let mut graph = TaskGraph::new();
        graph
            .add_task(make_task("backend-api", "backend").with_estimate(60))
            .expect("add");
        graph
            .add_task(make_task("frontend-ui", "frontend").with_estimate(60))
            .expect("add");
        graph
            .add_task(make_task("devops-ci", "devops").with_estimate(30))
            .expect("add");

        let executor = setup_executor(graph);
        let report = executor.execute_graph().await.expect("should succeed");

        assert_eq!(report.total_tasks, 3);
        assert_eq!(report.completed, 3);
    }

    #[tokio::test]
    async fn test_dependent_tasks() {
        let mut graph = TaskGraph::new();
        let t1 = make_task("architecture", "architect").with_estimate(30);
        let t1_id = t1.id.clone();
        graph.add_task(t1).expect("add");

        let t2 = make_task("backend", "backend")
            .depends_on(&t1_id)
            .with_estimate(60);
        let t2_id = t2.id.clone();
        graph.add_task(t2).expect("add");

        let t3 = make_task("frontend", "frontend")
            .depends_on(&t1_id)
            .with_estimate(60);
        graph.add_task(t3).expect("add");

        let t4 = make_task("deploy", "devops")
            .depends_on(&t2_id)
            .with_estimate(30);
        graph.add_task(t4).expect("add");

        let executor = setup_executor(graph);
        let report = executor.execute_graph().await.expect("should succeed");

        assert_eq!(report.total_tasks, 4);
        assert_eq!(report.completed, 4);

        // Verify execution order via task graph state
        let g = executor.task_graph().read().await;
        let arch = g.get_task(&t1_id).expect("task exists");
        assert_eq!(arch.status, TaskStatus::Completed);
        assert!(arch.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_metrics_recorded() {
        let mut graph = TaskGraph::new();
        graph.add_task(make_task("t1", "backend")).expect("add");
        graph.add_task(make_task("t2", "frontend")).expect("add");

        let executor = setup_executor(graph);
        executor.execute_graph().await.expect("should succeed");

        let metrics = executor.metrics().await;
        assert_eq!(metrics.total_tasks_executed, 2);
        assert_eq!(metrics.task_latencies.len(), 2);
        assert!(metrics.total_tokens() > 0);
        assert_eq!(metrics.total_errors(), 0);
    }

    #[tokio::test]
    async fn test_metrics_avg_latency() {
        let mut metrics = ExecutionMetrics::default();
        assert_eq!(metrics.avg_latency_secs(), 0.0);

        metrics.task_latencies = vec![1.0, 2.0, 3.0];
        assert!((metrics.avg_latency_secs() - 2.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_metrics_cache_hit_rate() {
        let mut metrics = ExecutionMetrics::default();
        assert_eq!(metrics.cache_hit_rate(), 0.0);

        metrics.cache_hits = 8;
        metrics.cache_misses = 2;
        assert!((metrics.cache_hit_rate() - 0.8).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_metrics_error_rate() {
        let mut metrics = ExecutionMetrics::default();
        metrics.total_tasks_executed = 10;
        metrics
            .errors_per_provider
            .insert("anthropic".to_string(), 2);

        assert!((metrics.error_rate("anthropic") - 0.2).abs() < f64::EPSILON);
        assert_eq!(metrics.error_rate("ollama"), 0.0);
    }

    #[tokio::test]
    async fn test_resolve_role() {
        assert_eq!(resolve_role("cto"), AgentRole::Cto);
        assert_eq!(resolve_role("BACKEND"), AgentRole::Backend);
        assert_eq!(resolve_role("DevOps"), AgentRole::DevOps);
        assert_eq!(resolve_role("unknown"), AgentRole::Backend);
    }

    #[tokio::test]
    async fn test_halted_manager_cancels_tasks() {
        let mut graph = TaskGraph::new();
        graph.add_task(make_task("t1", "backend")).expect("add");

        let mut agent_mgr = AgentManager::new();
        agent_mgr.halt_all();

        let msg_bus = MessageBus::new(64);
        let healer = SelfHealer::new();
        let executor = ParallelExecutor::new(graph, agent_mgr, msg_bus, healer);

        let report = executor.execute_graph().await.expect("should succeed");
        assert_eq!(report.completed, 0);
        assert_eq!(report.cancelled, 1);
    }

    #[tokio::test]
    async fn test_execution_report_serialization() {
        let report = ExecutionReport {
            total_tasks: 10,
            completed: 8,
            failed: 1,
            cancelled: 1,
            wall_time_secs: 45.5,
            sequential_estimate_secs: 120.0,
            parallel_efficiency: 2.64,
            metrics: ExecutionMetrics::default(),
        };

        let json = serde_json::to_string(&report).expect("serialize");
        let decoded: ExecutionReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.total_tasks, 10);
        assert_eq!(decoded.completed, 8);
    }
}
