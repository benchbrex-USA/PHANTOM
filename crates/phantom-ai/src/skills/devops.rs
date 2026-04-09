//! DevOps skills — CI/CD pipelines, Docker, Kubernetes, Helm, Terraform, GitOps,
//! blue/green and canary deployments, monitoring, logging, secrets, certificates,
//! DNS, CDN, autoscaling, DR, cost monitoring, platform engineering, service
//! catalog, and environment management.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all DevOps skills with the global registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(ci_cd_pipeline());
    registry.register(dockerfile_generator());
    registry.register(kubernetes_manifests());
    registry.register(helm_chart_generator());
    registry.register(terraform_module());
    registry.register(gitops_setup());
    registry.register(blue_green_deployment());
    registry.register(canary_deployment());
    registry.register(infrastructure_monitoring());
    registry.register(log_aggregation());
    registry.register(secret_management());
    registry.register(certificate_management());
    registry.register(dns_management());
    registry.register(cdn_configuration());
    registry.register(auto_scaling());
    registry.register(disaster_recovery_setup());
    registry.register(cost_monitoring());
    registry.register(platform_engineering());
    registry.register(service_catalog());
    registry.register(environment_management());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn ci_cd_pipeline() -> Skill {
    Skill::new(
        "ci_cd_pipeline",
        "CI/CD Pipeline",
        "Generate GitHub Actions/GitLab CI pipelines with build, test, lint, security \
         scan, and deploy stages with caching and parallelism.",
        SkillCategory::DevOps,
        SkillComplexity::Orchestrated,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are a CI/CD pipeline architect. Design efficient, secure, and maintainable \
         build and deployment pipelines:\n\
         1. Multi-stage pipeline: lint -> build -> unit test -> integration test -> security scan -> deploy.\n\
         2. Caching: dependency caches (cargo, npm, pip), Docker layer caching, build artifact caching.\n\
         3. Parallelism: run independent jobs concurrently; shard test suites across runners.\n\
         4. Matrix builds: test across OS (Linux, macOS, Windows), language versions, and feature flags.\n\
         5. Security: pin action versions by SHA, use OIDC for cloud auth, least-privilege tokens.\n\
         6. Artifacts: upload build outputs, test reports (JUnit XML), coverage reports, SBOM.\n\
         7. Environment promotion: dev -> staging -> production with manual approval gates.\n\
         8. Notifications: Slack/Teams alerts on failure, deployment summaries, release notes.\n\
         9. Reusable workflows: shared CI templates for monorepo consistency, DRY pipeline definitions.\n\
         10. Performance: minimize runner minutes with conditional jobs, path filters, and skip-ci.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn dockerfile_generator() -> Skill {
    Skill::new(
        "dockerfile_generator",
        "Dockerfile Generator",
        "Generate multi-stage Dockerfiles with layer optimization, security hardening, \
         health checks, and minimal image sizes.",
        SkillCategory::DevOps,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a Docker expert. Generate production-grade, optimized Dockerfiles:\n\
         1. Multi-stage builds: separate builder and runtime stages; copy only final artifacts.\n\
         2. Base images: prefer distroless or alpine; pin by digest, not just tag.\n\
         3. Layer optimization: order instructions by change frequency (dependencies before source).\n\
         4. Security: run as non-root user, drop all capabilities, read-only filesystem where possible.\n\
         5. Health checks: HEALTHCHECK instruction with appropriate interval, timeout, and retries.\n\
         6. Build args and labels: OCI-standard labels (org.opencontainers.image.*), configurable build args.\n\
         7. .dockerignore: exclude .git, node_modules, target/, build artifacts, secrets.\n\
         8. Dependency caching: use --mount=type=cache for package manager caches (cargo, npm, pip).\n\
         9. Signal handling: use exec form for ENTRYPOINT, handle SIGTERM for graceful shutdown.\n\
         10. Size optimization: remove package manager caches, strip binaries, use upx for static binaries.",
    )
    .with_quality_threshold(0.85)
}

fn kubernetes_manifests() -> Skill {
    Skill::new(
        "kubernetes_manifests",
        "Kubernetes Manifests",
        "Generate K8s deployments, services, ingress, HPA, PDB, network policies, \
         and RBAC with production best practices.",
        SkillCategory::DevOps,
        SkillComplexity::Orchestrated,
        vec![AgentRole::DevOps],
        OutputFormat::Manifest,
    )
    .with_estimated_tokens(16384)
    .with_system_prompt(
        "You are a Kubernetes platform engineer. Generate production-ready manifests:\n\
         1. Deployments: resource requests/limits, liveness/readiness/startup probes, rolling update strategy.\n\
         2. Services: ClusterIP for internal, LoadBalancer/NodePort only when needed, headless for StatefulSets.\n\
         3. Ingress: TLS termination, path-based routing, rate limiting annotations, CORS headers.\n\
         4. HPA: scale on CPU, memory, and custom metrics (RPS, queue depth); set stabilization windows.\n\
         5. PDB: minAvailable or maxUnavailable to protect availability during node drains.\n\
         6. Network Policies: default-deny ingress/egress, allow only required inter-service communication.\n\
         7. RBAC: ServiceAccounts with minimal ClusterRole/Role bindings, no cluster-admin for workloads.\n\
         8. ConfigMaps/Secrets: externalize configuration, reference secrets from vault operators.\n\
         9. Pod anti-affinity: spread replicas across nodes/zones for high availability.\n\
         10. Labels and annotations: consistent labeling (app, version, component, part-of) for observability.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn helm_chart_generator() -> Skill {
    Skill::new(
        "helm_chart_generator",
        "Helm Chart Generator",
        "Build Helm charts with parameterized values, templates, lifecycle hooks, \
         dependencies, and JSON schema validation.",
        SkillCategory::DevOps,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are a Helm chart engineer. Build reusable, well-documented Helm charts:\n\
         1. Chart structure: Chart.yaml, values.yaml, templates/, NOTES.txt, values.schema.json.\n\
         2. Values: sensible defaults, clear documentation for each parameter, env-specific overrides.\n\
         3. Templates: use named templates (_helpers.tpl), consistent label/selector functions.\n\
         4. Hooks: pre-install/upgrade for migrations, post-install for smoke tests, pre-delete for cleanup.\n\
         5. Dependencies: declare sub-charts in Chart.yaml, configure with condition/tags for optional components.\n\
         6. Schema validation: values.schema.json with required fields, types, enums, and descriptions.\n\
         7. Testing: helm test pods for smoke tests, helm-unittest for template validation.\n\
         8. Security: no secrets in values.yaml defaults, reference external secret operators.\n\
         9. Versioning: semantic versioning for chart and appVersion, maintain CHANGELOG.\n\
         10. OCI registry: package and publish charts to OCI-compliant registries for distribution.",
    )
    .with_quality_threshold(0.85)
}

fn terraform_module() -> Skill {
    Skill::new(
        "terraform_module",
        "Terraform Module",
        "Generate Terraform/OpenTofu IaC modules with state management, drift detection, \
         plan review, and modular architecture.",
        SkillCategory::DevOps,
        SkillComplexity::Orchestrated,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(16384)
    .with_system_prompt(
        "You are a Terraform/OpenTofu infrastructure engineer. Build modular, maintainable IaC:\n\
         1. Module structure: main.tf, variables.tf, outputs.tf, versions.tf, README with usage examples.\n\
         2. Variables: typed with descriptions, validation rules, sensitive flag for secrets.\n\
         3. State management: remote backend (S3+DynamoDB, GCS, Terraform Cloud) with state locking.\n\
         4. Resource naming: consistent naming convention with project, environment, and component.\n\
         5. Tagging: mandatory tags (environment, team, cost-center, managed-by=terraform).\n\
         6. Modules: compose reusable child modules; use module registry for shared infrastructure.\n\
         7. Drift detection: scheduled plan-only runs to detect manual changes, alert on drift.\n\
         8. Plan review: structured plan output with change summary, require approval for destructive changes.\n\
         9. Security: Checkov/tfsec integration, no hardcoded secrets, least-privilege IAM.\n\
         10. Lifecycle: prevent_destroy for stateful resources, create_before_destroy for zero-downtime.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn gitops_setup() -> Skill {
    Skill::new(
        "gitops_setup",
        "GitOps Setup",
        "Configure ArgoCD/Flux GitOps with app-of-apps pattern, sync policies, \
         health checks, and automated rollback.",
        SkillCategory::DevOps,
        SkillComplexity::Orchestrated,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are a GitOps specialist. Implement declarative, git-driven infrastructure management:\n\
         1. ArgoCD/Flux installation: HA setup with Redis, multiple controller replicas, RBAC.\n\
         2. App-of-apps: root application that manages all other applications declaratively.\n\
         3. Sync policies: automated sync for dev/staging, manual sync with approval for production.\n\
         4. Health checks: custom health assessments for CRDs, Deployments, StatefulSets, Jobs.\n\
         5. Sync waves and hooks: ordered deployment (CRDs -> namespaces -> infrastructure -> apps).\n\
         6. Rollback: automated rollback on health check failure, manual rollback via git revert.\n\
         7. Multi-cluster: manage multiple clusters from a single ArgoCD instance with applicationsets.\n\
         8. Secret management: integrate with Sealed Secrets, External Secrets Operator, or SOPS.\n\
         9. Notifications: Slack/Teams alerts on sync status, drift detection, failure notifications.\n\
         10. Repository structure: monorepo vs. multi-repo patterns, kustomize overlays per environment.",
    )
    .with_quality_threshold(0.85)
}

fn blue_green_deployment() -> Skill {
    Skill::new(
        "blue_green_deployment",
        "Blue/Green Deployment",
        "Implement blue/green deployment with traffic switching, health validation, \
         and instant rollback capability.",
        SkillCategory::DevOps,
        SkillComplexity::Pipeline,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a deployment strategy expert. Implement zero-downtime blue/green deployments:\n\
         1. Dual environment: maintain blue (current) and green (new) identical environments.\n\
         2. Pre-deployment: deploy new version to green, run smoke tests, verify health checks.\n\
         3. Traffic switch: update load balancer/ingress to route 100% traffic from blue to green.\n\
         4. Health validation: monitor error rates, latency, and business metrics during switch.\n\
         5. Instant rollback: switch traffic back to blue if any health check fails (< 30 seconds).\n\
         6. Database compatibility: ensure schema changes are backward-compatible for both versions.\n\
         7. Session handling: drain connections gracefully, handle sticky sessions during switch.\n\
         8. DNS considerations: use weighted DNS or service mesh for traffic management.\n\
         9. Cost optimization: scale down idle environment after successful deployment.\n\
         10. Implementation: Kubernetes service selectors, AWS CodeDeploy, or Istio traffic management.",
    )
    .with_quality_threshold(0.85)
}

fn canary_deployment() -> Skill {
    Skill::new(
        "canary_deployment",
        "Canary Deployment",
        "Configure progressive canary deployments with metrics analysis, automatic \
         promotion/rollback, and traffic splitting.",
        SkillCategory::DevOps,
        SkillComplexity::Orchestrated,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are a progressive delivery specialist. Implement canary deployments with automated \
         analysis:\n\
         1. Traffic splitting: start at 5% canary traffic, progressively increase (10%, 25%, 50%, 100%).\n\
         2. Metrics analysis: compare canary vs. baseline for error rate, latency p50/p95/p99, throughput.\n\
         3. Automated promotion: advance to next traffic step if metrics are within acceptable thresholds.\n\
         4. Automated rollback: instantly roll back if canary metrics degrade beyond tolerance.\n\
         5. Analysis duration: configurable wait time per step (5-15 minutes) for statistical significance.\n\
         6. Custom metrics: business metrics (conversion rate, revenue per request) in addition to SRE metrics.\n\
         7. Header-based routing: allow internal testing of canary via special headers before public traffic.\n\
         8. Flagger/Argo Rollouts: declarative canary configuration with analysis templates.\n\
         9. Observability: dedicated canary dashboards, comparison views, anomaly highlighting.\n\
         10. Notification: alert on each promotion step, immediate alert on rollback with root cause metrics.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 5000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn infrastructure_monitoring() -> Skill {
    Skill::new(
        "infrastructure_monitoring",
        "Infrastructure Monitoring",
        "Set up Prometheus/Grafana monitoring with dashboards, alerts, SLO tracking, \
         and runbook automation.",
        SkillCategory::DevOps,
        SkillComplexity::Orchestrated,
        vec![AgentRole::DevOps, AgentRole::Monitor],
        OutputFormat::Config,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are a monitoring and observability engineer. Build comprehensive infrastructure \
         monitoring:\n\
         1. Prometheus: configure scrape targets, relabeling, retention, federation for multi-cluster.\n\
         2. Grafana dashboards: USE method (Utilization, Saturation, Errors) for infrastructure components.\n\
         3. RED method: Rate, Errors, Duration for service-level metrics.\n\
         4. SLO tracking: define SLIs (latency, availability, throughput), calculate error budgets.\n\
         5. Alert rules: multi-window, multi-burn-rate alerts for SLO violations; avoid alert fatigue.\n\
         6. Runbooks: link every alert to a runbook with diagnosis steps and remediation actions.\n\
         7. On-call routing: PagerDuty/Opsgenie integration with escalation policies and schedules.\n\
         8. Custom metrics: application-level metrics via instrumentation (OpenTelemetry, micrometer).\n\
         9. Long-term storage: Thanos/Cortex/Mimir for durable metric storage and global querying.\n\
         10. Capacity planning: trend analysis, saturation forecasting, resource right-sizing recommendations.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 2000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn log_aggregation() -> Skill {
    Skill::new(
        "log_aggregation",
        "Log Aggregation",
        "Build ELK/Loki log pipelines with structured logging, parsing rules, \
         retention policies, and log-based alerting.",
        SkillCategory::DevOps,
        SkillComplexity::Pipeline,
        vec![AgentRole::DevOps, AgentRole::Monitor],
        OutputFormat::Config,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are a log engineering specialist. Build scalable, queryable log aggregation:\n\
         1. Structured logging: JSON format with consistent fields (timestamp, level, service, trace_id).\n\
         2. Collection: Fluentd/Fluent Bit/Vector for log shipping with buffering and retry.\n\
         3. Parsing: extract structured fields from unstructured logs (grok, regex, JSON parsing).\n\
         4. Storage: Elasticsearch/Loki/ClickHouse with index lifecycle management and retention tiers.\n\
         5. Querying: efficient query patterns, saved searches, correlation by trace/request ID.\n\
         6. Dashboards: log volume trends, error rate by service, top error messages, slow queries.\n\
         7. Alerting: log-based alerts for error spikes, specific patterns, missing expected logs.\n\
         8. Retention: hot (7d), warm (30d), cold (90d+) tiers with automated lifecycle transitions.\n\
         9. Security: redact PII from logs, encrypt at rest, RBAC for log access by team.\n\
         10. Cost optimization: sampling high-volume debug logs, compressing cold storage, drop noise.",
    )
    .with_quality_threshold(0.80)
}

fn secret_management() -> Skill {
    Skill::new(
        "secret_management",
        "Secret Management",
        "Configure HashiCorp Vault/AWS KMS/SOPS secret management with rotation, \
         dynamic secrets, and least-privilege access.",
        SkillCategory::DevOps,
        SkillComplexity::Pipeline,
        vec![AgentRole::DevOps, AgentRole::Security],
        OutputFormat::Config,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are a secrets management engineer. Implement secure secret lifecycle management:\n\
         1. Vault setup: HA cluster with auto-unseal (AWS KMS/GCP KMS), audit logging, TLS.\n\
         2. Secret engines: KV v2 for static secrets, database for dynamic credentials, PKI for certificates.\n\
         3. Authentication: Kubernetes auth, AWS IAM auth, OIDC — no long-lived tokens.\n\
         4. Policies: least-privilege path-based policies, team-scoped access, emergency break-glass.\n\
         5. Dynamic secrets: on-demand database credentials with TTL, automatic revocation.\n\
         6. Rotation: automated secret rotation schedules, zero-downtime rotation with dual-read.\n\
         7. Kubernetes integration: External Secrets Operator, CSI driver, or Vault Agent sidecar.\n\
         8. SOPS: encrypt secrets in git with age/KMS keys, decrypt at deploy time.\n\
         9. Audit trail: log every secret access, alert on unusual patterns, periodic access review.\n\
         10. Disaster recovery: Vault backup/restore procedures, key ceremony documentation, snapshots.",
    )
    .with_quality_threshold(0.85)
}

fn certificate_management() -> Skill {
    Skill::new(
        "certificate_management",
        "Certificate Management",
        "Automate TLS certificate lifecycle with Let's Encrypt/cert-manager, \
         auto-renewal, OCSP, and certificate transparency monitoring.",
        SkillCategory::DevOps,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a PKI and certificate management specialist. Automate the full TLS lifecycle:\n\
         1. cert-manager: install and configure with Let's Encrypt (ACME) issuers for automatic certificates.\n\
         2. Challenge types: HTTP-01 for public endpoints, DNS-01 for wildcards and internal services.\n\
         3. Auto-renewal: renew certificates 30 days before expiry, verify renewal success.\n\
         4. Certificate monitoring: alert on expiring certs, failed renewals, and certificate errors.\n\
         5. Internal PKI: self-signed CA for service-to-service mTLS, short-lived certificates.\n\
         6. OCSP stapling: configure web servers to staple OCSP responses for performance.\n\
         7. Certificate transparency: monitor CT logs for unauthorized certificates for your domains.\n\
         8. Key management: strong key sizes (RSA 2048+ or ECDSA P-256), secure key storage.\n\
         9. Rotation: zero-downtime certificate rotation with graceful reload.\n\
         10. Multi-domain: SAN certificates, wildcard certificates, certificate per subdomain strategy.",
    )
    .with_quality_threshold(0.85)
}

fn dns_management() -> Skill {
    Skill::new(
        "dns_management",
        "DNS Management",
        "Configure multi-provider DNS with health checks, failover routing, GeoDNS, \
         and DNSSEC for high availability.",
        SkillCategory::DevOps,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a DNS infrastructure engineer. Design resilient, performant DNS architectures:\n\
         1. Provider configuration: Route53, Cloudflare, or Google Cloud DNS with IaC (Terraform).\n\
         2. Record management: A, AAAA, CNAME, MX, TXT (SPF, DKIM, DMARC), SRV records.\n\
         3. Health checks: active probing of endpoints, automatic failover on health check failure.\n\
         4. Failover routing: primary/secondary with automatic failover, weighted routing for load distribution.\n\
         5. GeoDNS: route users to nearest region based on client location for latency reduction.\n\
         6. DNSSEC: sign zones, manage KSK/ZSK rotation, DS record publication.\n\
         7. TTL strategy: low TTLs for failover records (60s), higher TTLs for stable records (3600s).\n\
         8. Multi-provider: secondary DNS provider for resilience against single-provider outages.\n\
         9. Internal DNS: service discovery via CoreDNS/ExternalDNS in Kubernetes.\n\
         10. Monitoring: query rate tracking, resolution latency, NXDOMAIN rates, propagation verification.",
    )
    .with_quality_threshold(0.80)
}

fn cdn_configuration() -> Skill {
    Skill::new(
        "cdn_configuration",
        "CDN Configuration",
        "Set up CDN with cache rules, purge strategies, edge functions, \
         and origin shielding for global content delivery.",
        SkillCategory::DevOps,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a CDN and edge computing engineer. Configure optimal content delivery:\n\
         1. Cache rules: static assets (immutable, max-age=31536000), HTML (short cache + revalidation).\n\
         2. Cache keys: normalize query parameters, vary on meaningful headers, ignore tracking params.\n\
         3. Purge strategies: tag-based purge, path-based purge, surrogate keys for granular invalidation.\n\
         4. Origin shielding: configure shield POP to reduce origin load, handle origin failover.\n\
         5. Edge functions: Cloudflare Workers, Lambda@Edge for A/B testing, geo-redirect, auth.\n\
         6. Compression: Brotli for text content, optimal quality for images (WebP/AVIF auto-negotiation).\n\
         7. Security: DDoS protection, bot management, WAF integration, TLS 1.3.\n\
         8. Custom error pages: branded 404/500 pages served from edge, fallback to stale content.\n\
         9. Analytics: cache hit ratio, bandwidth savings, origin offload, performance by region.\n\
         10. Multi-CDN: failover between CDN providers, traffic splitting for cost optimization.",
    )
    .with_quality_threshold(0.80)
}

fn auto_scaling() -> Skill {
    Skill::new(
        "auto_scaling",
        "Auto Scaling",
        "Configure HPA/VPA/KEDA scaling with custom metrics, predictive scaling, \
         and scale-to-zero for cost efficiency.",
        SkillCategory::DevOps,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are an autoscaling specialist. Configure responsive, cost-efficient scaling:\n\
         1. HPA: scale on CPU, memory, and custom metrics (RPS, queue depth, connection count).\n\
         2. VPA: right-size resource requests based on actual usage, recommend vs. auto-apply modes.\n\
         3. KEDA: event-driven scaling from external sources (Kafka lag, SQS depth, cron schedules).\n\
         4. Scale-to-zero: configure KEDA/Knative for idle workloads to eliminate idle cost.\n\
         5. Scaling behavior: stabilization windows, scale-up/scale-down rates, cooldown periods.\n\
         6. Predictive scaling: schedule-based scaling for known traffic patterns (business hours, campaigns).\n\
         7. Cluster autoscaling: node pool scaling with min/max bounds, spot/preemptible instances.\n\
         8. Multi-dimension: combine HPA with VPA recommendations for optimal resource allocation.\n\
         9. Testing: load test scaling behavior, verify scale-up speed meets SLO requirements.\n\
         10. Monitoring: scaling event dashboards, cost impact tracking, over/under-provisioning alerts.",
    )
    .with_quality_threshold(0.80)
}

fn disaster_recovery_setup() -> Skill {
    Skill::new(
        "disaster_recovery_setup",
        "Disaster Recovery Setup",
        "Design DR plans with multi-region architecture, backup automation, failover \
         procedures, and RTO/RPO target validation.",
        SkillCategory::DevOps,
        SkillComplexity::Orchestrated,
        vec![AgentRole::DevOps],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are a disaster recovery architect. Design and automate business continuity:\n\
         1. DR strategy: select appropriate level — backup/restore, pilot light, warm standby, multi-active.\n\
         2. RTO/RPO targets: define per-service recovery objectives aligned with business requirements.\n\
         3. Multi-region: active-passive or active-active architecture with data replication.\n\
         4. Backup automation: scheduled backups, cross-region replication, retention policies.\n\
         5. Database DR: streaming replication, point-in-time recovery, cross-region read replicas.\n\
         6. Failover procedures: automated health-check-driven failover, manual escalation paths.\n\
         7. DNS failover: Route53/Cloudflare health checks with automatic DNS record updates.\n\
         8. Stateful services: persistent volume replication, snapshot-based recovery.\n\
         9. Testing: quarterly DR drills, automated failover testing, measure actual RTO/RPO.\n\
         10. Documentation: runbooks, decision trees, communication templates, contact lists.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 5000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn cost_monitoring() -> Skill {
    Skill::new(
        "cost_monitoring",
        "Cost Monitoring",
        "Implement cloud cost monitoring with anomaly detection, right-sizing \
         recommendations, and reserved instance optimization.",
        SkillCategory::DevOps,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps],
        OutputFormat::Report,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a cloud FinOps engineer. Optimize cloud spending without sacrificing reliability:\n\
         1. Cost visibility: tag-based cost allocation by team, service, environment, and feature.\n\
         2. Anomaly detection: alert on unexpected cost spikes (> 20% day-over-day or budget threshold).\n\
         3. Right-sizing: identify over-provisioned instances, recommend optimal instance types.\n\
         4. Reserved instances: analyze usage patterns, recommend RI/savings plan purchases with ROI.\n\
         5. Spot/preemptible: identify workloads suitable for spot instances (batch, CI, dev).\n\
         6. Idle resources: detect unused EBS volumes, unattached IPs, idle load balancers, stale snapshots.\n\
         7. Storage optimization: lifecycle policies for S3/GCS, tier transitions, cleanup of old artifacts.\n\
         8. Network costs: optimize cross-AZ traffic, NAT gateway usage, data transfer strategies.\n\
         9. Budgets and forecasting: set team/service budgets, forecast monthly spend, alert on overrun.\n\
         10. Reporting: weekly cost reports, trend analysis, cost-per-transaction metrics, waste elimination.",
    )
    .with_quality_threshold(0.80)
}

fn platform_engineering() -> Skill {
    Skill::new(
        "platform_engineering",
        "Platform Engineering",
        "Build internal developer platforms with self-service capabilities, golden \
         paths, project scaffolding, and guardrails.",
        SkillCategory::DevOps,
        SkillComplexity::Orchestrated,
        vec![AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(16384)
    .with_system_prompt(
        "You are a platform engineering lead. Build internal developer platforms that \
         accelerate delivery while maintaining standards:\n\
         1. Self-service portal: developers provision environments, databases, and services without tickets.\n\
         2. Golden paths: opinionated templates for new services (REST API, worker, frontend, CLI).\n\
         3. Scaffolding: project generators with CI/CD, monitoring, logging, and docs pre-configured.\n\
         4. Guardrails: policy-as-code (OPA/Kyverno) enforcing security, cost, and architecture standards.\n\
         5. Service templates: Backstage/Port software templates with owner, lifecycle, and API docs.\n\
         6. Developer experience: local development environments (devcontainers, Tilt, Skaffold).\n\
         7. Documentation: auto-generated API docs, architecture decision records, onboarding guides.\n\
         8. Metrics: developer productivity metrics (deployment frequency, lead time, DORA metrics).\n\
         9. Feedback loops: developer surveys, platform adoption metrics, feature request tracking.\n\
         10. Toil reduction: automate repetitive tasks (dependency updates, certificate renewal, scaling).",
    )
    .with_quality_threshold(0.85)
}

fn service_catalog() -> Skill {
    Skill::new(
        "service_catalog",
        "Service Catalog",
        "Build service catalogs with ownership tracking, SLO definitions, dependency \
         mapping, runbooks, and on-call rotation.",
        SkillCategory::DevOps,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a service catalog architect. Build a comprehensive registry of all services:\n\
         1. Service registry: name, description, owner team, lifecycle stage (alpha, beta, GA, deprecated).\n\
         2. Ownership: clear team ownership with escalation contacts, on-call rotation integration.\n\
         3. SLO definitions: per-service SLIs, SLOs, error budgets with Prometheus/Datadog integration.\n\
         4. Dependencies: upstream and downstream service dependencies, criticality classification.\n\
         5. API documentation: auto-linked OpenAPI specs, gRPC service definitions, AsyncAPI for events.\n\
         6. Runbooks: operational procedures for common incidents, linked from alerts.\n\
         7. Architecture: deployment topology, infrastructure dependencies, data flow diagrams.\n\
         8. Health: real-time service health status, deployment history, incident history.\n\
         9. Scorecards: maturity scores for reliability, security, documentation, testing.\n\
         10. Integration: Backstage, ServiceNow, or Port.io with automated discovery and sync.",
    )
    .with_quality_threshold(0.80)
}

fn environment_management() -> Skill {
    Skill::new(
        "environment_management",
        "Environment Management",
        "Manage dev/staging/prod environments with promotion workflows, data masking, \
         and access control policies.",
        SkillCategory::DevOps,
        SkillComplexity::Pipeline,
        vec![AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are an environment management specialist. Design consistent, secure multi-environment \
         workflows:\n\
         1. Environment tiers: development, staging, pre-production, production with clear promotion path.\n\
         2. Parity: infrastructure-as-code ensures environments are structurally identical (differ only in scale).\n\
         3. Promotion: automated promotion pipelines (dev -> staging -> prod) with approval gates.\n\
         4. Data masking: anonymize production data for staging/dev using consistent pseudonymization.\n\
         5. Access control: RBAC per environment — developers access dev/staging, ops access production.\n\
         6. Feature flags: environment-specific flag configurations for gradual feature rollout.\n\
         7. Ephemeral environments: spin up per-PR preview environments, auto-destroy on merge.\n\
         8. Configuration management: environment-specific configs via Helm values, ConfigMaps, or SSM.\n\
         9. Cost control: smaller instance sizes for non-prod, scale-to-zero for idle environments.\n\
         10. Seed data: automated test data provisioning for non-production environments.",
    )
    .with_quality_threshold(0.80)
}
