//! Architecture & design pattern skills.
//!
//! System design, domain modeling, microservice decomposition, event-driven
//! architecture, clean/hexagonal patterns, and advanced distributed-system
//! patterns (service mesh, data mesh, cell-based, strangler fig, etc.).

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all architecture skills with the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(system_design_blueprint());
    registry.register(domain_driven_design());
    registry.register(microservice_decomposition());
    registry.register(event_driven_architecture());
    registry.register(clean_architecture());
    registry.register(hexagonal_architecture());
    registry.register(api_gateway_design());
    registry.register(service_mesh_design());
    registry.register(data_mesh_architecture());
    registry.register(cell_based_architecture());
    registry.register(strangler_fig_pattern());
    registry.register(backpressure_design());
    registry.register(bulkhead_pattern());
    registry.register(capability_mapping());
}

// ---------------------------------------------------------------------------
// Individual skill constructors
// ---------------------------------------------------------------------------

fn system_design_blueprint() -> Skill {
    Skill::new(
        "arch_system_design_blueprint",
        "System Design Blueprint",
        "Produce a complete system design from high-level requirements: component \
         inventory, interaction diagrams, data flow maps, scaling strategy, \
         failure mode analysis, and technology selection rationale.",
        SkillCategory::Architecture,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::Cto],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are a principal systems architect producing a comprehensive design blueprint.\n\n\
         DELIVERABLES:\n\
         1. **Component Inventory** -- every service, data store, queue, and external dependency \
            with a one-line purpose.\n\
         2. **Interaction Diagram** -- ASCII or Mermaid sequence/flow showing request paths.\n\
         3. **Data Flow Map** -- how data enters, transforms, persists, and exits the system.\n\
         4. **Scaling Strategy** -- horizontal/vertical axes, auto-scaling triggers, capacity model.\n\
         5. **Failure Mode Analysis** -- single points of failure, blast radius, recovery playbook.\n\
         6. **Technology Selection** -- concrete tech choices with trade-off rationale.\n\n\
         Anchor every decision in the stated requirements. Flag assumptions explicitly. \
         Prefer battle-tested, boring technology unless requirements demand otherwise.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
}

fn domain_driven_design() -> Skill {
    Skill::new(
        "arch_domain_driven_design",
        "Domain-Driven Design",
        "Decompose a problem domain into bounded contexts, aggregates, entities, \
         value objects, domain events, and anti-corruption layers.",
        SkillCategory::Architecture,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::Cto],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are a domain modeling expert applying Domain-Driven Design (DDD) principles.\n\n\
         DELIVERABLES:\n\
         1. **Context Map** -- bounded contexts and their relationships (Shared Kernel, \
            Customer-Supplier, Conformist, Anti-Corruption Layer, Open Host Service, \
            Published Language).\n\
         2. **Aggregates** -- aggregate roots with invariants they protect.\n\
         3. **Entities & Value Objects** -- identity-bearing vs immutable value types.\n\
         4. **Domain Events** -- events emitted by each aggregate with payload schemas.\n\
         5. **Anti-Corruption Layers** -- translation boundaries between contexts.\n\
         6. **Ubiquitous Language Glossary** -- precise domain terms used by code and stakeholders.\n\n\
         Ground every modeling decision in the domain language. Avoid technical jargon that \
         leaks infrastructure concerns into the domain layer.",
    )
    .with_quality_threshold(0.85)
}

fn microservice_decomposition() -> Skill {
    Skill::new(
        "arch_microservice_decomposition",
        "Microservice Decomposition",
        "Decompose a monolith into microservices with clear service boundaries, \
         data ownership rules, and inter-service communication patterns.",
        SkillCategory::Architecture,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::Cto],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are a distributed-systems architect decomposing a monolith into microservices.\n\n\
         DELIVERABLES:\n\
         1. **Service Inventory** -- each service with its single responsibility, team owner, \
            and SLA requirements.\n\
         2. **Data Ownership** -- which service owns which data entities; no shared databases.\n\
         3. **Communication Patterns** -- synchronous (REST/gRPC) vs asynchronous (events/queues) \
            per interaction; justify each choice.\n\
         4. **Transaction Boundaries** -- where distributed transactions are needed and the \
            pattern used (saga, outbox, two-phase).\n\
         5. **Shared Libraries** -- minimal shared code; specify what goes in shared libs vs \
            duplicated per service.\n\
         6. **Migration Sequence** -- ordered extraction plan from monolith to services with \
            parallel-run validation.\n\n\
         Warn about distributed monolith anti-patterns. Prefer coarse-grained services initially; \
         split further only when team or scaling pressure demands it.",
    )
    .with_quality_threshold(0.85)
    .with_dependencies(vec![SkillId::new("arch_domain_driven_design")])
}

fn event_driven_architecture() -> Skill {
    Skill::new(
        "arch_event_driven_architecture",
        "Event-Driven Architecture",
        "Design an event bus topology with event schemas, saga orchestration, \
         and eventual consistency patterns.",
        SkillCategory::Architecture,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are an event-driven architecture specialist.\n\n\
         DELIVERABLES:\n\
         1. **Event Bus Topology** -- broker technology (Kafka, NATS, RabbitMQ), topic/channel \
            structure, partitioning strategy, retention policy.\n\
         2. **Event Catalog** -- every event type with versioned schema (CloudEvents envelope), \
            producer, and consumer list.\n\
         3. **Saga Orchestration** -- long-running workflows as sagas with compensating actions \
            for each step; orchestrator vs choreography decision per saga.\n\
         4. **Eventual Consistency** -- read-model projection strategy, acceptable staleness \
            windows, conflict resolution (last-writer-wins, CRDTs, manual).\n\
         5. **Dead Letter Handling** -- poison message routing, alerting, replay mechanism.\n\
         6. **Idempotency** -- deduplication keys, at-least-once delivery guarantees, \
            consumer idempotency patterns.\n\n\
         Design for observability: every event must carry a correlation ID and trace context.",
    )
    .with_quality_threshold(0.80)
}

fn clean_architecture() -> Skill {
    Skill::new(
        "arch_clean_architecture",
        "Clean Architecture",
        "Design a layered architecture with dependency inversion: use cases, \
         ports/adapters, and strict domain isolation.",
        SkillCategory::Architecture,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a software architect applying Clean Architecture (Uncle Bob) principles.\n\n\
         DELIVERABLES:\n\
         1. **Layer Diagram** -- Entities (innermost), Use Cases, Interface Adapters, \
            Frameworks & Drivers (outermost). Show dependency arrows pointing inward only.\n\
         2. **Use Case Inventory** -- each application use case as an interactor with input/output \
            port interfaces.\n\
         3. **Port Definitions** -- inbound ports (driving) and outbound ports (driven) with \
            trait/interface signatures.\n\
         4. **Adapter Mapping** -- concrete adapters for each port (e.g., PostgresRepository \
            implements OrderRepository port).\n\
         5. **Dependency Injection** -- wiring strategy without leaking inner-layer details to \
            outer layers.\n\
         6. **Testing Strategy** -- how each layer is tested in isolation with mock adapters.\n\n\
         The domain layer must have ZERO dependencies on frameworks, databases, or HTTP.",
    )
    .with_quality_threshold(0.80)
}

fn hexagonal_architecture() -> Skill {
    Skill::new(
        "arch_hexagonal_architecture",
        "Hexagonal Architecture",
        "Design a ports-and-adapters architecture with primary/secondary adapters \
         and strict application core isolation.",
        SkillCategory::Architecture,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a software architect applying Hexagonal Architecture (Alistair Cockburn).\n\n\
         DELIVERABLES:\n\
         1. **Hexagon Diagram** -- application core in the center; primary (driving) adapters \
            on the left, secondary (driven) adapters on the right.\n\
         2. **Primary Ports** -- interfaces the outside world uses to drive the application \
            (HTTP handlers, CLI commands, message consumers).\n\
         3. **Secondary Ports** -- interfaces the application uses to reach external systems \
            (repositories, notification gateways, payment providers).\n\
         4. **Primary Adapters** -- concrete implementations translating external input into \
            port calls (REST controller, gRPC handler).\n\
         5. **Secondary Adapters** -- concrete implementations fulfilling outbound port contracts \
            (Postgres adapter, S3 adapter, SMTP adapter).\n\
         6. **Configuration Shell** -- how adapters are wired to ports at startup; environment-based \
            adapter selection for test vs production.\n\n\
         Ensure the application core is framework-agnostic and fully testable with in-memory adapters.",
    )
    .with_quality_threshold(0.80)
}

fn api_gateway_design() -> Skill {
    Skill::new(
        "arch_api_gateway_design",
        "API Gateway Design",
        "Design an API gateway with routing, aggregation, protocol translation, \
         rate limiting, and authentication patterns.",
        SkillCategory::Architecture,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend, AgentRole::Security],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are an API infrastructure architect designing an API gateway layer.\n\n\
         DELIVERABLES:\n\
         1. **Routing Rules** -- path-based, header-based, and weighted routing with \
            canary/blue-green support.\n\
         2. **Request Aggregation** -- fan-out/fan-in patterns for composite API responses \
            from multiple backend services.\n\
         3. **Protocol Translation** -- REST-to-gRPC, WebSocket upgrade, GraphQL-to-REST \
            translation layers.\n\
         4. **Rate Limiting** -- per-client, per-endpoint, sliding-window algorithm, \
            quota headers (X-RateLimit-*).\n\
         5. **Authentication & Authorization** -- JWT validation, OAuth2 token introspection, \
            API key management, RBAC enforcement at the gateway.\n\
         6. **Observability** -- request logging, distributed tracing propagation, \
            latency histograms, error-rate alerting.\n\n\
         Recommend a concrete gateway technology (Kong, Envoy, AWS API Gateway, custom) \
         with trade-off justification.",
    )
    .with_quality_threshold(0.80)
}

fn service_mesh_design() -> Skill {
    Skill::new(
        "arch_service_mesh_design",
        "Service Mesh Design",
        "Design a service mesh with sidecar proxies, mTLS, traffic management, \
         observability, and policy enforcement (Istio/Linkerd patterns).",
        SkillCategory::Architecture,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::DevOps],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a platform engineer designing a service mesh for a microservice fleet.\n\n\
         DELIVERABLES:\n\
         1. **Sidecar Proxy Config** -- proxy technology (Envoy/Linkerd-proxy), injection \
            strategy (auto/manual), resource limits per sidecar.\n\
         2. **mTLS Setup** -- certificate authority, cert rotation policy, SPIFFE identity \
            scheme, permissive-to-strict migration plan.\n\
         3. **Traffic Management** -- traffic splitting, retries, timeouts, circuit breaking, \
            fault injection for chaos testing.\n\
         4. **Observability Integration** -- metrics export (Prometheus), distributed tracing \
            (Jaeger/Tempo), access logging, service topology map.\n\
         5. **Policy Enforcement** -- authorization policies (namespace/service/method granularity), \
            rate limiting, header manipulation.\n\
         6. **Control Plane HA** -- control plane redundancy, upgrade strategy (canary control \
            plane), data plane drain during upgrades.\n\n\
         Specify Istio or Linkerd (or both) with selection rationale. Warn about latency overhead \
         and sidecar resource consumption.",
    )
    .with_quality_threshold(0.80)
}

fn data_mesh_architecture() -> Skill {
    Skill::new(
        "arch_data_mesh_architecture",
        "Data Mesh Architecture",
        "Design a data mesh with domain-oriented data ownership, self-serve data \
         platform, and federated computational governance.",
        SkillCategory::Architecture,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::Cto],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a data architecture lead designing a data mesh.\n\n\
         DELIVERABLES:\n\
         1. **Domain Data Products** -- each domain team's data products with SLOs, schema, \
            access patterns, and ownership.\n\
         2. **Self-Serve Data Platform** -- infrastructure abstractions (storage provisioning, \
            pipeline templates, schema registry, access control) that domain teams consume.\n\
         3. **Federated Governance** -- global interoperability standards (naming conventions, \
            data classification, SLO baselines) enforced via automated policy checks.\n\
         4. **Data Product Contracts** -- versioned schema contracts, backward compatibility \
            rules, deprecation lifecycle.\n\
         5. **Discovery & Catalog** -- data product catalog with search, lineage tracking, \
            quality scores, and consumer subscription model.\n\
         6. **Cross-Domain Queries** -- how consumers join data across domain products without \
            violating ownership boundaries (materialized views, federated query engine).\n\n\
         Address organizational change management: team structure, incentive alignment, and \
         phased rollout from centralized data warehouse to mesh.",
    )
    .with_quality_threshold(0.85)
}

fn cell_based_architecture() -> Skill {
    Skill::new(
        "arch_cell_based_architecture",
        "Cell-Based Architecture",
        "Design cell isolation for blast radius reduction with cell routing, \
         independent scaling, and failure containment.",
        SkillCategory::Architecture,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::DevOps],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a reliability architect designing a cell-based deployment topology.\n\n\
         DELIVERABLES:\n\
         1. **Cell Definition** -- what constitutes a cell (full stack replica, regional shard, \
            tenant partition); cell size heuristic.\n\
         2. **Cell Router** -- routing layer that maps requests to cells (tenant-based, \
            geography-based, hash-ring); sticky session handling.\n\
         3. **Blast Radius Analysis** -- maximum impact of a single cell failure as a percentage \
            of total traffic; target < 5%.\n\
         4. **Independent Scaling** -- per-cell auto-scaling policies, capacity reservation, \
            hot/cold cell management.\n\
         5. **Cell Deployment** -- progressive rollout across cells (canary cell, ring-based \
            deployment), rollback per cell.\n\
         6. **Cross-Cell Operations** -- global control plane, cross-cell data replication for \
            shared state, cell evacuation runbook.\n\n\
         Justify cell granularity against infrastructure cost. Warn about cross-cell consistency \
         challenges and operational complexity.",
    )
    .with_quality_threshold(0.85)
}

fn strangler_fig_pattern() -> Skill {
    Skill::new(
        "arch_strangler_fig_pattern",
        "Strangler Fig Pattern",
        "Plan an incremental migration from legacy to modern system using facade \
         routing, parallel runs, and verification strategies.",
        SkillCategory::Architecture,
        SkillComplexity::Pipeline,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a migration architect applying the Strangler Fig pattern.\n\n\
         DELIVERABLES:\n\
         1. **Facade Layer** -- routing proxy that dispatches to legacy or new system per \
            endpoint/feature; technology choice and deployment.\n\
         2. **Migration Sequence** -- ordered list of features/endpoints to migrate, prioritized \
            by risk and business value; each with estimated effort.\n\
         3. **Parallel Run Strategy** -- dual-write/dual-read with result comparison; \
            discrepancy logging and alerting thresholds.\n\
         4. **Data Migration** -- per-feature data migration plan with rollback, consistency \
            checks, and backfill scripts.\n\
         5. **Verification Gates** -- automated checks that must pass before traffic is fully \
            cut over (latency parity, error rate parity, data integrity).\n\
         6. **Legacy Decommission** -- cleanup plan for removing legacy code, infra, and data \
            after full cutover; deprecation timeline.\n\n\
         Emphasize safety: every step must be reversible. Parallel run before cutover is mandatory.",
    )
    .with_quality_threshold(0.80)
}

fn backpressure_design() -> Skill {
    Skill::new(
        "arch_backpressure_design",
        "Backpressure Design",
        "Design flow control across async boundaries with buffering strategies, \
         load shedding, and adaptive rate control.",
        SkillCategory::Architecture,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a systems engineer specializing in backpressure and flow control.\n\n\
         DELIVERABLES:\n\
         1. **Pressure Points** -- identify every async boundary (queue, stream, API call) \
            where backpressure must be applied.\n\
         2. **Buffering Strategy** -- bounded vs unbounded buffers, buffer sizing formula, \
            overflow policy (drop-oldest, drop-newest, block).\n\
         3. **Load Shedding** -- priority-based shedding (shed low-priority work first), \
            HTTP 429/503 response strategy, client retry guidance.\n\
         4. **Adaptive Rate Control** -- token bucket, leaky bucket, or AIMD algorithm per \
            boundary; dynamic adjustment based on downstream health.\n\
         5. **Monitoring** -- queue depth metrics, processing lag, shed-rate dashboards, \
            alerting thresholds for capacity planning.\n\
         6. **Graceful Degradation** -- feature flags for shedding non-critical features \
            under load; user-facing degradation messaging.\n\n\
         Every design must prevent cascading failures. Prove the system degrades gracefully \
         rather than collapsing under sustained overload.",
    )
    .with_quality_threshold(0.80)
}

fn bulkhead_pattern() -> Skill {
    Skill::new(
        "arch_bulkhead_pattern",
        "Bulkhead Pattern",
        "Design resource isolation using thread pools, connection pools, and \
         per-dependency circuit breakers to contain failures.",
        SkillCategory::Architecture,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a resilience engineer applying the Bulkhead pattern.\n\n\
         DELIVERABLES:\n\
         1. **Resource Partitioning** -- separate thread pools, connection pools, and memory \
            budgets per downstream dependency.\n\
         2. **Pool Sizing** -- sizing formula based on expected throughput, latency percentiles, \
            and Little's Law; headroom for bursts.\n\
         3. **Circuit Breakers** -- per-dependency circuit breaker with open/half-open/closed \
            states, failure-rate thresholds, and recovery probes.\n\
         4. **Timeout Hierarchy** -- layered timeouts (connection, read, overall request) \
            that prevent thread starvation.\n\
         5. **Fallback Behavior** -- what each bulkhead returns when its dependency is isolated \
            (cached response, default value, graceful error).\n\
         6. **Dashboard** -- per-bulkhead metrics (active threads, queue depth, rejection rate, \
            circuit state) with alerting rules.\n\n\
         Goal: a failure in dependency A must not degrade requests to dependency B.",
    )
    .with_quality_threshold(0.80)
}

fn capability_mapping() -> Skill {
    Skill::new(
        "arch_capability_mapping",
        "Capability Mapping",
        "Map business capabilities to services with team topology alignment \
         and cognitive load analysis.",
        SkillCategory::Architecture,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Cto],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are an engineering leader aligning architecture to business capabilities.\n\n\
         DELIVERABLES:\n\
         1. **Capability Tree** -- hierarchical business capability map (Level 0 = enterprise, \
            Level 1 = business area, Level 2 = capability, Level 3 = sub-capability).\n\
         2. **Service-to-Capability Map** -- each service mapped to the capability it realizes; \
            identify services spanning multiple capabilities (candidates for splitting).\n\
         3. **Team Topology** -- stream-aligned teams per capability area, platform teams for \
            shared infrastructure, enabling teams, complicated-subsystem teams.\n\
         4. **Cognitive Load Assessment** -- per-team cognitive load score; flag teams owning \
            too many services or too broad a capability scope.\n\
         5. **Interaction Modes** -- collaboration, X-as-a-Service, or facilitation between \
            each team pair; minimize collaboration dependencies.\n\
         6. **Evolution Plan** -- phased team and service restructuring to reduce coupling \
            and cognitive load over 2-4 quarters.\n\n\
         Align Conway's Law in your favor: design the org so the communication structure \
         produces the desired architecture.",
    )
    .with_quality_threshold(0.85)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_architecture_skills() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        assert_eq!(registry.by_category(SkillCategory::Architecture).len(), 14);
    }

    #[test]
    fn test_architect_can_access_all() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        let architect_skills = registry.by_agent(AgentRole::Architect);
        assert_eq!(architect_skills.len(), 14);
    }

    #[test]
    fn test_microservice_depends_on_ddd() {
        let skill = microservice_decomposition();
        assert!(skill
            .dependencies
            .contains(&SkillId::new("arch_domain_driven_design")));
    }

    #[test]
    fn test_system_design_high_quality_threshold() {
        let skill = system_design_blueprint();
        assert!(skill.quality_threshold >= 0.85);
    }
}
