//! Resilience and fault-tolerance skills for the Phantom autonomous AI engineering system.
//!
//! Covers circuit breakers, retry strategies, bulkhead isolation, timeout management,
//! graceful degradation, health checks, load balancing, fault injection, data
//! replication, and auto-recovery.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all resilience skills into the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(circuit_breaker_pattern());
    registry.register(retry_with_backoff());
    registry.register(bulkhead_isolation());
    registry.register(timeout_management());
    registry.register(graceful_degradation());
    registry.register(health_check_framework());
    registry.register(load_balancing_strategy());
    registry.register(fault_injection());
    registry.register(data_replication());
    registry.register(auto_recovery());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn circuit_breaker_pattern() -> Skill {
    Skill::new(
        "circuit_breaker_pattern",
        "Circuit Breaker Pattern",
        "Implements the circuit breaker pattern with closed/open/half-open states, \
         configurable failure thresholds, timeout-based transitions, fallback \
         responses, and per-dependency isolation.",
        SkillCategory::Resilience,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build a circuit breaker that transitions between closed (normal), open \
         (failing fast), and half-open (probing) states. In the closed state, track \
         consecutive failures and error rate over a sliding window. Trip to open when \
         either threshold is exceeded. In the open state, reject all calls immediately \
         with a fallback response and start a configurable cooldown timer. On timer \
         expiry, transition to half-open and allow a limited number of probe requests. \
         If probes succeed, reset to closed; if any probe fails, return to open with \
         an extended cooldown. Each downstream dependency gets its own circuit breaker \
         instance to prevent one failing service from tripping breakers for healthy \
         services. Emit state transition events for dashboards and alerting. The \
         fallback must be configurable per call site: cached response, default value, \
         or graceful error.",
    )
    .with_quality_threshold(0.85)
}

fn retry_with_backoff() -> Skill {
    Skill::new(
        "retry_with_backoff",
        "Retry with Exponential Backoff",
        "Generates retry logic with exponential backoff, jitter, deadline awareness, \
         retry budgets, circuit breaker integration, and per-exception retry filters.",
        SkillCategory::Resilience,
        SkillComplexity::Atomic,
        vec![AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(15_000)
    .with_system_prompt(
        "Implement a retry policy that uses exponential backoff with full jitter \
         (random delay between 0 and the exponential cap) to prevent thundering herd. \
         Enforce a deadline: if the remaining time budget is less than the next \
         backoff interval, fail immediately instead of sleeping. Maintain a retry \
         budget as a token bucket (e.g., 10 retries per 10 seconds across all callers) \
         to prevent retry storms during widespread outages. Only retry on retryable \
         errors (network timeout, 503, connection reset) and immediately propagate \
         non-retryable errors (400, 401, 404). Integrate with the circuit breaker: \
         count retries toward the failure threshold so that a dependency requiring \
         constant retries eventually trips the breaker. Log each retry attempt with \
         attempt number, delay, and error for observability.",
    )
    .with_quality_threshold(0.85)
}

fn bulkhead_isolation() -> Skill {
    Skill::new(
        "bulkhead_isolation",
        "Bulkhead Isolation Pattern",
        "Creates resource isolation using semaphores, thread pool partitioning, and \
         queue depth limits per downstream dependency to prevent cascade failures.",
        SkillCategory::Resilience,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Implement bulkhead isolation that partitions resources per downstream \
         dependency. Each bulkhead has a concurrency semaphore (max in-flight \
         requests), a queue with bounded depth for requests awaiting a permit, \
         and a queue timeout after which waiting requests are rejected. Thread pool \
         bulkheads use dedicated pools so a slow dependency cannot consume all \
         application threads. Semaphore bulkheads share the application thread pool \
         but limit concurrency with async-aware permits. Configure limits based on \
         the dependency's expected latency and throughput: fast dependencies get \
         fewer permits (they release quickly), slow dependencies get more queue \
         depth. Emit metrics on permit utilization, queue depth, and rejection \
         rate per bulkhead. Integrate with the circuit breaker so that sustained \
         high rejection rates trip the breaker.",
    )
    .with_quality_threshold(0.85)
}

fn timeout_management() -> Skill {
    Skill::new(
        "timeout_management",
        "Cascading Timeout Management",
        "Implements cascading timeout budgets with deadline propagation across \
         service boundaries, partial result assembly, and timeout hierarchy \
         configuration.",
        SkillCategory::Resilience,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Design a timeout system where every inbound request carries a deadline \
         propagated through all downstream calls. The top-level handler sets the \
         initial deadline from the client timeout or a server-side maximum. Each \
         outbound call subtracts elapsed time and overhead budget from the remaining \
         deadline before setting its timeout. If the remaining deadline is too short \
         for a downstream call, skip it and use a fallback or partial result. \
         Propagate deadlines via request headers (e.g., grpc-timeout, x-deadline-ms) \
         across service boundaries. For fan-out patterns, use the minimum remaining \
         deadline across all parallel calls. Assemble partial results when some \
         downstream calls complete but others timeout, clearly marking which data \
         is missing. Log timeout events with the full call chain for latency \
         debugging.",
    )
    .with_quality_threshold(0.85)
}

fn graceful_degradation() -> Skill {
    Skill::new(
        "graceful_degradation",
        "Graceful Degradation Strategy",
        "Generates graceful degradation logic with feature priority levels, static \
         fallback content, load-based feature shedding, and user-tier-aware \
         degradation policies.",
        SkillCategory::Resilience,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Implement graceful degradation where features are classified into priority \
         tiers: critical (authentication, core transactions), important (search, \
         recommendations), and optional (analytics, personalization). Under load \
         pressure or dependency failures, shed features from the lowest tier first. \
         Each degraded feature has a static fallback: cached response, default \
         content, or simplified logic that avoids the failing dependency. Define \
         degradation triggers based on error rate thresholds, latency percentiles, \
         or system resource utilization. User-tier-aware policies ensure premium \
         users are the last to experience degradation. Expose a degradation status \
         API that returns which features are currently degraded and why, for both \
         internal dashboards and client-side adaptive UI. All degradation state \
         changes must be logged with timestamp, trigger reason, and affected features.",
    )
    .with_quality_threshold(0.85)
}

fn health_check_framework() -> Skill {
    Skill::new(
        "health_check_framework",
        "Deep Health Check Framework",
        "Creates a comprehensive health check system with dependency tree traversal, \
         degraded state reporting, liveness vs readiness separation, and self-healing \
         trigger integration.",
        SkillCategory::Resilience,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::DevOps, AgentRole::Monitor],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build a health check framework that distinguishes liveness (process is \
         running, not deadlocked) from readiness (can serve traffic, all critical \
         dependencies reachable). Deep health checks traverse the dependency tree: \
         database connectivity with a test query, cache ping, external API probe, \
         disk space, memory usage. Each dependency reports healthy, degraded, or \
         unhealthy with latency and optional diagnostic message. Aggregate into an \
         overall status: healthy if all critical deps are healthy, degraded if any \
         non-critical dep is unhealthy, unhealthy if any critical dep is unhealthy. \
         Expose standard endpoints (/healthz for liveness, /readyz for readiness, \
         /health/deep for full report). Health check results are cached for a short \
         TTL to avoid thundering herd on health endpoints. Unhealthy states trigger \
         self-healing actions (connection pool reset, cache flush, restart) via \
         configurable hooks.",
    )
    .with_quality_threshold(0.85)
}

fn load_balancing_strategy() -> Skill {
    Skill::new(
        "load_balancing_strategy",
        "Client-Side Load Balancing",
        "Implements client-side load balancing with power-of-two-choices, weighted \
         round-robin, health-aware routing, and adaptive algorithms that respond \
         to latency and error signals.",
        SkillCategory::Resilience,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Implement client-side load balancing behind a pluggable strategy interface. \
         Power-of-two-choices (P2C) picks two random backends and routes to the one \
         with fewer in-flight requests, providing near-optimal load distribution with \
         minimal coordination. Weighted round-robin assigns traffic proportional to \
         configured weights, useful for canary deployments. Health-aware routing \
         removes backends that fail health checks from the rotation and re-adds them \
         after recovery with a warm-up period of reduced traffic. Adaptive algorithms \
         adjust weights based on observed P99 latency and error rates using an EWMA \
         (exponentially weighted moving average) with configurable decay. The load \
         balancer must support service discovery integration (DNS SRV, Consul, K8s \
         endpoints) with background refresh. Emit per-backend metrics: request count, \
         latency histogram, error rate, and current in-flight count.",
    )
    .with_quality_threshold(0.85)
}

fn fault_injection() -> Skill {
    Skill::new(
        "fault_injection",
        "Fault Injection / Chaos Engineering",
        "Creates a chaos engineering framework with configurable failure injection, \
         latency injection, error rate simulation, kill switch for safety, and \
         experiment tracking with blast radius control.",
        SkillCategory::Resilience,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::DevOps, AgentRole::Qa],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Build a fault injection framework where experiments are defined as \
         configurations specifying the target (service, endpoint, dependency), fault \
         type (exception, timeout, latency, HTTP error code, connection reset), \
         injection rate (0-100%), duration, and scope (specific users, percentage \
         of traffic, specific regions). A middleware intercepts requests and applies \
         faults based on active experiments. Include a kill switch that immediately \
         disables all active experiments when triggered via API, CLI, or automated \
         alert. Blast radius controls limit the maximum percentage of traffic \
         affected and automatically abort experiments if error rates exceed a safety \
         threshold. Track experiment results: baseline vs experiment error rates, \
         latency distributions, and user impact. Integrate with the observability \
         stack to correlate fault injection periods with metric anomalies.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn data_replication() -> Skill {
    Skill::new(
        "data_replication",
        "Multi-Region Data Replication",
        "Implements multi-region data replication with configurable consistency levels, \
         conflict resolution strategies, replication lag monitoring, and failover \
         routing.",
        SkillCategory::Resilience,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Architect, AgentRole::DevOps],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Design a data replication system that synchronizes data across regions with \
         configurable consistency: strong (synchronous replication, higher latency), \
         eventual (asynchronous, lower latency), and bounded staleness (reads \
         guaranteed within N seconds). Conflict resolution supports last-writer-wins \
         (timestamp-based), application-specific merge functions, and CRDT-based \
         automatic resolution for supported data types (counters, sets, LWW registers). \
         Monitor replication lag per region with alerting when lag exceeds SLA. \
         Implement read routing that directs reads to the nearest replica when eventual \
         consistency is acceptable and to the primary when strong consistency is \
         required. Failover promotes a replica to primary when the primary region is \
         unreachable, with automatic client redirection. Include a reconciliation job \
         that detects and repairs drift between replicas on a schedule.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 10_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.90)
}

fn auto_recovery() -> Skill {
    Skill::new(
        "auto_recovery",
        "Automatic Recovery & Self-Healing",
        "Creates self-healing infrastructure with watchdog processes, restart policies, \
         state recovery from checkpoints, notification on recovery actions, and \
         escalation when auto-recovery fails.",
        SkillCategory::Resilience,
        SkillComplexity::Orchestrated,
        vec![AgentRole::DevOps, AgentRole::Backend, AgentRole::Monitor],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Implement an auto-recovery system with a watchdog that monitors process \
         health via heartbeats, resource utilization, and application-level health \
         checks. Restart policies define per-service behavior: always restart, restart \
         on failure only, restart with exponential backoff (max 3 attempts in 5 \
         minutes before giving up). State recovery loads the last checkpoint on \
         restart so in-flight work resumes from a known-good point rather than \
         replaying from the beginning. Checkpoints are persisted to durable storage \
         with atomic writes and garbage collection of old checkpoints. Send \
         notifications (Slack, PagerDuty) on every recovery action with context: \
         which service, failure reason, recovery action taken, and current attempt \
         count. If auto-recovery exhausts all attempts, escalate to on-call with a \
         detailed diagnostic bundle (logs, metrics, heap dump if applicable). Include \
         a dead-man's switch that alerts if the watchdog itself stops reporting.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}
