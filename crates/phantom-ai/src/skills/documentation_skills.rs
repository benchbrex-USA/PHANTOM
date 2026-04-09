//! Documentation skills for the Phantom autonomous AI engineering system.
//!
//! Covers API docs, ADRs, runbooks, changelogs, design docs, onboarding docs,
//! postmortems, compliance docs, user docs, and diagram generation.

use super::{OutputFormat, Skill, SkillCategory, SkillComplexity, SkillRegistry};
use crate::agents::AgentRole;

/// Register all documentation skills into the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(api_documentation());
    registry.register(architecture_decision_record());
    registry.register(runbook_generator());
    registry.register(changelog_generator());
    registry.register(technical_design_doc());
    registry.register(onboarding_documentation());
    registry.register(postmortem_template());
    registry.register(compliance_documentation());
    registry.register(user_documentation());
    registry.register(diagram_generator());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn api_documentation() -> Skill {
    Skill::new(
        "api_documentation",
        "API Documentation Generator",
        "Auto-generates comprehensive API documentation with endpoint references, \
         request/response examples, authentication guides, error catalogs, SDK \
         quickstart snippets, and interactive playground links.",
        SkillCategory::Documentation,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Generate API documentation from OpenAPI/Swagger specs or source code \
         annotations. Each endpoint must include: HTTP method and path, description, \
         authentication requirements, request parameters (path, query, header, body) \
         with types and validation rules, response schemas for all status codes \
         (200, 400, 401, 403, 404, 500) with example payloads, and curl/httpie \
         examples. Group endpoints by resource and provide a getting-started guide \
         covering authentication setup (API keys, OAuth flows), rate limits, \
         pagination patterns, and error handling conventions. Include SDK quickstart \
         snippets for the top three languages used by consumers. Generate changelog \
         diffs between API versions highlighting breaking changes, new endpoints, \
         and deprecated fields. All examples must use realistic but obviously \
         fake data (no real PII).",
    )
    .with_quality_threshold(0.85)
}

fn architecture_decision_record() -> Skill {
    Skill::new(
        "architecture_decision_record",
        "Architecture Decision Record",
        "Produces ADRs with structured context, decision rationale, consequences \
         analysis, alternatives considered with trade-off matrices, status tracking, \
         and supersession chains.",
        SkillCategory::Documentation,
        SkillComplexity::Atomic,
        vec![AgentRole::Architect, AgentRole::Cto],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(15_000)
    .with_system_prompt(
        "Write Architecture Decision Records following the standard template: \
         Title (short, descriptive), Status (proposed/accepted/deprecated/superseded), \
         Context (what is the issue and why does it need a decision now), Decision \
         (what was decided and why), Consequences (both positive and negative, with \
         mitigation for negatives), and Alternatives Considered (each with a brief \
         analysis of pros/cons). Include a trade-off matrix comparing alternatives \
         across dimensions: complexity, performance, cost, team familiarity, \
         maintenance burden, and lock-in risk. Link to related ADRs via supersession \
         chains (this ADR supersedes ADR-NNN). Use concrete metrics and benchmarks \
         where available rather than vague statements. The ADR must be understandable \
         by someone joining the team six months from now.",
    )
    .with_quality_threshold(0.85)
}

fn runbook_generator() -> Skill {
    Skill::new(
        "runbook_generator",
        "Operational Runbook Generator",
        "Creates operational runbooks with step-by-step procedures, troubleshooting \
         decision trees, escalation paths, rollback instructions, and verification \
         commands at each step.",
        SkillCategory::Documentation,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Backend],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Generate operational runbooks that an on-call engineer can follow at 3 AM \
         under stress. Each runbook has a clear title, triggering condition (what \
         alert or symptom leads here), impact assessment (what is affected and \
         severity), and numbered steps. Each step includes: the exact command to run \
         (copy-pasteable), expected output, what to do if output differs, and a \
         verification command to confirm the step succeeded before proceeding. \
         Troubleshooting sections use decision trees: if X then go to step N, if Y \
         then go to step M. Include rollback instructions that undo each step in \
         reverse order. Escalation paths specify who to contact (role, not person), \
         when to escalate (time-based and severity-based criteria), and what \
         information to include in the escalation. Link to dashboards, log queries, \
         and relevant architecture diagrams.",
    )
    .with_quality_threshold(0.85)
}

fn changelog_generator() -> Skill {
    Skill::new(
        "changelog_generator",
        "Semantic Changelog Generator",
        "Generates semantic changelogs from git history with automatic categorization \
         (features, fixes, breaking changes), migration guides for breaking changes, \
         contributor attribution, and release note formatting.",
        SkillCategory::Documentation,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(15_000)
    .with_system_prompt(
        "Generate changelogs by analyzing git commits between releases using \
         conventional commit prefixes (feat, fix, refactor, perf, docs, chore, \
         BREAKING CHANGE). Categorize entries under Added, Changed, Deprecated, \
         Removed, Fixed, and Security sections following Keep a Changelog format. \
         Breaking changes get a prominent section with a migration guide: what \
         changed, why, and exact steps to update consuming code with before/after \
         examples. Attribute entries to contributors with GitHub usernames. Detect \
         dependency version bumps and include notable upstream changes. Filter out \
         chore and CI commits from user-facing changelogs but include them in \
         developer changelogs. Generate both Markdown for the repository and HTML \
         for the documentation site. Include a summary paragraph at the top that \
         highlights the most important changes for quick scanning.",
    )
    .with_quality_threshold(0.80)
}

fn technical_design_doc() -> Skill {
    Skill::new(
        "technical_design_doc",
        "Technical Design Document",
        "Produces technical design documents with problem statement, proposed \
         solution architecture, API contracts, data model changes, alternatives \
         analysis, risk assessment, and implementation timeline.",
        SkillCategory::Documentation,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend, AgentRole::Cto],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Write technical design documents with the following structure: Problem \
         Statement (what problem are we solving, who is affected, what is the \
         business impact), Goals and Non-Goals (explicit scope boundaries), Proposed \
         Solution (architecture overview with diagrams, API contracts with example \
         payloads, data model changes with migration plan, key algorithms with \
         complexity analysis), Alternatives Considered (each with pros/cons and \
         reason for rejection), System Dependencies (upstream and downstream services \
         affected), Risks and Mitigations (technical risks, operational risks, \
         timeline risks with specific mitigation actions), Observability Plan \
         (metrics, logs, alerts to add), Rollout Plan (feature flag strategy, \
         canary percentage, rollback criteria), Timeline (milestones with estimated \
         effort). The design must be reviewable: call out open questions and areas \
         where reviewer input is needed.",
    )
    .with_quality_threshold(0.85)
}

fn onboarding_documentation() -> Skill {
    Skill::new(
        "onboarding_documentation",
        "Developer Onboarding Documentation",
        "Creates developer onboarding guides with local setup instructions, \
         architecture overview, coding standards reference, common workflows, \
         and troubleshooting FAQ.",
        SkillCategory::Documentation,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Write developer onboarding documentation that gets a new engineer to their \
         first meaningful contribution within one day. Local setup: prerequisites \
         (exact versions), clone and build commands, environment variable setup with \
         a template .env file, database seeding, and a verification step that confirms \
         everything works. Architecture overview: system diagram with data flow arrows, \
         service responsibilities in one sentence each, communication patterns (sync \
         vs async), and where to find the code for each service. Coding standards: \
         language-specific style guide references, naming conventions, error handling \
         patterns, testing expectations (unit/integration/e2e), and PR review \
         checklist. Common workflows: how to add a new API endpoint end-to-end, how \
         to run tests, how to deploy, how to access logs. Troubleshooting FAQ: top \
         ten issues new engineers hit with solutions. Keep prose minimal; prefer \
         commands, code snippets, and diagrams.",
    )
    .with_quality_threshold(0.80)
}

fn postmortem_template() -> Skill {
    Skill::new(
        "postmortem_template",
        "Incident Postmortem Generator",
        "Generates blameless incident postmortems with timeline reconstruction, \
         impact quantification, root cause analysis (5 Whys), action items with \
         owners, and lessons learned.",
        SkillCategory::Documentation,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Backend, AgentRole::Monitor],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(15_000)
    .with_system_prompt(
        "Generate blameless incident postmortems with the following structure: \
         Summary (one paragraph: what happened, impact, duration, resolution). \
         Impact (quantified: users affected, revenue impact, SLA breach, error rate). \
         Timeline (chronological events with timestamps: detection, response actions, \
         communication, mitigation, resolution, all-clear). Root Cause Analysis \
         (5 Whys chain from symptom to underlying cause, distinguishing proximate \
         cause from contributing factors). Detection (how was the incident detected, \
         what monitoring gaps existed). Resolution (what fixed it, temporary vs \
         permanent). Action Items (specific, measurable tasks with an owner and \
         deadline, categorized as: prevent recurrence, improve detection, improve \
         response). Lessons Learned (what went well, what went poorly, where we \
         got lucky). Use neutral, blameless language throughout. Include links to \
         relevant dashboards, logs, and PRs.",
    )
    .with_quality_threshold(0.85)
}

fn compliance_documentation() -> Skill {
    Skill::new(
        "compliance_documentation",
        "Compliance Documentation Generator",
        "Produces compliance documentation with control descriptions, evidence \
         references, test procedures and results, exception tracking, and \
         framework-specific formatting (SOC2, ISO 27001, HIPAA).",
        SkillCategory::Documentation,
        SkillComplexity::Composite,
        vec![AgentRole::Security, AgentRole::DevOps],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Generate compliance documentation that satisfies auditor requirements. \
         For each control: unique ID (mapped to the relevant framework's numbering), \
         control description (what is required), implementation description (how we \
         implement it, specific tools and configurations), evidence references \
         (links to configs, screenshots, log queries that demonstrate compliance), \
         test procedure (exact steps an auditor or automated tool follows to verify), \
         test results (last test date, pass/fail, findings). Track exceptions: \
         controls that are not fully implemented with compensating controls and \
         remediation timeline. Format output to match the target framework: SOC2 \
         trust service criteria, ISO 27001 Annex A controls, or HIPAA safeguards. \
         Include a control matrix that maps controls across multiple frameworks \
         for organizations with overlapping compliance requirements.",
    )
    .with_quality_threshold(0.85)
}

fn user_documentation() -> Skill {
    Skill::new(
        "user_documentation",
        "End-User Documentation",
        "Creates end-user documentation with getting-started guides, feature \
         tutorials, conceptual overviews, FAQ, troubleshooting guides, and \
         searchable knowledge base structure.",
        SkillCategory::Documentation,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend, AgentRole::Backend],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Write user documentation for non-technical audiences using clear, concise \
         language. Getting-started guide: the shortest path from signup to first \
         value, with annotated screenshots showing exactly where to click. Feature \
         tutorials: task-oriented (how to X) rather than feature-oriented (about the \
         X screen), with numbered steps and expected outcomes. Conceptual overviews: \
         explain the mental model behind complex features using analogies and \
         diagrams, not implementation details. FAQ: organized by topic, answering \
         actual frequently asked questions from support tickets, not imagined ones. \
         Troubleshooting: symptom-based (I see error X) with step-by-step resolution. \
         Structure content for searchability: descriptive headings, consistent \
         terminology (maintain a glossary), and cross-links between related topics. \
         Every page must have a clear audience statement and prerequisites list.",
    )
    .with_quality_threshold(0.80)
}

fn diagram_generator() -> Skill {
    Skill::new(
        "diagram_generator",
        "Architecture Diagram Generator",
        "Generates architecture diagrams including C4 model views, sequence diagrams, \
         deployment diagrams, and data flow diagrams using Mermaid or PlantUML syntax \
         with consistent styling.",
        SkillCategory::Documentation,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Documentation,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Generate architecture diagrams as code using Mermaid (preferred for GitHub \
         rendering) or PlantUML. C4 model: produce Context (system and external \
         actors), Container (services and data stores), and Component (internal \
         structure of key containers) diagrams with consistent notation. Sequence \
         diagrams: show the request flow for critical user journeys with all service \
         interactions, including error paths and async operations. Deployment \
         diagrams: map containers to infrastructure (regions, VPCs, clusters, nodes) \
         with network boundaries and protocols. Data flow diagrams: trace data from \
         ingestion through processing to storage and consumption, marking PII \
         touchpoints. Apply consistent styling: color-code by system ownership, \
         use standard shapes (rectangles for services, cylinders for databases, \
         clouds for external systems), and include a legend. Each diagram must have \
         a title, brief description of what it shows, and when it was last updated.",
    )
    .with_quality_threshold(0.80)
}
