//! Database skills.
//!
//! Schema design, migration planning, query optimization, sharding, replication,
//! time-series/graph/vector stores, CDC pipelines, connection pool tuning,
//! partitioning, schema evolution, polyglot persistence, observability, and
//! backup/recovery strategies.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all database skills with the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(schema_design());
    registry.register(migration_planner());
    registry.register(query_optimizer());
    registry.register(database_sharding());
    registry.register(read_replica_setup());
    registry.register(time_series_schema());
    registry.register(graph_database_model());
    registry.register(vector_database_setup());
    registry.register(cdc_pipeline());
    registry.register(connection_pool_optimizer());
    registry.register(data_partitioning());
    registry.register(schema_evolution());
    registry.register(multi_model_database());
    registry.register(database_observability());
    registry.register(backup_recovery_strategy());
}

// ---------------------------------------------------------------------------
// Individual skill constructors
// ---------------------------------------------------------------------------

fn schema_design() -> Skill {
    Skill::new(
        "db_schema_design",
        "Schema Design",
        "Produce a normalized or strategically denormalized database schema from a \
         domain model, including indexes, constraints, partitioning strategy, and \
         storage estimates.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a database architect designing a production schema.\n\n\
         DELIVERABLES:\n\
         1. **Table Definitions** -- CREATE TABLE DDL with column types, NOT NULL constraints, \
            DEFAULT values, and CHECK constraints.\n\
         2. **Primary & Foreign Keys** -- composite keys only where semantically justified; \
            prefer surrogate keys with natural-key unique constraints.\n\
         3. **Index Strategy** -- B-tree, GIN, GiST, or BRIN indexes per query pattern; \
            partial indexes where selective; covering indexes for hot queries.\n\
         4. **Normalization Level** -- target 3NF minimum; document every intentional \
            denormalization with the read-pattern justification.\n\
         5. **Partitioning Plan** -- partition large tables by range (time) or hash (tenant); \
            include partition maintenance (creation, detach, archival).\n\
         6. **Storage Estimates** -- row size, expected row count at 1yr and 3yr, total \
            storage projection including indexes.\n\n\
         Use PostgreSQL syntax by default. Flag any vendor-specific extensions explicitly.",
    )
    .with_quality_threshold(0.85)
}

fn migration_planner() -> Skill {
    Skill::new(
        "db_migration_planner",
        "Migration Planner",
        "Plan a safe database migration with rollback procedures, zero-downtime \
         strategy, data backfill, and validation queries.",
        SkillCategory::Database,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Migration,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a database reliability engineer planning a zero-downtime migration.\n\n\
         DELIVERABLES:\n\
         1. **Migration Steps** -- ordered DDL/DML statements, each in its own transaction \
            where possible; flag statements that acquire ACCESS EXCLUSIVE locks.\n\
         2. **Zero-Downtime Strategy** -- expand-then-contract pattern: add new columns/tables \
            first, dual-write, backfill, cut reads, drop old.\n\
         3. **Backfill Plan** -- batched UPDATE/INSERT with configurable batch size, rate \
            limiting, and progress tracking; estimated wall-clock time.\n\
         4. **Rollback Procedure** -- for each forward step, a corresponding reverse step; \
            clearly mark any point-of-no-return.\n\
         5. **Validation Queries** -- pre-migration and post-migration consistency checks \
            (row counts, checksum aggregates, constraint verification).\n\
         6. **Lock Analysis** -- expected lock duration per statement; flag any statement \
            that could block reads/writes longer than 1 second.\n\n\
         Every migration must be idempotent. Use IF EXISTS / IF NOT EXISTS guards.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 500,
        fallback_skill: None,
    })
}

fn query_optimizer() -> Skill {
    Skill::new(
        "db_query_optimizer",
        "Query Optimizer",
        "Analyze slow queries, interpret EXPLAIN plans, recommend indexes, and \
         rewrite queries for optimal performance.",
        SkillCategory::Database,
        SkillComplexity::Atomic,
        vec![AgentRole::Backend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a database performance specialist optimizing SQL queries.\n\n\
         DELIVERABLES:\n\
         1. **EXPLAIN Analysis** -- interpret the provided EXPLAIN (ANALYZE, BUFFERS) output; \
            identify sequential scans, nested loops, hash joins, sort spills, and their costs.\n\
         2. **Bottleneck Identification** -- pinpoint the most expensive node in the plan; \
            estimate I/O vs CPU cost split.\n\
         3. **Index Recommendations** -- suggest indexes (including composite, partial, \
            covering) that eliminate sequential scans; include CREATE INDEX statements.\n\
         4. **Query Rewrite** -- rewrite the query for better plan selection (CTE \
            materialization control, join reordering hints, subquery flattening).\n\
         5. **Statistics Check** -- verify table statistics are current; recommend \
            ANALYZE or adjusted statistics targets for skewed columns.\n\
         6. **Performance Projection** -- estimated improvement (rows scanned reduction, \
            buffer hit ratio improvement, latency reduction).\n\n\
         Always show before/after EXPLAIN plans. Warn about index write amplification trade-offs.",
    )
    .with_quality_threshold(0.80)
}

fn database_sharding() -> Skill {
    Skill::new(
        "db_database_sharding",
        "Database Sharding",
        "Design a sharding strategy with shard key selection, consistent hashing, \
         cross-shard query patterns, and rebalancing procedures.",
        SkillCategory::Database,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a distributed database architect designing a sharding strategy.\n\n\
         DELIVERABLES:\n\
         1. **Shard Key Selection** -- candidate keys evaluated on cardinality, distribution \
            uniformity, query affinity, and growth pattern; final recommendation with rationale.\n\
         2. **Hashing Strategy** -- consistent hashing (virtual nodes), range-based, or \
            directory-based; trade-offs for each, final choice.\n\
         3. **Shard Topology** -- number of initial shards, expected shard size, when to split; \
            shard-to-node mapping.\n\
         4. **Cross-Shard Queries** -- scatter-gather pattern, fan-out limits, aggregation \
            strategy; identify queries that must be avoided or redesigned.\n\
         5. **Rebalancing Procedure** -- online resharding with backfill, dual-read during \
            migration, consistency verification after rebalance.\n\
         6. **Routing Layer** -- application-level or proxy-level shard routing; connection \
            management per shard; failover behavior.\n\n\
         Warn about hotspot risk. Provide a monitoring plan for shard size skew detection.",
    )
    .with_quality_threshold(0.85)
}

fn read_replica_setup() -> Skill {
    Skill::new(
        "db_read_replica_setup",
        "Read Replica Setup",
        "Design read replica topology with replication lag monitoring, automatic \
         failover, and intelligent connection routing.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a database infrastructure engineer setting up read replicas.\n\n\
         DELIVERABLES:\n\
         1. **Replica Topology** -- number of replicas, region placement, synchronous vs \
            asynchronous replication per replica; cascading replication if applicable.\n\
         2. **Lag Monitoring** -- replication lag metric collection, acceptable lag thresholds \
            per use case (e.g., <1s for dashboards, <100ms for read-after-write).\n\
         3. **Connection Routing** -- read/write splitting at application or proxy layer \
            (PgBouncer, ProxySQL, application-level); sticky reads after writes.\n\
         4. **Failover Strategy** -- automatic promotion criteria, DNS/endpoint switchover, \
            client reconnection handling, split-brain prevention.\n\
         5. **Replica Health Checks** -- query-based health probes, automatic removal of \
            unhealthy replicas from the read pool.\n\
         6. **Capacity Planning** -- when to add replicas based on read throughput and \
            CPU utilization; cost vs performance trade-off.\n\n\
         Specify the target database engine (PostgreSQL, MySQL) and cloud provider \
         managed vs self-hosted considerations.",
    )
    .with_quality_threshold(0.80)
}

fn time_series_schema() -> Skill {
    Skill::new(
        "db_time_series_schema",
        "Time-Series Schema",
        "Design a time-series optimized schema with retention policies, \
         downsampling, continuous aggregates, and efficient range queries.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a time-series data specialist designing an optimized storage schema.\n\n\
         DELIVERABLES:\n\
         1. **Hypertable / Partition Design** -- time-based partitioning (chunk interval), \
            space partitioning (device_id, tenant), compression policy per chunk age.\n\
         2. **Schema** -- table DDL optimized for append-heavy writes; column ordering for \
            compression ratio; appropriate data types (timestamptz, real vs double).\n\
         3. **Retention Policy** -- raw data retention window, automated drop/detach of \
            expired chunks, archival to cold storage (S3/GCS).\n\
         4. **Continuous Aggregates** -- pre-materialized rollups (1-min, 1-hr, 1-day) with \
            refresh policies; query routing to aggregates for dashboard queries.\n\
         5. **Downsampling Pipeline** -- configurable resolution reduction for historical data; \
            LTTB or average-based algorithms.\n\
         6. **Query Patterns** -- optimized queries for latest-value, range scans, \
            percentile calculations, and gap filling.\n\n\
         Default to TimescaleDB on PostgreSQL. Note InfluxDB / ClickHouse alternatives \
         where they outperform.",
    )
    .with_quality_threshold(0.80)
}

fn graph_database_model() -> Skill {
    Skill::new(
        "db_graph_database_model",
        "Graph Database Model",
        "Design a property graph model with traversal patterns, index strategies, \
         and query optimization for Neo4j or Dgraph.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a graph database specialist designing a property graph model.\n\n\
         DELIVERABLES:\n\
         1. **Node Labels & Properties** -- each node type with its properties, data types, \
            and uniqueness constraints.\n\
         2. **Relationship Types** -- directed relationships with cardinality (1:1, 1:N, M:N), \
            properties on relationships where meaningful.\n\
         3. **Index Strategy** -- native indexes on lookup properties, full-text indexes for \
            search, composite indexes for multi-property lookups.\n\
         4. **Traversal Patterns** -- common query patterns as Cypher/DQL queries; depth limits \
            for variable-length paths; BFS vs DFS considerations.\n\
         5. **Performance Optimization** -- query profiling, index hints, relationship direction \
            alignment with traversal direction, supernodes mitigation.\n\
         6. **Data Import** -- bulk import strategy (CSV, APOC, admin import) with relationship \
            resolution and deduplication.\n\n\
         Default to Neo4j with Cypher. Note Dgraph/DQL where RDF triples or GraphQL-native \
         queries are advantageous.",
    )
    .with_quality_threshold(0.80)
}

fn vector_database_setup() -> Skill {
    Skill::new(
        "db_vector_database_setup",
        "Vector Database Setup",
        "Configure a vector store with embedding dimensions, similarity metrics, \
         HNSW index tuning, and hybrid search.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Config,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a vector search engineer configuring a vector database.\n\n\
         DELIVERABLES:\n\
         1. **Embedding Model Selection** -- model name, output dimensions, normalized vs \
            unnormalized vectors, quantization (float32/float16/int8).\n\
         2. **Similarity Metric** -- cosine, L2, inner product; justify based on embedding \
            model properties and use case.\n\
         3. **HNSW Tuning** -- M (max connections), ef_construction, ef_search parameters; \
            trade-off analysis between recall, latency, and memory.\n\
         4. **Collection Schema** -- vector field, metadata fields (filterable attributes), \
            payload storage strategy.\n\
         5. **Hybrid Search** -- combining vector similarity with keyword (BM25) or metadata \
            filters; re-ranking strategy (reciprocal rank fusion, cross-encoder).\n\
         6. **Scaling Plan** -- sharding by collection, replication for read throughput, \
            memory estimation per million vectors, disk-backed indexes for cost optimization.\n\n\
         Default to pgvector for PostgreSQL-native or Qdrant for standalone. Note Pinecone, \
         Weaviate, and Milvus alternatives with trade-offs.",
    )
    .with_quality_threshold(0.80)
}

fn cdc_pipeline() -> Skill {
    Skill::new(
        "db_cdc_pipeline",
        "CDC Pipeline",
        "Set up Change Data Capture with Debezium or native CDC, event transformation, \
         and sink connectors for downstream consumers.",
        SkillCategory::Database,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a data integration engineer designing a Change Data Capture pipeline.\n\n\
         DELIVERABLES:\n\
         1. **Source Connector Config** -- Debezium connector for the source database \
            (PostgreSQL WAL, MySQL binlog, MongoDB oplog); slot management, snapshot mode.\n\
         2. **Event Schema** -- CDC event envelope (before/after images, operation type, \
            source metadata, transaction info); schema registry integration (Avro/JSON Schema).\n\
         3. **Transformation Layer** -- Single Message Transforms (SMTs) or stream processor \
            (Kafka Streams, Flink) for filtering, enrichment, field renaming, tombstone handling.\n\
         4. **Sink Connectors** -- downstream targets (Elasticsearch, data warehouse, cache \
            invalidation, event bus) with delivery guarantees and batching config.\n\
         5. **Exactly-Once Semantics** -- Kafka transaction IDs, consumer offset management, \
            idempotent sink writes; known limitations per sink type.\n\
         6. **Operational Runbook** -- monitoring (lag, throughput, error rate), connector \
            restart procedures, schema evolution handling, slot cleanup.\n\n\
         Default to Debezium + Kafka Connect. Note alternatives (DynamoDB Streams, \
         Supabase Realtime, Prisma Pulse) where applicable.",
    )
    .with_quality_threshold(0.80)
}

fn connection_pool_optimizer() -> Skill {
    Skill::new(
        "db_connection_pool_optimizer",
        "Connection Pool Optimizer",
        "Optimize connection pool sizing, timeout tuning, health checks, and \
         connection lifecycle management.",
        SkillCategory::Database,
        SkillComplexity::Atomic,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a database performance engineer optimizing connection pools.\n\n\
         DELIVERABLES:\n\
         1. **Pool Size Calculation** -- formula based on: database max_connections, number \
            of application instances, expected concurrent queries, query duration P95; \
            apply Little's Law.\n\
         2. **Min/Max Idle** -- minimum idle connections for latency-sensitive paths; maximum \
            idle to avoid wasting database slots; idle timeout.\n\
         3. **Timeout Configuration** -- connection acquisition timeout, validation query \
            timeout, socket connect/read timeouts; values per environment (dev/staging/prod).\n\
         4. **Health Checks** -- validation query (SELECT 1), check interval, eviction policy \
            for stale connections, test-on-borrow vs background validation.\n\
         5. **Connection Lifecycle** -- max connection age, max uses per connection, graceful \
            drain on deployment; leak detection (log stack trace of unreturned connections).\n\
         6. **PgBouncer/ProxySQL** -- external pooler configuration as an alternative to \
            application-level pooling; transaction vs session vs statement pooling mode.\n\n\
         Provide concrete config snippets for HikariCP (Java), SQLx (Rust), or asyncpg (Python) \
         as appropriate.",
    )
    .with_quality_threshold(0.80)
}

fn data_partitioning() -> Skill {
    Skill::new(
        "db_data_partitioning",
        "Data Partitioning",
        "Design range, hash, or list partitioning with partition pruning, \
         maintenance automation, and query optimization.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a database architect designing a table partitioning strategy.\n\n\
         DELIVERABLES:\n\
         1. **Partitioning Method** -- range (time-based), hash (tenant/user), or list \
            (status/region); justify based on query patterns and data distribution.\n\
         2. **Partition DDL** -- CREATE TABLE ... PARTITION BY with child partition definitions; \
            default partition for unmatched values.\n\
         3. **Partition Pruning** -- demonstrate that common queries trigger partition pruning; \
            EXPLAIN output showing only relevant partitions scanned.\n\
         4. **Maintenance Automation** -- scheduled job to create future partitions (e.g., \
            monthly), detach and archive old partitions, VACUUM per partition.\n\
         5. **Index Strategy** -- local indexes per partition vs global indexes; unique \
            constraint implications across partitions.\n\
         6. **Migration Plan** -- how to partition an existing large table online (pg_partman, \
            CREATE TABLE AS, or logical replication approach).\n\n\
         Target PostgreSQL declarative partitioning. Flag limits (max partitions, join \
         performance with many partitions).",
    )
    .with_quality_threshold(0.80)
}

fn schema_evolution() -> Skill {
    Skill::new(
        "db_schema_evolution",
        "Schema Evolution",
        "Design backward and forward compatible schema changes using Avro, Protobuf, \
         or JSON Schema evolution rules.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a data contract engineer designing schema evolution rules.\n\n\
         DELIVERABLES:\n\
         1. **Compatibility Mode** -- BACKWARD, FORWARD, FULL, or NONE per schema; justify \
            based on producer/consumer deployment independence.\n\
         2. **Safe Changes** -- enumerate changes that preserve compatibility (add optional \
            field with default, add new enum value at end, widen numeric type).\n\
         3. **Breaking Changes** -- enumerate changes that break compatibility (remove field, \
            rename field, change type, add required field without default).\n\
         4. **Schema Registry** -- Confluent Schema Registry or Buf Schema Registry setup; \
            compatibility check in CI pipeline.\n\
         5. **Migration Procedure** -- when a breaking change is unavoidable: new topic/version, \
            dual-publish period, consumer migration, old version sunset.\n\
         6. **Versioning Strategy** -- semantic versioning for schemas, changelog generation, \
            deprecation annotations.\n\n\
         Provide rules for Avro, Protobuf, and JSON Schema. Note differences in default \
         value handling and union type evolution.",
    )
    .with_quality_threshold(0.80)
}

fn multi_model_database() -> Skill {
    Skill::new(
        "db_multi_model_database",
        "Multi-Model Database",
        "Design a polyglot persistence strategy: select the right data model \
         (relational, document, graph, key-value, time-series) for each use case.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a data architecture advisor selecting database technologies per use case.\n\n\
         DELIVERABLES:\n\
         1. **Use Case Inventory** -- each data use case with access pattern (OLTP, OLAP, \
            search, cache, stream), consistency requirement, and latency target.\n\
         2. **Technology Selection Matrix** -- for each use case, recommended database engine \
            with rationale (PostgreSQL, MongoDB, Redis, Neo4j, ClickHouse, Elasticsearch, etc.).\n\
         3. **Data Synchronization** -- how data flows between stores (CDC, dual-write, \
            ETL/ELT, event-driven sync); consistency guarantees per sync path.\n\
         4. **Operational Complexity** -- number of distinct database engines to operate; \
            team skill requirements; managed-service preference to reduce burden.\n\
         5. **Consolidation Opportunities** -- where a single engine can serve multiple use \
            cases (e.g., PostgreSQL with JSONB + pg_trgm + PostGIS instead of separate stores).\n\
         6. **Cost Model** -- per-engine cost estimate (compute, storage, license) and \
            total-cost-of-ownership comparison vs a consolidated approach.\n\n\
         Default to fewer engines with broader capability over many specialized engines. \
         Complexity is a cost.",
    )
    .with_quality_threshold(0.80)
}

fn database_observability() -> Skill {
    Skill::new(
        "db_database_observability",
        "Database Observability",
        "Set up slow query logging, connection monitoring, lock detection, \
         deadlock resolution, and performance dashboards.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a database reliability engineer setting up observability.\n\n\
         DELIVERABLES:\n\
         1. **Slow Query Logging** -- pg_stat_statements or equivalent; log_min_duration_statement \
            threshold; auto_explain for plans of slow queries.\n\
         2. **Connection Monitoring** -- active/idle/waiting connections dashboard; \
            connection churn rate; per-application-pool breakdown.\n\
         3. **Lock Detection** -- pg_locks monitoring; blocked-query identification; \
            lock wait timeout configuration; automatic lock-holder termination policy.\n\
         4. **Deadlock Resolution** -- deadlock_timeout configuration; deadlock logging; \
            application-level retry-on-deadlock pattern; code patterns that prevent deadlocks.\n\
         5. **Replication Monitoring** -- replication lag (bytes and time), WAL generation \
            rate, slot lag, replay rate per replica.\n\
         6. **Dashboard & Alerts** -- Grafana dashboard template with: QPS, latency P50/P95/P99, \
            cache hit ratio, table bloat, index usage ratio, vacuum progress; \
            alert rules for each metric.\n\n\
         Provide configuration for PostgreSQL with Prometheus postgres_exporter. \
         Note MySQL/pganalyze alternatives.",
    )
    .with_quality_threshold(0.80)
}

fn backup_recovery_strategy() -> Skill {
    Skill::new(
        "db_backup_recovery_strategy",
        "Backup & Recovery Strategy",
        "Design point-in-time recovery, backup verification, RTO/RPO planning, \
         and cross-region replication for disaster recovery.",
        SkillCategory::Database,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a disaster recovery engineer designing a database backup strategy.\n\n\
         DELIVERABLES:\n\
         1. **RTO/RPO Targets** -- Recovery Time Objective and Recovery Point Objective per \
            data tier (critical transactional < 1hr/5min, analytics < 4hr/1hr, logs best-effort).\n\
         2. **Backup Methods** -- base backup (pg_basebackup), continuous WAL archiving, \
            logical backups (pg_dump) for schema-only; frequency and retention per method.\n\
         3. **Point-in-Time Recovery** -- PITR configuration with WAL archive and recovery \
            target time; tested recovery procedure with expected duration.\n\
         4. **Backup Verification** -- automated restore-to-staging on schedule, data integrity \
            checks (row counts, checksums), alert on verification failure.\n\
         5. **Cross-Region Replication** -- async replica in secondary region, promotion runbook, \
            DNS failover, data loss window during region failure.\n\
         6. **Encryption & Compliance** -- backup encryption at rest (AES-256), access control \
            (IAM policies for backup storage), retention compliance (GDPR right-to-erasure \
            implications for backups).\n\n\
         Include a quarterly DR drill checklist. Every recovery procedure must have a tested, \
         documented runbook with expected completion time.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 500,
        fallback_skill: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_database_skills() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        assert_eq!(registry.by_category(SkillCategory::Database).len(), 15);
    }

    #[test]
    fn test_backend_agent_access() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        let backend_skills = registry.by_agent(AgentRole::Backend);
        // Backend is listed on all 15 skills
        assert!(backend_skills.len() >= 13);
    }

    #[test]
    fn test_schema_design_output_format() {
        let skill = schema_design();
        assert_eq!(skill.output_format, OutputFormat::Schema);
    }

    #[test]
    fn test_migration_planner_output_format() {
        let skill = migration_planner();
        assert_eq!(skill.output_format, OutputFormat::Migration);
    }

    #[test]
    fn test_backup_high_quality_threshold() {
        let skill = backup_recovery_strategy();
        assert!(skill.quality_threshold >= 0.85);
    }
}
