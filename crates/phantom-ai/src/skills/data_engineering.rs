//! Data engineering skills for the Phantom autonomous AI engineering system.
//!
//! Covers ETL/ELT pipelines, stream processing, lakehouse architecture, data
//! quality, cataloging, feature stores, orchestration, real-time analytics,
//! migration, and anonymization.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all data-engineering skills into the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(etl_pipeline());
    registry.register(stream_processing());
    registry.register(data_lakehouse());
    registry.register(data_quality_framework());
    registry.register(data_catalog());
    registry.register(feature_store());
    registry.register(data_pipeline_orchestrator());
    registry.register(real_time_analytics());
    registry.register(data_migration());
    registry.register(data_anonymization());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn etl_pipeline() -> Skill {
    Skill::new(
        "etl_pipeline",
        "ETL/ELT Pipeline",
        "Generates production ETL/ELT pipelines with configurable extraction from \
         heterogeneous sources (databases, APIs, files), transformation logic with \
         schema evolution handling, incremental/full loading strategies, cron-based \
         and event-driven scheduling, pipeline monitoring, and data quality gates.",
        SkillCategory::DataEngineering,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Design ETL/ELT pipelines that cleanly separate extraction, transformation, \
         and loading concerns. Extractors must support incremental reads via watermarks \
         or CDC cursors. Transformations should be pure functions that map source schemas \
         to target schemas with explicit handling of schema drift (new columns, type \
         changes, dropped fields). Loaders must implement idempotent upserts to prevent \
         duplicates on retry. Include a scheduling layer with cron expressions and \
         event triggers, a monitoring sidecar that emits row counts, latency, and error \
         rates per stage, and data quality checks between each phase that halt the \
         pipeline on threshold violations. Prefer partitioned staging tables over \
         in-memory buffering for large volumes.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn stream_processing() -> Skill {
    Skill::new(
        "stream_processing",
        "Stream Processing Pipeline",
        "Builds real-time stream processing systems on Kafka, Pulsar, or NATS with \
         windowed aggregations, exactly-once semantics, dead-letter queues, back-pressure \
         handling, and consumer group management.",
        SkillCategory::DataEngineering,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(35_000)
    .with_system_prompt(
        "Implement stream processing topologies with explicit windowing strategies \
         (tumbling, sliding, session) and watermark-based late-arrival handling. \
         Ensure exactly-once semantics through idempotent consumers with transactional \
         offset commits. Wire dead-letter queues for poison pills with structured error \
         metadata. Implement back-pressure via consumer pause/resume rather than \
         unbounded buffering. Include consumer group rebalancing hooks, partition \
         assignment logging, and lag monitoring. Serialization must use schema-registry \
         backed Avro or Protobuf with forward/backward compatibility checks. Provide \
         graceful shutdown that drains in-flight messages before committing final offsets.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 3_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn data_lakehouse() -> Skill {
    Skill::new(
        "data_lakehouse",
        "Data Lakehouse Architecture",
        "Designs lakehouse architecture using Delta Lake or Apache Iceberg with \
         table partitioning, compaction policies, time-travel queries, schema evolution, \
         and unified batch/streaming ingestion.",
        SkillCategory::DataEngineering,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Architect a lakehouse that unifies batch and streaming on a single table \
         format (Delta Lake or Iceberg). Define partition schemes aligned to query \
         patterns (date-based for time-series, hash for point lookups). Implement \
         automatic compaction that merges small files on a schedule without blocking \
         readers. Enable time-travel with configurable retention (default 30 days) \
         and expose snapshot IDs in query APIs. Handle schema evolution through column \
         mapping so renames and drops are metadata-only. Include a medallion \
         architecture (bronze/silver/gold) with clear promotion criteria, and wire \
         table maintenance operations (vacuum, optimize, analyze) as scheduled tasks.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn data_quality_framework() -> Skill {
    Skill::new(
        "data_quality_framework",
        "Data Quality Framework",
        "Creates a Great Expectations-style data validation framework with declarative \
         expectations, automatic profiling, quality scoring, alerting on threshold \
         breaches, and historical quality trend tracking.",
        SkillCategory::DataEngineering,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Qa],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Build a data quality framework where expectations are declared as composable \
         rules (not_null, unique, range, regex, referential integrity, statistical \
         distribution). The profiler must automatically generate baseline expectations \
         from a sample dataset. Each validation run produces a typed result with \
         pass/fail per expectation, observed vs expected values, and an aggregate \
         quality score (0.0-1.0). Integrate alerting that fires on Slack/PagerDuty \
         when scores drop below configurable thresholds. Store validation history in \
         a time-series format for trend dashboards. Expectations must be versioned \
         alongside the schema they validate and executable in both batch and streaming \
         contexts.",
    )
    .with_quality_threshold(0.85)
}

fn data_catalog() -> Skill {
    Skill::new(
        "data_catalog",
        "Data Catalog & Lineage",
        "Implements a data catalog with automated metadata harvesting, column-level \
         lineage tracking, schema registry integration, search/discovery, access \
         control policies, and data classification tagging.",
        SkillCategory::DataEngineering,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::Backend, AgentRole::Security],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Design a data catalog that automatically harvests metadata from databases, \
         object stores, streaming topics, and API endpoints. Track column-level lineage \
         by parsing transformation SQL/code to build a directed acyclic graph from \
         source to consumption. Integrate with a schema registry to surface schema \
         versions, compatibility status, and breaking change alerts. Implement \
         full-text and faceted search over table names, column names, descriptions, \
         tags, and owners. Enforce access control with row-level and column-level \
         policies mapped to IAM roles. Support data classification tags (PII, PHI, \
         financial, public) that automatically propagate downstream through lineage.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 3_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.80)
}

fn feature_store() -> Skill {
    Skill::new(
        "feature_store",
        "ML Feature Store",
        "Builds an ML feature store with dual online/offline serving paths, feature \
         computation pipelines, point-in-time correct joins, feature versioning, \
         monitoring for drift, and discovery APIs.",
        SkillCategory::DataEngineering,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Implement a feature store with separate online (low-latency key-value) and \
         offline (columnar batch) serving paths backed by appropriate storage engines. \
         Feature definitions must be declared once and materialized to both paths. \
         Batch pipelines compute features from warehouse tables; streaming pipelines \
         update the online store from event streams. Ensure point-in-time correctness \
         by joining features as-of the label timestamp, preventing future data leakage. \
         Version features with semantic versioning and support concurrent versions for \
         A/B model comparison. Monitor feature distributions for drift using PSI or \
         KL-divergence with configurable alert thresholds. Expose a discovery API \
         so data scientists can search, preview, and reuse existing features.",
    )
    .with_quality_threshold(0.85)
}

fn data_pipeline_orchestrator() -> Skill {
    Skill::new(
        "data_pipeline_orchestrator",
        "Data Pipeline Orchestrator",
        "Generates DAG-based pipeline orchestration (Airflow/Dagster/Prefect style) \
         with dependency resolution, retry policies, backfill support, data-aware \
         scheduling, SLA monitoring, and dynamic task generation.",
        SkillCategory::DataEngineering,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Build a DAG orchestrator where tasks declare explicit data dependencies \
         (not just execution order). The scheduler must resolve dependencies \
         topologically, parallelize independent branches, and enforce concurrency \
         limits per resource pool. Retry policies are per-task with exponential \
         backoff, max attempts, and retry-on-specific-exception filters. Backfill \
         runs must replay historical partitions without re-triggering downstream \
         consumers already up-to-date. Implement data-aware scheduling that triggers \
         downstream DAGs only when upstream partitions land (sensor pattern). Track \
         SLA deadlines per DAG with escalation alerts. Support dynamic task generation \
         at parse time for fan-out patterns like per-tenant or per-partition processing.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn real_time_analytics() -> Skill {
    Skill::new(
        "real_time_analytics",
        "Real-Time Analytics Engine",
        "Creates a real-time analytics system with materialized views, pre-aggregated \
         rollups, incremental computation, approximate distinct counts, and sub-second \
         query latency for dashboards.",
        SkillCategory::DataEngineering,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Design a real-time analytics layer that maintains materialized views updated \
         incrementally from streaming ingestion. Pre-aggregate common rollups (hourly, \
         daily, by dimension) at write time to avoid full scans at query time. Use \
         HyperLogLog for approximate distinct counts and quantile sketches (t-digest \
         or DDSketch) for percentile queries. Partition materialized views by time so \
         old partitions can be frozen and compacted. Query routing must check freshness \
         and transparently merge the materialized result with a real-time tail scan of \
         un-aggregated events. Target sub-second P99 for dashboard queries over the \
         most recent 24 hours of data.",
    )
    .with_quality_threshold(0.80)
}

fn data_migration() -> Skill {
    Skill::new(
        "data_migration",
        "Cross-System Data Migration",
        "Implements cross-system data migration with schema mapping, validation, \
         reconciliation counts, checkpointed progress, rollback capability, and \
         zero-downtime cutover strategies.",
        SkillCategory::DataEngineering,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Build a data migration framework that maps source schemas to target schemas \
         with explicit transformation rules per column. Validate every row against \
         target constraints before writing, quarantining failures to an error table \
         with the original row and failure reason. Track migration progress via \
         checkpoints (last migrated primary key / offset) so interrupted runs resume \
         without re-processing. Provide reconciliation queries that compare row counts, \
         checksums, and sample-based value comparisons between source and target. \
         Implement rollback by maintaining a reverse-migration script that undoes \
         schema changes and deletes migrated rows. For zero-downtime cutovers, use \
         dual-write with a synchronization lag monitor.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 10_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.90)
}

fn data_anonymization() -> Skill {
    Skill::new(
        "data_anonymization",
        "Data Anonymization & Masking",
        "Generates data anonymization pipelines with deterministic masking, format-\
         preserving tokenization, k-anonymity enforcement, differential privacy noise \
         injection, and automated PII detection for non-production environments.",
        SkillCategory::DataEngineering,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Security],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Implement data anonymization that applies per-column strategies based on \
         data classification. Use deterministic masking (HMAC-based) so the same \
         input always maps to the same output, preserving referential integrity \
         across tables. Format-preserving encryption must maintain string length and \
         character class (e.g., phone numbers stay phone-shaped). Enforce k-anonymity \
         by generalizing quasi-identifiers until each equivalence class has at least \
         k records. For analytics datasets, add calibrated Laplace noise for \
         differential privacy with a configurable epsilon budget. Automatically detect \
         PII columns via regex patterns and NER classification before any data leaves \
         production. Generate a lineage record mapping original columns to their \
         anonymization strategy for audit purposes.",
    )
    .with_quality_threshold(0.90)
}
