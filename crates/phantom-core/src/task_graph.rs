//! Task Graph — DAG of tasks with dependencies, parallel execution, topological ordering.
//!
//! Architecture Framework → parsed → dependency DAG → parallel execution plan.
//!
//! The CTO Agent decomposes a project spec into a task graph. Each task is
//! assigned to a specialist agent. Tasks with satisfied dependencies can
//! execute in parallel.

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CoreError;

/// A task in the execution graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task ID
    pub id: String,
    /// Human-readable task name
    pub name: String,
    /// Detailed description of what the task should accomplish
    pub description: String,
    /// Which agent role should execute this task
    pub agent_role: String,
    /// IDs of tasks that must complete before this one starts
    pub dependencies: Vec<String>,
    /// Current execution status
    pub status: TaskStatus,
    /// Estimated execution time in seconds
    pub estimated_seconds: u32,
    /// Actual start time
    pub started_at: Option<DateTime<Utc>>,
    /// Actual completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Number of retry attempts so far
    pub retry_count: u32,
    /// Maximum retry attempts before escalation
    pub max_retries: u32,
    /// Error message if failed
    pub error: Option<String>,
    /// Output/result of the task (JSON blob)
    pub output: Option<serde_json::Value>,
    /// Build phase this task belongs to
    pub phase: Option<String>,
    /// Knowledge Brain query hint (what to search for before executing)
    pub knowledge_query: Option<String>,
}

impl Task {
    /// Create a new pending task.
    pub fn new(name: impl Into<String>, description: impl Into<String>, agent_role: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: description.into(),
            agent_role: agent_role.into(),
            dependencies: Vec::new(),
            status: TaskStatus::Pending,
            estimated_seconds: 0,
            started_at: None,
            completed_at: None,
            retry_count: 0,
            max_retries: 5,
            error: None,
            output: None,
            phase: None,
            knowledge_query: None,
        }
    }

    /// Add a dependency on another task.
    pub fn depends_on(mut self, task_id: impl Into<String>) -> Self {
        self.dependencies.push(task_id.into());
        self
    }

    /// Set the estimated execution time.
    pub fn with_estimate(mut self, seconds: u32) -> Self {
        self.estimated_seconds = seconds;
        self
    }

    /// Set the build phase.
    pub fn with_phase(mut self, phase: impl Into<String>) -> Self {
        self.phase = Some(phase.into());
        self
    }

    /// Set a knowledge query hint.
    pub fn with_knowledge_query(mut self, query: impl Into<String>) -> Self {
        self.knowledge_query = Some(query.into());
        self
    }

    /// Mark the task as running.
    pub fn start(&mut self) {
        self.status = TaskStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Mark the task as completed with optional output.
    pub fn complete(&mut self, output: Option<serde_json::Value>) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.output = output;
        self.error = None;
    }

    /// Mark the task as failed.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error.into());
    }

    /// Mark the task for retry.
    pub fn retry(&mut self) {
        self.status = TaskStatus::Retrying;
        self.retry_count += 1;
        self.error = None;
    }

    /// Check if the task can be retried.
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Elapsed time if started.
    pub fn elapsed_seconds(&self) -> Option<f64> {
        self.started_at.map(|start| {
            let end = self.completed_at.unwrap_or_else(Utc::now);
            (end - start).num_milliseconds() as f64 / 1000.0
        })
    }
}

/// Task execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Waiting for dependencies to complete
    Pending,
    /// Currently being executed by an agent
    Running,
    /// Successfully completed
    Completed,
    /// Execution failed
    Failed,
    /// Being retried after failure
    Retrying,
    /// Blocked by a failed dependency
    Blocked,
    /// Cancelled by owner or system
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Retrying => write!(f, "retrying"),
            Self::Blocked => write!(f, "blocked"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// The task execution DAG — manages task lifecycle, ordering, and parallel execution.
pub struct TaskGraph {
    /// All tasks indexed by ID
    tasks: HashMap<String, Task>,
    /// Insertion order (for deterministic iteration)
    order: Vec<String>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Add a task to the graph. Returns the task ID.
    pub fn add_task(&mut self, task: Task) -> Result<String, CoreError> {
        let id = task.id.clone();
        if self.tasks.contains_key(&id) {
            return Err(CoreError::TaskAlreadyExists(id));
        }
        self.order.push(id.clone());
        self.tasks.insert(id.clone(), task);
        Ok(id)
    }

    /// Get a task by ID.
    pub fn get_task(&self, id: &str) -> Option<&Task> {
        self.tasks.get(id)
    }

    /// Get a mutable task by ID.
    pub fn get_task_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.get_mut(id)
    }

    /// Get all tasks.
    pub fn tasks(&self) -> impl Iterator<Item = &Task> {
        self.order.iter().filter_map(|id| self.tasks.get(id))
    }

    /// Get tasks that are ready to execute (all deps completed, status = Pending or Retrying).
    pub fn ready_tasks(&self) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| {
                (t.status == TaskStatus::Pending || t.status == TaskStatus::Retrying)
                    && t.dependencies.iter().all(|dep_id| {
                        self.tasks
                            .get(dep_id)
                            .map(|d| d.status == TaskStatus::Completed)
                            .unwrap_or(false)
                    })
            })
            .collect()
    }

    /// Get tasks currently running.
    pub fn running_tasks(&self) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Running)
            .collect()
    }

    /// Get tasks that have failed.
    pub fn failed_tasks(&self) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Failed)
            .collect()
    }

    /// Check if all tasks are completed.
    pub fn is_complete(&self) -> bool {
        self.tasks.values().all(|t| t.status == TaskStatus::Completed)
    }

    /// Check if the graph has any terminal failures (failed + can't retry).
    pub fn has_terminal_failure(&self) -> bool {
        self.tasks
            .values()
            .any(|t| t.status == TaskStatus::Failed && !t.can_retry())
    }

    /// Get tasks blocked by failed dependencies.
    pub fn blocked_tasks(&self) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| {
                t.status == TaskStatus::Pending
                    && t.dependencies.iter().any(|dep_id| {
                        self.tasks
                            .get(dep_id)
                            .map(|d| d.status == TaskStatus::Failed)
                            .unwrap_or(false)
                    })
            })
            .collect()
    }

    /// Mark tasks blocked by failed dependencies.
    pub fn propagate_failures(&mut self) {
        let blocked_ids: Vec<String> = self
            .tasks
            .values()
            .filter(|t| {
                t.status == TaskStatus::Pending
                    && t.dependencies.iter().any(|dep_id| {
                        self.tasks
                            .get(dep_id)
                            .map(|d| d.status == TaskStatus::Failed && !d.can_retry())
                            .unwrap_or(false)
                    })
            })
            .map(|t| t.id.clone())
            .collect();

        for id in blocked_ids {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.status = TaskStatus::Blocked;
            }
        }
    }

    /// Validate the graph — check for cycles and missing dependencies.
    pub fn validate(&self) -> Result<(), CoreError> {
        // Check for missing dependencies
        for task in self.tasks.values() {
            for dep_id in &task.dependencies {
                if !self.tasks.contains_key(dep_id) {
                    return Err(CoreError::UnresolvedDependencies {
                        task_id: task.id.clone(),
                        deps: vec![dep_id.clone()],
                    });
                }
            }
        }

        // Check for cycles using Kahn's algorithm
        self.detect_cycles()?;

        Ok(())
    }

    /// Detect cycles using Kahn's algorithm (topological sort).
    fn detect_cycles(&self) -> Result<(), CoreError> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for task in self.tasks.values() {
            in_degree.entry(&task.id).or_insert(0);
            for dep_id in &task.dependencies {
                adj.entry(dep_id.as_str())
                    .or_default()
                    .push(&task.id);
                *in_degree.entry(&task.id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut visited = 0;

        while let Some(node) = queue.pop_front() {
            visited += 1;
            if let Some(neighbors) = adj.get(node) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        if visited != self.tasks.len() {
            // Find nodes involved in cycle
            let cycle_nodes: Vec<String> = in_degree
                .iter()
                .filter(|(_, &deg)| deg > 0)
                .map(|(&id, _)| id.to_string())
                .collect();

            return Err(CoreError::DependencyCycle(cycle_nodes.join(", ")));
        }

        Ok(())
    }

    /// Get a topological ordering of task IDs.
    pub fn topological_order(&self) -> Result<Vec<String>, CoreError> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for task in self.tasks.values() {
            in_degree.entry(&task.id).or_insert(0);
            for dep_id in &task.dependencies {
                adj.entry(dep_id.as_str())
                    .or_default()
                    .push(&task.id);
                *in_degree.entry(&task.id).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::new();

        while let Some(node) = queue.pop_front() {
            order.push(node.to_string());
            if let Some(neighbors) = adj.get(node) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        if order.len() != self.tasks.len() {
            return Err(CoreError::DependencyCycle("cycle detected".into()));
        }

        Ok(order)
    }

    /// Get parallel execution layers (tasks in same layer can run concurrently).
    pub fn parallel_layers(&self) -> Result<Vec<Vec<String>>, CoreError> {
        let topo = self.topological_order()?;

        // Calculate depth for each task
        let mut depth: HashMap<String, usize> = HashMap::new();
        for id in &topo {
            let task = self.tasks.get(id).unwrap();
            let max_dep_depth = task
                .dependencies
                .iter()
                .filter_map(|dep| depth.get(dep))
                .max()
                .copied()
                .unwrap_or(0);

            let d = if task.dependencies.is_empty() {
                0
            } else {
                max_dep_depth + 1
            };
            depth.insert(id.clone(), d);
        }

        // Group by depth
        let max_depth = depth.values().max().copied().unwrap_or(0);
        let mut layers = vec![Vec::new(); max_depth + 1];
        for (id, d) in &depth {
            layers[*d].push(id.clone());
        }

        Ok(layers)
    }

    /// Cancel all non-completed tasks.
    pub fn cancel_all(&mut self) {
        for task in self.tasks.values_mut() {
            if task.status != TaskStatus::Completed {
                task.status = TaskStatus::Cancelled;
            }
        }
    }

    /// Summary statistics.
    pub fn stats(&self) -> TaskGraphStats {
        let mut stats = TaskGraphStats::default();
        for task in self.tasks.values() {
            stats.total += 1;
            match task.status {
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::Running => stats.running += 1,
                TaskStatus::Completed => stats.completed += 1,
                TaskStatus::Failed => stats.failed += 1,
                TaskStatus::Retrying => stats.retrying += 1,
                TaskStatus::Blocked => stats.blocked += 1,
                TaskStatus::Cancelled => stats.cancelled += 1,
            }
            stats.total_estimated_seconds += task.estimated_seconds;
        }
        stats
    }

    /// Number of tasks in the graph.
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

/// Summary statistics for the task graph.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskGraphStats {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub retrying: usize,
    pub blocked: usize,
    pub cancelled: usize,
    pub total_estimated_seconds: u32,
}

impl std::fmt::Display for TaskGraphStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{} completed, {} running, {} pending, {} failed, {} blocked",
            self.completed, self.total, self.running, self.pending, self.failed, self.blocked
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(name: &str, role: &str) -> Task {
        Task::new(name, format!("{} description", name), role)
    }

    #[test]
    fn test_add_and_get_task() {
        let mut graph = TaskGraph::new();
        let task = make_task("setup-db", "backend");
        let id = graph.add_task(task).unwrap();
        assert!(graph.get_task(&id).is_some());
        assert_eq!(graph.len(), 1);
    }

    #[test]
    fn test_duplicate_task_rejected() {
        let mut graph = TaskGraph::new();
        let task = make_task("t1", "cto");
        let id = task.id.clone();
        graph.add_task(task).unwrap();

        let dup = Task { id, ..make_task("t1-dup", "cto") };
        assert!(graph.add_task(dup).is_err());
    }

    #[test]
    fn test_ready_tasks_no_deps() {
        let mut graph = TaskGraph::new();
        graph.add_task(make_task("t1", "backend")).unwrap();
        graph.add_task(make_task("t2", "frontend")).unwrap();

        let ready = graph.ready_tasks();
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_ready_tasks_with_deps() {
        let mut graph = TaskGraph::new();
        let t1 = make_task("t1", "architect");
        let t1_id = t1.id.clone();
        graph.add_task(t1).unwrap();

        let t2 = make_task("t2", "backend").depends_on(&t1_id);
        graph.add_task(t2).unwrap();

        // Only t1 is ready (t2 depends on t1)
        let ready = graph.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].name, "t1");

        // Complete t1
        graph.get_task_mut(&t1_id).unwrap().complete(None);

        // Now t2 is ready
        let ready = graph.ready_tasks();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].name, "t2");
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = TaskGraph::new();
        let mut t1 = make_task("t1", "cto");
        let mut t2 = make_task("t2", "cto");
        let t1_id = t1.id.clone();
        let t2_id = t2.id.clone();

        t1.dependencies.push(t2_id.clone());
        t2.dependencies.push(t1_id.clone());

        graph.add_task(t1).unwrap();
        graph.add_task(t2).unwrap();

        assert!(graph.validate().is_err());
    }

    #[test]
    fn test_missing_dependency() {
        let mut graph = TaskGraph::new();
        let t1 = make_task("t1", "backend").depends_on("nonexistent");
        graph.add_task(t1).unwrap();
        assert!(graph.validate().is_err());
    }

    #[test]
    fn test_topological_order() {
        let mut graph = TaskGraph::new();
        let t1 = make_task("t1", "architect");
        let t1_id = t1.id.clone();
        graph.add_task(t1).unwrap();

        let t2 = make_task("t2", "backend").depends_on(&t1_id);
        let t2_id = t2.id.clone();
        graph.add_task(t2).unwrap();

        let t3 = make_task("t3", "frontend").depends_on(&t1_id);
        graph.add_task(t3).unwrap();

        let t4 = make_task("t4", "devops").depends_on(&t2_id);
        graph.add_task(t4).unwrap();

        let order = graph.topological_order().unwrap();
        let t1_pos = order.iter().position(|id| id == &t1_id).unwrap();
        let t2_pos = order.iter().position(|id| id == &t2_id).unwrap();
        assert!(t1_pos < t2_pos, "t1 must come before t2");
    }

    #[test]
    fn test_parallel_layers() {
        let mut graph = TaskGraph::new();
        let t1 = make_task("arch", "architect");
        let t1_id = t1.id.clone();
        graph.add_task(t1).unwrap();

        let t2 = make_task("backend", "backend").depends_on(&t1_id);
        let t2_id = t2.id.clone();
        graph.add_task(t2).unwrap();

        let t3 = make_task("frontend", "frontend").depends_on(&t1_id);
        graph.add_task(t3).unwrap();

        let t4 = make_task("deploy", "devops").depends_on(&t2_id);
        graph.add_task(t4).unwrap();

        let layers = graph.parallel_layers().unwrap();
        // Layer 0: arch
        // Layer 1: backend, frontend (parallel)
        // Layer 2: deploy
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].len(), 1); // arch
        assert_eq!(layers[1].len(), 2); // backend + frontend
        assert_eq!(layers[2].len(), 1); // deploy
    }

    #[test]
    fn test_failure_propagation() {
        let mut graph = TaskGraph::new();
        let t1 = make_task("t1", "backend");
        let t1_id = t1.id.clone();
        graph.add_task(t1).unwrap();

        let t2 = make_task("t2", "devops").depends_on(&t1_id);
        graph.add_task(t2).unwrap();

        // Fail t1 with no retries left
        let task = graph.get_task_mut(&t1_id).unwrap();
        task.retry_count = task.max_retries; // exhaust retries
        task.fail("fatal error");

        // t2 should be detected as blocked before propagation
        let blocked_before = graph.blocked_tasks();
        assert_eq!(blocked_before.len(), 1);
        assert_eq!(blocked_before[0].name, "t2");

        graph.propagate_failures();

        // After propagation, t2 should have Blocked status
        let t2 = graph.tasks().find(|t| t.name == "t2").unwrap();
        assert_eq!(t2.status, TaskStatus::Blocked);
    }

    #[test]
    fn test_is_complete() {
        let mut graph = TaskGraph::new();
        let t1 = make_task("t1", "cto");
        let id = t1.id.clone();
        graph.add_task(t1).unwrap();

        assert!(!graph.is_complete());

        graph.get_task_mut(&id).unwrap().complete(None);
        assert!(graph.is_complete());
    }

    #[test]
    fn test_cancel_all() {
        let mut graph = TaskGraph::new();
        let t1 = make_task("t1", "cto");
        let t1_id = t1.id.clone();
        graph.add_task(t1).unwrap();
        graph.add_task(make_task("t2", "backend")).unwrap();

        graph.get_task_mut(&t1_id).unwrap().complete(None);
        graph.cancel_all();

        // t1 stays completed, t2 gets cancelled
        assert_eq!(graph.get_task(&t1_id).unwrap().status, TaskStatus::Completed);
        assert_eq!(graph.stats().cancelled, 1);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = make_task("test", "backend");
        assert_eq!(task.status, TaskStatus::Pending);

        task.start();
        assert_eq!(task.status, TaskStatus::Running);
        assert!(task.started_at.is_some());

        task.fail("some error");
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.error.as_deref(), Some("some error"));

        assert!(task.can_retry());
        task.retry();
        assert_eq!(task.status, TaskStatus::Retrying);
        assert_eq!(task.retry_count, 1);

        task.complete(Some(serde_json::json!({"result": "ok"})));
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.output.is_some());
    }

    #[test]
    fn test_stats() {
        let mut graph = TaskGraph::new();
        let t1 = make_task("t1", "cto").with_estimate(60);
        let t1_id = t1.id.clone();
        graph.add_task(t1).unwrap();
        graph.add_task(make_task("t2", "backend").with_estimate(120)).unwrap();

        graph.get_task_mut(&t1_id).unwrap().complete(None);

        let stats = graph.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.total_estimated_seconds, 180);
    }
}
