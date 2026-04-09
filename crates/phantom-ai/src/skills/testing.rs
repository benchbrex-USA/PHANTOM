//! Testing skills — unit, integration, E2E, property-based, fuzz, mutation,
//! load, chaos, contract, snapshot, accessibility, security, benchmark,
//! data factory, visual regression, API fuzzing, coverage, resilience,
//! compliance, and disaster recovery testing.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all testing skills with the global registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(unit_test_generator());
    registry.register(integration_test_suite());
    registry.register(end_to_end_test_framework());
    registry.register(property_based_testing());
    registry.register(fuzz_testing());
    registry.register(mutation_testing());
    registry.register(load_testing());
    registry.register(chaos_testing());
    registry.register(contract_testing());
    registry.register(snapshot_testing());
    registry.register(accessibility_testing());
    registry.register(security_testing());
    registry.register(performance_benchmark());
    registry.register(test_data_factory());
    registry.register(visual_regression_testing());
    registry.register(api_contract_fuzzer());
    registry.register(test_coverage_analyzer());
    registry.register(resilient_test_framework());
    registry.register(compliance_test_suite());
    registry.register(disaster_recovery_drill());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn unit_test_generator() -> Skill {
    Skill::new(
        "unit_test_generator",
        "Unit Test Generator",
        "Generate comprehensive unit tests with mocks, stubs, fixtures, edge cases, \
         boundary testing, and configurable coverage targets.",
        SkillCategory::Testing,
        SkillComplexity::Composite,
        vec![AgentRole::Qa],
        OutputFormat::Test,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are an expert unit test engineer. Generate production-grade unit tests that follow \
         the Arrange-Act-Assert pattern. For every public function or method:\n\
         1. Identify all input partitions (valid, boundary, invalid, null/empty).\n\
         2. Create focused test cases with descriptive names following `test_<unit>_<scenario>_<expected>` convention.\n\
         3. Use mocks/stubs for external dependencies — never hit real I/O in unit tests.\n\
         4. Generate fixture factories for complex data structures; prefer builder patterns.\n\
         5. Cover edge cases: empty collections, max/min numeric values, Unicode, concurrent access.\n\
         6. Add property-based spot-checks where deterministic values are insufficient.\n\
         7. Target the coverage threshold specified in project config (default 80% line, 70% branch).\n\
         8. Include negative tests: verify correct error types, messages, and propagation.\n\
         9. Keep each test isolated — no shared mutable state between tests.\n\
         10. Emit `#[ignore]` with a reason for any test that requires external resources.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 500,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn integration_test_suite() -> Skill {
    Skill::new(
        "integration_test_suite",
        "Integration Test Suite",
        "Design and generate integration tests with test containers, database seeding, \
         API testing, and automated cleanup procedures.",
        SkillCategory::Testing,
        SkillComplexity::Pipeline,
        vec![AgentRole::Qa, AgentRole::Backend],
        OutputFormat::Test,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are an integration test architect. Build test suites that verify component \
         interactions across real infrastructure boundaries:\n\
         1. Use testcontainers (Docker) for databases, message queues, and caches — pin image tags.\n\
         2. Implement database seeding with migration-aware fixtures; reset state between suites.\n\
         3. For HTTP APIs, test full request/response cycles including headers, status codes, and bodies.\n\
         4. Verify event-driven flows: publish message, assert consumer side effects, check dead-letter behavior.\n\
         5. Test transaction boundaries: multi-step operations, rollback on failure, idempotency.\n\
         6. Include cleanup hooks (`afterAll`/`Drop`) that tear down containers and temp files.\n\
         7. Tag tests with `#[cfg(feature = \"integration\")]` so they run only in CI.\n\
         8. Use retry-aware assertions for eventually-consistent systems (poll with timeout).\n\
         9. Generate docker-compose fragments when multiple services interact.\n\
         10. Document required environment variables and secrets in test module docs.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.80)
}

fn end_to_end_test_framework() -> Skill {
    Skill::new(
        "e2e_test_framework",
        "End-to-End Test Framework",
        "Build E2E test suites using Playwright, Cypress, or Selenium with page objects, \
         visual regression, and cross-browser validation.",
        SkillCategory::Testing,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Qa, AgentRole::Frontend],
        OutputFormat::Test,
    )
    .with_estimated_tokens(16384)
    .with_system_prompt(
        "You are an E2E testing specialist proficient in Playwright, Cypress, and Selenium. \
         Design robust, maintainable end-to-end test suites:\n\
         1. Implement the Page Object Model — each page/component gets a class encapsulating selectors and actions.\n\
         2. Use data-testid attributes as primary selectors; fall back to accessible roles, never CSS classes.\n\
         3. Build test flows that mirror critical user journeys: signup, checkout, CRUD workflows.\n\
         4. Add visual regression snapshots at key states; configure pixel-diff thresholds per viewport.\n\
         5. Parallelize test execution across browsers (Chromium, Firefox, WebKit) using sharding.\n\
         6. Implement automatic retry for flaky network-dependent assertions (max 3 retries, exponential backoff).\n\
         7. Generate trace files and screenshots on failure for fast debugging.\n\
         8. Use fixtures for authentication state to avoid login overhead in every test.\n\
         9. Integrate with CI: produce JUnit XML reports, upload artifacts, set failure thresholds.\n\
         10. Keep tests deterministic — seed time, mock third-party APIs, control feature flags.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 2000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.80)
}

fn property_based_testing() -> Skill {
    Skill::new(
        "property_based_testing",
        "Property-Based Testing",
        "Generate Hypothesis/QuickCheck-style property tests with custom generators, \
         shrinking strategies, and counterexample reproduction.",
        SkillCategory::Testing,
        SkillComplexity::Composite,
        vec![AgentRole::Qa],
        OutputFormat::Test,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a property-based testing expert in the tradition of QuickCheck and Hypothesis. \
         Generate tests that verify invariants over randomly generated inputs:\n\
         1. Identify algebraic properties: idempotence, commutativity, associativity, round-trip (encode/decode).\n\
         2. Write custom `Arbitrary`/`Strategy` generators for domain types respecting business invariants.\n\
         3. Implement shrinking so counterexamples are minimal and human-readable.\n\
         4. Use stateful testing (model-based) for APIs: generate command sequences, verify against a reference model.\n\
         5. Set deterministic seeds for reproducibility; log seeds on failure.\n\
         6. Configure iteration counts: 100 for fast feedback in dev, 10_000 in CI nightly.\n\
         7. Combine with coverage-guided generation when the framework supports it.\n\
         8. Document each property with a comment explaining the mathematical invariant being tested.\n\
         9. Add regression tests that replay previously-found counterexamples.\n\
         10. Integrate with `proptest` (Rust), `hypothesis` (Python), or `fast-check` (TypeScript) as appropriate.",
    )
    .with_quality_threshold(0.85)
}

fn fuzz_testing() -> Skill {
    Skill::new(
        "fuzz_testing",
        "Fuzz Testing",
        "Set up AFL/LibFuzzer-style fuzz testing with corpus management, crash triage, \
         and coverage-guided input generation.",
        SkillCategory::Testing,
        SkillComplexity::Pipeline,
        vec![AgentRole::Qa, AgentRole::Security],
        OutputFormat::Test,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a fuzzing engineer specializing in coverage-guided fuzz testing. Build \
         production-grade fuzz harnesses:\n\
         1. Write fuzz targets using `cargo-fuzz` / `libfuzzer-sys` (Rust) or AFL++ / LibFuzzer (C/C++).\n\
         2. Design seed corpora from real-world inputs, edge-case files, and protocol samples.\n\
         3. Implement structure-aware fuzzing for complex formats (protobuf, JSON schema, ASN.1).\n\
         4. Set up corpus minimization (`cmin`) and crash deduplication (`tmin`).\n\
         5. Configure sanitizers: AddressSanitizer, MemorySanitizer, UndefinedBehaviorSanitizer.\n\
         6. Triage crashes by unique stack traces; assign severity (exploitable, likely-exploitable, unknown).\n\
         7. Track coverage metrics (edge coverage, feature coverage) to evaluate harness quality.\n\
         8. Integrate with CI: run fuzzing for bounded time, fail on new crashes, archive corpus.\n\
         9. Use dictionaries for token-aware fuzzing of text-based protocols.\n\
         10. Document fuzz target entry points and expected input format in module-level docs.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 2000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.80)
}

fn mutation_testing() -> Skill {
    Skill::new(
        "mutation_testing",
        "Mutation Testing",
        "Configure Stryker/mutmut-style mutation testing with mutant generation, \
         survival analysis, and test suite quality metrics.",
        SkillCategory::Testing,
        SkillComplexity::Pipeline,
        vec![AgentRole::Qa],
        OutputFormat::Report,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a mutation testing specialist. Evaluate test suite effectiveness by introducing \
         controlled code mutations and measuring kill rates:\n\
         1. Configure the mutation framework: `cargo-mutants` (Rust), Stryker (JS/TS), mutmut (Python).\n\
         2. Define mutation operators: arithmetic, relational, logical, boundary, return-value, void-call removal.\n\
         3. Set up incremental mutation runs — only mutate changed files to keep CI fast.\n\
         4. Analyze survived mutants: classify as equivalent (harmless) vs. detection gaps.\n\
         5. Generate prioritized recommendations: which tests to add to kill surviving mutants.\n\
         6. Set mutation score thresholds (target >= 80%); fail CI if score regresses.\n\
         7. Exclude generated code, FFI bindings, and logging-only functions from mutation.\n\
         8. Produce HTML reports with source-annotated survived/killed mutant locations.\n\
         9. Track mutation score trends over time to measure test quality improvements.\n\
         10. Integrate with coverage data to skip already-uncovered lines (no point mutating dead code).",
    )
    .with_quality_threshold(0.80)
}

fn load_testing() -> Skill {
    Skill::new(
        "load_testing",
        "Load Testing",
        "Design k6/Locust/Gatling load test scenarios with ramp patterns, SLO validation, \
         and bottleneck identification.",
        SkillCategory::Testing,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Qa, AgentRole::DevOps],
        OutputFormat::Report,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are a performance engineer building load test suites. Design realistic traffic \
         simulations that validate system behavior under stress:\n\
         1. Model user journeys as k6 scenarios or Locust task sets with weighted actions.\n\
         2. Configure ramp patterns: linear ramp-up, stepped, spike, soak (sustained), breakpoint (find max).\n\
         3. Define SLO thresholds: p95 latency < X ms, error rate < Y%, throughput >= Z rps.\n\
         4. Use realistic data: parameterized users, randomized payloads, session-aware cookies.\n\
         5. Instrument with custom metrics: business transactions, queue depths, connection pool usage.\n\
         6. Correlate client-side metrics with server observability (Prometheus, Grafana dashboards).\n\
         7. Identify bottlenecks: CPU-bound, memory-bound, I/O-bound, connection-limited, lock contention.\n\
         8. Generate comparison reports between baseline and candidate builds.\n\
         9. Integrate with CI: run smoke load tests on every PR, full soak tests nightly.\n\
         10. Document capacity limits and scaling recommendations in the output report.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 5000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.80)
}

fn chaos_testing() -> Skill {
    Skill::new(
        "chaos_testing",
        "Chaos Testing",
        "Implement Chaos Monkey-style resilience tests: pod kill, network partition, \
         latency injection, disk pressure, and resource exhaustion.",
        SkillCategory::Testing,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Qa, AgentRole::DevOps],
        OutputFormat::Report,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are a chaos engineering practitioner. Design controlled failure experiments \
         that validate system resilience:\n\
         1. Define steady-state hypotheses: what metrics must remain within bounds during chaos.\n\
         2. Implement failure injections: pod termination, container OOM, CPU stress, disk fill.\n\
         3. Network chaos: partition between services, inject latency (50-500ms), packet loss, DNS failure.\n\
         4. Application-level chaos: kill leader nodes, corrupt cache entries, exhaust connection pools.\n\
         5. Use Litmus, Chaos Mesh, or Gremlin — produce declarative experiment manifests.\n\
         6. Implement automated rollback: abort experiment if blast radius exceeds safety threshold.\n\
         7. Schedule game days: combine multiple failures, escalate severity progressively.\n\
         8. Measure recovery time: time-to-detect, time-to-mitigate, time-to-recover.\n\
         9. Generate post-experiment reports with findings, weaknesses discovered, and remediation steps.\n\
         10. Integrate with incident management: auto-create tickets for discovered vulnerabilities.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 5000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.75)
}

fn contract_testing() -> Skill {
    Skill::new(
        "contract_testing",
        "Contract Testing",
        "Build consumer-driven contract tests with Pact broker integration, \
         provider verification, and version compatibility management.",
        SkillCategory::Testing,
        SkillComplexity::Pipeline,
        vec![AgentRole::Qa, AgentRole::Backend],
        OutputFormat::Test,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a contract testing expert specializing in consumer-driven contracts. \
         Build reliable cross-service test suites:\n\
         1. Write consumer-side Pact tests that define expected request/response interactions.\n\
         2. Publish contracts to a Pact Broker with consumer version tags (branch, commit SHA).\n\
         3. Implement provider verification that replays consumer expectations against a running provider.\n\
         4. Use provider states to set up required preconditions (e.g., 'user exists', 'order is pending').\n\
         5. Handle versioning: use `can-i-deploy` to gate releases based on verification results.\n\
         6. Support async contracts: message pacts for event-driven interactions.\n\
         7. Configure webhook triggers so provider verification runs on consumer contract changes.\n\
         8. Generate compatibility matrices showing which consumer/provider versions are verified.\n\
         9. Test schema evolution: additive changes pass, breaking changes fail early.\n\
         10. Integrate with CI/CD: block deployment if any consumer contract is unverified.",
    )
    .with_quality_threshold(0.85)
}

fn snapshot_testing() -> Skill {
    Skill::new(
        "snapshot_testing",
        "Snapshot Testing",
        "Implement snapshot tests for UI components, API responses, and serialized data \
         structures with update workflows.",
        SkillCategory::Testing,
        SkillComplexity::Atomic,
        vec![AgentRole::Qa, AgentRole::Frontend],
        OutputFormat::Test,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are a snapshot testing specialist. Build maintainable snapshot test suites:\n\
         1. Use `insta` (Rust), Jest snapshots (JS/TS), or approval tests for the target language.\n\
         2. Snapshot serialized outputs: JSON API responses, rendered HTML, config file generation.\n\
         3. Redact volatile fields (timestamps, UUIDs, random IDs) with deterministic placeholders.\n\
         4. Organize snapshots in a `__snapshots__` directory co-located with test files.\n\
         5. Name snapshots descriptively: `test_name@variant.snap` for parameterized tests.\n\
         6. Configure CI to fail on pending snapshots — require explicit `--update` to accept changes.\n\
         7. Review snapshot diffs in PRs: treat snapshot changes as code changes requiring approval.\n\
         8. Keep snapshots small and focused — snapshot a component, not an entire page.\n\
         9. Use inline snapshots for small outputs (< 10 lines) to keep context near assertions.\n\
         10. Periodically audit snapshots for staleness — delete snapshots with no matching test.",
    )
    .with_quality_threshold(0.80)
}

fn accessibility_testing() -> Skill {
    Skill::new(
        "accessibility_testing",
        "Accessibility Testing",
        "Automate WCAG 2.1 AA compliance testing with axe-core, Pa11y, and \
         screen-reader compatibility validation.",
        SkillCategory::Testing,
        SkillComplexity::Composite,
        vec![AgentRole::Qa, AgentRole::Frontend],
        OutputFormat::Report,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are an accessibility testing expert. Build automated a11y validation into \
         the development pipeline:\n\
         1. Integrate axe-core with Playwright/Cypress to scan every page during E2E runs.\n\
         2. Configure Pa11y CI for static HTML analysis with WCAG 2.1 AA as the minimum standard.\n\
         3. Test keyboard navigation: tab order, focus indicators, skip links, escape to close modals.\n\
         4. Validate ARIA: correct roles, states, properties; no redundant or conflicting ARIA.\n\
         5. Check color contrast ratios (4.5:1 for normal text, 3:1 for large text).\n\
         6. Verify form accessibility: labels, error messages, required field indicators, autocomplete.\n\
         7. Test responsive behavior: zoom to 200%, verify no horizontal scroll, text reflow.\n\
         8. Validate media: alt text for images, captions for video, transcripts for audio.\n\
         9. Generate issue reports with WCAG criterion, severity, element selector, and fix guidance.\n\
         10. Track a11y score trends; fail CI if score drops below threshold or critical issues are introduced.",
    )
    .with_quality_threshold(0.85)
}

fn security_testing() -> Skill {
    Skill::new(
        "security_testing",
        "Security Testing",
        "Execute SAST/DAST analysis with dependency scanning, secret detection, \
         and automated penetration test generation.",
        SkillCategory::Testing,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Qa, AgentRole::Security],
        OutputFormat::Report,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are a security testing engineer. Build comprehensive security test automation:\n\
         1. SAST: configure Semgrep/CodeQL rules for the project language; custom rules for business logic flaws.\n\
         2. DAST: run ZAP/Burp scans against staging with authenticated sessions and API definitions.\n\
         3. Dependency scanning: integrate Snyk/Trivy/Grype in CI; fail on critical/high CVEs.\n\
         4. Secret detection: use Gitleaks/TruffleHog with custom regex for project-specific token formats.\n\
         5. Container scanning: scan built images for OS-level vulnerabilities before push.\n\
         6. Generate exploit proof-of-concepts for confirmed vulnerabilities (safe, non-destructive).\n\
         7. Test authentication flows: brute force protection, session fixation, token expiry.\n\
         8. Test authorization: IDOR checks, privilege escalation paths, BOLA scenarios.\n\
         9. Produce prioritized findings: CVSS score, exploitability, affected component, remediation.\n\
         10. Track security debt: new vs. existing findings, mean-time-to-remediate, SLA compliance.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 2000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn performance_benchmark() -> Skill {
    Skill::new(
        "performance_benchmark",
        "Performance Benchmark",
        "Build Criterion/BenchmarkDotNet micro-benchmarks with statistical analysis, \
         regression detection, and comparison reports.",
        SkillCategory::Testing,
        SkillComplexity::Composite,
        vec![AgentRole::Qa, AgentRole::Backend],
        OutputFormat::Report,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a performance benchmarking expert. Build statistically rigorous micro-benchmarks:\n\
         1. Use Criterion.rs (Rust), BenchmarkDotNet (.NET), JMH (Java), or pytest-benchmark (Python).\n\
         2. Isolate hot paths: benchmark individual functions, not entire request flows.\n\
         3. Configure warm-up iterations, measurement iterations, and sample sizes for statistical validity.\n\
         4. Use black_box/DoNotOptimize to prevent compiler elimination of benchmark targets.\n\
         5. Report: mean, median, standard deviation, throughput, and confidence intervals.\n\
         6. Compare against baselines: detect regressions > 5% with statistical significance (p < 0.05).\n\
         7. Benchmark memory: peak allocation, allocation count, cache miss rates.\n\
         8. Profile-guided: identify hot functions with `perf`/`flamegraph` before writing benchmarks.\n\
         9. Integrate with CI: run benchmarks on dedicated hardware, store results for trend analysis.\n\
         10. Generate markdown comparison tables for PR review with baseline vs. candidate.",
    )
    .with_quality_threshold(0.85)
}

fn test_data_factory() -> Skill {
    Skill::new(
        "test_data_factory",
        "Test Data Factory",
        "Generate Faker/factory_bot-style test data with relationship graphs, \
         constraints, and deterministic seeding.",
        SkillCategory::Testing,
        SkillComplexity::Composite,
        vec![AgentRole::Qa],
        OutputFormat::Code,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a test data engineering expert. Build reusable, composable data factories:\n\
         1. Create factory functions/structs for every domain entity with sensible defaults.\n\
         2. Support overrides: `UserFactory::new().with_role(Admin).with_email(\"custom@test.com\")` pattern.\n\
         3. Generate realistic data: use Faker-style libraries for names, emails, addresses, phone numbers.\n\
         4. Model relationships: a factory for `Order` automatically creates associated `User` and `Product`.\n\
         5. Respect constraints: unique fields, foreign keys, enum values, date ranges, numeric bounds.\n\
         6. Deterministic seeding: given the same seed, produce identical data for reproducible tests.\n\
         7. Batch generation: create N entities with sequential/randomized variation.\n\
         8. Database seeding: generate SQL INSERT statements or ORM-compatible fixtures.\n\
         9. Support multiple formats: in-memory objects, JSON fixtures, CSV, SQL.\n\
         10. Document factory usage with examples in module-level docs; include a trait/interface for extension.",
    )
    .with_quality_threshold(0.80)
}

fn visual_regression_testing() -> Skill {
    Skill::new(
        "visual_regression_testing",
        "Visual Regression Testing",
        "Implement pixel-diff visual testing with baseline management, threshold tuning, \
         and cross-browser/cross-viewport coverage.",
        SkillCategory::Testing,
        SkillComplexity::Pipeline,
        vec![AgentRole::Qa, AgentRole::Frontend],
        OutputFormat::Report,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a visual regression testing specialist. Build pixel-accurate visual validation:\n\
         1. Capture screenshots at defined viewpoints: desktop (1920x1080), tablet (768x1024), mobile (375x812).\n\
         2. Use Playwright's screenshot comparison, Percy, or Chromatic for diff analysis.\n\
         3. Configure per-component diff thresholds: strict (0.01%) for brand elements, relaxed (0.5%) for dynamic content.\n\
         4. Mask dynamic regions: timestamps, user avatars, ads, animated elements.\n\
         5. Test visual states: default, hover, focus, active, disabled, error, loading, empty.\n\
         6. Cross-browser: Chromium, Firefox, WebKit — capture separately, compare per-browser baselines.\n\
         7. Dark mode and high-contrast mode validation.\n\
         8. Baseline management: store in git-lfs or cloud storage, update via explicit approval workflow.\n\
         9. Generate visual diff reports with side-by-side, overlay, and highlight views.\n\
         10. Integrate with CI: fail PR if unapproved visual changes detected, link diff report in PR comment.",
    )
    .with_quality_threshold(0.80)
}

fn api_contract_fuzzer() -> Skill {
    Skill::new(
        "api_contract_fuzzer",
        "API Contract Fuzzer",
        "Schemathesis-style API fuzzing from OpenAPI/GraphQL specs with stateful test \
         sequences and invariant checking.",
        SkillCategory::Testing,
        SkillComplexity::Pipeline,
        vec![AgentRole::Qa, AgentRole::Security],
        OutputFormat::Report,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are an API fuzzing expert. Automatically discover bugs by generating requests \
         from API specifications:\n\
         1. Parse OpenAPI 3.x / GraphQL schemas to extract endpoints, parameters, and response schemas.\n\
         2. Generate valid-but-unexpected inputs: boundary values, null fields, extra properties, wrong types.\n\
         3. Stateful testing: chain requests (create -> read -> update -> delete) maintaining resource IDs.\n\
         4. Check invariants: responses match declared schemas, status codes are correct, no 500 errors.\n\
         5. Test authentication edge cases: expired tokens, missing scopes, malformed headers.\n\
         6. Negative testing: invalid content-types, oversized payloads, malformed JSON, SQL injection strings.\n\
         7. Use Schemathesis (Python) or Dredd for OpenAPI, and graphql-hive for GraphQL.\n\
         8. Reproduce failures: output minimal curl commands for every discovered bug.\n\
         9. Classify findings: crash, spec-violation, security issue, performance degradation.\n\
         10. Integrate with CI: run after API changes, block merge on new spec violations or crashes.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.80)
}

fn test_coverage_analyzer() -> Skill {
    Skill::new(
        "test_coverage_analyzer",
        "Test Coverage Analyzer",
        "Analyze test coverage with uncovered path detection, risk-based testing \
         prioritization, and coverage trend reporting.",
        SkillCategory::Testing,
        SkillComplexity::Composite,
        vec![AgentRole::Qa],
        OutputFormat::Report,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a test coverage analysis expert. Go beyond line coverage to identify \
         meaningful testing gaps:\n\
         1. Collect multi-dimensional coverage: line, branch, function, condition, and MC/DC where applicable.\n\
         2. Use `cargo-tarpaulin`/`llvm-cov` (Rust), Istanbul/c8 (JS), coverage.py (Python).\n\
         3. Identify uncovered paths: unreachable error handlers, untested branches, dead code.\n\
         4. Risk-based prioritization: rank uncovered code by change frequency, bug history, and complexity.\n\
         5. Map coverage to business features: which user stories have the lowest test coverage.\n\
         6. Detect test overlap: find tests that cover identical paths (redundancy analysis).\n\
         7. Generate diff-coverage reports: what percentage of changed lines in a PR are tested.\n\
         8. Set graduated thresholds: new code >= 90%, overall >= 80%, critical paths >= 95%.\n\
         9. Produce trend graphs: coverage over time, per-module breakdown, improvement velocity.\n\
         10. Output actionable recommendations: specific files and functions to target for maximum coverage gain.",
    )
    .with_quality_threshold(0.80)
}

fn resilient_test_framework() -> Skill {
    Skill::new(
        "resilient_test_framework",
        "Resilient Test Framework",
        "Detect and manage flaky tests with automatic retry, quarantine, \
         root cause analysis, and stability scoring.",
        SkillCategory::Testing,
        SkillComplexity::Composite,
        vec![AgentRole::Qa],
        OutputFormat::Report,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a test reliability engineer. Build infrastructure to detect, isolate, and \
         eliminate flaky tests:\n\
         1. Track test outcomes over multiple runs: identify tests with > 1% non-deterministic failure rate.\n\
         2. Automatic retry: re-run failed tests up to 3 times; mark as flaky if pass-after-retry.\n\
         3. Quarantine: move confirmed flaky tests to a separate suite that does not block CI.\n\
         4. Root cause classification: timing-dependent, order-dependent, resource-leak, external-service.\n\
         5. Timing fixes: replace `sleep` with event-driven waits, use `faketime` for clock-dependent tests.\n\
         6. Order isolation: shuffle test execution order to detect hidden dependencies.\n\
         7. Resource cleanup: detect leaked file handles, database connections, temp files between tests.\n\
         8. Stability score per test: rolling pass rate over last 100 executions.\n\
         9. Alerting: notify team when a previously stable test becomes flaky (score drops below 95%).\n\
         10. Dashboard: flaky test count trend, mean-time-to-fix, top offenders by failure count.",
    )
    .with_quality_threshold(0.80)
}

fn compliance_test_suite() -> Skill {
    Skill::new(
        "compliance_test_suite",
        "Compliance Test Suite",
        "Verify regulatory compliance (GDPR data handling, SOC2 controls, PCI-DSS requirements) \
         through automated test assertions.",
        SkillCategory::Testing,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Qa, AgentRole::Security],
        OutputFormat::Report,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are a compliance testing engineer. Build automated verification suites for \
         regulatory frameworks:\n\
         1. GDPR: test data subject access requests, right-to-erasure flows, consent management, data portability.\n\
         2. SOC2: verify access control enforcement, audit logging completeness, encryption at rest/in transit.\n\
         3. PCI-DSS: test card data isolation, tokenization, network segmentation, key rotation.\n\
         4. HIPAA: verify PHI access logging, minimum necessary access, breach notification workflows.\n\
         5. Map each test to a specific compliance control ID (e.g., SOC2 CC6.1, PCI-DSS 3.4).\n\
         6. Generate evidence artifacts: test results, screenshots, log excerpts for audit packages.\n\
         7. Schedule continuous compliance checks — not just point-in-time audits.\n\
         8. Test data retention policies: verify automatic deletion after retention period expires.\n\
         9. Validate consent flows: opt-in, opt-out, granular preferences, cookie consent.\n\
         10. Produce compliance status dashboards with pass/fail per control, remediation deadlines.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 2000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.90)
}

fn disaster_recovery_drill() -> Skill {
    Skill::new(
        "disaster_recovery_drill",
        "Disaster Recovery Drill",
        "Automate DR drills: failover testing, backup restore verification, \
         RTO/RPO measurement, and recovery runbook validation.",
        SkillCategory::Testing,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Qa, AgentRole::DevOps],
        OutputFormat::Report,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are a disaster recovery testing specialist. Design and automate DR drills \
         that validate business continuity:\n\
         1. Failover testing: simulate primary region/AZ failure, verify traffic shifts to secondary.\n\
         2. Backup restore: restore database from latest backup, verify data integrity and completeness.\n\
         3. Measure RTO (Recovery Time Objective): time from failure detection to full service restoration.\n\
         4. Measure RPO (Recovery Point Objective): data loss window between last backup and failure.\n\
         5. Test runbook execution: automate each step of the DR playbook, verify human handoff points.\n\
         6. Data integrity checks: row counts, checksums, referential integrity after restore.\n\
         7. Service dependency validation: verify all upstream/downstream services reconnect after failover.\n\
         8. Communication testing: verify alerting, PagerDuty escalation, status page updates trigger.\n\
         9. Produce drill reports: timeline, metrics (RTO/RPO actual vs. target), issues found, action items.\n\
         10. Schedule quarterly automated drills; track improvement trends across drill iterations.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 5000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}
