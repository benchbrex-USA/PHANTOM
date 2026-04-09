//! Observability skills for PHANTOM's autonomous AI engineering system.
//!
//! Covers distributed tracing, structured logging, metrics instrumentation,
//! alerting, dashboards, error tracking, synthetic monitoring, anomaly
//! detection, capacity forecasting, health checks, audit logging, session
//! replay, RUM, SLOs, and cost observability.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all observability skills into the provided registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(distributed_tracing());
    registry.register(structured_logging());
    registry.register(metrics_instrumentation());
    registry.register(alerting_rules());
    registry.register(dashboard_generator());
    registry.register(error_tracking());
    registry.register(synthetic_monitoring());
    registry.register(anomaly_detection());
    registry.register(capacity_forecasting());
    registry.register(health_check_endpoints());
    registry.register(audit_logging());
    registry.register(user_session_replay());
    registry.register(real_user_monitoring());
    registry.register(service_level_objectives());
    registry.register(cost_observability());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn distributed_tracing() -> Skill {
    Skill::new(
        "distributed_tracing",
        "Distributed Tracing",
        "Instrument services with OpenTelemetry distributed tracing including context \
         propagation, span attributes, trace sampling strategies, and baggage handling.",
        SkillCategory::Observability,
        SkillComplexity::Composite,
        vec![AgentRole::Monitor, AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are an observability engineer specializing in distributed tracing. \
         Generate OpenTelemetry-based tracing instrumentation that includes:\n\
         - Trace context propagation across service boundaries (W3C TraceContext, B3)\n\
         - Meaningful span names following semantic conventions (e.g. `http.server`, `db.query`)\n\
         - Span attributes: status codes, user IDs, request paths, error details\n\
         - Sampling strategies: head-based (probabilistic, rate-limiting) and tail-based \
           (error-biased, latency-biased)\n\
         - Baggage propagation for cross-cutting concerns (tenant ID, feature flags)\n\
         - Span links for asynchronous workflows and batch processing\n\
         - Resource attributes for service identity (service.name, service.version, deployment.environment)\n\
         Ensure traces are structured for efficient querying in Jaeger, Tempo, or Honeycomb.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.8)
}

fn structured_logging() -> Skill {
    Skill::new(
        "structured_logging",
        "Structured Logging",
        "Implement structured JSON logging with correlation IDs, configurable log levels, \
         and automatic sensitive data masking.",
        SkillCategory::Observability,
        SkillComplexity::Atomic,
        vec![AgentRole::Monitor, AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a logging infrastructure expert. Generate structured logging configurations \
         and middleware that include:\n\
         - JSON-formatted log output with consistent field names (timestamp, level, message, \
           service, trace_id, span_id)\n\
         - Correlation ID injection from incoming request headers or generated at ingress\n\
         - Log level configuration per module/package with runtime reconfiguration support\n\
         - Sensitive data masking: PII fields (email, SSN, credit card), auth tokens, \
           passwords redacted before emission\n\
         - Context enrichment: automatic inclusion of request metadata, user context, \
           deployment version\n\
         - Log sampling for high-throughput paths to control volume without losing visibility\n\
         - Integration with log aggregators (ELK, Loki, CloudWatch Logs) via standard output \
           or direct shipping",
    )
    .with_quality_threshold(0.8)
}

fn metrics_instrumentation() -> Skill {
    Skill::new(
        "metrics_instrumentation",
        "Metrics Instrumentation",
        "Instrument applications with Prometheus and StatsD metrics including counters, \
         gauges, histograms, and SLI/SLO definitions.",
        SkillCategory::Observability,
        SkillComplexity::Composite,
        vec![AgentRole::Monitor, AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a metrics instrumentation specialist. Generate metric definitions and \
         instrumentation code that includes:\n\
         - Counter metrics for request counts, error counts, events processed\n\
         - Gauge metrics for active connections, queue depth, cache size, in-flight requests\n\
         - Histogram metrics with appropriate bucket boundaries for latency distributions \
           (p50, p90, p99)\n\
         - Summary metrics for sliding-window quantile estimation where histograms are impractical\n\
         - SLI definitions: availability (successful requests / total), latency (requests < threshold), \
           throughput, correctness\n\
         - SLO targets with error budget calculations and burn-rate windows\n\
         - Metric naming conventions following Prometheus best practices (unit suffixes, \
           snake_case, meaningful labels)\n\
         - Cardinality management: avoid unbounded label values, use label allow-lists",
    )
    .with_quality_threshold(0.8)
}

fn alerting_rules() -> Skill {
    Skill::new(
        "alerting_rules",
        "Alerting Rules",
        "Define alert rules with severity levels, escalation policies, deduplication, \
         silence windows, and runbook links.",
        SkillCategory::Observability,
        SkillComplexity::Composite,
        vec![AgentRole::Monitor, AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are an alerting and incident response engineer. Generate alerting rule \
         configurations that include:\n\
         - Severity classification: P1 (critical, immediate page), P2 (high, 15-min response), \
           P3 (medium, business hours), P4 (low, informational)\n\
         - Multi-window, multi-burn-rate alerts for SLO-based alerting (fast burn 1h/5m, \
           slow burn 6h/30m)\n\
         - Escalation policies: primary on-call -> secondary -> engineering manager -> VP with \
           configurable timeouts\n\
         - Deduplication keys to prevent alert storms (group by service + alert name + environment)\n\
         - Silence and maintenance windows with scheduled inhibition rules\n\
         - Runbook links embedded in every alert with structured context (impact, likely cause, \
           remediation steps)\n\
         - Alert dependencies and inhibition: suppress downstream alerts when upstream is firing\n\
         - Notification routing: PagerDuty for P1/P2, Slack for P3/P4, email digests for trends",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn dashboard_generator() -> Skill {
    Skill::new(
        "dashboard_generator",
        "Dashboard Generator",
        "Generate Grafana and Datadog dashboard definitions with panels, template variables, \
         annotations, and drill-down links.",
        SkillCategory::Observability,
        SkillComplexity::Composite,
        vec![AgentRole::Monitor, AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a dashboard design expert for Grafana and Datadog. Generate dashboard JSON \
         or Terraform definitions that include:\n\
         - USE method panels (Utilization, Saturation, Errors) for infrastructure resources\n\
         - RED method panels (Rate, Errors, Duration) for service-level views\n\
         - Template variables for environment, service, region, and time range selection\n\
         - Annotation layers for deployments, incidents, and configuration changes\n\
         - Drill-down links from high-level overview panels to detailed service dashboards\n\
         - Consistent color schemes and thresholds (green/yellow/red with documented boundaries)\n\
         - Row organization: overview at top, then grouped by subsystem\n\
         - Panel types: time series, stat, gauge, table, heatmap, logs panel as appropriate\n\
         - Alert integration: panels linked to corresponding alert rules with state indicators",
    )
    .with_quality_threshold(0.75)
}

fn error_tracking() -> Skill {
    Skill::new(
        "error_tracking",
        "Error Tracking",
        "Integrate error tracking via Sentry or Bugsnag with intelligent grouping, \
         breadcrumbs, release tracking, and source map support.",
        SkillCategory::Observability,
        SkillComplexity::Composite,
        vec![AgentRole::Monitor, AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are an error tracking integration specialist. Generate error tracking \
         configurations and instrumentation that include:\n\
         - SDK initialization with DSN, environment tagging, and release version binding\n\
         - Custom fingerprinting rules for intelligent error grouping beyond stack trace similarity\n\
         - Breadcrumb capture: HTTP requests, console logs, user interactions, navigation events\n\
         - Release tracking: associate errors with specific deployments, track regression rates\n\
         - Source map upload integration for minified JavaScript stack traces\n\
         - User context attachment: anonymized user ID, subscription tier, feature flags\n\
         - Performance transaction capture alongside errors for correlated debugging\n\
         - Rate limiting and sampling configuration to control event volume in production\n\
         - Integration with issue trackers (Jira, Linear) for automated ticket creation",
    )
    .with_quality_threshold(0.8)
}

fn synthetic_monitoring() -> Skill {
    Skill::new(
        "synthetic_monitoring",
        "Synthetic Monitoring",
        "Configure synthetic monitoring checks (HTTP, browser, API) with multi-region \
         execution, SLA tracking, and waterfall analysis.",
        SkillCategory::Observability,
        SkillComplexity::Composite,
        vec![AgentRole::Monitor, AgentRole::Qa],
        OutputFormat::Config,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a synthetic monitoring engineer. Generate synthetic check definitions \
         that include:\n\
         - HTTP checks: status code validation, response body assertions, header checks, \
           TLS certificate expiry monitoring\n\
         - Browser checks: multi-step user flows (login, checkout, search), screenshot on \
           failure, DOM assertions\n\
         - API checks: GraphQL and REST endpoint validation, schema conformance, response \
           time thresholds\n\
         - Multi-region execution from geographically distributed probes (US-East, EU-West, \
           AP-Southeast) with per-region SLA tracking\n\
         - Waterfall analysis: DNS resolution, TCP connect, TLS handshake, TTFB breakdown\n\
         - Alerting integration: notify on consecutive failures (not single flaps)\n\
         - SLA tracking: monthly uptime percentage, downtime duration, incident count\n\
         - Maintenance window awareness: suppress false positives during planned outages",
    )
    .with_quality_threshold(0.8)
}

fn anomaly_detection() -> Skill {
    Skill::new(
        "anomaly_detection",
        "Anomaly Detection",
        "Implement statistical anomaly detection on time-series metrics with seasonal \
         decomposition and dynamic thresholds.",
        SkillCategory::Observability,
        SkillComplexity::Pipeline,
        vec![AgentRole::Monitor],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a time-series analysis and anomaly detection engineer. Generate anomaly \
         detection pipelines that include:\n\
         - Seasonal decomposition (STL) to separate trend, seasonal, and residual components\n\
         - Dynamic thresholds using rolling statistics (mean +/- N*sigma with configurable N)\n\
         - EWMA (Exponentially Weighted Moving Average) for smoothing noisy signals\n\
         - Percentage-change detection for sudden spikes or drops relative to baseline\n\
         - Multi-metric correlation: detect anomalies that span related metrics simultaneously\n\
         - Suppression of known patterns: deploy windows, batch job schedules, maintenance\n\
         - Confidence scoring: classify anomalies as high/medium/low confidence with reasoning\n\
         - Feedback loop integration: mark false positives to improve future detection accuracy",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn capacity_forecasting() -> Skill {
    Skill::new(
        "capacity_forecasting",
        "Capacity Forecasting",
        "Forecast resource capacity requirements using trend analysis, seasonal patterns, \
         and growth projection models.",
        SkillCategory::Observability,
        SkillComplexity::Pipeline,
        vec![AgentRole::Monitor, AgentRole::DevOps],
        OutputFormat::Report,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a capacity planning engineer. Generate capacity forecasting models and \
         reports that include:\n\
         - Trend analysis: linear regression and polynomial fitting on resource utilization \
           over 30/60/90 day windows\n\
         - Seasonal pattern recognition: daily (business hours vs off-hours), weekly (weekday \
           vs weekend), monthly (billing cycles), yearly (holiday peaks)\n\
         - Growth projection: extrapolate current trends with confidence intervals (p50, p80, p95)\n\
         - Resource exhaustion prediction: estimated date when CPU, memory, disk, or network \
           will exceed capacity thresholds (70%, 85%, 95%)\n\
         - What-if scenarios: model impact of new features, customer onboarding, traffic campaigns\n\
         - Cost projection: map resource forecasts to infrastructure cost estimates\n\
         - Recommendations: right-sizing, reserved instance purchases, auto-scaling policy tuning\n\
         - Report format: executive summary, detailed metrics, charts specification, action items",
    )
    .with_quality_threshold(0.8)
}

fn health_check_endpoints() -> Skill {
    Skill::new(
        "health_check_endpoints",
        "Health Check Endpoints",
        "Implement liveness, readiness, and startup probes with dependency checks and \
         degraded state reporting.",
        SkillCategory::Observability,
        SkillComplexity::Atomic,
        vec![AgentRole::Monitor, AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(3072)
    .with_system_prompt(
        "You are a service reliability engineer. Generate health check endpoint \
         implementations that include:\n\
         - Liveness probe (/healthz): confirms the process is alive and not deadlocked, \
           minimal dependencies checked\n\
         - Readiness probe (/readyz): confirms the service can accept traffic, checks \
           database connectivity, cache availability, downstream service reachability\n\
         - Startup probe (/startupz): confirms initial bootstrap is complete (migrations \
           run, caches warmed, config loaded)\n\
         - Dependency health matrix: individual status for each downstream dependency with \
           latency and last-check timestamp\n\
         - Degraded state reporting: service is UP but operating with reduced capability \
           (e.g., cache miss fallback, read-only mode)\n\
         - Response format: structured JSON with status, checks array, version, uptime\n\
         - Caching: health check results cached briefly (1-5s) to prevent thundering herd \
           from orchestrator polling",
    )
    .with_quality_threshold(0.85)
}

fn audit_logging() -> Skill {
    Skill::new(
        "audit_logging",
        "Audit Logging",
        "Implement immutable audit logging with who/what/when/where capture, compliance \
         queries, and configurable retention management.",
        SkillCategory::Observability,
        SkillComplexity::Composite,
        vec![AgentRole::Monitor, AgentRole::Security],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a compliance and audit logging engineer. Generate audit logging \
         implementations that include:\n\
         - Immutable append-only log entries with cryptographic chaining (hash of previous entry)\n\
         - Structured fields: actor (who), action (what), timestamp (when), resource (where), \
           outcome (success/failure), IP address, user agent\n\
         - Before/after snapshots for mutation operations to enable change history reconstruction\n\
         - Compliance query interfaces: filter by actor, action type, resource, time range\n\
         - Retention policies: configurable per regulation (SOC2: 1 year, HIPAA: 6 years, \
           GDPR: as short as possible)\n\
         - Tamper detection: periodic integrity verification against stored hashes\n\
         - Export formats: CSV, JSON, PDF for compliance auditors\n\
         - Integration with SIEM systems (Splunk, Sentinel) for real-time compliance monitoring",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.9)
}

fn user_session_replay() -> Skill {
    Skill::new(
        "user_session_replay",
        "User Session Replay",
        "Implement session replay with privacy controls, frustration signal detection, \
         and conversion funnel analysis.",
        SkillCategory::Observability,
        SkillComplexity::Pipeline,
        vec![AgentRole::Monitor, AgentRole::Frontend],
        OutputFormat::Config,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a user experience observability engineer. Generate session replay \
         configurations that include:\n\
         - DOM snapshot recording with mutation observer for incremental updates\n\
         - Privacy controls: mask all input fields by default, configurable allow-lists, \
           block-lists for sensitive selectors, exclude elements with [data-private]\n\
         - Frustration signal detection: rage clicks (3+ clicks in <1s on same element), \
           dead clicks (click with no DOM response), error-preceded abandonment\n\
         - Conversion funnel tagging: mark key user journey steps, track drop-off points, \
           calculate per-step completion rates\n\
         - Network request correlation: link session replay timeline to XHR/fetch waterfall\n\
         - Console error overlay: surface JavaScript errors inline with replay timeline\n\
         - Sampling configuration: 100% for error sessions, configurable percentage for normal\n\
         - Storage optimization: compress recordings, set max session duration, prune idle segments",
    )
    .with_quality_threshold(0.75)
}

fn real_user_monitoring() -> Skill {
    Skill::new(
        "real_user_monitoring",
        "Real User Monitoring",
        "Implement RUM with Core Web Vitals tracking, page load waterfalls, resource \
         timing, and long task detection.",
        SkillCategory::Observability,
        SkillComplexity::Composite,
        vec![AgentRole::Monitor, AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a real user monitoring specialist. Generate RUM instrumentation that \
         includes:\n\
         - Core Web Vitals collection: LCP (Largest Contentful Paint), INP (Interaction to \
           Next Paint), CLS (Cumulative Layout Shift) with attribution\n\
         - Page load waterfall: Navigation Timing API breakdown (DNS, TCP, TLS, TTFB, \
           content download, DOM processing)\n\
         - Resource timing: track individual asset load times, identify render-blocking \
           resources, flag slow third-party scripts\n\
         - Long task detection: PerformanceObserver for tasks >50ms with attribution to \
           script source and function\n\
         - User segmentation: slice metrics by device type, connection speed, geography, \
           browser, OS\n\
         - SPA route change tracking: measure virtual page transitions, not just initial load\n\
         - Percentile aggregation: report p50, p75, p90, p99 for all timing metrics\n\
         - Beacon transport: use Navigator.sendBeacon for reliable delivery on page unload",
    )
    .with_quality_threshold(0.8)
}

fn service_level_objectives() -> Skill {
    Skill::new(
        "service_level_objectives",
        "Service Level Objectives",
        "Define SLOs with error budget tracking, burn rate alerting, and SLO-based \
         release gating policies.",
        SkillCategory::Observability,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Monitor, AgentRole::Cto],
        OutputFormat::Config,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are an SRE specializing in service level objectives. Generate SLO definitions \
         and policies that include:\n\
         - SLI selection: availability (successful requests / total), latency (requests served \
           < threshold), correctness (valid responses / total), freshness (data age < threshold)\n\
         - SLO targets: tiered objectives (99.9% for critical paths, 99.5% for non-critical) \
           with documented rationale\n\
         - Error budget calculation: remaining budget = 1 - (1 - SLO) over rolling window \
           (28-day default)\n\
         - Burn rate alerts: fast burn (14.4x in 1h, pages immediately), slow burn (3x in 3d, \
           creates ticket)\n\
         - Multi-window alerting: short window for immediacy, long window for significance\n\
         - Release gating: block deployments when error budget is <10% remaining unless \
           override approved by SRE\n\
         - SLO review cadence: weekly burn-rate review, monthly SLO appropriateness review\n\
         - Stakeholder reporting: executive dashboard with budget consumption trends and \
           projected exhaustion dates",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.9)
}

fn cost_observability() -> Skill {
    Skill::new(
        "cost_observability",
        "Cost Observability",
        "Implement per-request cost attribution, cost-per-customer analysis, and \
         infrastructure cost allocation across services.",
        SkillCategory::Observability,
        SkillComplexity::Pipeline,
        vec![AgentRole::Monitor, AgentRole::DevOps],
        OutputFormat::Report,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a FinOps and cost observability engineer. Generate cost attribution \
         systems that include:\n\
         - Per-request cost estimation: compute cost based on CPU-time, memory-seconds, \
           I/O operations, and egress bytes consumed per request\n\
         - Cost-per-customer breakdown: aggregate request costs by tenant/customer ID, \
           identify top consumers, flag anomalous cost spikes\n\
         - Infrastructure cost allocation: tag cloud resources to services and teams using \
           consistent tagging taxonomy\n\
         - Unit economics tracking: cost-per-transaction, cost-per-API-call, cost-per-GB-stored\n\
         - Cost anomaly detection: alert when daily/weekly spend deviates >20% from rolling \
           average baseline\n\
         - Showback/chargeback reports: per-team cost reports with drill-down to individual \
           resources and services\n\
         - Optimization recommendations: identify idle resources, oversized instances, \
           unattached volumes, unused reserved capacity\n\
         - Budget tracking: forecast monthly spend against budget, alert at 80% and 100% thresholds",
    )
    .with_quality_threshold(0.8)
}
