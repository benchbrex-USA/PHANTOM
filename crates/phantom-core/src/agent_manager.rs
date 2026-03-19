//! Agent lifecycle manager — spawn, monitor, track, terminate agents.
//!
//! Manages the 8-agent team: CTO, Architect, Backend, Frontend, DevOps, QA, Security, Monitor.
//! Each agent has scoped permissions, token budgets, and time limits.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use phantom_ai::agents::AgentRole;

use crate::CoreError;

/// Runtime state of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Agent is registered but not actively working
    Idle,
    /// Agent is executing a task
    Working,
    /// Agent is waiting for a dependency or input
    Waiting,
    /// Agent encountered an error and is being healed
    Healing,
    /// Agent has been stopped
    Stopped,
    /// Agent has been halted by emergency command
    Halted,
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Working => write!(f, "working"),
            Self::Waiting => write!(f, "waiting"),
            Self::Healing => write!(f, "healing"),
            Self::Stopped => write!(f, "stopped"),
            Self::Halted => write!(f, "halted"),
        }
    }
}

/// A handle to a running agent with its metadata and tracking info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandle {
    /// Agent ID (e.g., "backend-1")
    pub id: String,
    /// Agent role
    pub role: AgentRole,
    /// Current state
    pub state: AgentState,
    /// Current task ID (if working)
    pub current_task: Option<String>,
    /// Total tokens consumed by this agent
    pub tokens_consumed: u64,
    /// Token budget (max tokens this agent can use)
    pub token_budget: u64,
    /// Tasks completed by this agent
    pub tasks_completed: u32,
    /// Tasks failed by this agent
    pub tasks_failed: u32,
    /// When the agent was spawned
    pub spawned_at: DateTime<Utc>,
    /// When the agent last reported activity
    pub last_activity: DateTime<Utc>,
    /// Agent timeout in seconds (0 = no timeout)
    pub timeout_seconds: u64,
}

impl AgentHandle {
    /// Create a new agent handle.
    pub fn new(id: impl Into<String>, role: AgentRole) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            role,
            state: AgentState::Idle,
            current_task: None,
            tokens_consumed: 0,
            token_budget: default_token_budget(role),
            tasks_completed: 0,
            tasks_failed: 0,
            spawned_at: now,
            last_activity: now,
            timeout_seconds: default_timeout(role),
        }
    }

    /// Check if the agent has exceeded its token budget.
    pub fn is_over_budget(&self) -> bool {
        self.token_budget > 0 && self.tokens_consumed >= self.token_budget
    }

    /// Check if the agent has timed out (no activity within timeout).
    pub fn is_timed_out(&self) -> bool {
        if self.timeout_seconds == 0 {
            return false;
        }
        let elapsed = (Utc::now() - self.last_activity).num_seconds() as u64;
        elapsed > self.timeout_seconds
    }

    /// Record token usage.
    pub fn record_tokens(&mut self, input_tokens: u64, output_tokens: u64) {
        self.tokens_consumed += input_tokens + output_tokens;
        self.last_activity = Utc::now();
    }

    /// Assign a task to this agent.
    pub fn assign_task(&mut self, task_id: &str) {
        self.current_task = Some(task_id.to_string());
        self.state = AgentState::Working;
        self.last_activity = Utc::now();
    }

    /// Mark current task as completed.
    pub fn complete_task(&mut self) {
        self.current_task = None;
        self.state = AgentState::Idle;
        self.tasks_completed += 1;
        self.last_activity = Utc::now();
    }

    /// Mark current task as failed.
    pub fn fail_task(&mut self) {
        self.current_task = None;
        self.state = AgentState::Idle;
        self.tasks_failed += 1;
        self.last_activity = Utc::now();
    }
}

/// Default token budgets per agent role.
fn default_token_budget(role: AgentRole) -> u64 {
    match role {
        AgentRole::Cto => 500_000,      // Orchestrator needs more
        AgentRole::Architect => 300_000,
        AgentRole::Backend => 400_000,   // Code generation is token-heavy
        AgentRole::Frontend => 400_000,
        AgentRole::DevOps => 200_000,
        AgentRole::Qa => 300_000,
        AgentRole::Security => 200_000,
        AgentRole::Monitor => 100_000,   // Haiku, minimal usage
    }
}

/// Default timeout per agent role (seconds).
fn default_timeout(role: AgentRole) -> u64 {
    match role {
        AgentRole::Cto => 3600,      // 1 hour
        AgentRole::Monitor => 0,     // No timeout (daemon)
        _ => 1800,                   // 30 minutes
    }
}

/// The agent manager — tracks all agents and their lifecycle.
pub struct AgentManager {
    agents: HashMap<String, AgentHandle>,
    halted: bool,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            halted: false,
        }
    }

    /// Spawn a new agent with the given role.
    pub fn spawn(&mut self, role: AgentRole) -> Result<String, CoreError> {
        if self.halted {
            return Err(CoreError::EmergencyHalt);
        }

        let instance = self
            .agents
            .values()
            .filter(|a| a.role == role)
            .count();
        let id = format!("{}-{}", role.display_name().to_lowercase().replace(' ', "-"), instance);

        info!(agent_id = %id, role = ?role, model = role.model(), "spawning agent");

        let handle = AgentHandle::new(&id, role);
        self.agents.insert(id.clone(), handle);
        Ok(id)
    }

    /// Spawn all 8 agents (the full team).
    pub fn spawn_full_team(&mut self) -> Result<Vec<String>, CoreError> {
        let roles = [
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::DevOps,
            AgentRole::Qa,
            AgentRole::Security,
            AgentRole::Monitor,
        ];

        let mut ids = Vec::new();
        for role in roles {
            ids.push(self.spawn(role)?);
        }
        Ok(ids)
    }

    /// Get an agent handle.
    pub fn get(&self, id: &str) -> Option<&AgentHandle> {
        self.agents.get(id)
    }

    /// Get a mutable agent handle.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut AgentHandle> {
        self.agents.get_mut(id)
    }

    /// Get all agents.
    pub fn agents(&self) -> impl Iterator<Item = &AgentHandle> {
        self.agents.values()
    }

    /// Get agents by role.
    pub fn agents_by_role(&self, role: AgentRole) -> Vec<&AgentHandle> {
        self.agents.values().filter(|a| a.role == role).collect()
    }

    /// Find an idle agent with the given role.
    pub fn find_idle(&self, role: AgentRole) -> Option<&AgentHandle> {
        self.agents
            .values()
            .find(|a| a.role == role && a.state == AgentState::Idle && !a.is_over_budget())
    }

    /// Stop a specific agent.
    pub fn stop(&mut self, id: &str) -> Result<(), CoreError> {
        let agent = self
            .agents
            .get_mut(id)
            .ok_or_else(|| CoreError::AgentNotFound(id.to_string()))?;

        agent.state = AgentState::Stopped;
        agent.current_task = None;
        debug!(agent_id = id, "agent stopped");
        Ok(())
    }

    /// Emergency halt all agents.
    pub fn halt_all(&mut self) {
        warn!("EMERGENCY HALT: stopping all agents");
        self.halted = true;
        for agent in self.agents.values_mut() {
            agent.state = AgentState::Halted;
            agent.current_task = None;
        }
    }

    /// Check for timed-out agents and return their IDs.
    pub fn check_timeouts(&self) -> Vec<String> {
        self.agents
            .values()
            .filter(|a| a.state == AgentState::Working && a.is_timed_out())
            .map(|a| a.id.clone())
            .collect()
    }

    /// Check for over-budget agents and return their IDs.
    pub fn check_budgets(&self) -> Vec<String> {
        self.agents
            .values()
            .filter(|a| a.is_over_budget())
            .map(|a| a.id.clone())
            .collect()
    }

    /// Get summary stats.
    pub fn stats(&self) -> AgentManagerStats {
        let mut stats = AgentManagerStats::default();
        for agent in self.agents.values() {
            stats.total += 1;
            match agent.state {
                AgentState::Idle => stats.idle += 1,
                AgentState::Working => stats.working += 1,
                AgentState::Waiting => stats.waiting += 1,
                AgentState::Healing => stats.healing += 1,
                AgentState::Stopped => stats.stopped += 1,
                AgentState::Halted => stats.halted += 1,
            }
            stats.total_tokens += agent.tokens_consumed;
            stats.total_tasks_completed += agent.tasks_completed as u64;
        }
        stats
    }

    /// Is the manager in halted state?
    pub fn is_halted(&self) -> bool {
        self.halted
    }
}

/// Summary stats for the agent manager.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentManagerStats {
    pub total: usize,
    pub idle: usize,
    pub working: usize,
    pub waiting: usize,
    pub healing: usize,
    pub stopped: usize,
    pub halted: usize,
    pub total_tokens: u64,
    pub total_tasks_completed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_agent() {
        let mut mgr = AgentManager::new();
        let id = mgr.spawn(AgentRole::Backend).unwrap();
        assert!(mgr.get(&id).is_some());
        assert_eq!(mgr.get(&id).unwrap().role, AgentRole::Backend);
        assert_eq!(mgr.get(&id).unwrap().state, AgentState::Idle);
    }

    #[test]
    fn test_spawn_full_team() {
        let mut mgr = AgentManager::new();
        let ids = mgr.spawn_full_team().unwrap();
        assert_eq!(ids.len(), 8);
    }

    #[test]
    fn test_agent_task_lifecycle() {
        let mut mgr = AgentManager::new();
        let id = mgr.spawn(AgentRole::Backend).unwrap();

        mgr.get_mut(&id).unwrap().assign_task("task-1");
        assert_eq!(mgr.get(&id).unwrap().state, AgentState::Working);
        assert_eq!(mgr.get(&id).unwrap().current_task.as_deref(), Some("task-1"));

        mgr.get_mut(&id).unwrap().complete_task();
        assert_eq!(mgr.get(&id).unwrap().state, AgentState::Idle);
        assert_eq!(mgr.get(&id).unwrap().tasks_completed, 1);
    }

    #[test]
    fn test_find_idle() {
        let mut mgr = AgentManager::new();
        let id1 = mgr.spawn(AgentRole::Backend).unwrap();
        let _id2 = mgr.spawn(AgentRole::Backend).unwrap();

        mgr.get_mut(&id1).unwrap().assign_task("task-1");

        let idle = mgr.find_idle(AgentRole::Backend);
        assert!(idle.is_some());
        assert_ne!(idle.unwrap().id, id1);
    }

    #[test]
    fn test_halt_all() {
        let mut mgr = AgentManager::new();
        mgr.spawn_full_team().unwrap();

        mgr.halt_all();

        assert!(mgr.is_halted());
        assert!(mgr.agents().all(|a| a.state == AgentState::Halted));

        // Can't spawn after halt
        assert!(mgr.spawn(AgentRole::Backend).is_err());
    }

    #[test]
    fn test_token_budget() {
        let mut handle = AgentHandle::new("test", AgentRole::Monitor);
        assert!(!handle.is_over_budget());

        handle.tokens_consumed = handle.token_budget;
        assert!(handle.is_over_budget());
    }

    #[test]
    fn test_agent_stats() {
        let mut mgr = AgentManager::new();
        let id = mgr.spawn(AgentRole::Backend).unwrap();
        mgr.get_mut(&id).unwrap().assign_task("t1");

        let stats = mgr.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.working, 1);
    }

    #[test]
    fn test_stop_agent() {
        let mut mgr = AgentManager::new();
        let id = mgr.spawn(AgentRole::Qa).unwrap();
        mgr.stop(&id).unwrap();
        assert_eq!(mgr.get(&id).unwrap().state, AgentState::Stopped);
    }

    #[test]
    fn test_stop_unknown_agent() {
        let mut mgr = AgentManager::new();
        assert!(mgr.stop("nonexistent").is_err());
    }
}
