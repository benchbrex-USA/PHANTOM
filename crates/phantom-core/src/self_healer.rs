//! 5-layer self-healing engine.
//!
//! Core Law 7: Self-healing at every layer.
//!
//! Layer 1: RETRY         (80% of failures) — exponential backoff, 5 attempts
//! Layer 2: ALTERNATIVE   (10%) — different tool, provider, or approach
//! Layer 3: DECOMPOSE     (5%) — split complex task into smaller pieces
//! Layer 4: ESCALATE      (3%) — ask another agent for help
//! Layer 5: PAUSE & ALERT (2%) — save state, ask owner, resume on reply

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Self-healing layers, in escalation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealingLayer {
    /// Retry with exponential backoff (80% of failures resolve here)
    Retry,
    /// Try an alternative approach/tool/provider (10%)
    Alternative,
    /// Decompose the task into smaller sub-tasks (5%)
    Decompose,
    /// Escalate to another agent for help (3%)
    Escalate,
    /// Pause and alert the owner (2%)
    PauseAndAlert,
}

impl HealingLayer {
    /// Get the next escalation layer.
    pub fn next(&self) -> Option<HealingLayer> {
        match self {
            Self::Retry => Some(Self::Alternative),
            Self::Alternative => Some(Self::Decompose),
            Self::Decompose => Some(Self::Escalate),
            Self::Escalate => Some(Self::PauseAndAlert),
            Self::PauseAndAlert => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Retry => "retry",
            Self::Alternative => "alternative",
            Self::Decompose => "decompose",
            Self::Escalate => "escalate",
            Self::PauseAndAlert => "pause_and_alert",
        }
    }
}

impl std::fmt::Display for HealingLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Result of a healing attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingResult {
    /// Which layer handled the failure
    pub layer: HealingLayer,
    /// Whether healing was successful
    pub success: bool,
    /// Description of what was done
    pub action: String,
    /// Number of attempts at this layer
    pub attempts: u32,
    /// If decomposed, the sub-task IDs created
    pub sub_tasks: Vec<String>,
    /// If escalated, which agent was asked
    pub escalated_to: Option<String>,
    /// If paused, whether owner has been notified
    pub owner_notified: bool,
}

/// Configuration for the self-healing engine.
#[derive(Debug, Clone)]
pub struct HealingConfig {
    /// Maximum retry attempts (Layer 1)
    pub max_retries: u32,
    /// Base delay for exponential backoff (milliseconds)
    pub retry_base_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub retry_max_delay_ms: u64,
    /// Maximum alternative approaches to try (Layer 2)
    pub max_alternatives: u32,
    /// Maximum decomposition depth (Layer 3)
    pub max_decompose_depth: u32,
}

impl Default for HealingConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            retry_base_delay_ms: 1000,
            retry_max_delay_ms: 30_000,
            max_alternatives: 3,
            max_decompose_depth: 2,
        }
    }
}

/// The self-healing engine — manages failure recovery across all 5 layers.
pub struct SelfHealer {
    config: HealingConfig,
}

impl Default for SelfHealer {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfHealer {
    pub fn new() -> Self {
        Self {
            config: HealingConfig::default(),
        }
    }

    pub fn with_config(config: HealingConfig) -> Self {
        Self { config }
    }

    /// Calculate exponential backoff delay for a given attempt.
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.config.retry_base_delay_ms * 2u64.pow(attempt.min(10));
        let capped = delay_ms.min(self.config.retry_max_delay_ms);
        Duration::from_millis(capped)
    }

    /// Determine which healing layer should handle a failure.
    pub fn determine_layer(&self, retry_count: u32, error: &str) -> HealingLayer {
        // Layer 1: Retry — transient errors, timeouts, rate limits
        if retry_count < self.config.max_retries && is_retryable_error(error) {
            return HealingLayer::Retry;
        }

        // Layer 2: Alternative — tool/provider failures
        if is_alternative_possible(error) {
            return HealingLayer::Alternative;
        }

        // Layer 3: Decompose — complex task failures
        if is_decomposable_error(error) {
            return HealingLayer::Decompose;
        }

        // Layer 4: Escalate — need help from another agent
        if retry_count >= self.config.max_retries {
            return HealingLayer::Escalate;
        }

        // Layer 5: Pause & Alert — nothing else worked
        HealingLayer::PauseAndAlert
    }

    /// Create a healing result for a retry attempt.
    pub fn create_retry_result(&self, attempt: u32, success: bool) -> HealingResult {
        let delay = self.backoff_delay(attempt);
        let action = format!(
            "retry attempt {}/{} (backoff: {:?})",
            attempt + 1,
            self.config.max_retries,
            delay
        );

        debug!(
            attempt,
            delay_ms = delay.as_millis(),
            success,
            "retry healing"
        );

        HealingResult {
            layer: HealingLayer::Retry,
            success,
            action,
            attempts: attempt + 1,
            sub_tasks: Vec::new(),
            escalated_to: None,
            owner_notified: false,
        }
    }

    /// Create a healing result for an alternative approach.
    pub fn create_alternative_result(&self, alternative: &str, success: bool) -> HealingResult {
        info!(alternative, success, "alternative approach attempted");

        HealingResult {
            layer: HealingLayer::Alternative,
            success,
            action: format!("tried alternative: {}", alternative),
            attempts: 1,
            sub_tasks: Vec::new(),
            escalated_to: None,
            owner_notified: false,
        }
    }

    /// Create a healing result for task decomposition.
    pub fn create_decompose_result(&self, sub_task_ids: Vec<String>) -> HealingResult {
        info!(sub_task_count = sub_task_ids.len(), "task decomposed");

        HealingResult {
            layer: HealingLayer::Decompose,
            success: true,
            action: format!("decomposed into {} sub-tasks", sub_task_ids.len()),
            attempts: 1,
            sub_tasks: sub_task_ids,
            escalated_to: None,
            owner_notified: false,
        }
    }

    /// Create a healing result for escalation.
    pub fn create_escalation_result(&self, target_agent: &str, success: bool) -> HealingResult {
        warn!(target_agent, success, "escalated to another agent");

        HealingResult {
            layer: HealingLayer::Escalate,
            success,
            action: format!("escalated to {}", target_agent),
            attempts: 1,
            sub_tasks: Vec::new(),
            escalated_to: Some(target_agent.to_string()),
            owner_notified: false,
        }
    }

    /// Create a healing result for pause & alert.
    pub fn create_pause_result(&self, reason: &str) -> HealingResult {
        warn!(reason, "pausing and alerting owner");

        HealingResult {
            layer: HealingLayer::PauseAndAlert,
            success: false,
            action: format!("paused: {}", reason),
            attempts: 1,
            sub_tasks: Vec::new(),
            escalated_to: None,
            owner_notified: true,
        }
    }

    /// Check if we've exhausted all healing options.
    pub fn is_exhausted(&self, retry_count: u32, layers_tried: &[HealingLayer]) -> bool {
        retry_count >= self.config.max_retries
            && layers_tried.contains(&HealingLayer::PauseAndAlert)
    }

    pub fn config(&self) -> &HealingConfig {
        &self.config
    }
}

/// Check if an error is likely transient and retryable.
fn is_retryable_error(error: &str) -> bool {
    let retryable_patterns = [
        "timeout",
        "timed out",
        "rate limit",
        "429",
        "503",
        "502",
        "connection reset",
        "connection refused",
        "network",
        "temporary",
        "EAGAIN",
        "ECONNRESET",
    ];

    let lower = error.to_lowercase();
    retryable_patterns
        .iter()
        .any(|p| lower.contains(&p.to_lowercase()))
}

/// Check if an alternative approach might work.
fn is_alternative_possible(error: &str) -> bool {
    let alternative_patterns = [
        "not supported",
        "deprecated",
        "provider unavailable",
        "tool not found",
        "command not found",
        "permission denied",
    ];

    let lower = error.to_lowercase();
    alternative_patterns
        .iter()
        .any(|p| lower.contains(&p.to_lowercase()))
}

/// Check if the task can be decomposed.
fn is_decomposable_error(error: &str) -> bool {
    let decompose_patterns = [
        "too complex",
        "context overflow",
        "token limit",
        "out of memory",
        "too large",
    ];

    let lower = error.to_lowercase();
    decompose_patterns
        .iter()
        .any(|p| lower.contains(&p.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healing_layer_escalation() {
        assert_eq!(HealingLayer::Retry.next(), Some(HealingLayer::Alternative));
        assert_eq!(
            HealingLayer::Alternative.next(),
            Some(HealingLayer::Decompose)
        );
        assert_eq!(HealingLayer::Decompose.next(), Some(HealingLayer::Escalate));
        assert_eq!(
            HealingLayer::Escalate.next(),
            Some(HealingLayer::PauseAndAlert)
        );
        assert_eq!(HealingLayer::PauseAndAlert.next(), None);
    }

    #[test]
    fn test_exponential_backoff() {
        let healer = SelfHealer::new();
        let d0 = healer.backoff_delay(0);
        let d1 = healer.backoff_delay(1);
        let d2 = healer.backoff_delay(2);

        assert_eq!(d0, Duration::from_millis(1000));
        assert_eq!(d1, Duration::from_millis(2000));
        assert_eq!(d2, Duration::from_millis(4000));
    }

    #[test]
    fn test_backoff_capped() {
        let healer = SelfHealer::new();
        let d_max = healer.backoff_delay(20); // Very high attempt
        assert!(d_max <= Duration::from_millis(30_000));
    }

    #[test]
    fn test_retryable_errors() {
        assert!(is_retryable_error("connection timeout after 30s"));
        assert!(is_retryable_error("HTTP 429 rate limit exceeded"));
        assert!(is_retryable_error("503 Service Unavailable"));
        assert!(!is_retryable_error("syntax error in code"));
        assert!(!is_retryable_error("file not found"));
    }

    #[test]
    fn test_alternative_errors() {
        assert!(is_alternative_possible(
            "provider unavailable: Oracle Cloud"
        ));
        assert!(is_alternative_possible("command not found: npm"));
        assert!(!is_alternative_possible("timeout"));
    }

    #[test]
    fn test_decomposable_errors() {
        assert!(is_decomposable_error("context overflow: 200K tokens"));
        assert!(is_decomposable_error("task too complex for single agent"));
        assert!(!is_decomposable_error("permission denied"));
    }

    #[test]
    fn test_determine_layer_retry() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "connection timeout");
        assert_eq!(layer, HealingLayer::Retry);
    }

    #[test]
    fn test_determine_layer_alternative() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "provider unavailable");
        assert_eq!(layer, HealingLayer::Alternative);
    }

    #[test]
    fn test_determine_layer_decompose() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(0, "context overflow");
        assert_eq!(layer, HealingLayer::Decompose);
    }

    #[test]
    fn test_determine_layer_escalate() {
        let healer = SelfHealer::new();
        let layer = healer.determine_layer(5, "unknown persistent error");
        assert_eq!(layer, HealingLayer::Escalate);
    }

    #[test]
    fn test_is_exhausted() {
        let healer = SelfHealer::new();
        assert!(!healer.is_exhausted(0, &[]));
        assert!(!healer.is_exhausted(5, &[HealingLayer::Retry]));
        assert!(healer.is_exhausted(5, &[HealingLayer::Retry, HealingLayer::PauseAndAlert]));
    }

    #[test]
    fn test_healing_results() {
        let healer = SelfHealer::new();

        let retry = healer.create_retry_result(2, true);
        assert_eq!(retry.layer, HealingLayer::Retry);
        assert!(retry.success);

        let alt = healer.create_alternative_result("use npm instead of yarn", false);
        assert_eq!(alt.layer, HealingLayer::Alternative);

        let decomp = healer.create_decompose_result(vec!["sub1".into(), "sub2".into()]);
        assert_eq!(decomp.sub_tasks.len(), 2);

        let esc = healer.create_escalation_result("cto", true);
        assert_eq!(esc.escalated_to.as_deref(), Some("cto"));

        let pause = healer.create_pause_result("need API key");
        assert!(pause.owner_notified);
    }
}
