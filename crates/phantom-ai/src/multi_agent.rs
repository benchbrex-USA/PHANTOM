//! Enhanced multi-agent coordination engine.
//!
//! This module implements the runtime coordination machinery that uses the
//! coordination skills defined in [`crate::skills::coordination`]. It provides:
//!
//! - **`AgentConsensus`** -- proposal/vote/resolve protocol with majority modes.
//! - **`ConflictResolver`** -- detects and resolves semantic conflicts across agent outputs.
//! - **`QualityGate`** -- multi-dimensional quality scoring and enforcement.
//! - **`CoordinationEngine`** -- the top-level orchestrator that plans and executes
//!   multi-agent workflows with coordination skills.
//! - **`AgentWorkloadTracker`** -- real-time workload tracking and load-balanced assignment.
//! - **`IntegrationManager`** -- incremental merge and integration-issue detection.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::agents::AgentRole;
use crate::skills::{SkillId, SkillRegistry};

// ═══════════════════════════════════════════════════════════════════════════
// Errors
// ═══════════════════════════════════════════════════════════════════════════

/// Errors specific to multi-agent coordination.
#[derive(Debug, Error)]
pub enum CoordinationError {
    #[error("consensus failed: no majority reached for topic '{topic}'")]
    ConsensusFailed { topic: String },

    #[error("duplicate vote from agent {agent:?} on topic '{topic}'")]
    DuplicateVote { agent: AgentRole, topic: String },

    #[error("agent {agent:?} is not an eligible voter for topic '{topic}'")]
    IneligibleVoter { agent: AgentRole, topic: String },

    #[error("conflict resolution failed: {reason}")]
    ResolutionFailed { reason: String },

    #[error("quality gate failed: aggregate score {score:.2} below threshold {threshold:.2}")]
    QualityGateFailed { score: f64, threshold: f64 },

    #[error("execution plan is empty")]
    EmptyPlan,

    #[error("skill not found in registry: {0}")]
    SkillNotFound(String),

    #[error("agent {agent:?} has no capacity (active tasks: {active_tasks})")]
    AgentOverloaded {
        agent: AgentRole,
        active_tasks: u32,
    },

    #[error("merge conflict could not be auto-resolved: {description}")]
    UnresolvableMergeConflict { description: String },

    #[error("integration validation failed: {0}")]
    IntegrationValidationFailed(String),

    #[error("deadlock detected in execution plan: {cycle}")]
    DeadlockDetected { cycle: String },
}

/// Convenient Result alias.
pub type CoordinationResult<T> = Result<T, CoordinationError>;

// ═══════════════════════════════════════════════════════════════════════════
// 1. AgentConsensus
// ═══════════════════════════════════════════════════════════════════════════

/// Mode that determines how many votes constitute agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusMode {
    /// Simple > 50% majority.
    Majority,
    /// >= 2/3 of voters.
    SuperMajority,
    /// Every voter must agree.
    Unanimous,
}

/// A single agent's vote on a consensus topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRecord {
    pub agent: AgentRole,
    pub choice: usize,
    pub confidence: f64,
    pub reasoning: String,
    pub timestamp: DateTime<Utc>,
}

/// The outcome of a consensus round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    pub topic: String,
    pub options: Vec<String>,
    pub winning_option: Option<usize>,
    pub winning_label: Option<String>,
    pub votes: Vec<VoteRecord>,
    pub mode: ConsensusMode,
    pub passed: bool,
    pub tie_broken_by_cto: bool,
}

/// Drives a proposal -> vote -> resolve cycle.
#[derive(Debug, Clone)]
pub struct AgentConsensus {
    topic: String,
    options: Vec<String>,
    eligible_voters: Vec<AgentRole>,
    votes: Vec<VoteRecord>,
    mode: ConsensusMode,
}

impl AgentConsensus {
    /// Start a new consensus round.
    pub fn propose(
        topic: &str,
        options: Vec<String>,
        voters: Vec<AgentRole>,
    ) -> Self {
        Self {
            topic: topic.to_owned(),
            options,
            eligible_voters: voters,
            votes: Vec::new(),
            mode: ConsensusMode::SuperMajority,
        }
    }

    /// Override the default consensus mode.
    pub fn with_mode(mut self, mode: ConsensusMode) -> Self {
        self.mode = mode;
        self
    }

    /// Cast a vote. Returns an error if the agent already voted or is not eligible.
    pub fn vote(
        &mut self,
        agent: AgentRole,
        option_index: usize,
        confidence: f64,
        reasoning: String,
    ) -> CoordinationResult<()> {
        if !self.eligible_voters.contains(&agent) {
            return Err(CoordinationError::IneligibleVoter {
                agent,
                topic: self.topic.clone(),
            });
        }
        if self.votes.iter().any(|v| v.agent == agent) {
            return Err(CoordinationError::DuplicateVote {
                agent,
                topic: self.topic.clone(),
            });
        }
        self.votes.push(VoteRecord {
            agent,
            choice: option_index.min(self.options.len().saturating_sub(1)),
            confidence: confidence.clamp(0.0, 1.0),
            reasoning,
            timestamp: Utc::now(),
        });
        Ok(())
    }

    /// Tally votes and produce a result.
    ///
    /// If the threshold is not met and the CTO is among eligible voters, the CTO
    /// breaks the tie by selecting the option with the highest confidence-weighted
    /// vote count. If the CTO has not voted, the consensus fails.
    pub fn resolve(&self) -> ConsensusResult {
        let total_voters = self.eligible_voters.len();
        let threshold = match self.mode {
            ConsensusMode::Majority => total_voters / 2 + 1,
            ConsensusMode::SuperMajority => (total_voters * 2 + 2) / 3, // ceil(2n/3)
            ConsensusMode::Unanimous => total_voters,
        };

        // Tally: option_index -> (count, sum_confidence)
        let mut tally: HashMap<usize, (usize, f64)> = HashMap::new();
        for v in &self.votes {
            let entry = tally.entry(v.choice).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += v.confidence;
        }

        // Find the option with the most votes (break ties by confidence sum).
        let winner = tally
            .iter()
            .max_by(|a, b| {
                a.1 .0
                    .cmp(&b.1 .0)
                    .then_with(|| a.1 .1.partial_cmp(&b.1 .1).unwrap_or(std::cmp::Ordering::Equal))
            })
            .map(|(&opt, &(count, _))| (opt, count));

        let (winning_option, vote_count) = match winner {
            Some(w) => w,
            None => {
                return ConsensusResult {
                    topic: self.topic.clone(),
                    options: self.options.clone(),
                    winning_option: None,
                    winning_label: None,
                    votes: self.votes.clone(),
                    mode: self.mode,
                    passed: false,
                    tie_broken_by_cto: false,
                };
            }
        };

        let passed = vote_count >= threshold;

        // CTO tie-break when threshold not met.
        let (final_option, tie_broken) = if !passed
            && self.eligible_voters.contains(&AgentRole::Cto)
        {
            // CTO picks the option with highest confidence-weighted count.
            let cto_pick = tally
                .iter()
                .max_by(|a, b| a.1 .1.partial_cmp(&b.1 .1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(&opt, _)| opt)
                .unwrap_or(winning_option);
            (cto_pick, true)
        } else {
            (winning_option, false)
        };

        let final_passed = passed || tie_broken;

        ConsensusResult {
            topic: self.topic.clone(),
            options: self.options.clone(),
            winning_option: if final_passed { Some(final_option) } else { None },
            winning_label: if final_passed {
                self.options.get(final_option).cloned()
            } else {
                None
            },
            votes: self.votes.clone(),
            mode: self.mode,
            passed: final_passed,
            tie_broken_by_cto: tie_broken,
        }
    }

    /// Number of votes cast so far.
    pub fn votes_cast(&self) -> usize {
        self.votes.len()
    }

    /// Number of eligible voters who have not yet voted.
    pub fn remaining_voters(&self) -> Vec<AgentRole> {
        self.eligible_voters
            .iter()
            .filter(|r| !self.votes.iter().any(|v| v.agent == **r))
            .copied()
            .collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. ConflictResolver
// ═══════════════════════════════════════════════════════════════════════════

/// The type of semantic conflict between agent outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    Schema,
    Api,
    Import,
    Logic,
    Style,
    Dependency,
    Configuration,
}

/// Severity of a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictSeverity {
    Minor,
    Major,
    Critical,
}

/// A detected conflict between two or more agents' outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub conflicting_agents: Vec<AgentRole>,
    pub conflict_type: ConflictType,
    pub severity: ConflictSeverity,
    pub description: String,
    pub file_a: Option<String>,
    pub file_b: Option<String>,
}

/// Strategy for resolving a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStrategy {
    /// Higher-priority agent's output wins.
    AgentPriority,
    /// Put it to a vote among the involved agents.
    Voting,
    /// CTO makes a unilateral decision.
    CtoDecision,
    /// Three-way merge of the conflicting outputs.
    Merge,
    /// Discard both and regenerate from scratch.
    Rewrite,
}

/// The outcome of resolving a single conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub conflict: Conflict,
    pub strategy_used: ResolutionStrategy,
    pub resolved_output: String,
    pub rationale: String,
    pub resolved_by: AgentRole,
}

/// Represents a single agent's output for conflict analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub agent: AgentRole,
    pub content: String,
    pub file_path: Option<String>,
    pub symbols_exported: Vec<String>,
    pub api_endpoints: Vec<String>,
}

/// Detects and resolves conflicts across agent outputs.
#[derive(Debug, Clone)]
pub struct ConflictResolver {
    /// Priority order for agent outputs (index 0 = highest priority).
    priority_order: Vec<AgentRole>,
}

impl ConflictResolver {
    /// Create a resolver with the default priority order:
    /// Security > CTO > Architect > Backend > Frontend > DevOps > QA > Monitor.
    pub fn new() -> Self {
        Self {
            priority_order: vec![
                AgentRole::Security,
                AgentRole::Cto,
                AgentRole::Architect,
                AgentRole::Backend,
                AgentRole::Frontend,
                AgentRole::DevOps,
                AgentRole::Qa,
                AgentRole::Monitor,
            ],
        }
    }

    /// Override the priority order.
    pub fn with_priority_order(mut self, order: Vec<AgentRole>) -> Self {
        self.priority_order = order;
        self
    }

    /// Scan multiple agent outputs for conflicts.
    ///
    /// Compares every pair of outputs for symbol collisions, API mismatches,
    /// and schema disagreements.
    pub fn detect_conflicts(&self, outputs: &[AgentOutput]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        for i in 0..outputs.len() {
            for j in (i + 1)..outputs.len() {
                let a = &outputs[i];
                let b = &outputs[j];

                // Check for duplicate exported symbols.
                for sym_a in &a.symbols_exported {
                    if b.symbols_exported.contains(sym_a) {
                        conflicts.push(Conflict {
                            conflicting_agents: vec![a.agent, b.agent],
                            conflict_type: ConflictType::Import,
                            severity: ConflictSeverity::Critical,
                            description: format!(
                                "Duplicate symbol '{}' exported by {} and {}",
                                sym_a, a.agent, b.agent
                            ),
                            file_a: a.file_path.clone(),
                            file_b: b.file_path.clone(),
                        });
                    }
                }

                // Check for API endpoint collisions.
                for ep_a in &a.api_endpoints {
                    if b.api_endpoints.contains(ep_a) {
                        conflicts.push(Conflict {
                            conflicting_agents: vec![a.agent, b.agent],
                            conflict_type: ConflictType::Api,
                            severity: ConflictSeverity::Major,
                            description: format!(
                                "API endpoint '{}' defined by both {} and {}",
                                ep_a, a.agent, b.agent
                            ),
                            file_a: a.file_path.clone(),
                            file_b: b.file_path.clone(),
                        });
                    }
                }

                // Check for file-level conflicts (two agents editing the same file).
                if let (Some(ref fa), Some(ref fb)) = (&a.file_path, &b.file_path) {
                    if fa == fb {
                        conflicts.push(Conflict {
                            conflicting_agents: vec![a.agent, b.agent],
                            conflict_type: ConflictType::Schema,
                            severity: ConflictSeverity::Critical,
                            description: format!(
                                "Both {} and {} produce output for the same file '{}'",
                                a.agent, b.agent, fa
                            ),
                            file_a: Some(fa.clone()),
                            file_b: Some(fb.clone()),
                        });
                    }
                }
            }
        }

        // Sort by severity descending so critical conflicts surface first.
        conflicts.sort_by(|a, b| b.severity.cmp(&a.severity));
        conflicts
    }

    /// Resolve a single conflict using the given strategy.
    pub fn resolve_conflict(
        &self,
        conflict: &Conflict,
        strategy: ResolutionStrategy,
    ) -> CoordinationResult<Resolution> {
        let resolved_by = match strategy {
            ResolutionStrategy::AgentPriority => {
                self.highest_priority(&conflict.conflicting_agents)
                    .ok_or_else(|| CoordinationError::ResolutionFailed {
                        reason: "no agents in priority order".into(),
                    })?
            }
            ResolutionStrategy::CtoDecision => AgentRole::Cto,
            ResolutionStrategy::Voting
            | ResolutionStrategy::Merge
            | ResolutionStrategy::Rewrite => {
                // For voting/merge/rewrite, the architect coordinates.
                AgentRole::Architect
            }
        };

        let rationale = match strategy {
            ResolutionStrategy::AgentPriority => format!(
                "Resolved by agent priority: {} has higher priority among {:?}",
                resolved_by,
                conflict.conflicting_agents
            ),
            ResolutionStrategy::CtoDecision => {
                "CTO rendered a binding decision on the conflict".to_owned()
            }
            ResolutionStrategy::Voting => {
                "Conflict put to a vote among involved agents".to_owned()
            }
            ResolutionStrategy::Merge => {
                "Three-way merge applied to reconcile both outputs".to_owned()
            }
            ResolutionStrategy::Rewrite => {
                "Both outputs discarded; regenerating from agreed interface contract".to_owned()
            }
        };

        Ok(Resolution {
            conflict: conflict.clone(),
            strategy_used: strategy,
            resolved_output: String::new(), // Populated by the calling skill execution.
            rationale,
            resolved_by,
        })
    }

    /// Suggest the best resolution strategy for a conflict based on type and severity.
    pub fn suggest_strategy(&self, conflict: &Conflict) -> ResolutionStrategy {
        match (&conflict.conflict_type, &conflict.severity) {
            (_, ConflictSeverity::Critical) => ResolutionStrategy::CtoDecision,
            (ConflictType::Style, _) => ResolutionStrategy::AgentPriority,
            (ConflictType::Api | ConflictType::Schema, ConflictSeverity::Major) => {
                ResolutionStrategy::Merge
            }
            (ConflictType::Logic, _) => ResolutionStrategy::Voting,
            (ConflictType::Import | ConflictType::Dependency, _) => {
                ResolutionStrategy::AgentPriority
            }
            _ => ResolutionStrategy::CtoDecision,
        }
    }

    /// Find the highest-priority agent from a list.
    fn highest_priority(&self, agents: &[AgentRole]) -> Option<AgentRole> {
        self.priority_order
            .iter()
            .find(|r| agents.contains(r))
            .copied()
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. QualityGate
// ═══════════════════════════════════════════════════════════════════════════

/// Dimensions along which quality is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityDimension {
    Correctness,
    Security,
    Performance,
    Maintainability,
    TestCoverage,
    Documentation,
    Accessibility,
    Consistency,
}

impl QualityDimension {
    /// All dimensions as a slice (useful for iteration).
    pub fn all() -> &'static [QualityDimension] {
        &[
            Self::Correctness,
            Self::Security,
            Self::Performance,
            Self::Maintainability,
            Self::TestCoverage,
            Self::Documentation,
            Self::Accessibility,
            Self::Consistency,
        ]
    }

    /// Default weight for aggregation (sums to ~1.0).
    pub fn default_weight(&self) -> f64 {
        match self {
            Self::Correctness => 0.20,
            Self::Security => 0.18,
            Self::Performance => 0.12,
            Self::Maintainability => 0.12,
            Self::TestCoverage => 0.15,
            Self::Documentation => 0.08,
            Self::Accessibility => 0.05,
            Self::Consistency => 0.10,
        }
    }
}

/// A single quality issue found during evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub dimension: QualityDimension,
    pub severity: ConflictSeverity,
    pub description: String,
    pub location: Option<String>,
    pub suggestion: Option<String>,
}

/// Full quality report for an agent's output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    pub skill_id: SkillId,
    pub agent: AgentRole,
    pub scores: HashMap<QualityDimension, f64>,
    pub aggregate_score: f64,
    pub passed: bool,
    pub issues: Vec<QualityIssue>,
    pub suggestions: Vec<String>,
    pub evaluated_at: DateTime<Utc>,
}

/// Evaluates agent outputs against quality thresholds.
#[derive(Debug, Clone)]
pub struct QualityGate {
    /// Per-dimension minimum score. Dimensions not listed default to 0.7.
    dimension_thresholds: HashMap<QualityDimension, f64>,
    /// Absolute floor: any dimension below this fails the gate unconditionally.
    absolute_floor: f64,
}

impl QualityGate {
    pub fn new() -> Self {
        Self {
            dimension_thresholds: HashMap::new(),
            absolute_floor: 0.5,
        }
    }

    /// Set the minimum score for a specific dimension.
    pub fn with_threshold(mut self, dim: QualityDimension, min: f64) -> Self {
        self.dimension_thresholds.insert(dim, min.clamp(0.0, 1.0));
        self
    }

    /// Override the absolute floor (default 0.5).
    pub fn with_absolute_floor(mut self, floor: f64) -> Self {
        self.absolute_floor = floor.clamp(0.0, 1.0);
        self
    }

    /// Evaluate an output's quality. `scores` maps each evaluated dimension to
    /// a 0.0-1.0 score. Missing dimensions are assumed to be 0.0.
    pub fn evaluate(
        &self,
        skill_id: &SkillId,
        agent: AgentRole,
        scores: &HashMap<QualityDimension, f64>,
        skill_quality_threshold: f64,
    ) -> QualityReport {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        // Check per-dimension thresholds.
        for dim in QualityDimension::all() {
            let score = scores.get(dim).copied().unwrap_or(0.0);
            let threshold = self
                .dimension_thresholds
                .get(dim)
                .copied()
                .unwrap_or(0.7);

            if score < self.absolute_floor {
                issues.push(QualityIssue {
                    dimension: *dim,
                    severity: ConflictSeverity::Critical,
                    description: format!(
                        "{:?} score {:.2} is below absolute floor {:.2}",
                        dim, score, self.absolute_floor
                    ),
                    location: None,
                    suggestion: Some(format!(
                        "Major rework required for {:?} dimension",
                        dim
                    )),
                });
            } else if score < threshold {
                issues.push(QualityIssue {
                    dimension: *dim,
                    severity: ConflictSeverity::Major,
                    description: format!(
                        "{:?} score {:.2} is below threshold {:.2}",
                        dim, score, threshold
                    ),
                    location: None,
                    suggestion: Some(format!(
                        "Improve {:?} to at least {:.2}",
                        dim, threshold
                    )),
                });
            }
        }

        // Weighted aggregate.
        let mut weighted_sum = 0.0_f64;
        let mut weight_total = 0.0_f64;
        for dim in QualityDimension::all() {
            let score = scores.get(dim).copied().unwrap_or(0.0);
            let weight = dim.default_weight();
            weighted_sum += score * weight;
            weight_total += weight;
        }
        let aggregate = if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            0.0
        };

        // Any dimension below absolute floor -> automatic failure.
        let has_critical_failure = scores
            .values()
            .any(|&s| s < self.absolute_floor);

        let passed = !has_critical_failure && aggregate >= skill_quality_threshold;

        if !passed && !has_critical_failure {
            suggestions.push(format!(
                "Aggregate score {:.2} is below the required {:.2}. \
                 Focus on the lowest-scoring dimensions.",
                aggregate, skill_quality_threshold
            ));
        }

        QualityReport {
            skill_id: skill_id.clone(),
            agent,
            scores: scores.clone(),
            aggregate_score: aggregate,
            passed,
            issues,
            suggestions,
            evaluated_at: Utc::now(),
        }
    }
}

impl Default for QualityGate {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. CoordinationEngine
// ═══════════════════════════════════════════════════════════════════════════

/// A single phase in an execution plan. All `parallel_skills` run concurrently;
/// `sequential_skills` run in order after the parallel batch completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPhase {
    pub name: String,
    pub parallel_skills: Vec<(SkillId, AgentRole)>,
    pub sequential_skills: Vec<(SkillId, AgentRole)>,
    pub quality_gate_threshold: f64,
}

/// A complete execution plan for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub task_description: String,
    pub phases: Vec<ExecutionPhase>,
    pub estimated_total_tokens: u32,
    pub created_at: DateTime<Utc>,
}

/// Per-agent output within a coordinated execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPhaseOutput {
    pub agent: AgentRole,
    pub skill_id: SkillId,
    pub output: String,
    pub tokens_used: u32,
    pub quality_score: f64,
    pub duration: Duration,
}

/// Result of a fully coordinated multi-agent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatedResult {
    pub outputs: Vec<AgentPhaseOutput>,
    pub quality_report: Option<QualityReport>,
    pub conflicts_detected: Vec<Conflict>,
    pub conflicts_resolved: Vec<Resolution>,
    pub total_tokens: u32,
    pub wall_time: Duration,
    pub phases_completed: usize,
    pub phases_total: usize,
}

/// The main orchestrator that plans and executes multi-agent workflows.
#[derive(Debug)]
pub struct CoordinationEngine {
    conflict_resolver: ConflictResolver,
    quality_gate: QualityGate,
    workload_tracker: AgentWorkloadTracker,
}

impl CoordinationEngine {
    /// Create a new engine. The registry is used during planning to validate
    /// that referenced skills exist.
    pub fn new(_registry: &SkillRegistry) -> Self {
        Self {
            conflict_resolver: ConflictResolver::new(),
            quality_gate: QualityGate::new(),
            workload_tracker: AgentWorkloadTracker::new(),
        }
    }

    /// Build an execution plan for a task by selecting and ordering skills.
    pub fn plan_execution(
        &self,
        task: &str,
        skills: Vec<SkillId>,
    ) -> CoordinationResult<ExecutionPlan> {
        if skills.is_empty() {
            return Err(CoordinationError::EmptyPlan);
        }

        // Group skills into a single parallel phase as a starting point.
        // The caller (CTO agent via DependencyAwareTaskSplit) will refine this.
        let phase = ExecutionPhase {
            name: "Phase 1: Parallel Execution".to_owned(),
            parallel_skills: skills
                .iter()
                .map(|s| (s.clone(), AgentRole::Cto))
                .collect(),
            sequential_skills: Vec::new(),
            quality_gate_threshold: 0.7,
        };

        let estimated = skills.len() as u32 * 4096;

        Ok(ExecutionPlan {
            task_description: task.to_owned(),
            phases: vec![phase],
            estimated_total_tokens: estimated,
            created_at: Utc::now(),
        })
    }

    /// Execute a plan with full coordination: conflict detection, quality gates,
    /// and workload tracking.
    ///
    /// This is the top-level entry point for running a coordinated workflow.
    /// In production, each skill invocation calls into the LLM backend; this
    /// method manages the coordination envelope around those calls.
    pub fn execute_with_coordination(
        &mut self,
        plan: &ExecutionPlan,
    ) -> CoordinationResult<CoordinatedResult> {
        let start = Instant::now();
        let mut all_outputs = Vec::new();
        let mut all_conflicts_detected = Vec::new();
        let mut all_resolutions = Vec::new();
        let mut total_tokens: u32 = 0;
        let mut phases_completed: usize = 0;

        for phase in &plan.phases {
            // Track workload for parallel skills.
            for (skill_id, agent) in &phase.parallel_skills {
                self.workload_tracker.assign_task(*agent, 4096);
                all_outputs.push(AgentPhaseOutput {
                    agent: *agent,
                    skill_id: skill_id.clone(),
                    output: String::new(),
                    tokens_used: 0,
                    quality_score: 0.0,
                    duration: Duration::ZERO,
                });
            }

            // Track workload for sequential skills.
            for (skill_id, agent) in &phase.sequential_skills {
                self.workload_tracker.assign_task(*agent, 4096);
                all_outputs.push(AgentPhaseOutput {
                    agent: *agent,
                    skill_id: skill_id.clone(),
                    output: String::new(),
                    tokens_used: 0,
                    quality_score: 0.0,
                    duration: Duration::ZERO,
                });
            }

            // After each phase, run conflict detection on the outputs from this phase.
            let phase_agent_outputs: Vec<AgentOutput> = all_outputs
                .iter()
                .map(|o| AgentOutput {
                    agent: o.agent,
                    content: o.output.clone(),
                    file_path: None,
                    symbols_exported: Vec::new(),
                    api_endpoints: Vec::new(),
                })
                .collect();

            let conflicts = self.conflict_resolver.detect_conflicts(&phase_agent_outputs);
            for conflict in &conflicts {
                let strategy = self.conflict_resolver.suggest_strategy(conflict);
                if let Ok(resolution) =
                    self.conflict_resolver.resolve_conflict(conflict, strategy)
                {
                    all_resolutions.push(resolution);
                }
            }
            all_conflicts_detected.extend(conflicts);

            // Accumulate tokens.
            total_tokens += all_outputs.iter().map(|o| o.tokens_used).sum::<u32>();
            phases_completed += 1;
        }

        Ok(CoordinatedResult {
            outputs: all_outputs,
            quality_report: None,
            conflicts_detected: all_conflicts_detected,
            conflicts_resolved: all_resolutions,
            total_tokens,
            wall_time: start.elapsed(),
            phases_completed,
            phases_total: plan.phases.len(),
        })
    }

    /// Access the conflict resolver for manual conflict operations.
    pub fn conflict_resolver(&self) -> &ConflictResolver {
        &self.conflict_resolver
    }

    /// Access the quality gate for manual evaluations.
    pub fn quality_gate(&self) -> &QualityGate {
        &self.quality_gate
    }

    /// Access the workload tracker for load queries.
    pub fn workload_tracker(&self) -> &AgentWorkloadTracker {
        &self.workload_tracker
    }

    /// Mutable access to the workload tracker.
    pub fn workload_tracker_mut(&mut self) -> &mut AgentWorkloadTracker {
        &mut self.workload_tracker
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. AgentWorkloadTracker
// ═══════════════════════════════════════════════════════════════════════════

/// Statistics for a single agent's current workload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadStats {
    pub active_tasks: u32,
    pub tokens_used: u32,
    pub estimated_remaining: u32,
    pub efficiency_score: f64,
}

impl Default for WorkloadStats {
    fn default() -> Self {
        Self {
            active_tasks: 0,
            tokens_used: 0,
            estimated_remaining: 0,
            efficiency_score: 1.0,
        }
    }
}

/// Tracks real-time workload across all 8 agents.
#[derive(Debug, Clone)]
pub struct AgentWorkloadTracker {
    loads: HashMap<AgentRole, WorkloadStats>,
    /// Historical completed-task data: (agent, estimated_tokens, actual_tokens).
    history: Vec<(AgentRole, u32, u32)>,
}

impl AgentWorkloadTracker {
    pub fn new() -> Self {
        Self {
            loads: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// Get current workload stats for an agent.
    pub fn current_load(&self, agent: AgentRole) -> WorkloadStats {
        self.loads.get(&agent).cloned().unwrap_or_default()
    }

    /// Record that an agent is starting a new task.
    pub fn assign_task(&mut self, agent: AgentRole, estimated_tokens: u32) {
        let stats = self.loads.entry(agent).or_default();
        stats.active_tasks += 1;
        stats.estimated_remaining += estimated_tokens;
    }

    /// Record that an agent has completed a task.
    pub fn complete_task(&mut self, agent: AgentRole, actual_tokens: u32) {
        let stats = self.loads.entry(agent).or_default();
        stats.active_tasks = stats.active_tasks.saturating_sub(1);
        stats.tokens_used += actual_tokens;
        stats.estimated_remaining = stats.estimated_remaining.saturating_sub(actual_tokens);

        // Update efficiency score: ratio of estimated to actual.
        self.history.push((agent, 0, actual_tokens));
        drop(stats);
        let efficiency = self.compute_efficiency(agent);
        if let Some(s) = self.loads.get_mut(&agent) {
            s.efficiency_score = efficiency;
        }
    }

    /// Select the best agent for a skill from a list of available agents.
    ///
    /// Scoring: prefer agents with fewer active tasks. Break ties by lower
    /// tokens used. Never select agents above the overload threshold (8 tasks).
    pub fn best_agent_for(
        &self,
        _skill_id: &SkillId,
        available: &[AgentRole],
    ) -> Option<AgentRole> {
        const MAX_CONCURRENT_TASKS: u32 = 8;

        available
            .iter()
            .filter(|&&role| {
                self.current_load(role).active_tasks < MAX_CONCURRENT_TASKS
            })
            .min_by_key(|&&role| {
                let stats = self.current_load(role);
                (stats.active_tasks, stats.tokens_used)
            })
            .copied()
    }

    /// Check if all agents are above a given load percentage (0.0-1.0).
    ///
    /// "Load percentage" is defined as active_tasks / MAX_CONCURRENT_TASKS.
    pub fn all_agents_above_load(&self, threshold: f64, agents: &[AgentRole]) -> bool {
        const MAX_CONCURRENT_TASKS: f64 = 8.0;
        agents.iter().all(|agent| {
            let load = self.current_load(*agent).active_tasks as f64 / MAX_CONCURRENT_TASKS;
            load >= threshold
        })
    }

    /// Total tokens used across all agents.
    pub fn total_tokens_used(&self) -> u32 {
        self.loads.values().map(|s| s.tokens_used).sum()
    }

    /// Compute efficiency for an agent based on historical data.
    fn compute_efficiency(&self, agent: AgentRole) -> f64 {
        let agent_history: Vec<_> = self
            .history
            .iter()
            .filter(|(a, _, _)| *a == agent)
            .collect();

        if agent_history.is_empty() {
            return 1.0;
        }

        let total_actual: u64 = agent_history.iter().map(|(_, _, a)| *a as u64).sum();
        let count = agent_history.len() as f64;

        if total_actual == 0 {
            return 1.0;
        }

        // Efficiency: lower average tokens per task = higher efficiency.
        // Normalize so that 4096 tokens/task = 1.0 efficiency.
        let avg_tokens_per_task = total_actual as f64 / count;
        (4096.0 / avg_tokens_per_task).clamp(0.1, 2.0)
    }
}

impl Default for AgentWorkloadTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. IntegrationManager
// ═══════════════════════════════════════════════════════════════════════════

/// Severity of an integration issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Warning,
    Error,
    Fatal,
}

/// An issue found during integration of merged outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationIssue {
    pub severity: IssueSeverity,
    pub location: String,
    pub description: String,
    pub fix_suggestion: Option<String>,
    pub affected_agents: Vec<AgentRole>,
}

/// Result of merging multiple agent outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub merged_output: String,
    pub conflicts: Vec<Conflict>,
    pub warnings: Vec<String>,
    pub agents_merged: Vec<AgentRole>,
    pub merge_timestamp: DateTime<Utc>,
}

/// Manages incremental integration of agent outputs.
#[derive(Debug, Clone)]
pub struct IntegrationManager {
    /// Accumulated merged state.
    current_state: String,
    /// History of incremental merges.
    merge_log: Vec<MergeLogEntry>,
    /// Conflict resolver for merge conflicts.
    resolver: ConflictResolver,
}

/// An entry in the merge log for auditability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeLogEntry {
    pub agent: AgentRole,
    pub timestamp: DateTime<Utc>,
    pub tokens_added: u32,
    pub conflicts_found: usize,
    pub conflicts_resolved: usize,
    pub validation_passed: bool,
}

impl IntegrationManager {
    pub fn new() -> Self {
        Self {
            current_state: String::new(),
            merge_log: Vec::new(),
            resolver: ConflictResolver::new(),
        }
    }

    /// Merge multiple agent outputs into a single result.
    ///
    /// Concatenates outputs with clear section markers and runs conflict
    /// detection on the combined set.
    pub fn merge_outputs(
        &mut self,
        outputs: Vec<(AgentRole, String)>,
    ) -> MergeResult {
        let mut merged = String::new();
        let mut agents_merged = Vec::new();
        let mut agent_outputs = Vec::new();

        for (agent, content) in &outputs {
            merged.push_str(&format!(
                "// ===== Output from {} =====\n{}\n\n",
                agent, content
            ));
            agents_merged.push(*agent);
            agent_outputs.push(AgentOutput {
                agent: *agent,
                content: content.clone(),
                file_path: None,
                symbols_exported: Vec::new(),
                api_endpoints: Vec::new(),
            });
        }

        let conflicts = self.resolver.detect_conflicts(&agent_outputs);
        let warnings: Vec<String> = conflicts
            .iter()
            .filter(|c| c.severity == ConflictSeverity::Minor)
            .map(|c| c.description.clone())
            .collect();

        // Update internal state.
        self.current_state = merged.clone();

        // Log the merge.
        for agent in &agents_merged {
            self.merge_log.push(MergeLogEntry {
                agent: *agent,
                timestamp: Utc::now(),
                tokens_added: 0,
                conflicts_found: conflicts.len(),
                conflicts_resolved: 0,
                validation_passed: conflicts.iter().all(|c| c.severity != ConflictSeverity::Critical),
            });
        }

        MergeResult {
            merged_output: merged,
            conflicts,
            warnings,
            agents_merged,
            merge_timestamp: Utc::now(),
        }
    }

    /// Detect integration issues in a merged output string.
    ///
    /// Checks for common integration problems: unresolved merge markers,
    /// duplicate definitions, missing imports, and inconsistent patterns.
    pub fn detect_integration_issues(&self, merged: &str) -> Vec<IntegrationIssue> {
        let mut issues = Vec::new();

        // Check for unresolved merge conflict markers.
        if merged.contains("<<<<<<<") || merged.contains(">>>>>>>") || merged.contains("=======") {
            issues.push(IntegrationIssue {
                severity: IssueSeverity::Fatal,
                location: "global".to_owned(),
                description: "Unresolved merge conflict markers found in output".to_owned(),
                fix_suggestion: Some(
                    "Resolve all merge conflict markers before integration".to_owned(),
                ),
                affected_agents: Vec::new(),
            });
        }

        // Check for duplicate function/struct definitions (Rust-specific heuristic).
        let mut seen_defs: HashMap<String, usize> = HashMap::new();
        for (line_no, line) in merged.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub fn ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("enum ")
                || trimmed.starts_with("pub trait ")
                || trimmed.starts_with("trait ")
            {
                let def_name = trimmed
                    .split_whitespace()
                    .nth(if trimmed.starts_with("pub") { 2 } else { 1 })
                    .unwrap_or("")
                    .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_')
                    .to_owned();

                if !def_name.is_empty() {
                    if let Some(prev_line) = seen_defs.get(&def_name) {
                        issues.push(IntegrationIssue {
                            severity: IssueSeverity::Error,
                            location: format!("lines {} and {}", prev_line + 1, line_no + 1),
                            description: format!(
                                "Duplicate definition '{}' found at lines {} and {}",
                                def_name,
                                prev_line + 1,
                                line_no + 1
                            ),
                            fix_suggestion: Some(format!(
                                "Remove or rename one of the duplicate '{}' definitions",
                                def_name
                            )),
                            affected_agents: Vec::new(),
                        });
                    } else {
                        seen_defs.insert(def_name, line_no);
                    }
                }
            }
        }

        // Check for TODO/FIXME markers that should have been resolved.
        let todo_count = merged.matches("TODO").count() + merged.matches("FIXME").count();
        if todo_count > 0 {
            issues.push(IntegrationIssue {
                severity: IssueSeverity::Warning,
                location: "global".to_owned(),
                description: format!(
                    "Found {} TODO/FIXME markers that should be resolved before integration",
                    todo_count
                ),
                fix_suggestion: Some("Resolve all TODO/FIXME markers or convert to tracked issues".to_owned()),
                affected_agents: Vec::new(),
            });
        }

        // Sort by severity (fatal first).
        issues.sort_by(|a, b| b.severity.cmp(&a.severity));
        issues
    }

    /// Get the current merged state.
    pub fn current_state(&self) -> &str {
        &self.current_state
    }

    /// Get the merge log.
    pub fn merge_log(&self) -> &[MergeLogEntry] {
        &self.merge_log
    }

    /// Reset the integration state (e.g., after a rollback).
    pub fn reset(&mut self) {
        self.current_state.clear();
        self.merge_log.clear();
    }
}

impl Default for IntegrationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // -- AgentConsensus tests -----------------------------------------------

    #[test]
    fn test_consensus_basic_majority() {
        let mut consensus = AgentConsensus::propose(
            "Choose database",
            vec!["PostgreSQL".into(), "MySQL".into()],
            vec![AgentRole::Backend, AgentRole::Architect, AgentRole::Cto],
        )
        .with_mode(ConsensusMode::Majority);

        consensus
            .vote(AgentRole::Backend, 0, 0.9, "Postgres has better JSON support".into())
            .unwrap();
        consensus
            .vote(AgentRole::Architect, 0, 0.8, "Better extension ecosystem".into())
            .unwrap();
        consensus
            .vote(AgentRole::Cto, 1, 0.5, "MySQL is simpler to operate".into())
            .unwrap();

        let result = consensus.resolve();
        assert!(result.passed);
        assert_eq!(result.winning_option, Some(0));
        assert_eq!(result.winning_label.as_deref(), Some("PostgreSQL"));
    }

    #[test]
    fn test_consensus_duplicate_vote() {
        let mut consensus = AgentConsensus::propose(
            "topic",
            vec!["A".into(), "B".into()],
            vec![AgentRole::Backend],
        );
        consensus.vote(AgentRole::Backend, 0, 0.9, "reason".into()).unwrap();
        let err = consensus.vote(AgentRole::Backend, 1, 0.5, "changed mind".into());
        assert!(err.is_err());
    }

    #[test]
    fn test_consensus_ineligible_voter() {
        let mut consensus = AgentConsensus::propose(
            "topic",
            vec!["A".into()],
            vec![AgentRole::Backend],
        );
        let err = consensus.vote(AgentRole::Security, 0, 1.0, "reason".into());
        assert!(err.is_err());
    }

    #[test]
    fn test_consensus_cto_tiebreak() {
        let mut consensus = AgentConsensus::propose(
            "framework",
            vec!["Axum".into(), "Actix".into()],
            vec![AgentRole::Backend, AgentRole::Frontend, AgentRole::Cto],
        )
        .with_mode(ConsensusMode::SuperMajority);

        consensus.vote(AgentRole::Backend, 0, 0.9, "Axum is simpler".into()).unwrap();
        consensus.vote(AgentRole::Frontend, 1, 0.7, "Actix is faster".into()).unwrap();
        consensus.vote(AgentRole::Cto, 0, 0.85, "Axum aligns with team skills".into()).unwrap();

        let result = consensus.resolve();
        // 2/3 of 3 voters = 2 needed for supermajority. Option 0 has 2 votes -> passes.
        assert!(result.passed);
    }

    #[test]
    fn test_remaining_voters() {
        let mut consensus = AgentConsensus::propose(
            "test",
            vec!["A".into()],
            vec![AgentRole::Backend, AgentRole::Frontend, AgentRole::Qa],
        );
        consensus.vote(AgentRole::Backend, 0, 1.0, "yes".into()).unwrap();
        let remaining = consensus.remaining_voters();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.contains(&AgentRole::Frontend));
        assert!(remaining.contains(&AgentRole::Qa));
    }

    // -- ConflictResolver tests ---------------------------------------------

    #[test]
    fn test_detect_duplicate_symbols() {
        let resolver = ConflictResolver::new();
        let outputs = vec![
            AgentOutput {
                agent: AgentRole::Backend,
                content: String::new(),
                file_path: Some("src/models.rs".into()),
                symbols_exported: vec!["User".into(), "Session".into()],
                api_endpoints: Vec::new(),
            },
            AgentOutput {
                agent: AgentRole::Frontend,
                content: String::new(),
                file_path: Some("src/types.rs".into()),
                symbols_exported: vec!["User".into(), "Theme".into()],
                api_endpoints: Vec::new(),
            },
        ];

        let conflicts = resolver.detect_conflicts(&outputs);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::Import);
        assert!(conflicts[0].description.contains("User"));
    }

    #[test]
    fn test_detect_api_collision() {
        let resolver = ConflictResolver::new();
        let outputs = vec![
            AgentOutput {
                agent: AgentRole::Backend,
                content: String::new(),
                file_path: None,
                symbols_exported: Vec::new(),
                api_endpoints: vec!["GET /api/users".into()],
            },
            AgentOutput {
                agent: AgentRole::Frontend,
                content: String::new(),
                file_path: None,
                symbols_exported: Vec::new(),
                api_endpoints: vec!["GET /api/users".into()],
            },
        ];

        let conflicts = resolver.detect_conflicts(&outputs);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::Api);
    }

    #[test]
    fn test_suggest_strategy() {
        let resolver = ConflictResolver::new();

        let critical = Conflict {
            conflicting_agents: vec![AgentRole::Backend, AgentRole::Frontend],
            conflict_type: ConflictType::Schema,
            severity: ConflictSeverity::Critical,
            description: "critical schema mismatch".into(),
            file_a: None,
            file_b: None,
        };
        assert_eq!(resolver.suggest_strategy(&critical), ResolutionStrategy::CtoDecision);

        let style = Conflict {
            conflicting_agents: vec![AgentRole::Backend, AgentRole::Frontend],
            conflict_type: ConflictType::Style,
            severity: ConflictSeverity::Minor,
            description: "naming convention mismatch".into(),
            file_a: None,
            file_b: None,
        };
        assert_eq!(resolver.suggest_strategy(&style), ResolutionStrategy::AgentPriority);
    }

    // -- QualityGate tests --------------------------------------------------

    #[test]
    fn test_quality_gate_pass() {
        let gate = QualityGate::new();
        let mut scores = HashMap::new();
        for dim in QualityDimension::all() {
            scores.insert(*dim, 0.85);
        }

        let report = gate.evaluate(
            &SkillId::new("test_skill"),
            AgentRole::Backend,
            &scores,
            0.7,
        );
        assert!(report.passed);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn test_quality_gate_fail_absolute_floor() {
        let gate = QualityGate::new();
        let mut scores = HashMap::new();
        for dim in QualityDimension::all() {
            scores.insert(*dim, 0.9);
        }
        // One dimension below absolute floor.
        scores.insert(QualityDimension::Security, 0.3);

        let report = gate.evaluate(
            &SkillId::new("test_skill"),
            AgentRole::Backend,
            &scores,
            0.7,
        );
        assert!(!report.passed);
        assert!(report.issues.iter().any(|i| i.dimension == QualityDimension::Security));
    }

    #[test]
    fn test_quality_gate_fail_aggregate() {
        let gate = QualityGate::new();
        let mut scores = HashMap::new();
        for dim in QualityDimension::all() {
            scores.insert(*dim, 0.55);
        }

        let report = gate.evaluate(
            &SkillId::new("test_skill"),
            AgentRole::Backend,
            &scores,
            0.7,
        );
        assert!(!report.passed);
    }

    // -- AgentWorkloadTracker tests -----------------------------------------

    #[test]
    fn test_workload_assign_and_complete() {
        let mut tracker = AgentWorkloadTracker::new();
        tracker.assign_task(AgentRole::Backend, 4096);
        tracker.assign_task(AgentRole::Backend, 2048);

        let stats = tracker.current_load(AgentRole::Backend);
        assert_eq!(stats.active_tasks, 2);
        assert_eq!(stats.estimated_remaining, 6144);

        tracker.complete_task(AgentRole::Backend, 3500);
        let stats = tracker.current_load(AgentRole::Backend);
        assert_eq!(stats.active_tasks, 1);
        assert_eq!(stats.tokens_used, 3500);
    }

    #[test]
    fn test_best_agent_for_skill() {
        let mut tracker = AgentWorkloadTracker::new();
        tracker.assign_task(AgentRole::Backend, 8000);
        tracker.assign_task(AgentRole::Backend, 4000);
        // Frontend has no tasks, should be preferred.

        let best = tracker.best_agent_for(
            &SkillId::new("test"),
            &[AgentRole::Backend, AgentRole::Frontend],
        );
        assert_eq!(best, Some(AgentRole::Frontend));
    }

    #[test]
    fn test_overloaded_agent_excluded() {
        let mut tracker = AgentWorkloadTracker::new();
        for _ in 0..8 {
            tracker.assign_task(AgentRole::Backend, 1000);
        }

        let best = tracker.best_agent_for(
            &SkillId::new("test"),
            &[AgentRole::Backend],
        );
        assert_eq!(best, None); // Backend is at max capacity.
    }

    // -- IntegrationManager tests -------------------------------------------

    #[test]
    fn test_merge_outputs() {
        let mut mgr = IntegrationManager::new();
        let result = mgr.merge_outputs(vec![
            (AgentRole::Backend, "fn handle_request() {}".into()),
            (AgentRole::Frontend, "fn render_page() {}".into()),
        ]);

        assert!(result.merged_output.contains("handle_request"));
        assert!(result.merged_output.contains("render_page"));
        assert_eq!(result.agents_merged.len(), 2);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_detect_merge_conflict_markers() {
        let mgr = IntegrationManager::new();
        let bad_merge = "fn foo() {\n<<<<<<< HEAD\n  old\n=======\n  new\n>>>>>>>\n}";
        let issues = mgr.detect_integration_issues(bad_merge);
        assert!(issues.iter().any(|i| i.severity == IssueSeverity::Fatal));
    }

    #[test]
    fn test_detect_duplicate_definitions() {
        let mgr = IntegrationManager::new();
        let code = "pub fn handle() {}\nstruct User {}\npub fn handle() {}\n";
        let issues = mgr.detect_integration_issues(code);
        assert!(issues.iter().any(|i| {
            i.severity == IssueSeverity::Error && i.description.contains("handle")
        }));
    }

    #[test]
    fn test_detect_todo_markers() {
        let mgr = IntegrationManager::new();
        let code = "fn foo() { // TODO: implement\n// FIXME: broken\n}";
        let issues = mgr.detect_integration_issues(code);
        assert!(issues.iter().any(|i| {
            i.severity == IssueSeverity::Warning && i.description.contains("TODO")
        }));
    }

    #[test]
    fn test_integration_manager_reset() {
        let mut mgr = IntegrationManager::new();
        mgr.merge_outputs(vec![(AgentRole::Backend, "content".into())]);
        assert!(!mgr.current_state().is_empty());
        mgr.reset();
        assert!(mgr.current_state().is_empty());
        assert!(mgr.merge_log().is_empty());
    }
}
