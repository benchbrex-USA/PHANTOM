//! API design skills.
//!
//! REST, GraphQL, gRPC, and async API design; OpenAPI/AsyncAPI spec generation;
//! versioning, gateway config, webhooks, error taxonomy, pagination, batching,
//! idempotency, contract testing, documentation, and breaking-change detection.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all API design skills with the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(rest_api_design());
    registry.register(openapi_spec_generator());
    registry.register(graphql_schema_design());
    registry.register(grpc_proto_design());
    registry.register(async_api_spec());
    registry.register(api_versioning_strategy());
    registry.register(api_gateway_config());
    registry.register(webhook_system());
    registry.register(api_error_taxonomy());
    registry.register(api_pagination_strategy());
    registry.register(api_batching_strategy());
    registry.register(api_idempotency_layer());
    registry.register(contract_testing_setup());
    registry.register(api_documentation_site());
    registry.register(breaking_change_detector());
}

// ---------------------------------------------------------------------------
// Individual skill constructors
// ---------------------------------------------------------------------------

fn rest_api_design() -> Skill {
    Skill::new(
        "api_rest_api_design",
        "REST API Design",
        "Design a RESTful API with HATEOAS links, content negotiation, versioning, \
         pagination, filtering, and sorting conventions.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a senior API architect designing a RESTful API.\n\n\
         DELIVERABLES:\n\
         1. **Resource Model** -- nouns as resources, proper URI hierarchy (/collections/{id}/sub), \
            no verbs in paths, plural collection names.\n\
         2. **HTTP Method Semantics** -- GET (safe, cacheable), POST (create), PUT (full replace), \
            PATCH (partial update, JSON Merge Patch or JSON Patch), DELETE (idempotent).\n\
         3. **HATEOAS Links** -- _links object in responses with self, next, prev, related \
            resource URIs; HAL or JSON:API format.\n\
         4. **Content Negotiation** -- Accept/Content-Type headers, support for JSON and \
            optionally JSON:API, CSV export; charset and compression (gzip, br).\n\
         5. **Filtering, Sorting, Pagination** -- query parameter conventions (?filter[status]=active, \
            ?sort=-created_at, ?page[cursor]=xxx&page[size]=25); document limits.\n\
         6. **Versioning** -- URL prefix (/v1/) or Accept header versioning; deprecation \
            policy with Sunset and Deprecation headers.\n\n\
         Follow RFC 7231 (HTTP semantics) and RFC 8288 (Web Linking). Every endpoint must \
         return consistent envelope structure.",
    )
    .with_quality_threshold(0.85)
}

fn openapi_spec_generator() -> Skill {
    Skill::new(
        "api_openapi_spec_generator",
        "OpenAPI Spec Generator",
        "Generate an OpenAPI 3.1 specification with schemas, examples, security \
         schemes, webhooks, and server definitions.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are an API specification expert generating OpenAPI 3.1 documents.\n\n\
         DELIVERABLES:\n\
         1. **Info & Servers** -- API title, description, version, contact, license; \
            server URLs for dev/staging/production with variables.\n\
         2. **Paths** -- every endpoint with operationId, summary, description, parameters \
            (path, query, header), requestBody, and responses (2xx, 4xx, 5xx).\n\
         3. **Schemas** -- reusable component schemas with JSON Schema 2020-12 keywords \
            (type, format, pattern, minimum, maximum, examples, $ref).\n\
         4. **Examples** -- per-endpoint request/response examples that form a coherent \
            narrative (create -> read -> update -> delete).\n\
         5. **Security Schemes** -- Bearer JWT, API key, OAuth2 (authorization_code, \
            client_credentials) with scopes; security requirement per endpoint.\n\
         6. **Webhooks** -- webhook event definitions with payload schema, delivery \
            headers, and retry policy documentation.\n\n\
         Output valid YAML. Validate against the OpenAPI 3.1 JSON Schema. Use $ref \
         extensively to avoid schema duplication.",
    )
    .with_quality_threshold(0.85)
    .with_dependencies(vec![SkillId::new("api_rest_api_design")])
}

fn graphql_schema_design() -> Skill {
    Skill::new(
        "api_graphql_schema_design",
        "GraphQL Schema Design",
        "Design a schema-first GraphQL API with federation, subscriptions, \
         custom directives, and query complexity limiting.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a GraphQL architect designing a production schema.\n\n\
         DELIVERABLES:\n\
         1. **Type System** -- Query, Mutation, Subscription root types; object types with \
            field resolvers; input types for mutations; enum types; interface/union for \
            polymorphism.\n\
         2. **Federation** -- @key directives for entity types, @external/@requires/@provides \
            for cross-service fields; subgraph boundaries aligned to domain contexts.\n\
         3. **Subscriptions** -- real-time event types, WebSocket transport (graphql-ws protocol), \
            subscription filter arguments, connection lifecycle.\n\
         4. **Custom Directives** -- @auth(requires: ADMIN), @deprecated(reason: \"...\"), \
            @cacheControl(maxAge: 300); schema-level and field-level.\n\
         5. **Complexity & Depth Limiting** -- per-field cost annotations, maximum query \
            depth, maximum total cost per request; persisted queries for production.\n\
         6. **N+1 Prevention** -- DataLoader pattern per resolver, batching strategy, \
            look-ahead optimization for eager loading.\n\n\
         Output SDL (Schema Definition Language). Prefer Relay-style connections for pagination \
         (edges/nodes/pageInfo). Avoid over-fetching by design.",
    )
    .with_quality_threshold(0.85)
}

fn grpc_proto_design() -> Skill {
    Skill::new(
        "api_grpc_proto_design",
        "gRPC Proto Design",
        "Design Protocol Buffers schemas with service definitions, streaming \
         patterns, error model, and deadline propagation.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a gRPC API designer creating Protocol Buffers service definitions.\n\n\
         DELIVERABLES:\n\
         1. **Package & Import Structure** -- proto package naming (com.company.service.v1), \
            file organization, import paths, buf.yaml configuration.\n\
         2. **Message Design** -- request/response messages with field numbering strategy \
            (reserve ranges for future), wrapper types for nullable fields, well-known types \
            (Timestamp, Duration, FieldMask).\n\
         3. **Service Definitions** -- RPC methods with unary, server-streaming, \
            client-streaming, and bidirectional-streaming patterns; when to use each.\n\
         4. **Error Model** -- google.rpc.Status with code, message, details; rich error \
            details (BadRequest, RetryInfo, DebugInfo); error code selection guide.\n\
         5. **Deadline Propagation** -- client deadline setting, server deadline checking, \
            cascading deadline reduction across service hops.\n\
         6. **Interceptors & Metadata** -- authentication via metadata, request-id propagation, \
            logging interceptor, retry policy configuration.\n\n\
         Output valid proto3 syntax. Follow the Google API Design Guide and Buf style guide. \
         Run buf lint mentally against the output.",
    )
    .with_quality_threshold(0.85)
}

fn async_api_spec() -> Skill {
    Skill::new(
        "api_async_api_spec",
        "AsyncAPI Spec",
        "Generate an AsyncAPI specification for event-driven APIs with channels, \
         messages, protocol bindings, and correlation IDs.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are an event-driven API specialist generating AsyncAPI 3.0 specifications.\n\n\
         DELIVERABLES:\n\
         1. **Info & Servers** -- API metadata, broker server URLs (Kafka, NATS, RabbitMQ), \
            protocol and protocolVersion, security scheme.\n\
         2. **Channels** -- topic/queue definitions with address pattern, publish/subscribe \
            operations, channel bindings (partitions, retention).\n\
         3. **Messages** -- message definitions with headers (correlationId, contentType, \
            schemaVersion), payload schema ($ref to components), examples.\n\
         4. **Correlation IDs** -- location expression for correlation ID extraction, \
            propagation rules across message chains.\n\
         5. **Protocol Bindings** -- Kafka bindings (key, partition, groupId), AMQP bindings \
            (exchange, queue, routingKey), WebSocket bindings.\n\
         6. **Traits & Reuse** -- message traits for common headers, operation traits for \
            shared bindings; $ref for DRY specifications.\n\n\
         Output valid YAML conforming to AsyncAPI 3.0. Use CloudEvents envelope format \
         where applicable.",
    )
    .with_quality_threshold(0.80)
}

fn api_versioning_strategy() -> Skill {
    Skill::new(
        "api_versioning_strategy",
        "API Versioning Strategy",
        "Design an API versioning approach with deprecation policy, sunset headers, \
         and consumer migration support.",
        SkillCategory::ApiDesign,
        SkillComplexity::Atomic,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are an API lifecycle manager designing a versioning strategy.\n\n\
         DELIVERABLES:\n\
         1. **Versioning Method** -- URL path (/v1/), custom header (API-Version), \
            content-type (application/vnd.api.v1+json); trade-offs and recommendation.\n\
         2. **Version Lifecycle** -- version states (alpha, beta, stable, deprecated, sunset); \
            minimum support duration per state; SLA differences.\n\
         3. **Deprecation Policy** -- minimum notice period before sunset (e.g., 6 months), \
            Deprecation and Sunset HTTP headers (RFC 8594), deprecation notices in response body.\n\
         4. **Breaking Change Definition** -- exhaustive list of what constitutes a breaking \
            change (remove field, change type, tighten validation, change error codes).\n\
         5. **Migration Support** -- migration guide template, SDK version pinning, automated \
            compatibility tests between versions, dual-version serving period.\n\
         6. **Version Header Propagation** -- how version context flows through internal \
            services; gateway-level version routing vs per-service handling.\n\n\
         Recommend URL-path versioning for public APIs (simplest for consumers) and \
         header versioning for internal APIs (cleaner URLs).",
    )
    .with_quality_threshold(0.80)
}

fn api_gateway_config() -> Skill {
    Skill::new(
        "api_gateway_config",
        "API Gateway Config",
        "Configure an API gateway with routing rules, request transformations, \
         rate limits, and authentication policies.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a platform engineer configuring an API gateway.\n\n\
         DELIVERABLES:\n\
         1. **Route Definitions** -- path-based and header-based routing rules mapping \
            external paths to internal service endpoints; path rewriting.\n\
         2. **Request/Response Transforms** -- header injection (X-Request-ID, X-Forwarded-For), \
            body transformation (field mapping, envelope wrapping), query parameter mapping.\n\
         3. **Rate Limiting** -- per-client rate limits (token bucket), per-endpoint limits, \
            global limits; rate limit headers (X-RateLimit-Limit, Remaining, Reset).\n\
         4. **Authentication** -- JWT validation (issuer, audience, signing key rotation), \
            API key validation, OAuth2 token introspection; per-route auth requirements.\n\
         5. **CORS Configuration** -- allowed origins, methods, headers; preflight caching; \
            credentials handling; per-route CORS overrides.\n\
         6. **Health & Observability** -- health check endpoints, request logging format, \
            distributed tracing header propagation, error rate circuit breaker.\n\n\
         Provide configuration in Kong declarative YAML or Envoy xDS format. Note \
         AWS API Gateway and Cloudflare Workers alternatives.",
    )
    .with_quality_threshold(0.80)
}

fn webhook_system() -> Skill {
    Skill::new(
        "api_webhook_system",
        "Webhook System",
        "Design a webhook delivery system with retry logic, signature verification, \
         event filtering, and delivery logging.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a webhook infrastructure engineer designing a delivery system.\n\n\
         DELIVERABLES:\n\
         1. **Registration API** -- CRUD endpoints for webhook subscriptions with URL validation, \
            event type filtering, secret generation, and status management.\n\
         2. **Delivery Pipeline** -- async delivery queue, per-subscription ordering guarantees, \
            concurrent delivery limits, timeout per delivery attempt (5-30s).\n\
         3. **Retry Strategy** -- exponential backoff schedule (e.g., 1min, 5min, 30min, 2hr, \
            12hr, 24hr), maximum retry count, automatic disabling after N consecutive failures.\n\
         4. **Signature Verification** -- HMAC-SHA256 signature in webhook header \
            (X-Webhook-Signature), timestamp inclusion for replay protection, \
            secret rotation support (dual-secret validation window).\n\
         5. **Event Filtering** -- per-subscription event type filter, optional payload \
            field-level filter expressions, wildcard event types.\n\
         6. **Delivery Logs** -- per-delivery attempt log (status code, latency, response body \
            snippet), searchable delivery history API, manual retry trigger.\n\n\
         Model after Stripe/GitHub webhook patterns. Include a webhook testing endpoint \
         that echoes events to aid consumer development.",
    )
    .with_quality_threshold(0.80)
}

fn api_error_taxonomy() -> Skill {
    Skill::new(
        "api_error_taxonomy",
        "API Error Taxonomy",
        "Design structured error responses with error codes, RFC 7807 Problem \
         Details, localization, and client recovery guidance.",
        SkillCategory::ApiDesign,
        SkillComplexity::Atomic,
        vec![AgentRole::Backend],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are an API standards engineer designing an error response taxonomy.\n\n\
         DELIVERABLES:\n\
         1. **Problem Details Format** -- RFC 7807/9457 compliant response: type (URI), \
            title, status, detail, instance; Content-Type: application/problem+json.\n\
         2. **Error Code Registry** -- hierarchical error codes (e.g., VALIDATION_001, \
            AUTH_002, RATE_LIMIT_001) with human-readable titles and suggested HTTP status.\n\
         3. **Validation Errors** -- array of field-level errors with pointer (JSON Pointer \
            to field), code, and message; consistent structure for all input validation.\n\
         4. **Localization** -- Accept-Language driven error message translation; error codes \
            as stable keys for client-side i18n; fallback to English.\n\
         5. **Client Recovery Guidance** -- machine-readable hints: retryable (boolean), \
            retry_after (seconds), documentation_url, suggested_action.\n\
         6. **Internal Error Masking** -- never expose stack traces, SQL errors, or internal \
            IDs in production; correlation ID for support-team lookup; verbose mode for \
            development environments only.\n\n\
         Provide JSON schema for the error response. Include examples for 400, 401, 403, \
         404, 409, 422, 429, and 500 scenarios.",
    )
    .with_quality_threshold(0.80)
}

fn api_pagination_strategy() -> Skill {
    Skill::new(
        "api_pagination_strategy",
        "API Pagination Strategy",
        "Design cursor vs offset pagination with total counts, page info metadata, \
         and deep pagination handling.",
        SkillCategory::ApiDesign,
        SkillComplexity::Atomic,
        vec![AgentRole::Backend],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are an API design specialist defining pagination patterns.\n\n\
         DELIVERABLES:\n\
         1. **Cursor Pagination** -- opaque cursor encoding (base64 of composite key), \
            first/after and last/before parameters, Relay-style connection spec \
            (edges, nodes, pageInfo with hasNextPage, hasPreviousPage, startCursor, endCursor).\n\
         2. **Offset Pagination** -- page[number] and page[size] parameters, total count \
            header or response field, maximum page size enforcement.\n\
         3. **Selection Guide** -- when to use cursor (real-time feeds, large datasets, \
            concurrent inserts) vs offset (admin UIs, small datasets, jump-to-page).\n\
         4. **Deep Pagination Mitigation** -- maximum offset limit (e.g., 10,000), forced \
            cursor migration for deep pages, search_after for Elasticsearch.\n\
         5. **Total Count Strategy** -- exact count (expensive for large tables), estimated \
            count (pg_class.reltuples), count caching, or omit (cursor-only APIs).\n\
         6. **Response Envelope** -- consistent wrapper with data array, pagination metadata, \
            and Link headers (RFC 8288) for next/prev/first/last.\n\n\
         Default to cursor pagination for all new APIs. Provide SQL query patterns for \
         both cursor and offset implementations.",
    )
    .with_quality_threshold(0.80)
}

fn api_batching_strategy() -> Skill {
    Skill::new(
        "api_batching_strategy",
        "API Batching Strategy",
        "Design batch and bulk API endpoints with partial success handling, \
         transaction semantics, and progress tracking.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Schema,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are an API architect designing batch operation endpoints.\n\n\
         DELIVERABLES:\n\
         1. **Batch Endpoint Design** -- POST /resources/batch with array of operations; \
            maximum batch size (100-1000); content-length limits.\n\
         2. **Partial Success Model** -- HTTP 207 Multi-Status response with per-item \
            status codes; client can distinguish succeeded vs failed items.\n\
         3. **Transaction Semantics** -- all-or-nothing mode (atomic batch) vs best-effort \
            mode (partial success); client-selectable via header or parameter.\n\
         4. **Progress Tracking** -- for large batches: async processing with job ID, \
            polling endpoint (GET /jobs/{id}), progress percentage, estimated completion.\n\
         5. **Rate Limiting** -- batch requests count as N individual requests for rate \
            limit purposes; batch size factored into quota consumption.\n\
         6. **Idempotency** -- per-item idempotency keys within a batch; batch-level \
            idempotency key for the entire request; deduplication window.\n\n\
         Provide request/response JSON examples for both synchronous (small batch) and \
         asynchronous (large batch) flows.",
    )
    .with_quality_threshold(0.80)
}

fn api_idempotency_layer() -> Skill {
    Skill::new(
        "api_idempotency_layer",
        "API Idempotency Layer",
        "Design an idempotency key system with request deduplication, stored \
         responses, and retry-safe mutation endpoints.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a reliability engineer designing an API idempotency layer.\n\n\
         DELIVERABLES:\n\
         1. **Idempotency Key Protocol** -- Idempotency-Key header (UUID v4), required on \
            all POST/PATCH mutations, 400 error if missing on non-idempotent endpoints.\n\
         2. **Key Storage** -- Redis or database table storing (key, request_hash, response, \
            status, created_at); TTL for key expiration (24-48 hours).\n\
         3. **Request Fingerprint** -- hash of (method, path, body) associated with the key; \
            reject mismatched fingerprints with 422 (key reuse with different request).\n\
         4. **Concurrent Request Handling** -- lock on key during processing; second request \
            with same key returns 409 Conflict while first is in-flight, or blocks and \
            returns cached response.\n\
         5. **Response Caching** -- store full response (status, headers, body) on first \
            completion; replay exact response on duplicate requests.\n\
         6. **Failure Handling** -- do not cache 5xx responses (allow retry); cache 4xx \
            responses (client error is deterministic); partial failure cleanup.\n\n\
         Model after Stripe's idempotency implementation. Provide middleware/interceptor \
         code pattern for the application framework.",
    )
    .with_quality_threshold(0.85)
}

fn contract_testing_setup() -> Skill {
    Skill::new(
        "api_contract_testing_setup",
        "Contract Testing Setup",
        "Set up consumer-driven contract tests with Pact or Spring Cloud Contract \
         for API compatibility verification.",
        SkillCategory::ApiDesign,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::Qa],
        OutputFormat::Test,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a testing architect setting up consumer-driven contract tests.\n\n\
         DELIVERABLES:\n\
         1. **Consumer Test** -- consumer-side test that defines expected interactions \
            (request shape, response shape, status codes); generates a pact file.\n\
         2. **Provider Verification** -- provider-side test that replays pact interactions \
            against the real provider with state setup hooks.\n\
         3. **Pact Broker** -- central broker for pact storage, version tagging, can-i-deploy \
            check, webhook triggers on new pacts.\n\
         4. **CI Integration** -- consumer publishes pact on PR merge; provider verifies \
            against latest consumer pacts before deployment; can-i-deploy gate.\n\
         5. **State Management** -- provider state setup (given/when/then) for test data \
            provisioning; cleanup between interactions.\n\
         6. **Evolution Workflow** -- when a consumer needs a new field: consumer writes \
            failing contract, provider implements, both merge; no manual coordination.\n\n\
         Default to Pact (pact-js, pact-rust, or pact-python). Note Spring Cloud Contract \
         for Spring Boot ecosystems. Include Makefile/CI snippets.",
    )
    .with_quality_threshold(0.80)
}

fn api_documentation_site() -> Skill {
    Skill::new(
        "api_documentation_site",
        "API Documentation Site",
        "Generate auto-updated API documentation with interactive playground, \
         multi-language code samples, and versioned changelog.",
        SkillCategory::ApiDesign,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a developer experience engineer building API documentation.\n\n\
         DELIVERABLES:\n\
         1. **Documentation Generator** -- tool selection (Redocly, Stoplight, Swagger UI, \
            Mintlify) with trade-off analysis; auto-generation from OpenAPI spec.\n\
         2. **Interactive Playground** -- try-it-now panel with authentication token input, \
            request builder, live response display; sandbox environment.\n\
         3. **Code Samples** -- auto-generated examples in curl, Python (requests), \
            JavaScript (fetch), Go (net/http), Rust (reqwest); copy-to-clipboard.\n\
         4. **Authentication Guide** -- step-by-step auth flow (obtain token, set header, \
            refresh token) with screenshots and code for each supported auth method.\n\
         5. **Changelog** -- per-version changelog with added/changed/deprecated/removed \
            sections; RSS feed for change notifications; migration guides for breaking changes.\n\
         6. **Search & Navigation** -- full-text search across endpoints, schemas, and guides; \
            sidebar navigation grouped by resource; deep-linking to specific endpoints.\n\n\
         Documentation must auto-update on OpenAPI spec changes via CI. Broken examples \
         are worse than no examples -- validate all code samples in CI.",
    )
    .with_quality_threshold(0.80)
}

fn breaking_change_detector() -> Skill {
    Skill::new(
        "api_breaking_change_detector",
        "Breaking Change Detector",
        "Detect breaking changes between API specification versions and generate \
         migration guides for affected consumers.",
        SkillCategory::ApiDesign,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::Qa],
        OutputFormat::Report,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are an API compatibility analyst detecting breaking changes.\n\n\
         DELIVERABLES:\n\
         1. **Diff Analysis** -- structural diff between old and new OpenAPI specs; \
            categorize each change as breaking, non-breaking, or potentially-breaking.\n\
         2. **Breaking Change Catalog** -- every breaking change with: affected endpoint, \
            change type (removed endpoint, removed field, type change, new required field, \
            changed status code), severity (critical, major, minor).\n\
         3. **Impact Assessment** -- which known consumers are affected (from API analytics \
            or consumer registry); estimated consumer count per breaking change.\n\
         4. **Migration Guide** -- per-breaking-change: what consumers must change, code \
            before/after examples, timeline for mandatory migration.\n\
         5. **Compatibility Shim** -- where possible, suggest a backward-compatible shim \
            (alias old field to new field, default value for new required field) to \
            avoid forcing immediate consumer changes.\n\
         6. **CI Gate** -- oasdiff, optic, or custom tool configuration to block PRs that \
            introduce unintentional breaking changes; allowlist for approved breaks.\n\n\
         Run this analysis on every PR that modifies an OpenAPI spec. Zero breaking changes \
         should reach production without explicit approval and a migration plan.",
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
    fn test_register_api_design_skills() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        assert_eq!(registry.by_category(SkillCategory::ApiDesign).len(), 15);
    }

    #[test]
    fn test_backend_agent_access() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        let backend_skills = registry.by_agent(AgentRole::Backend);
        assert_eq!(backend_skills.len(), 15);
    }

    #[test]
    fn test_openapi_depends_on_rest() {
        let skill = openapi_spec_generator();
        assert!(skill
            .dependencies
            .contains(&SkillId::new("api_rest_api_design")));
    }

    #[test]
    fn test_contract_testing_includes_qa() {
        let skill = contract_testing_setup();
        assert!(skill.required_agents.contains(&AgentRole::Qa));
    }

    #[test]
    fn test_breaking_change_high_quality() {
        let skill = breaking_change_detector();
        assert!(skill.quality_threshold >= 0.85);
    }
}
