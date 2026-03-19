//! Autonomous Build Pipeline — Spec to Production.
//!
//! 8 phases from Architecture Framework §13:
//!   Phase 0: INGEST     (5 min)   — Parse framework, plan
//!   Phase 1: INFRA      (15-30m)  — Provision servers, create accounts, setup CI/CD
//!   Phase 2: ARCH       (15 min)  — System design, DB schema, API contracts, ADRs
//!   Phase 3: CODE       (1-3h)    — 4 parallel streams: Backend + Frontend + DevOps + Docs
//!   Phase 4: TEST       (30-60m)  — Unit + integration + E2E, 80%+ coverage
//!   Phase 5: SECURITY   (15-30m)  — Dependency audit, OWASP, auth review, secret scan
//!   Phase 6: DEPLOY     (15-30m)  — Push → CI → Docker → deploy → DNS → TLS → health
//!   Phase 7: DELIVER    (5 min)   — Report, URLs, credentials, architecture log

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::task_graph::TaskGraph;

/// Build phases in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildPhase {
    Ingest,
    Infrastructure,
    Architecture,
    Code,
    Test,
    Security,
    Deploy,
    Deliver,
}

impl BuildPhase {
    /// All phases in order.
    pub fn all() -> &'static [BuildPhase] {
        &[
            Self::Ingest,
            Self::Infrastructure,
            Self::Architecture,
            Self::Code,
            Self::Test,
            Self::Security,
            Self::Deploy,
            Self::Deliver,
        ]
    }

    /// Get the next phase.
    pub fn next(&self) -> Option<BuildPhase> {
        let all = Self::all();
        let pos = all.iter().position(|p| p == self)?;
        all.get(pos + 1).copied()
    }

    /// Estimated duration in seconds.
    pub fn estimated_seconds(&self) -> u32 {
        match self {
            Self::Ingest => 300,         // 5 min
            Self::Infrastructure => 1500, // 25 min avg
            Self::Architecture => 900,    // 15 min
            Self::Code => 7200,           // 2h avg
            Self::Test => 2700,           // 45 min avg
            Self::Security => 1350,       // 22 min avg
            Self::Deploy => 1350,         // 22 min avg
            Self::Deliver => 300,         // 5 min
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Ingest => "Phase 0: Ingest",
            Self::Infrastructure => "Phase 1: Infrastructure",
            Self::Architecture => "Phase 2: Architecture",
            Self::Code => "Phase 3: Code",
            Self::Test => "Phase 4: Test",
            Self::Security => "Phase 5: Security",
            Self::Deploy => "Phase 6: Deploy",
            Self::Deliver => "Phase 7: Deliver",
        }
    }
}

impl std::fmt::Display for BuildPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Status of a build phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseStatus {
    pub phase: BuildPhase,
    pub status: PhaseState,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub tasks_total: usize,
    pub tasks_completed: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseState {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// The autonomous build pipeline.
pub struct BuildPipeline {
    /// Path to the architecture framework file
    pub framework_path: Option<String>,
    /// Current phase
    pub current_phase: Option<BuildPhase>,
    /// Phase statuses
    pub phases: Vec<PhaseStatus>,
    /// The task graph for this build
    pub task_graph: TaskGraph,
    /// When the build started
    pub started_at: Option<DateTime<Utc>>,
    /// When the build completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Whether the build was halted
    pub halted: bool,
}

impl BuildPipeline {
    /// Create a new build pipeline.
    pub fn new(framework_path: Option<String>) -> Self {
        let phases = BuildPhase::all()
            .iter()
            .map(|&phase| PhaseStatus {
                phase,
                status: PhaseState::Pending,
                started_at: None,
                completed_at: None,
                tasks_total: 0,
                tasks_completed: 0,
                error: None,
            })
            .collect();

        Self {
            framework_path,
            current_phase: None,
            phases,
            task_graph: TaskGraph::new(),
            started_at: None,
            completed_at: None,
            halted: false,
        }
    }

    /// Start the pipeline.
    pub fn start(&mut self) {
        self.started_at = Some(Utc::now());
        self.advance_to_phase(BuildPhase::Ingest);
    }

    /// Advance to a specific phase.
    pub fn advance_to_phase(&mut self, phase: BuildPhase) {
        self.current_phase = Some(phase);
        if let Some(ps) = self.phases.iter_mut().find(|p| p.phase == phase) {
            ps.status = PhaseState::Running;
            ps.started_at = Some(Utc::now());
        }
    }

    /// Mark the current phase as completed and advance to the next.
    pub fn complete_current_phase(&mut self) -> Option<BuildPhase> {
        if let Some(current) = self.current_phase {
            if let Some(ps) = self.phases.iter_mut().find(|p| p.phase == current) {
                ps.status = PhaseState::Completed;
                ps.completed_at = Some(Utc::now());
            }

            if let Some(next) = current.next() {
                self.advance_to_phase(next);
                return Some(next);
            } else {
                // All phases complete
                self.current_phase = None;
                self.completed_at = Some(Utc::now());
                return None;
            }
        }
        None
    }

    /// Mark the current phase as failed.
    pub fn fail_current_phase(&mut self, error: impl Into<String>) {
        if let Some(current) = self.current_phase {
            if let Some(ps) = self.phases.iter_mut().find(|p| p.phase == current) {
                ps.status = PhaseState::Failed;
                ps.completed_at = Some(Utc::now());
                ps.error = Some(error.into());
            }
        }
    }

    /// Halt the pipeline.
    pub fn halt(&mut self) {
        self.halted = true;
        self.task_graph.cancel_all();
    }

    /// Check if the pipeline is complete.
    pub fn is_complete(&self) -> bool {
        self.completed_at.is_some()
    }

    /// Check if the pipeline has failed.
    pub fn is_failed(&self) -> bool {
        self.phases.iter().any(|p| p.status == PhaseState::Failed)
    }

    /// Total elapsed time in seconds.
    pub fn elapsed_seconds(&self) -> f64 {
        self.started_at
            .map(|start| {
                let end = self.completed_at.unwrap_or_else(Utc::now);
                (end - start).num_milliseconds() as f64 / 1000.0
            })
            .unwrap_or(0.0)
    }

    /// Total estimated time in seconds.
    pub fn total_estimated_seconds(&self) -> u32 {
        BuildPhase::all().iter().map(|p| p.estimated_seconds()).sum()
    }

    /// Get the status of a specific phase.
    pub fn phase_status(&self, phase: BuildPhase) -> Option<&PhaseStatus> {
        self.phases.iter().find(|p| p.phase == phase)
    }

    /// Get a summary of completed phases.
    pub fn completed_phases(&self) -> Vec<BuildPhase> {
        self.phases
            .iter()
            .filter(|p| p.status == PhaseState::Completed)
            .map(|p| p.phase)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_phase_ordering() {
        let phases = BuildPhase::all();
        assert_eq!(phases.len(), 8);
        assert_eq!(phases[0], BuildPhase::Ingest);
        assert_eq!(phases[7], BuildPhase::Deliver);
    }

    #[test]
    fn test_phase_next() {
        assert_eq!(BuildPhase::Ingest.next(), Some(BuildPhase::Infrastructure));
        assert_eq!(BuildPhase::Deploy.next(), Some(BuildPhase::Deliver));
        assert_eq!(BuildPhase::Deliver.next(), None);
    }

    #[test]
    fn test_pipeline_lifecycle() {
        let mut pipeline = BuildPipeline::new(Some("./framework.md".into()));

        pipeline.start();
        assert_eq!(pipeline.current_phase, Some(BuildPhase::Ingest));

        let next = pipeline.complete_current_phase();
        assert_eq!(next, Some(BuildPhase::Infrastructure));

        // Complete all remaining phases
        while pipeline.complete_current_phase().is_some() {}

        assert!(pipeline.is_complete());
        assert!(!pipeline.is_failed());
        assert_eq!(pipeline.completed_phases().len(), 8);
    }

    #[test]
    fn test_pipeline_failure() {
        let mut pipeline = BuildPipeline::new(None);
        pipeline.start();
        pipeline.fail_current_phase("dependency installation failed");

        assert!(pipeline.is_failed());
    }

    #[test]
    fn test_pipeline_halt() {
        let mut pipeline = BuildPipeline::new(None);
        pipeline.start();
        pipeline.halt();

        assert!(pipeline.halted);
    }

    #[test]
    fn test_total_estimated_time() {
        let pipeline = BuildPipeline::new(None);
        let total = pipeline.total_estimated_seconds();
        // Should be roughly 4-5 hours in seconds
        assert!(total > 10000);
        assert!(total < 20000);
    }
}
