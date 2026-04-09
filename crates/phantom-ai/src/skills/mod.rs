//! Core skills registry for PHANTOM's multi-agent system.
//!
//! Every capability an agent can invoke is modeled as a [`Skill`]. Skills are
//! registered in a [`SkillRegistry`], routed to agents via [`SkillRouter`],
//! and their executions tracked through [`SkillExecution`].

use std::collections::HashMap;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::agents::AgentRole;

// ---------------------------------------------------------------------------
// Sub-module declarations
// ---------------------------------------------------------------------------

pub mod ai_ml;
pub mod api_design;
pub mod architecture;
pub mod business;
pub mod code_gen;
pub mod compliance;
pub mod coordination;
pub mod cost_optimization;
pub mod data_engineering;
pub mod database;
pub mod devops;
pub mod documentation_skills;
pub mod frontend;
pub mod observability;
pub mod performance;
pub mod resilience;
pub mod security_skills;
pub mod testing;

// ---------------------------------------------------------------------------
// SkillId
// ---------------------------------------------------------------------------

/// Unique, opaque identifier for a registered skill.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(pub String);

impl SkillId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for SkillId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// High-level category that groups related skills together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    CodeGeneration,
    Architecture,
    Database,
    ApiDesign,
    Testing,
    Security,
    DevOps,
    Observability,
    Performance,
    Frontend,
    DataEngineering,
    AiMl,
    Business,
    Resilience,
    Coordination,
    Compliance,
    CostOptimization,
    Documentation,
}

/// How complex a skill's execution graph is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillComplexity {
    /// Single-step, self-contained action.
    Atomic,
    /// Moderate complexity, single agent.
    Medium,
    /// Combines multiple atomic skills executed by a single agent.
    Composite,
    /// High complexity, may need multiple passes.
    High,
    /// Ordered chain of skills that flow data between stages.
    Pipeline,
    /// Multi-agent, multi-step workflow requiring coordination.
    Orchestrated,
    /// Security/safety critical — requires maximum quality gates.
    Critical,
}

/// The shape of data a skill produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Code,
    SingleFile,
    MultiFile,
    Schema,
    Config,
    Documentation,
    Analysis,
    Plan,
    Manifest,
    Migration,
    Test,
    Report,
}

/// Terminal status of a skill execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Retrying,
    Skipped,
}

// ---------------------------------------------------------------------------
// RetryStrategy
// ---------------------------------------------------------------------------

/// Controls how a failed skill execution is retried.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStrategy {
    /// Maximum number of retry attempts before giving up.
    pub max_retries: u32,
    /// Base back-off duration in milliseconds (doubled each attempt).
    pub backoff_ms: u64,
    /// Optional skill to invoke if all retries are exhausted.
    pub fallback_skill: Option<SkillId>,
}

impl RetryStrategy {
    /// Create a new retry strategy. If `use_backoff` is true, `backoff_ms` doubles each attempt.
    pub fn new(max_retries: u32, backoff_ms: u64, _use_backoff: bool) -> Self {
        Self {
            max_retries,
            backoff_ms,
            fallback_skill: None,
        }
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            backoff_ms: 500,
            fallback_skill: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Skill
// ---------------------------------------------------------------------------

/// A registered capability that one or more agents can execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: SkillId,
    pub name: String,
    pub description: String,
    pub category: SkillCategory,
    pub complexity: SkillComplexity,
    /// Which agent roles are allowed to execute this skill.
    pub required_agents: Vec<AgentRole>,
    /// Skills that must complete before this one can start.
    pub dependencies: Vec<SkillId>,
    /// Estimated token budget for a single execution.
    pub estimated_tokens: u32,
    /// Extra system-prompt fragment injected when this skill is active.
    pub system_prompt_extension: String,
    pub output_format: OutputFormat,
    pub retry_strategy: RetryStrategy,
    /// Minimum quality score (0.0..=1.0) to accept the output.
    pub quality_threshold: f64,
}

impl Skill {
    /// Convenience builder that fills in sensible defaults.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        category: SkillCategory,
        complexity: SkillComplexity,
        required_agents: Vec<AgentRole>,
        output_format: OutputFormat,
    ) -> Self {
        Self {
            id: SkillId::new(id),
            name: name.into(),
            description: description.into(),
            category,
            complexity,
            required_agents,
            dependencies: Vec::new(),
            estimated_tokens: 4096,
            system_prompt_extension: String::new(),
            output_format,
            retry_strategy: RetryStrategy::default(),
            quality_threshold: 0.7,
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<SkillId>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_estimated_tokens(mut self, tokens: u32) -> Self {
        self.estimated_tokens = tokens;
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt_extension = prompt.into();
        self
    }

    pub fn with_retry_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = strategy;
        self
    }

    pub fn with_quality_threshold(mut self, threshold: f64) -> Self {
        self.quality_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}

// ---------------------------------------------------------------------------
// SkillExecution
// ---------------------------------------------------------------------------

/// Tracks a single invocation of a skill by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecution {
    pub skill_id: SkillId,
    pub agent_role: AgentRole,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub tokens_used: u32,
    pub quality_score: f64,
    pub output: String,
    pub status: ExecutionStatus,
}

impl SkillExecution {
    /// Start tracking a new execution.
    pub fn begin(skill_id: SkillId, agent_role: AgentRole) -> Self {
        Self {
            skill_id,
            agent_role,
            start_time: Utc::now(),
            end_time: None,
            tokens_used: 0,
            quality_score: 0.0,
            output: String::new(),
            status: ExecutionStatus::Running,
        }
    }

    /// Mark the execution as completed.
    pub fn complete(&mut self, output: String, tokens_used: u32, quality_score: f64) {
        self.end_time = Some(Utc::now());
        self.output = output;
        self.tokens_used = tokens_used;
        self.quality_score = quality_score;
        self.status = ExecutionStatus::Succeeded;
    }

    /// Mark the execution as failed.
    pub fn fail(&mut self, error: String) {
        self.end_time = Some(Utc::now());
        self.output = error;
        self.status = ExecutionStatus::Failed;
    }

    /// Duration of the execution so far (or total if finished).
    pub fn elapsed(&self) -> chrono::Duration {
        let end = self.end_time.unwrap_or_else(Utc::now);
        end - self.start_time
    }
}

// ---------------------------------------------------------------------------
// AgentLoad  (internal bookkeeping for the router)
// ---------------------------------------------------------------------------

/// Snapshot of an agent's current workload.
#[derive(Debug, Clone, Default)]
struct AgentLoad {
    active_executions: u32,
    total_tokens_in_flight: u32,
}

// ---------------------------------------------------------------------------
// SkillRouter
// ---------------------------------------------------------------------------

/// Routes skills to the best-fit agent considering role compatibility and load.
#[derive(Debug)]
pub struct SkillRouter {
    /// Current load per agent role.
    load: HashMap<AgentRole, AgentLoad>,
    /// Full execution history for learning / analytics.
    history: Vec<SkillExecution>,
}

impl SkillRouter {
    pub fn new() -> Self {
        Self {
            load: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// Pick the best agent role to run `skill` given current load.
    ///
    /// Selection heuristic: among allowed roles, choose the one with the fewest
    /// active executions.  Ties are broken by fewest tokens in flight.
    pub fn route(&self, skill: &Skill) -> Option<AgentRole> {
        skill
            .required_agents
            .iter()
            .min_by_key(|role| {
                let load = self.load.get(role).cloned().unwrap_or_default();
                (load.active_executions, load.total_tokens_in_flight)
            })
            .copied()
    }

    /// Decompose a composite skill into its atomic dependencies (in topological
    /// order) using the registry.
    pub fn decompose<'a>(
        &self,
        skill: &'a Skill,
        registry: &'a SkillRegistry,
    ) -> Vec<&'a Skill> {
        let mut ordered: Vec<&Skill> = Vec::new();
        let mut visited: std::collections::HashSet<&SkillId> = std::collections::HashSet::new();
        self.topo_visit(skill, registry, &mut visited, &mut ordered);
        ordered
    }

    /// Recursive topological traversal.
    fn topo_visit<'a>(
        &self,
        skill: &'a Skill,
        registry: &'a SkillRegistry,
        visited: &mut std::collections::HashSet<&'a SkillId>,
        ordered: &mut Vec<&'a Skill>,
    ) {
        if !visited.insert(&skill.id) {
            return;
        }
        for dep_id in &skill.dependencies {
            if let Some(dep) = registry.get(dep_id) {
                self.topo_visit(dep, registry, visited, ordered);
            }
        }
        ordered.push(skill);
    }

    /// Record that an agent started executing a skill.
    pub fn begin_execution(&mut self, role: AgentRole, estimated_tokens: u32) {
        let entry = self.load.entry(role).or_default();
        entry.active_executions += 1;
        entry.total_tokens_in_flight += estimated_tokens;
    }

    /// Record that an agent finished executing a skill.
    pub fn end_execution(&mut self, execution: SkillExecution) {
        if let Some(entry) = self.load.get_mut(&execution.agent_role) {
            entry.active_executions = entry.active_executions.saturating_sub(1);
            entry.total_tokens_in_flight = entry
                .total_tokens_in_flight
                .saturating_sub(execution.tokens_used);
        }
        self.history.push(execution);
    }

    /// Average quality score for a given skill across all recorded executions.
    pub fn avg_quality(&self, skill_id: &SkillId) -> Option<f64> {
        let (sum, count) = self
            .history
            .iter()
            .filter(|e| e.skill_id == *skill_id && e.status == ExecutionStatus::Succeeded)
            .fold((0.0_f64, 0u32), |(s, c), e| (s + e.quality_score, c + 1));
        if count == 0 {
            None
        } else {
            Some(sum / f64::from(count))
        }
    }

    /// All historical executions (immutable access).
    pub fn history(&self) -> &[SkillExecution] {
        &self.history
    }
}

impl Default for SkillRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// SkillRegistry
// ---------------------------------------------------------------------------

/// Central store of every skill available to the agent swarm.
#[derive(Debug)]
pub struct SkillRegistry {
    skills: HashMap<SkillId, Skill>,
    by_category: HashMap<SkillCategory, Vec<SkillId>>,
    by_agent: HashMap<AgentRole, Vec<SkillId>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            by_category: HashMap::new(),
            by_agent: HashMap::new(),
        }
    }

    /// Register a skill. Overwrites any existing skill with the same id.
    pub fn register(&mut self, skill: Skill) {
        let id = skill.id.clone();
        let category = skill.category;
        let agents = skill.required_agents.clone();

        self.skills.insert(id.clone(), skill);

        self.by_category
            .entry(category)
            .or_default()
            .push(id.clone());

        for role in agents {
            self.by_agent.entry(role).or_default().push(id.clone());
        }
    }

    /// Lookup a skill by its unique id.
    pub fn get(&self, id: &SkillId) -> Option<&Skill> {
        self.skills.get(id)
    }

    /// All skills in a given category.
    pub fn by_category(&self, category: SkillCategory) -> Vec<&Skill> {
        self.by_category
            .get(&category)
            .map(|ids| ids.iter().filter_map(|id| self.skills.get(id)).collect())
            .unwrap_or_default()
    }

    /// All skills a particular agent role is allowed to execute.
    pub fn by_agent(&self, role: AgentRole) -> Vec<&Skill> {
        self.by_agent
            .get(&role)
            .map(|ids| ids.iter().filter_map(|id| self.skills.get(id)).collect())
            .unwrap_or_default()
    }

    /// Resolve a dependency chain: returns the full ordered list of skills
    /// required to execute the target, including the target itself.
    pub fn skill_chain(&self, target: &SkillId) -> Vec<&Skill> {
        let mut chain = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.resolve_chain(target, &mut visited, &mut chain);
        chain
    }

    fn resolve_chain<'a>(
        &'a self,
        id: &SkillId,
        visited: &mut std::collections::HashSet<SkillId>,
        chain: &mut Vec<&'a Skill>,
    ) {
        if !visited.insert(id.clone()) {
            return;
        }
        if let Some(skill) = self.skills.get(id) {
            for dep in &skill.dependencies {
                self.resolve_chain(dep, visited, chain);
            }
            chain.push(skill);
        }
    }

    /// Total number of registered skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Iterate over all registered skills.
    pub fn iter(&self) -> impl Iterator<Item = (&SkillId, &Skill)> {
        self.skills.iter()
    }

    /// Register every built-in skill from all sub-modules.
    ///
    /// Each sub-module exposes a `register(registry: &mut SkillRegistry)` fn
    /// that populates its category's skills.
    pub fn register_all_defaults(&mut self) {
        code_gen::register(self);
        architecture::register(self);
        database::register(self);
        api_design::register(self);
        testing::register(self);
        security_skills::register(self);
        devops::register(self);
        observability::register(self);
        performance::register(self);
        frontend::register(self);
        data_engineering::register(self);
        ai_ml::register(self);
        business::register(self);
        resilience::register(self);
        coordination::register(self);
        compliance::register(self);
        cost_optimization::register(self);
        documentation_skills::register(self);
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}
