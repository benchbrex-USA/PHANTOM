//! Code generation skills for the Phantom autonomous AI engineering system.
//!
//! Defines all skills that agents use to generate production software artifacts:
//! scaffolds, APIs, auth systems, infrastructure patterns, and more.
//! Each skill is registered into the global `SkillRegistry` via `register()`.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all code-generation skills into the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(full_stack_scaffold());
    registry.register(rest_api_endpoint());
    registry.register(graphql_schema());
    registry.register(grpc_service());
    registry.register(websocket_handler());
    registry.register(crud_generator());
    registry.register(auth_system());
    registry.register(rbac_abac_system());
    registry.register(payment_integration());
    registry.register(email_system());
    registry.register(notification_engine());
    registry.register(search_engine());
    registry.register(file_processor());
    registry.register(background_job_system());
    registry.register(cache_layer());
    registry.register(rate_limiter());
    registry.register(feature_flag_system());
    registry.register(state_machine_generator());
    registry.register(event_sourcing_cqrs());
    registry.register(microservice_template());
    registry.register(serverless_function());
    registry.register(cli_tool_generator());
    registry.register(sdk_generator());
    registry.register(plugin_system());
    registry.register(workflow_engine());
    registry.register(realtime_collaboration());
    registry.register(multi_tenancy());
    registry.register(internationalization_system());
    registry.register(audit_trail_system());
    registry.register(data_export_import());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn full_stack_scaffold() -> Skill {
    Skill {
        id: SkillId::new("full_stack_scaffold"),
        name: "Full-Stack Project Scaffold".into(),
        description: "Generates a complete project structure including package manifests \
            (package.json, Cargo.toml), source directories, configuration files, CI/CD \
            pipelines, Dockerfiles, docker-compose, and README with architecture notes."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Architect, AgentRole::Backend, AgentRole::DevOps],
        dependencies: vec![],
        estimated_tokens: 50_000,
        system_prompt_extension: "Generate a production-grade project scaffold following \
            the target language's canonical directory layout. Include a lockfile-ready \
            manifest, editorconfig, linting config, pre-commit hooks, multi-stage \
            Dockerfile, GitHub Actions CI with caching, and a concise README documenting \
            the architecture decisions. Ensure all paths are relative and the project \
            builds from a clean clone with a single command."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.80,
    }
}

fn rest_api_endpoint() -> Skill {
    Skill {
        id: SkillId::new("rest_api_endpoint"),
        name: "REST API Endpoint".into(),
        description: "Creates REST endpoints with route handlers, request/response DTOs, \
            input validation, structured error handling, and middleware integration."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Backend],
        dependencies: vec![],
        estimated_tokens: 15_000,
        system_prompt_extension: "Produce idiomatic route handlers with typed request and \
            response structs, exhaustive input validation using the framework's native \
            validators, and a unified error type that maps to correct HTTP status codes. \
            Include OpenAPI doc-comments and integration-test stubs for happy path, \
            validation failure, and auth rejection."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(3, 3_000, true),
        quality_threshold: 0.85,
    }
}

fn graphql_schema() -> Skill {
    Skill {
        id: SkillId::new("graphql_schema"),
        name: "GraphQL Schema & Resolvers".into(),
        description: "Generates GraphQL type definitions, query/mutation/subscription \
            resolvers, dataloader patterns for N+1 prevention, and input validation."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::Architect],
        dependencies: vec![],
        estimated_tokens: 25_000,
        system_prompt_extension: "Define a strongly-typed GraphQL schema with relay-style \
            cursor pagination, dataloader batching for every association to eliminate N+1 \
            queries, and custom scalars for dates and IDs. Mutations must be idempotent \
            where possible, and subscriptions should use filtered topics to minimize \
            broadcast traffic."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.85,
    }
}

fn grpc_service() -> Skill {
    Skill {
        id: SkillId::new("grpc_service"),
        name: "gRPC Service".into(),
        description: "Generates .proto files with service definitions, server/client \
            implementations, streaming handlers, interceptors, and reflection support."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::Architect],
        dependencies: vec![],
        estimated_tokens: 20_000,
        system_prompt_extension: "Write proto3 IDL with well-documented messages, use \
            meaningful field numbers, and include both unary and streaming RPCs. Server \
            implementations must handle cancellation, deadlines, and metadata propagation. \
            Add interceptors for logging and auth, and enable gRPC server reflection for \
            tooling compatibility."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.85,
    }
}

fn websocket_handler() -> Skill {
    Skill {
        id: SkillId::new("websocket_handler"),
        name: "WebSocket Handler".into(),
        description: "Real-time WebSocket server and client with room management, \
            broadcast channels, heartbeat/ping-pong, automatic reconnection, and \
            back-pressure handling."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::Frontend],
        dependencies: vec![],
        estimated_tokens: 20_000,
        system_prompt_extension: "Implement a WebSocket server with room-based pub/sub, \
            periodic heartbeat pings to detect dead connections, graceful shutdown with \
            drain, and bounded send buffers to apply back-pressure. The client wrapper \
            must implement exponential-backoff reconnection and message queuing during \
            disconnects."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.82,
    }
}

fn crud_generator() -> Skill {
    Skill {
        id: SkillId::new("crud_generator"),
        name: "CRUD Generator".into(),
        description: "Full CRUD operations with cursor and offset pagination, dynamic \
            filtering, multi-column sorting, soft-delete with restore, and automatic \
            audit trail columns."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Backend],
        dependencies: vec![],
        estimated_tokens: 18_000,
        system_prompt_extension: "Generate repository, service, and handler layers with \
            strict separation of concerns. Queries must use parameterized SQL to prevent \
            injection. Soft-delete uses a `deleted_at` timestamp with a unique partial \
            index to preserve uniqueness constraints. Include `created_at`, `updated_at`, \
            and `created_by` audit columns on every table."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(3, 3_000, true),
        quality_threshold: 0.85,
    }
}

fn auth_system() -> Skill {
    Skill {
        id: SkillId::new("auth_system"),
        name: "Authentication System".into(),
        description: "JWT, OAuth2, OIDC, SAML, and passkey authentication with refresh \
            token rotation, secure session management, MFA (TOTP/WebAuthn), and account \
            lockout policies."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Critical,
        required_agents: vec![AgentRole::Security, AgentRole::Backend],
        dependencies: vec![],
        estimated_tokens: 35_000,
        system_prompt_extension: "All tokens must be signed with RS256 or ES256; never \
            use HS256 with a shared secret in production. Refresh tokens are single-use \
            with automatic rotation and family-based revocation on reuse detection. Store \
            password hashes using Argon2id with recommended OWASP parameters. MFA \
            enrollment must require re-authentication, and recovery codes must be \
            displayed exactly once."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 8_000, true),
        quality_threshold: 0.95,
    }
}

fn rbac_abac_system() -> Skill {
    Skill {
        id: SkillId::new("rbac_abac_system"),
        name: "RBAC + ABAC Access Control".into(),
        description: "Role-based and attribute-based access control with a policy engine, \
            hierarchical permission inheritance, resource scoping, and deny-override \
            conflict resolution."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Critical,
        required_agents: vec![AgentRole::Security, AgentRole::Architect],
        dependencies: vec![SkillId::new("auth_system")],
        estimated_tokens: 30_000,
        system_prompt_extension: "Design a policy engine that evaluates RBAC roles first, \
            then ABAC attribute predicates, with explicit deny always overriding allow. \
            Permission inheritance must follow a directed acyclic graph with cycle \
            detection at write time. Include a dry-run evaluation mode for debugging \
            policies and a migration path from flat roles to hierarchical ABAC."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 8_000, true),
        quality_threshold: 0.95,
    }
}

fn payment_integration() -> Skill {
    Skill {
        id: SkillId::new("payment_integration"),
        name: "Payment Integration".into(),
        description: "Stripe and generic payment processor integration with webhook \
            verification, subscription billing, usage-based metering, invoicing, \
            refund workflows, and PCI-compliant card handling."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Critical,
        required_agents: vec![AgentRole::Backend, AgentRole::Security],
        dependencies: vec![SkillId::new("auth_system")],
        estimated_tokens: 30_000,
        system_prompt_extension: "Never store raw card numbers; use tokenized payment \
            methods via Stripe Elements or equivalent. Webhook handlers must be idempotent \
            using the event ID as a deduplication key. Implement optimistic locking on \
            subscription state transitions and store all monetary amounts as integer cents \
            to avoid floating-point drift. Include dunning retry logic for failed charges."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 8_000, true),
        quality_threshold: 0.93,
    }
}

fn email_system() -> Skill {
    Skill {
        id: SkillId::new("email_system"),
        name: "Transactional Email System".into(),
        description: "Transactional email sending with MJML/handlebars templates, \
            background queue, bounce and complaint handling, DKIM/SPF/DMARC \
            configuration, open/click analytics, and unsubscribe management."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Backend],
        dependencies: vec![SkillId::new("background_job_system")],
        estimated_tokens: 18_000,
        system_prompt_extension: "Use a provider-agnostic interface (SendGrid, SES, \
            Postmark) behind a trait so the provider can be swapped without code changes. \
            Queue all sends through the background job system for resilience. Parse SNS \
            bounce notifications and automatically suppress hard-bounced addresses. \
            Templates must be precompiled at startup and cached."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(3, 3_000, true),
        quality_threshold: 0.82,
    }
}

fn notification_engine() -> Skill {
    Skill {
        id: SkillId::new("notification_engine"),
        name: "Notification Engine".into(),
        description: "Multi-channel notification delivery (push, email, SMS, in-app, \
            webhook) with user preference management, batching/digest, delivery \
            tracking, and retry on transient failures."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::Frontend],
        dependencies: vec![
            SkillId::new("email_system"),
            SkillId::new("background_job_system"),
        ],
        estimated_tokens: 25_000,
        system_prompt_extension: "Route each notification through a channel resolver that \
            checks user preferences, quiet hours, and frequency caps before dispatching. \
            Implement digest batching with configurable windows (instant, hourly, daily). \
            Each delivery attempt must be logged with status for audit, and webhook \
            deliveries must verify HMAC signatures on the receiving end."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(3, 5_000, true),
        quality_threshold: 0.83,
    }
}

fn search_engine() -> Skill {
    Skill {
        id: SkillId::new("search_engine"),
        name: "Full-Text Search Engine".into(),
        description: "Full-text search integration with Elasticsearch, Meilisearch, or \
            Typesense including index management, faceted search, autocomplete, fuzzy \
            matching, synonyms, and relevance tuning."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::Architect],
        dependencies: vec![],
        estimated_tokens: 22_000,
        system_prompt_extension: "Abstract the search backend behind a trait so the \
            engine can be swapped between Elasticsearch, Meilisearch, and Typesense. \
            Indexing must happen asynchronously via CDC or outbox pattern to avoid \
            blocking writes. Include analyzers for stemming, stop-words, and synonyms, \
            and expose faceted aggregation endpoints for filter UIs."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.83,
    }
}

fn file_processor() -> Skill {
    Skill {
        id: SkillId::new("file_processor"),
        name: "File Processor".into(),
        description: "File upload pipeline with image resizing, video transcoding, \
            document parsing, virus scanning (ClamAV), CDN distribution, and signed \
            URL generation."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::DevOps],
        dependencies: vec![SkillId::new("background_job_system")],
        estimated_tokens: 22_000,
        system_prompt_extension: "Accept uploads via multipart/form-data with size and \
            MIME-type validation at the edge. Process files through a pipeline of steps \
            (virus scan, resize/transcode, optimize) in the background job system. Store \
            originals in a private bucket and serve processed variants through a CDN with \
            signed, time-limited URLs. Never trust client-supplied content types; detect \
            via magic bytes."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.85,
    }
}

fn background_job_system() -> Skill {
    Skill {
        id: SkillId::new("background_job_system"),
        name: "Background Job System".into(),
        description: "Persistent job queue with priority levels, cron scheduling, \
            configurable retries with exponential backoff, dead-letter queue, \
            per-queue rate limiting, and distributed locking."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::DevOps],
        dependencies: vec![],
        estimated_tokens: 25_000,
        system_prompt_extension: "Jobs must be serializable to JSON and stored in a \
            durable backend (Postgres or Redis Streams). Use advisory locks or SKIP \
            LOCKED to prevent double-processing. Each job carries a max-attempts counter \
            and exponential backoff with jitter. Dead-letter jobs must retain full payload \
            and error history for manual retry or inspection."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.87,
    }
}

fn cache_layer() -> Skill {
    Skill {
        id: SkillId::new("cache_layer"),
        name: "Multi-Tier Cache Layer".into(),
        description: "L1 in-process (moka/dashmap), L2 Redis, L3 CDN caching with \
            cache-aside, write-through, and write-behind strategies, tag-based \
            invalidation, and stampede protection."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::Architect],
        dependencies: vec![],
        estimated_tokens: 20_000,
        system_prompt_extension: "Implement a CacheManager trait with get/set/invalidate \
            that cascades through L1 -> L2 -> L3 tiers. Use probabilistic early \
            expiration (XFetch) to prevent thundering-herd stampedes. Tag-based \
            invalidation must fan out to all tiers atomically. Serialize cached values \
            with versioned schemas so rolling deploys do not read stale formats."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.85,
    }
}

fn rate_limiter() -> Skill {
    Skill {
        id: SkillId::new("rate_limiter"),
        name: "Rate Limiter".into(),
        description: "Distributed rate limiting with token bucket, sliding window, and \
            leaky bucket algorithms. Supports per-user, per-IP, and per-API-key limits \
            with configurable quotas and Redis-backed coordination."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Backend, AgentRole::Security],
        dependencies: vec![],
        estimated_tokens: 15_000,
        system_prompt_extension: "Use Redis Lua scripts for atomic check-and-decrement \
            to guarantee correctness under concurrency. Return standard rate-limit \
            headers (X-RateLimit-Limit, Remaining, Reset) on every response. Support \
            hierarchical limits (global > tenant > user) and allow burst overrides for \
            specific API keys. Include a local in-memory pre-check to reduce Redis \
            round-trips under low load."
            .into(),
        output_format: OutputFormat::SingleFile,
        retry_strategy: RetryStrategy::new(3, 3_000, true),
        quality_threshold: 0.88,
    }
}

fn feature_flag_system() -> Skill {
    Skill {
        id: SkillId::new("feature_flag_system"),
        name: "Feature Flag System".into(),
        description: "Feature flag management with boolean, percentage, and targeting-rule \
            flags, gradual rollout, A/B test bucketing, kill switches, and a management \
            API with audit log."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Backend, AgentRole::Frontend],
        dependencies: vec![SkillId::new("cache_layer")],
        estimated_tokens: 18_000,
        system_prompt_extension: "Flags must be evaluated locally from a cached rule set \
            synced from the control plane to avoid per-request latency. Use consistent \
            hashing on user ID for percentage rollouts so users stay in the same bucket \
            across evaluations. The kill switch must bypass all targeting rules and take \
            effect within the cache TTL. Store every flag change in an append-only audit \
            table."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(3, 3_000, true),
        quality_threshold: 0.83,
    }
}

fn state_machine_generator() -> Skill {
    Skill {
        id: SkillId::new("state_machine_generator"),
        name: "State Machine Generator".into(),
        description: "Generates type-safe state machines from a specification with \
            enumerated states, guarded transitions, side-effect actions, persistent \
            state storage, and DOT/Mermaid visualization."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Backend, AgentRole::Architect],
        dependencies: vec![],
        estimated_tokens: 16_000,
        system_prompt_extension: "Model states and events as enums with exhaustive match \
            coverage so the compiler catches missing transitions. Guard functions must be \
            pure predicates; side-effects run only after the transition is persisted. \
            Store current state in a versioned row with optimistic locking. Emit a DOT \
            graph from the transition table for documentation."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.85,
    }
}

fn event_sourcing_cqrs() -> Skill {
    Skill {
        id: SkillId::new("event_sourcing_cqrs"),
        name: "Event Sourcing & CQRS".into(),
        description: "Event store with append-only streams, command and event handlers, \
            read-model projections, periodic snapshots, and full stream replay for \
            rebuilding state."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Critical,
        required_agents: vec![AgentRole::Architect, AgentRole::Backend],
        dependencies: vec![SkillId::new("background_job_system")],
        estimated_tokens: 30_000,
        system_prompt_extension: "Events are immutable facts stored in an append-only \
            table with a global sequence number. Command handlers must validate invariants \
            against the current aggregate state rebuilt from events. Projections run \
            asynchronously and track their last-processed sequence for idempotent replay. \
            Snapshots are taken every N events to bound rehydration time, and the snapshot \
            schema must be forward-compatible."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 8_000, true),
        quality_threshold: 0.90,
    }
}

fn microservice_template() -> Skill {
    Skill {
        id: SkillId::new("microservice_template"),
        name: "Microservice Template".into(),
        description: "Service-mesh-ready microservice with structured logging, health \
            and readiness probes, circuit breakers, service discovery registration, \
            distributed tracing, and graceful shutdown."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::DevOps, AgentRole::Architect],
        dependencies: vec![SkillId::new("full_stack_scaffold")],
        estimated_tokens: 25_000,
        system_prompt_extension: "Emit structured JSON logs with trace-id propagation \
            via OpenTelemetry. Health endpoints must distinguish liveness (process alive) \
            from readiness (dependencies connected). Circuit breakers use a half-open \
            probe after the timeout window. Graceful shutdown drains in-flight requests \
            before exiting, respecting the SIGTERM termination grace period."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.85,
    }
}

fn serverless_function() -> Skill {
    Skill {
        id: SkillId::new("serverless_function"),
        name: "Serverless Function".into(),
        description: "AWS Lambda, Cloudflare Workers, or Vercel Edge function with cold \
            start optimization, connection pooling, bundling, and environment-based \
            configuration."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Backend, AgentRole::DevOps],
        dependencies: vec![],
        estimated_tokens: 14_000,
        system_prompt_extension: "Minimize cold start by keeping the bundle small, \
            initializing heavy clients outside the handler (module scope), and using \
            provisioned concurrency hints where available. Reuse database connections \
            via an external pool (RDS Proxy, Hyperdrive). Include IaC definitions \
            (SAM/Terraform/Wrangler) alongside the function code."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(3, 3_000, true),
        quality_threshold: 0.82,
    }
}

fn cli_tool_generator() -> Skill {
    Skill {
        id: SkillId::new("cli_tool_generator"),
        name: "CLI Tool Generator".into(),
        description: "Command-line tool with subcommands, argument/flag parsing, \
            config file loading (TOML/YAML), shell completions (bash/zsh/fish), \
            man page generation, and colored output."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Backend],
        dependencies: vec![],
        estimated_tokens: 15_000,
        system_prompt_extension: "Use clap (Rust) or cobra (Go) derive macros for \
            declarative argument definitions. Config precedence: CLI flags > env vars > \
            config file > defaults. Generate shell completion scripts at build time and \
            include them in the release archive. Exit codes must follow sysexits.h \
            conventions and stderr must be used for diagnostics."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(3, 3_000, true),
        quality_threshold: 0.83,
    }
}

fn sdk_generator() -> Skill {
    Skill {
        id: SkillId::new("sdk_generator"),
        name: "Client SDK Generator".into(),
        description: "Generates typed client SDKs from OpenAPI or GraphQL schemas for \
            TypeScript, Python, Go, and Rust with authentication, pagination helpers, \
            retry logic, and per-language idiomatic error handling."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Backend, AgentRole::Architect],
        dependencies: vec![SkillId::new("rest_api_endpoint")],
        estimated_tokens: 25_000,
        system_prompt_extension: "Parse the OpenAPI/GraphQL schema to extract every \
            operation, type, and enum. Generate idiomatic code per target language: \
            async/await in TS, dataclasses in Python, interfaces in Go, strong types in \
            Rust. Include automatic retry with exponential backoff, pagination iterators, \
            and a configurable base-URL/auth-token constructor."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.85,
    }
}

fn plugin_system() -> Skill {
    Skill {
        id: SkillId::new("plugin_system"),
        name: "Plugin Architecture".into(),
        description: "Plugin system with dynamic discovery, safe loading, sandboxed \
            execution, versioned API contracts, dependency resolution, and hot-reload \
            without service restart."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Architect, AgentRole::Backend],
        dependencies: vec![],
        estimated_tokens: 25_000,
        system_prompt_extension: "Define a stable plugin API trait with semantic \
            versioning; the host checks compatibility before loading. Plugins are loaded \
            from shared libraries (cdylib) or WASM modules with capability-based \
            sandboxing. A file-system watcher triggers hot-reload by swapping the plugin \
            handle behind an RwLock. Include a manifest schema for declaring plugin \
            metadata, permissions, and dependencies."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.85,
    }
}

fn workflow_engine() -> Skill {
    Skill {
        id: SkillId::new("workflow_engine"),
        name: "Workflow Engine".into(),
        description: "Business process automation with BPMN-like workflow definitions, \
            human-task assignment, timer events, parallel gateways, compensation \
            handlers, and persistent execution state."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Critical,
        required_agents: vec![AgentRole::Architect, AgentRole::Backend],
        dependencies: vec![
            SkillId::new("state_machine_generator"),
            SkillId::new("background_job_system"),
        ],
        estimated_tokens: 30_000,
        system_prompt_extension: "Model workflows as a directed graph of activities, \
            gateways, and events persisted in a workflow-instance table. Each activity \
            execution is recorded for audit. Timer events use the background job \
            scheduler. Compensation handlers run in reverse order on failure. Human tasks \
            must support delegation, escalation timeouts, and form-schema attachment."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 8_000, true),
        quality_threshold: 0.88,
    }
}

fn realtime_collaboration() -> Skill {
    Skill {
        id: SkillId::new("realtime_collaboration"),
        name: "Real-Time Collaboration".into(),
        description: "Operational Transform or CRDT-based collaborative editing with \
            presence awareness, cursor tracking, undo/redo per user, conflict-free \
            merging, and offline support."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Critical,
        required_agents: vec![AgentRole::Architect, AgentRole::Backend, AgentRole::Frontend],
        dependencies: vec![SkillId::new("websocket_handler")],
        estimated_tokens: 35_000,
        system_prompt_extension: "Prefer CRDTs (Yjs/Automerge) over OT for simpler \
            conflict resolution. The server acts as an authoritative relay that persists \
            document snapshots periodically. Presence data (cursors, selections) is \
            broadcast on a separate lightweight channel to avoid inflating the document \
            history. Offline edits are queued locally and merged on reconnect."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 8_000, true),
        quality_threshold: 0.88,
    }
}

fn multi_tenancy() -> Skill {
    Skill {
        id: SkillId::new("multi_tenancy"),
        name: "Multi-Tenancy".into(),
        description: "Tenant isolation with schema-per-tenant, row-level security, or \
            database-per-tenant strategies. Includes tenant provisioning, onboarding \
            flows, data partitioning, and cross-tenant query prevention."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Critical,
        required_agents: vec![AgentRole::Architect, AgentRole::Backend, AgentRole::Security],
        dependencies: vec![SkillId::new("auth_system")],
        estimated_tokens: 28_000,
        system_prompt_extension: "Extract the tenant identifier from the JWT or request \
            header and set it on every database connection via `SET app.current_tenant`. \
            Row-level security policies must be enabled on every tenant-scoped table as a \
            defense-in-depth measure. Tenant provisioning must be idempotent and run \
            migrations in a transaction. Never allow cross-tenant joins in application \
            queries."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 8_000, true),
        quality_threshold: 0.93,
    }
}

fn internationalization_system() -> Skill {
    Skill {
        id: SkillId::new("internationalization_system"),
        name: "Internationalization System".into(),
        description: "i18n/l10n with ICU MessageFormat, CLDR-based pluralization rules, \
            locale-aware date/number/currency formatting, RTL layout support, and \
            translation workflow integration."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Frontend, AgentRole::Backend],
        dependencies: vec![],
        estimated_tokens: 16_000,
        system_prompt_extension: "Use ICU MessageFormat for all user-facing strings to \
            handle plurals, gender, and select patterns correctly. Locale is resolved \
            from Accept-Language header, user preference, then fallback default. Date and \
            number formatting must use Intl APIs (browser) or ICU4X (server). Extract \
            translation keys at build time and generate type-safe accessors to catch \
            missing keys at compile time."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(3, 3_000, true),
        quality_threshold: 0.82,
    }
}

fn audit_trail_system() -> Skill {
    Skill {
        id: SkillId::new("audit_trail_system"),
        name: "Audit Trail System".into(),
        description: "Immutable, append-only audit log with hash-chain tamper detection, \
            structured event schema, compliance reporting (SOC2/GDPR), configurable \
            retention policies, and secure export."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::High,
        required_agents: vec![AgentRole::Security, AgentRole::Backend],
        dependencies: vec![],
        estimated_tokens: 20_000,
        system_prompt_extension: "Every audit entry includes actor, action, resource, \
            timestamp, IP, and a SHA-256 hash chaining to the previous entry for tamper \
            evidence. The audit table must be append-only with no UPDATE or DELETE grants. \
            Retention policies archive old entries to cold storage before purging. Provide \
            pre-built compliance report queries for SOC2 access reviews and GDPR data \
            access logs."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(2, 5_000, true),
        quality_threshold: 0.92,
    }
}

fn data_export_import() -> Skill {
    Skill {
        id: SkillId::new("data_export_import"),
        name: "Data Export & Import".into(),
        description: "Streaming CSV, JSON, and XML import/export with schema validation, \
            row-level error reporting, configurable transformations, progress tracking, \
            and resumable uploads for large files."
            .into(),
        category: SkillCategory::CodeGeneration,
        complexity: SkillComplexity::Medium,
        required_agents: vec![AgentRole::Backend],
        dependencies: vec![SkillId::new("background_job_system")],
        estimated_tokens: 18_000,
        system_prompt_extension: "Stream records through a pipeline of parse, validate, \
            transform, and load stages without buffering the entire file in memory. \
            Validation errors are collected per-row and returned in a structured report \
            after the job completes. Large imports run as background jobs with progress \
            percentage exposed via SSE or polling endpoint. Exports must support cursor- \
            based chunking to avoid OOM on million-row tables."
            .into(),
        output_format: OutputFormat::MultiFile,
        retry_strategy: RetryStrategy::new(3, 3_000, true),
        quality_threshold: 0.83,
    }
}
