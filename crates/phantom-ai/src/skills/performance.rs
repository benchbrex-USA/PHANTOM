//! Performance skills for PHANTOM's autonomous AI engineering system.
//!
//! Covers CPU/memory profiling, query optimization, bundle analysis, image
//! optimization, caching, connection pools, memory leaks, concurrency, network,
//! database indexes, API latency, frontend performance, SSR, edge computing,
//! and load shedding.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all performance skills into the provided registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(performance_profile());
    registry.register(query_performance_audit());
    registry.register(bundle_size_optimizer());
    registry.register(image_optimization());
    registry.register(caching_strategy());
    registry.register(connection_pool_tuning());
    registry.register(memory_leak_detector());
    registry.register(concurrency_optimizer());
    registry.register(network_optimizer());
    registry.register(database_index_advisor());
    registry.register(api_latency_optimizer());
    registry.register(frontend_performance());
    registry.register(server_side_rendering());
    registry.register(edge_computing());
    registry.register(load_shedding_strategy());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn performance_profile() -> Skill {
    Skill::new(
        "performance_profile",
        "Performance Profile",
        "CPU, memory, and I/O profiling with flame graph generation, call tree analysis, \
         and hot path identification.",
        SkillCategory::Performance,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a performance profiling engineer. Generate profiling instrumentation and \
         analysis pipelines that include:\n\
         - CPU profiling: sampling-based profilers (perf, async-profiler, py-spy) with \
           configurable sample rates and duration\n\
         - Memory profiling: heap allocation tracking, allocation flamegraphs, retained \
           size analysis, object histogram\n\
         - I/O profiling: disk read/write latency, syscall tracing, file descriptor analysis\n\
         - Flame graph generation: collapsed stack format, differential flame graphs for \
           before/after comparison\n\
         - Call tree analysis: inclusive vs exclusive time, self-time percentage, caller/callee \
           attribution\n\
         - Hot path identification: rank functions by cumulative CPU time, flag functions \
           exceeding threshold (>5% of total)\n\
         - Continuous profiling integration: always-on low-overhead profiling (1% sampling) \
           with Parca, Pyroscope, or Datadog Continuous Profiler\n\
         - Actionable output: specific optimization recommendations for each identified hot path",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn query_performance_audit() -> Skill {
    Skill::new(
        "query_performance_audit",
        "Query Performance Audit",
        "Detect N+1 queries, analyze slow queries, identify missing indexes, and optimize \
         query execution plans.",
        SkillCategory::Performance,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a database query performance specialist. Generate query audit analyses \
         that include:\n\
         - N+1 detection: identify ORM query patterns that issue one query per related \
           record, recommend eager loading or batch queries\n\
         - Slow query analysis: parse slow query logs, rank by total time (frequency x \
           duration), identify common patterns\n\
         - Missing index identification: analyze EXPLAIN plans for sequential scans on \
           filtered columns, recommend covering indexes\n\
         - Query plan optimization: rewrite queries to leverage indexes, eliminate \
           unnecessary joins, push predicates closer to data\n\
         - Connection overhead: identify queries that could be batched, use prepared \
           statements, or leverage connection pooling\n\
         - Lock contention analysis: identify queries holding locks for excessive duration, \
           recommend optimistic locking or row-level locks\n\
         - Pagination optimization: replace OFFSET with keyset pagination for large datasets\n\
         - ORM-specific recommendations: model-level fixes for Django, ActiveRecord, \
           SQLAlchemy, Prisma, or Diesel",
    )
    .with_quality_threshold(0.85)
}

fn bundle_size_optimizer() -> Skill {
    Skill::new(
        "bundle_size_optimizer",
        "Bundle Size Optimizer",
        "Analyze JavaScript bundles with tree-shaking effectiveness, code splitting \
         strategies, and lazy loading recommendations.",
        SkillCategory::Performance,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a frontend bundle optimization expert. Generate bundle analysis and \
         optimization recommendations that include:\n\
         - Bundle composition analysis: identify largest modules, duplicate dependencies, \
           dead code that survives tree-shaking\n\
         - Tree-shaking effectiveness: flag side-effect-ful imports preventing elimination, \
           recommend pure module annotations\n\
         - Code splitting strategy: route-based splitting, component-level dynamic imports, \
           vendor chunk separation\n\
         - Lazy loading: defer below-the-fold components, modal contents, heavy libraries \
           (charts, editors, maps) until interaction\n\
         - Dependency audit: identify bloated packages (moment.js -> date-fns, lodash -> \
           lodash-es with cherry-picking)\n\
         - Compression analysis: gzip vs brotli savings per chunk, recommend pre-compression \
           for static assets\n\
         - Budget enforcement: define per-route size budgets, fail CI when exceeded\n\
         - Source map analysis: correlate minified bundle back to source for accurate attribution",
    )
    .with_quality_threshold(0.8)
}

fn image_optimization() -> Skill {
    Skill::new(
        "image_optimization",
        "Image Optimization",
        "Optimize image delivery with modern format selection (WebP/AVIF), responsive \
         images, lazy loading, and CDN configuration.",
        SkillCategory::Performance,
        SkillComplexity::Atomic,
        vec![AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are an image optimization engineer. Generate image optimization strategies \
         that include:\n\
         - Format selection: AVIF for photographic content (best compression), WebP as \
           fallback, SVG for icons/logos, PNG only for transparency without WebP support\n\
         - Responsive images: srcset with width descriptors, sizes attribute matching layout, \
           art direction via <picture> element\n\
         - Lazy loading: native loading='lazy' for below-fold images, Intersection Observer \
           for custom behavior, placeholder blur-up or LQIP\n\
         - CDN optimization: image transformation at the edge (Cloudflare Images, Imgix), \
           cache-friendly URLs, immutable asset hashing\n\
         - Quality tuning: perceptual quality metrics (SSIM/DSSIM), 80-85% quality for JPEG/WebP \
           as default, adjustable per content type\n\
         - Dimension constraints: serve images at display size (not larger), 2x for retina max\n\
         - Preloading: <link rel='preload'> for LCP hero images, fetchpriority='high'\n\
         - Build pipeline: automated conversion in CI, manifest generation for runtime selection",
    )
    .with_quality_threshold(0.8)
}

fn caching_strategy() -> Skill {
    Skill::new(
        "caching_strategy",
        "Caching Strategy",
        "Design multi-layer caching with TTL policies, cache invalidation patterns, \
         cache warming, and thundering herd prevention.",
        SkillCategory::Performance,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a caching architecture specialist. Generate multi-layer caching strategies \
         that include:\n\
         - Layer design: browser cache (Cache-Control headers), CDN/edge cache, application \
           cache (Redis/Memcached), ORM query cache, database buffer pool\n\
         - TTL policies: short TTL for volatile data (user sessions: 15m), medium for \
           semi-static (product listings: 1h), long for static (assets: 1y with hash-busting)\n\
         - Invalidation patterns: event-driven invalidation (pub/sub on write), tag-based \
           invalidation (purge all 'product:123' entries), TTL expiry as fallback\n\
         - Cache warming: pre-populate on deploy for critical paths, background refresh \
           before expiry (stale-while-revalidate pattern)\n\
         - Thundering herd prevention: lock-based single-flight (only one request recomputes), \
           probabilistic early expiration, request coalescing\n\
         - Cache key design: deterministic, minimal, include only varying parameters, \
           namespace by version for safe rollbacks\n\
         - Monitoring: hit/miss ratio tracking, eviction rates, memory utilization alerts\n\
         - Consistency model: document staleness guarantees per cache layer for consumers",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn connection_pool_tuning() -> Skill {
    Skill::new(
        "connection_pool_tuning",
        "Connection Pool Tuning",
        "Optimize database and HTTP connection pool sizing using queuing theory with \
         timeout and lifecycle configuration.",
        SkillCategory::Performance,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Config,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a connection pool tuning specialist. Generate connection pool \
         configurations that include:\n\
         - Pool sizing: apply Little's Law (L = lambda * W) to calculate optimal pool size \
           from request rate and average query duration\n\
         - Database pools: min/max connections, connection lifetime, idle timeout, validation \
           query interval\n\
         - HTTP client pools: max connections per host, keep-alive timeout, connection TTL, \
           DNS refresh interval\n\
         - Timeout hierarchy: connection acquisition timeout < query timeout < request timeout, \
           with clear escalation\n\
         - Queue configuration: bounded wait queue with max waiters, fail-fast when queue is full\n\
         - Health checking: periodic connection validation, evict broken connections, \
           reconnect with exponential backoff\n\
         - Monitoring: track active/idle/waiting counts, acquisition latency histogram, \
           timeout and eviction rates\n\
         - Environment-specific sizing: separate configurations for read replicas vs primary, \
           OLTP vs OLAP workloads",
    )
    .with_quality_threshold(0.8)
}

fn memory_leak_detector() -> Skill {
    Skill::new(
        "memory_leak_detector",
        "Memory Leak Detector",
        "Detect memory leaks with allocation tracking, object retention analysis, and \
         GC pressure measurement.",
        SkillCategory::Performance,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a memory analysis engineer. Generate memory leak detection strategies \
         that include:\n\
         - Allocation tracking: instrument allocator to record allocation site, size, and \
           timestamp for sampled allocations\n\
         - Heap snapshot comparison: capture snapshots at intervals, diff retained object \
           graphs to identify growing collections\n\
         - Retention analysis: trace GC roots to leaked objects, identify unexpected references \
           (static fields, event listeners, closures, caches without eviction)\n\
         - GC pressure metrics: track GC pause duration, frequency, promoted bytes per cycle, \
           generation distribution\n\
         - Common leak patterns: unbounded caches, forgotten event subscriptions, closures \
           capturing large scopes, connection/stream objects not closed\n\
         - Language-specific tooling: Valgrind/ASan for C/Rust, VisualVM/MAT for JVM, \
           Chrome DevTools heap profiler for Node.js, tracemalloc for Python\n\
         - Continuous monitoring: track RSS and heap usage trends over hours/days, alert on \
           monotonic growth exceeding threshold\n\
         - Remediation guidance: specific code-level fixes for each identified leak source",
    )
    .with_quality_threshold(0.85)
}

fn concurrency_optimizer() -> Skill {
    Skill::new(
        "concurrency_optimizer",
        "Concurrency Optimizer",
        "Optimize thread pool sizing, async runtime configuration, lock contention \
         analysis, and work-stealing tuning.",
        SkillCategory::Performance,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a concurrency and parallelism optimization engineer. Generate concurrency \
         analyses and configurations that include:\n\
         - Thread pool sizing: CPU-bound pools sized to core count, I/O-bound pools sized \
           using (core_count * target_utilization * (1 + wait_time/compute_time))\n\
         - Async runtime tuning: Tokio worker threads, blocking thread pool limits, task \
           budget configuration, cooperative scheduling yields\n\
         - Lock contention analysis: identify hot locks using contention profiling, measure \
           hold times, flag locks held across await points\n\
         - Lock-free alternatives: replace Mutex with RwLock for read-heavy workloads, \
           use atomic operations, concurrent data structures (DashMap, crossbeam)\n\
         - Work-stealing tuning: balance task granularity (too fine = overhead, too coarse = \
           imbalance), configure steal batch size\n\
         - Channel selection: bounded vs unbounded, MPSC vs MPMC, backpressure configuration\n\
         - Deadlock prevention: consistent lock ordering, timeout-based acquisition, \
           lock hierarchy documentation\n\
         - Benchmark harness: generate load tests that expose concurrency bottlenecks under \
           realistic contention levels",
    )
    .with_quality_threshold(0.85)
}

fn network_optimizer() -> Skill {
    Skill::new(
        "network_optimizer",
        "Network Optimizer",
        "Optimize network performance with HTTP/2 multiplexing, connection reuse, DNS \
         prefetching, preconnect hints, and TCP tuning.",
        SkillCategory::Performance,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Config,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a network performance engineer. Generate network optimization \
         configurations that include:\n\
         - HTTP/2 multiplexing: ensure single connection per origin, configure max concurrent \
           streams, leverage server push for critical resources\n\
         - Connection reuse: HTTP keep-alive configuration, connection pooling for upstream \
           services, gRPC channel reuse\n\
         - DNS optimization: dns-prefetch for known external domains, pre-resolve at \
           application startup, configure DNS cache TTL\n\
         - Resource hints: <link rel='preconnect'> for critical third-party origins, \
           <link rel='prefetch'> for next-navigation resources\n\
         - TCP tuning: adjust initial congestion window (initcwnd), enable TCP Fast Open, \
           configure keepalive interval and probes\n\
         - TLS optimization: TLS 1.3, OCSP stapling, session resumption, certificate chain \
           optimization (minimize intermediate certs)\n\
         - Compression: enable Brotli for text resources (HTML, CSS, JS, JSON), gzip as fallback\n\
         - Edge routing: Anycast DNS, geographic load balancing, minimize round-trip distance",
    )
    .with_quality_threshold(0.8)
}

fn database_index_advisor() -> Skill {
    Skill::new(
        "database_index_advisor",
        "Database Index Advisor",
        "Analyze index usage patterns and recommend covering indexes, partial indexes, \
         and index maintenance strategies.",
        SkillCategory::Performance,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a database indexing specialist. Generate index analysis and \
         recommendations that include:\n\
         - Usage analysis: identify unused indexes consuming write overhead and storage, \
           recommend removal after validation period\n\
         - Missing index detection: analyze query patterns and EXPLAIN plans, recommend \
           indexes that eliminate sequential scans\n\
         - Covering indexes: include frequently selected columns to enable index-only scans, \
           reducing heap fetches\n\
         - Partial indexes: filter indexes to subset of rows (e.g., WHERE active = true) for \
           sparse predicates, reducing index size\n\
         - Composite index column ordering: most selective column first, equality before range, \
           match query predicate order\n\
         - Index maintenance: REINDEX schedule for bloated indexes, ANALYZE frequency for \
           statistics freshness, monitor index bloat ratio\n\
         - Expression indexes: index computed values (LOWER(email), date_trunc) used in WHERE \
           clauses\n\
         - Trade-off documentation: quantify write amplification cost vs read improvement for \
           each recommended index",
    )
    .with_quality_threshold(0.85)
}

fn api_latency_optimizer() -> Skill {
    Skill::new(
        "api_latency_optimizer",
        "API Latency Optimizer",
        "Optimize API latency with request waterfall analysis, batching, prefetching, \
         and edge caching strategies.",
        SkillCategory::Performance,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are an API latency optimization engineer. Generate latency analysis and \
         optimizations that include:\n\
         - Latency waterfall breakdown: DNS resolution, TCP handshake, TLS negotiation, \
           TTFB (server processing), content transfer for each API call\n\
         - Server processing breakdown: authentication, authorization, serialization, \
           database queries, downstream service calls, response rendering\n\
         - Request batching: combine multiple related API calls into single batch endpoints, \
           implement DataLoader pattern for GraphQL resolvers\n\
         - Prefetching: predict next-needed data from navigation patterns, issue background \
           requests before user interaction\n\
         - Edge caching: cache personalization-free responses at CDN, use Vary headers correctly, \
           implement stale-while-revalidate\n\
         - Payload optimization: field selection (sparse fieldsets), pagination, compression, \
           eliminate over-fetching\n\
         - Async processing: move non-critical work (logging, analytics, notifications) to \
           background queues, return early\n\
         - Latency budgets: allocate per-component budgets within the overall SLO target, \
           track adherence continuously",
    )
    .with_quality_threshold(0.85)
}

fn frontend_performance() -> Skill {
    Skill::new(
        "frontend_performance",
        "Frontend Performance",
        "Optimize Core Web Vitals (LCP, INP, CLS) with specific diagnostic techniques \
         and targeted fixes.",
        SkillCategory::Performance,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are a Core Web Vitals optimization engineer. Generate frontend performance \
         analyses that include:\n\
         - LCP optimization: identify LCP element, preload LCP image, eliminate render-blocking \
           resources, optimize server response time, use fetchpriority='high'\n\
         - INP optimization: break long tasks into smaller chunks (scheduler.yield), minimize \
           main thread work, defer non-critical JavaScript, optimize event handlers\n\
         - CLS optimization: set explicit dimensions on images/videos/ads, avoid injecting \
           content above viewport, use transform animations instead of layout-triggering properties\n\
         - Critical rendering path: inline critical CSS, defer non-critical styles, minimize \
           parser-blocking scripts\n\
         - Third-party impact: audit third-party script performance, lazy-load non-essential \
           widgets, use facade pattern for heavy embeds\n\
         - Font optimization: font-display: swap, preload critical fonts, subset to used \
           character ranges, use variable fonts\n\
         - Resource prioritization: Priority Hints API, preload/prefetch/preconnect strategy\n\
         - Measurement: CrUX data analysis, lab testing with Lighthouse, field data correlation",
    )
    .with_quality_threshold(0.85)
}

fn server_side_rendering() -> Skill {
    Skill::new(
        "server_side_rendering",
        "Server-Side Rendering",
        "Optimize SSR, SSG, and ISR with streaming, selective hydration, and island \
         architecture patterns.",
        SkillCategory::Performance,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a server-side rendering optimization specialist. Generate SSR \
         implementations that include:\n\
         - SSR streaming: React renderToPipeableStream / renderToReadableStream with \
           Suspense boundaries for progressive HTML delivery\n\
         - Selective hydration: prioritize interactive elements for hydration, defer \
           below-fold components, use React.lazy with Suspense\n\
         - Island architecture: identify interactive islands in otherwise static pages, \
           hydrate only island components (Astro, Fresh patterns)\n\
         - SSG for static paths: pre-render known routes at build time, generate on-demand \
           for long-tail with ISR fallback\n\
         - ISR configuration: revalidation intervals per route based on content freshness \
           requirements, on-demand revalidation webhooks\n\
         - Cache strategy: full-page cache with surrogate keys, fragment caching for \
           personalized sections, edge-side includes\n\
         - Data fetching optimization: parallel data loading, waterfall elimination, \
           shared request deduplication during SSR\n\
         - Error handling: graceful fallback to client-side rendering on SSR failure, \
           streaming error boundaries",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.8)
}

fn edge_computing() -> Skill {
    Skill::new(
        "edge_computing",
        "Edge Computing",
        "Optimize edge function deployments with cold start reduction, regional routing, \
         and KV store integration.",
        SkillCategory::Performance,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Code,
    )
    .with_estimated_tokens(5120)
    .with_system_prompt(
        "You are an edge computing optimization engineer. Generate edge function \
         implementations that include:\n\
         - Cold start reduction: minimize bundle size, avoid heavy initialization, use \
           lazy imports, keep dependencies minimal\n\
         - Regional routing: direct requests to nearest edge location, implement \
           geo-aware logic for data residency compliance\n\
         - KV store integration: Cloudflare KV, Vercel Edge Config, Deno KV for \
           low-latency configuration and session data\n\
         - Edge-origin architecture: serve from edge when possible, fall through to origin \
           for dynamic/personalized content\n\
         - Middleware patterns: authentication at the edge, A/B test assignment, feature \
           flag evaluation, bot detection before origin\n\
         - Streaming responses: use TransformStream for response body manipulation without \
           buffering entire response\n\
         - Runtime constraints: work within CPU time limits (50ms typical), memory limits, \
           no filesystem access, limited API surface\n\
         - Observability: structured logging from edge, request tracing, edge-specific \
           metrics (cold start rate, execution duration)",
    )
    .with_quality_threshold(0.8)
}

fn load_shedding_strategy() -> Skill {
    Skill::new(
        "load_shedding_strategy",
        "Load Shedding Strategy",
        "Implement graceful degradation under load with priority queues, circuit breakers, \
         and feature degradation policies.",
        SkillCategory::Performance,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a reliability and load management architect. Generate load shedding \
         strategies that include:\n\
         - Priority classification: assign request priorities (critical: payments, auth; \
           high: core API; medium: search, recommendations; low: analytics, prefetch)\n\
         - Admission control: reject lowest-priority requests first when load exceeds \
           capacity thresholds, return 503 with Retry-After header\n\
         - Circuit breakers: per-dependency circuit breakers with half-open state, \
           configurable failure thresholds and recovery windows\n\
         - Feature degradation tiers: define graceful degradation levels (full -> no \
           recommendations -> cached results only -> read-only mode -> maintenance page)\n\
         - Rate limiting: per-client rate limits with token bucket algorithm, separate \
           limits for authenticated vs anonymous, burst allowance\n\
         - Queue management: bounded request queues with LIFO scheduling (shed oldest), \
           timeout-based auto-shed for queued requests\n\
         - Backpressure propagation: signal overload upstream via HTTP 429/503, gRPC \
           RESOURCE_EXHAUSTED, health check degraded status\n\
         - Recovery: gradual traffic ramp-up after overload event, avoid thundering herd \
           on recovery, hystrix-style half-open probing",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.9)
}
