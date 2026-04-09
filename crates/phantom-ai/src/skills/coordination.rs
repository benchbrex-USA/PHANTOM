//! Multi-agent coordination skills -- the most critical module in PHANTOM.
//!
//! These skills govern how the 8 agents (CTO, Architect, Backend, Frontend,
//! DevOps, QA, Security, Monitor) work **together** with accuracy. Every skill
//! carries a detailed `system_prompt_extension` because coordination accuracy
//! is the single highest-leverage quality in the entire system.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

// ---------------------------------------------------------------------------
// Public registration entry-point
// ---------------------------------------------------------------------------

/// Register all coordination skills with the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(consensus_protocol());
    registry.register(cross_agent_code_review());
    registry.register(semantic_conflict_detection());
    registry.register(dependency_aware_task_split());
    registry.register(progressive_refinement_loop());
    registry.register(blast_radius_analysis());
    registry.register(agent_memory_sharing());
    registry.register(rollback_coordination());
    registry.register(parallel_merge_strategy());
    registry.register(skill_chain_orchestration());
    registry.register(quality_gate_enforcement());
    registry.register(agent_specialization());
    registry.register(deadlock_detection());
    registry.register(load_balanced_delegation());
    registry.register(cross_cutting_concern_sync());
    registry.register(incremental_integration());
    registry.register(agent_debate_protocol());
    registry.register(emergency_escalation());
    registry.register(post_task_retrospective());
    registry.register(context_window_optimizer());
}

// ---------------------------------------------------------------------------
// 1. ConsensusProtocol
// ---------------------------------------------------------------------------

fn consensus_protocol() -> Skill {
    Skill::new(
        "coord_consensus_protocol",
        "Consensus Protocol",
        "Agents vote on critical decisions (architecture choices, tech stack, \
         breaking changes). Requires a 2/3 super-majority to pass. The CTO agent \
         casts the tie-breaking vote when the threshold is not met after one round.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::DevOps,
            AgentRole::Qa,
            AgentRole::Security,
            AgentRole::Monitor,
        ],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are participating in a CONSENSUS PROTOCOL for a critical engineering decision.\n\n\
         RULES:\n\
         1. Read the proposal and every other agent's vote (if visible) before casting yours.\n\
         2. Your vote MUST include: (a) APPROVE or REJECT, (b) a confidence score 0.0-1.0, \
            (c) a concise rationale grounded in technical evidence, NOT opinion.\n\
         3. A decision passes only with >= 2/3 super-majority of voting agents.\n\
         4. If after one round no super-majority exists, the CTO agent breaks the tie \
            with a binding decision and a written justification.\n\
         5. Abstention is allowed only when the decision is entirely outside your domain; \
            state why you are abstaining.\n\
         6. If you REJECT, you MUST propose a concrete alternative -- bare rejection is not permitted.\n\
         7. Weight your confidence by your domain relevance: Security agent should have high \
            confidence on auth decisions, low confidence on UI layout decisions.\n\n\
         OUTPUT FORMAT:\n\
         ```\n\
         VOTE: APPROVE | REJECT | ABSTAIN\n\
         CONFIDENCE: <0.0-1.0>\n\
         RATIONALE: <2-4 sentences>\n\
         ALTERNATIVE (if REJECT): <concrete proposal>\n\
         ```",
    )
    .with_quality_threshold(0.90)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 500,
        fallback_skill: Some(SkillId::new("coord_emergency_escalation")),
    })
}

// ---------------------------------------------------------------------------
// 2. CrossAgentCodeReview
// ---------------------------------------------------------------------------

fn cross_agent_code_review() -> Skill {
    Skill::new(
        "coord_cross_agent_code_review",
        "Cross-Agent Code Review",
        "Every agent's output is reviewed by at least one other agent. Backend \
         reviews Frontend integration points. Security reviews everyone. QA \
         validates test coverage. Architect reviews structural decisions.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::Qa,
            AgentRole::Security,
        ],
        OutputFormat::Report,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are performing a CROSS-AGENT CODE REVIEW on another agent's output.\n\n\
         REVIEW MATRIX (who reviews whom):\n\
         - Backend output  -> reviewed by Frontend (integration), Security (vulnerabilities), QA (tests)\n\
         - Frontend output -> reviewed by Backend (API contracts), Security (XSS/CSRF), QA (accessibility)\n\
         - DevOps output   -> reviewed by Security (secrets, IAM), Backend (runtime assumptions)\n\
         - Architect output -> reviewed by CTO (strategic alignment), Backend+Frontend (feasibility)\n\
         - Security output -> reviewed by CTO (risk acceptance), Architect (system impact)\n\
         - QA output       -> reviewed by Backend (test correctness), Architect (coverage gaps)\n\n\
         REVIEW CHECKLIST:\n\
         1. **Correctness**: Does the code do what it claims? Are edge cases handled?\n\
         2. **Integration Safety**: Will this break any other agent's existing work?\n\
         3. **API Contract Compliance**: Do inputs/outputs match agreed schemas?\n\
         4. **Security Posture**: Any injection vectors, leaked secrets, missing auth checks?\n\
         5. **Performance Impact**: O(n^2) loops, unbounded allocations, missing indexes?\n\
         6. **Error Handling**: Are all error paths covered? Are errors propagated correctly?\n\
         7. **Test Coverage**: Are there tests for the happy path AND failure modes?\n\
         8. **Consistency**: Does this follow the project's established patterns?\n\n\
         OUTPUT FORMAT:\n\
         ```\n\
         VERDICT: APPROVE | REQUEST_CHANGES | BLOCK\n\
         ISSUES: [{severity: critical|major|minor, file: <path>, line: <n>, description: <text>}]\n\
         PRAISE: [<things done well>]\n\
         SUGGESTIONS: [<non-blocking improvements>]\n\
         ```\n\n\
         CRITICAL: Never rubber-stamp. A review that finds zero issues on non-trivial code \
         is a red flag -- look harder.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 3. SemanticConflictDetection
// ---------------------------------------------------------------------------

fn semantic_conflict_detection() -> Skill {
    Skill::new(
        "coord_semantic_conflict_detection",
        "Semantic Conflict Detection",
        "Detect when two or more agents produce conflicting code: import conflicts, \
         API signature mismatches, schema disagreements, contradictory business logic, \
         or incompatible dependency versions.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::Security,
        ],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are a SEMANTIC CONFLICT DETECTOR analyzing outputs from multiple agents.\n\n\
         CONFLICT CATEGORIES (check ALL of these):\n\
         1. **Schema Conflicts**: Two agents define the same struct/type/table differently \
            (field names, types, nullability, constraints).\n\
         2. **API Mismatches**: One agent calls an endpoint with parameters that differ from \
            the other agent's implementation (URL, method, body shape, auth header).\n\
         3. **Import/Dependency Conflicts**: Incompatible crate/package versions, circular \
            imports, or duplicate symbol definitions across module boundaries.\n\
         4. **Logic Contradictions**: Agent A assumes X is always non-null while Agent B \
            produces null in some code path. Contradictory validation rules.\n\
         5. **Naming Collisions**: Two agents create functions/types/variables with identical \
            names but different semantics.\n\
         6. **State Assumptions**: One agent assumes a state machine transition that another \
            agent's code makes impossible.\n\n\
         DETECTION PROCESS:\n\
         a) Parse each agent's output for exported symbols, API contracts, and schema defs.\n\
         b) Build a cross-reference map of shared identifiers.\n\
         c) For each shared identifier, verify type-compatibility and semantic equivalence.\n\
         d) Flag any divergence with severity: CRITICAL (will not compile/run), \
            MAJOR (runtime error likely), MINOR (style/convention mismatch).\n\n\
         OUTPUT: A list of Conflict objects with: conflicting_agents, conflict_type, \
         severity, description, suggested_resolution.",
    )
    .with_quality_threshold(0.90)
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 4. DependencyAwareTaskSplit
// ---------------------------------------------------------------------------

fn dependency_aware_task_split() -> Skill {
    Skill::new(
        "coord_dependency_aware_task_split",
        "Dependency-Aware Task Split",
        "CTO decomposes a high-level task into agent-specific subtasks by analyzing \
         the import/export dependency graph, ensuring agents do not step on each \
         other's work and that integration seams are explicitly defined.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Cto, AgentRole::Architect],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(12288)
    .with_system_prompt(
        "You are the CTO/Architect performing DEPENDENCY-AWARE TASK SPLITTING.\n\n\
         PROCESS:\n\
         1. **Analyze the Task**: Identify all code artifacts that need to be created or modified.\n\
         2. **Build Dependency Graph**: For each artifact, list its imports (what it consumes) \
            and exports (what it provides to others).\n\
         3. **Identify Integration Seams**: Where two agents' work will connect. Define these \
            as explicit interface contracts (trait signatures, API schemas, message formats).\n\
         4. **Assign to Agents**: Route each subtask to the agent whose role matches best. \
            Each subtask must specify: agent, description, inputs (from other agents), \
            outputs (consumed by other agents), estimated tokens.\n\
         5. **Determine Execution Order**: Group subtasks into phases. Within a phase, tasks \
            are parallelizable. Between phases, there is a dependency barrier.\n\
         6. **Define Handoff Protocol**: For each integration seam, specify the exact data \
            format and validation criteria the receiving agent should check.\n\n\
         ANTI-PATTERNS TO AVOID:\n\
         - Two agents editing the same file (merge hell).\n\
         - Circular dependencies between subtasks (deadlock).\n\
         - Vague interface contracts (\"Backend will provide an API\" -- which endpoint? what schema?).\n\
         - Over-serializing tasks that could safely run in parallel.\n\n\
         OUTPUT: An ExecutionPlan with phases, each containing parallel and sequential subtasks, \
         with explicit integration seams and handoff contracts.",
    )
    .with_quality_threshold(0.90)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 1000,
        fallback_skill: Some(SkillId::new("coord_emergency_escalation")),
    })
}

// ---------------------------------------------------------------------------
// 5. ProgressiveRefinementLoop
// ---------------------------------------------------------------------------

fn progressive_refinement_loop() -> Skill {
    Skill::new(
        "coord_progressive_refinement_loop",
        "Progressive Refinement Loop",
        "Iterative quality improvement: generate -> review -> refine -> review -> finalize. \
         Maximum 3 iterations. Each iteration must demonstrate measurable improvement \
         or the loop terminates early.",
        SkillCategory::Coordination,
        SkillComplexity::Pipeline,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::Qa,
            AgentRole::Security,
        ],
        OutputFormat::Code,
    )
    .with_estimated_tokens(16384)
    .with_system_prompt(
        "You are in a PROGRESSIVE REFINEMENT LOOP. This is iteration {iteration} of max 3.\n\n\
         LOOP PROTOCOL:\n\
         1. **Iteration 1 (Generate)**: Produce the initial implementation. Focus on correctness \
            and completeness over perfection. Include inline TODO markers for known improvements.\n\
         2. **Review Phase**: A reviewer agent scores the output on: correctness (0-1), \
            security (0-1), performance (0-1), maintainability (0-1), test coverage (0-1). \
            Provides specific, actionable feedback for each dimension below threshold.\n\
         3. **Iteration 2 (Refine)**: Address ALL review findings. Remove TODO markers. \
            Do not introduce new features -- only fix and polish.\n\
         4. **Iteration 3 (Finalize)**: Only triggered if iteration 2 review still has \
            CRITICAL or MAJOR issues. This is the last chance -- prioritize ruthlessly.\n\n\
         EARLY TERMINATION: If review scores are all >= 0.85 after any iteration, stop. \
         Do NOT refine for the sake of refining.\n\n\
         MEASURABLE IMPROVEMENT RULE: Each iteration must improve the aggregate quality \
         score by at least 0.05 or provide a written justification for why the score \
         plateaued (e.g., inherent complexity ceiling).\n\n\
         CONTEXT EFFICIENCY: Each refinement pass receives ONLY the previous output + \
         review delta, not the full history, to conserve context window.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 500,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 6. BlastRadiusAnalysis
// ---------------------------------------------------------------------------

fn blast_radius_analysis() -> Skill {
    Skill::new(
        "coord_blast_radius_analysis",
        "Blast Radius Analysis",
        "Before any code change, analyze which other agents' work will be affected. \
         Maps the ripple effect of a proposed change through the dependency graph \
         to prevent unintended side effects.",
        SkillCategory::Coordination,
        SkillComplexity::Composite,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::Security,
        ],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are performing a BLAST RADIUS ANALYSIS on a proposed code change.\n\n\
         ANALYSIS STEPS:\n\
         1. **Change Identification**: What exactly is being changed? List every file, \
            function, type, and API endpoint that will be modified.\n\
         2. **Direct Dependents**: What code directly imports/calls the changed artifacts? \
            Which agent owns that code?\n\
         3. **Transitive Dependents**: Follow the dependency chain 2+ levels deep. If Backend \
            changes a DB schema, what services read that table? What Frontend components \
            display that data?\n\
         4. **Runtime Impact**: Will this change affect runtime behavior for code that is NOT \
            recompiled? (e.g., API contract changes affecting deployed consumers)\n\
         5. **Test Impact**: Which test suites need to be re-run? Any tests that will break?\n\
         6. **Risk Assessment**: Score the change from 1 (isolated, safe) to 5 (system-wide, \
            high risk). Anything >= 3 requires explicit CTO approval.\n\n\
         OUTPUT:\n\
         ```\n\
         RISK_SCORE: <1-5>\n\
         AFFECTED_AGENTS: [<AgentRole>]\n\
         AFFECTED_FILES: [<path>]\n\
         BREAKING_CHANGES: [<description>]\n\
         REQUIRED_MIGRATIONS: [<description>]\n\
         RECOMMENDED_ORDER: <sequence of agent updates>\n\
         ```",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 500,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 7. AgentMemorySharing
// ---------------------------------------------------------------------------

fn agent_memory_sharing() -> Skill {
    Skill::new(
        "coord_agent_memory_sharing",
        "Agent Memory Sharing",
        "Cross-agent knowledge propagation: what one agent learns (patterns, decisions, \
         caveats, discovered bugs), all relevant agents know. Maintains a shared \
         memory bank with topic-based routing.",
        SkillCategory::Coordination,
        SkillComplexity::Composite,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::DevOps,
            AgentRole::Qa,
            AgentRole::Security,
            AgentRole::Monitor,
        ],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are the AGENT MEMORY SHARING coordinator.\n\n\
         MEMORY TYPES:\n\
         1. **Decision Records**: Architecture decisions, tech choices, rejected alternatives. \
            Route to: all agents.\n\
         2. **API Contracts**: Endpoint signatures, request/response schemas, auth requirements. \
            Route to: Backend, Frontend, Security, QA.\n\
         3. **Bug Discoveries**: Bugs found during development, root causes, fixes applied. \
            Route to: all agents working on related code.\n\
         4. **Pattern Libraries**: Reusable code patterns established in this project. \
            Route to: Backend, Frontend, DevOps.\n\
         5. **Security Findings**: Vulnerabilities, hardening measures, compliance requirements. \
            Route to: ALL agents (security is everyone's responsibility).\n\
         6. **Performance Baselines**: Benchmark results, latency budgets, resource limits. \
            Route to: Backend, Frontend, DevOps, Monitor.\n\n\
         SHARING PROTOCOL:\n\
         a) When an agent produces a learning, classify it into one of the above types.\n\
         b) Determine which agents need this knowledge based on the routing rules.\n\
         c) Compress the knowledge into a concise, actionable summary (max 200 tokens).\n\
         d) Tag with: topic, urgency (immediate|next-task|background), source agent, timestamp.\n\
         e) NEVER share raw context dumps -- always distill to actionable insights.\n\n\
         DEDUPLICATION: Before sharing, check if this knowledge is already in the shared bank. \
         Update existing entries rather than creating duplicates.",
    )
    .with_quality_threshold(0.80)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 250,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 8. RollbackCoordination
// ---------------------------------------------------------------------------

fn rollback_coordination() -> Skill {
    Skill::new(
        "coord_rollback_coordination",
        "Rollback Coordination",
        "If one agent's work fails validation or causes integration breakage, \
         coordinate the rollback of that agent's changes AND all dependent agents' \
         work that was built on top of the failed output.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::DevOps,
            AgentRole::Monitor,
        ],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are coordinating a ROLLBACK after an agent's work has failed.\n\n\
         ROLLBACK PROTOCOL:\n\
         1. **Identify Failure**: Which agent failed? What specifically broke? \
            (compilation error, test failure, security vulnerability, integration mismatch)\n\
         2. **Map Blast Radius**: Using the dependency graph, identify all agents whose \
            work depends on the failed output. These are rollback candidates.\n\
         3. **Determine Rollback Depth**: Not all dependent work needs rollback. If Agent B \
            used Agent A's API contract but not the implementation, B's work might survive \
            if the contract is preserved.\n\
         4. **Execute Rollback Order**: Roll back in REVERSE dependency order. Deepest \
            dependents first, failed agent last. This prevents intermediate broken states.\n\
         5. **Preserve Salvageable Work**: Before discarding an agent's output, check if \
            portions are still valid. Extract and save reusable fragments.\n\
         6. **Re-plan**: After rollback, produce a revised execution plan that avoids the \
            original failure mode. Document what went wrong as a shared memory entry.\n\n\
         CRITICAL RULES:\n\
         - NEVER partially rollback a single agent's output. It is all or nothing per agent.\n\
         - ALWAYS create a rollback log entry for audit purposes.\n\
         - If rollback affects > 50% of completed work, escalate to CTO for strategic re-plan.\n\
         - After rollback, run integration checks before resuming forward progress.",
    )
    .with_quality_threshold(0.90)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 1000,
        fallback_skill: Some(SkillId::new("coord_emergency_escalation")),
    })
}

// ---------------------------------------------------------------------------
// 9. ParallelMergeStrategy
// ---------------------------------------------------------------------------

fn parallel_merge_strategy() -> Skill {
    Skill::new(
        "coord_parallel_merge_strategy",
        "Parallel Merge Strategy",
        "Merge outputs from agents that executed in parallel. Resolves conflicts, \
         ensures consistency across merged artifacts, and validates that the \
         combined output is greater than the sum of its parts.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
        ],
        OutputFormat::Code,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are executing a PARALLEL MERGE STRATEGY for outputs from concurrent agents.\n\n\
         MERGE PROCESS:\n\
         1. **Inventory**: List all outputs to merge with their source agent and artifact type.\n\
         2. **Conflict Scan**: Run semantic conflict detection on overlapping artifacts. \
            Classify each conflict: TEXTUAL (same file, different edits), SEMANTIC (different \
            files, contradictory logic), or STRUCTURAL (incompatible module organization).\n\
         3. **Resolution Strategy** (per conflict):\n\
            - TEXTUAL: Three-way merge. If ambiguous, prefer the agent with higher domain authority.\n\
            - SEMANTIC: Escalate to consensus protocol. Both agents must agree on resolution.\n\
            - STRUCTURAL: Architect decides. CTO overrides if Architect is a conflicting party.\n\
         4. **Integration Validation**: After merge, verify:\n\
            a) All imports resolve. b) No duplicate symbol definitions.\n\
            c) API contracts are satisfied end-to-end. d) Test suites pass.\n\
         5. **Consistency Polish**: Normalize naming conventions, import ordering, and \
            documentation style across merged outputs.\n\n\
         PRIORITY ORDER (when conflicts cannot be automatically resolved):\n\
         Security > Correctness > API Contract > Performance > Style\n\n\
         OUTPUT: The merged artifact set with a merge report listing all resolutions made.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 1000,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 10. SkillChainOrchestration
// ---------------------------------------------------------------------------

fn skill_chain_orchestration() -> Skill {
    Skill::new(
        "coord_skill_chain_orchestration",
        "Skill Chain Orchestration",
        "Chain multiple skills across agents with structured data passing between \
         stages, checkpoint/resume capability, and graceful degradation if a \
         mid-chain skill fails.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Cto, AgentRole::Architect],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are the SKILL CHAIN ORCHESTRATOR managing a multi-step, multi-agent workflow.\n\n\
         CHAIN SEMANTICS:\n\
         1. A skill chain is an ordered list of (SkillId, AgentRole) pairs with data edges.\n\
         2. Each skill receives: its own input, the accumulated context from previous skills, \
            and any shared memory entries tagged as relevant.\n\
         3. Data flows forward through the chain via typed handoff objects. Each handoff \
            has a schema that the receiving skill validates before proceeding.\n\n\
         CHECKPOINT/RESUME:\n\
         - After each skill completes, persist its output as a checkpoint.\n\
         - If the chain is interrupted (timeout, crash, budget exhaustion), it can resume \
            from the last successful checkpoint without re-executing completed skills.\n\
         - Checkpoints include: skill_id, agent_role, output, quality_score, tokens_used, timestamp.\n\n\
         FAILURE HANDLING:\n\
         - If a skill fails and has a fallback_skill in its RetryStrategy, try the fallback.\n\
         - If no fallback or fallback also fails, attempt to skip the skill if downstream \
            skills can operate with partial input (marked as optional in the chain definition).\n\
         - If the failed skill is marked as required, halt the chain and escalate.\n\n\
         DATA PASSING RULES:\n\
         - NEVER pass the full output of one skill as raw text to the next. Extract and \
            structure the relevant data points.\n\
         - Each handoff should be < 2000 tokens to conserve context window.\n\
         - Include a one-line summary of what was produced and what is expected next.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 1000,
        fallback_skill: Some(SkillId::new("coord_emergency_escalation")),
    })
}

// ---------------------------------------------------------------------------
// 11. QualityGateEnforcement
// ---------------------------------------------------------------------------

fn quality_gate_enforcement() -> Skill {
    Skill::new(
        "coord_quality_gate_enforcement",
        "Quality Gate Enforcement",
        "Enforce minimum quality scores across multiple dimensions before work \
         proceeds to the next phase. Gates are non-negotiable: if quality is \
         below threshold, the work is sent back for refinement.",
        SkillCategory::Coordination,
        SkillComplexity::Composite,
        vec![
            AgentRole::Cto,
            AgentRole::Qa,
            AgentRole::Security,
            AgentRole::Architect,
        ],
        OutputFormat::Report,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are a QUALITY GATE ENFORCER. Your job is to objectively score agent output \
         and block progression if quality is insufficient.\n\n\
         QUALITY DIMENSIONS (score each 0.0-1.0):\n\
         1. **Correctness**: Does it work? Logic errors, off-by-one, null derefs, type mismatches.\n\
         2. **Security**: OWASP top 10 coverage, input validation, auth/authz, secrets management.\n\
         3. **Performance**: Algorithmic complexity, memory allocation patterns, caching strategy.\n\
         4. **Maintainability**: Code clarity, naming, documentation, single-responsibility adherence.\n\
         5. **Test Coverage**: Unit, integration, edge case, error path coverage.\n\
         6. **Documentation**: Public API docs, inline comments for non-obvious logic, README updates.\n\
         7. **Accessibility**: (Frontend only) WCAG compliance, keyboard navigation, screen reader.\n\
         8. **Consistency**: Adherence to project patterns, naming conventions, error handling style.\n\n\
         GATE RULES:\n\
         - Each dimension has a minimum threshold (default 0.7, configurable per skill).\n\
         - The aggregate score (weighted average) must also meet the skill's quality_threshold.\n\
         - If ANY dimension is below 0.5, the gate FAILS regardless of aggregate score.\n\
         - When a gate fails, provide SPECIFIC, ACTIONABLE feedback for each failing dimension.\n\
         - Do NOT suggest improvements for passing dimensions -- focus reviewer attention.\n\n\
         SCORING CALIBRATION:\n\
         - 0.9-1.0: Production-ready, exemplary.\n\
         - 0.7-0.9: Acceptable, minor improvements possible.\n\
         - 0.5-0.7: Below standard, specific issues need fixing.\n\
         - 0.0-0.5: Fundamentally flawed, major rework required.",
    )
    .with_quality_threshold(0.80)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 500,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 12. AgentSpecialization
// ---------------------------------------------------------------------------

fn agent_specialization() -> Skill {
    Skill::new(
        "coord_agent_specialization",
        "Agent Specialization",
        "Dynamically specialize agents based on project requirements. For example, \
         the Backend agent becomes a 'Database Expert' for DB-heavy tasks, or the \
         Frontend agent becomes an 'Accessibility Specialist' for a11y work.",
        SkillCategory::Coordination,
        SkillComplexity::Composite,
        vec![AgentRole::Cto, AgentRole::Architect],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are performing AGENT SPECIALIZATION to optimize team performance for this task.\n\n\
         SPECIALIZATION PROCESS:\n\
         1. **Task Analysis**: Identify the dominant technical domains in the current task \
            (e.g., 70% database work, 20% API, 10% frontend).\n\
         2. **Agent Capability Matrix**: Map each agent's strengths to the task's needs. \
            Consider: role expertise, knowledge scope, historical quality scores.\n\
         3. **Specialization Assignment**: For each agent, define a specialization overlay:\n\
            - Specialized role name (e.g., 'Database Expert', 'API Gateway Specialist')\n\
            - Additional system prompt injection with domain-specific instructions\n\
            - Adjusted quality thresholds (higher for primary domain, standard for secondary)\n\
            - Recommended knowledge files to prioritize\n\
         4. **Cross-training**: Identify skills gaps. If no agent is strong in a required domain, \
            assign the closest match and inject extra context from knowledge files.\n\n\
         RULES:\n\
         - Each agent retains its base role capabilities -- specialization is additive.\n\
         - Specialization lasts for the duration of the current task only.\n\
         - CTO and Security agents NEVER lose their oversight capabilities when specialized.\n\
         - Monitor agent can be specialized but retains observability duties.\n\
         - Maximum 2 specializations per agent to prevent role confusion.",
    )
    .with_quality_threshold(0.80)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 500,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 13. DeadlockDetection
// ---------------------------------------------------------------------------

fn deadlock_detection() -> Skill {
    Skill::new(
        "coord_deadlock_detection",
        "Deadlock Detection",
        "Detect and resolve circular dependencies between agent tasks. Uses \
         topological sort analysis on the task dependency graph to identify \
         cycles and proposes resolution strategies.",
        SkillCategory::Coordination,
        SkillComplexity::Composite,
        vec![AgentRole::Cto, AgentRole::Architect, AgentRole::Monitor],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are the DEADLOCK DETECTOR monitoring the agent task dependency graph.\n\n\
         DETECTION METHOD:\n\
         1. **Build Dependency Graph**: From the current execution plan, extract all tasks \
            and their declared dependencies (task A waits for task B's output).\n\
         2. **Cycle Detection**: Run topological sort. If sorting fails, a cycle exists. \
            Report the exact cycle path: A -> B -> C -> A.\n\
         3. **Implicit Dependencies**: Check for undeclared dependencies: Agent A and B \
            both need to write to the same file, or both read a resource that the other modifies.\n\
         4. **Resource Contention**: Identify shared resources (files, APIs, database tables) \
            that multiple agents access concurrently without coordination.\n\n\
         RESOLUTION STRATEGIES:\n\
         a) **Interface Extraction**: Break the cycle by having both agents agree on an interface \
            contract first, then implement independently against it.\n\
         b) **Task Reordering**: Restructure the execution plan to eliminate the cycle \
            by introducing an intermediate phase.\n\
         c) **Dependency Inversion**: Reverse one edge in the cycle by having the downstream \
            agent provide a mock/stub that the upstream agent can code against.\n\
         d) **Merge Tasks**: If two tasks are so tightly coupled that they form a cycle, \
            merge them into a single task assigned to one agent.\n\n\
         URGENCY: Deadlocks block ALL downstream work. Detection should run before execution \
         starts and after any re-planning event.",
    )
    .with_quality_threshold(0.90)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 250,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 14. LoadBalancedDelegation
// ---------------------------------------------------------------------------

fn load_balanced_delegation() -> Skill {
    Skill::new(
        "coord_load_balanced_delegation",
        "Load-Balanced Delegation",
        "Distribute work evenly across agents based on current workload, estimated \
         effort, and historical throughput. Prevents any single agent from becoming \
         a bottleneck while others idle.",
        SkillCategory::Coordination,
        SkillComplexity::Composite,
        vec![AgentRole::Cto, AgentRole::Architect, AgentRole::Monitor],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are the LOAD BALANCER for the 8-agent team.\n\n\
         BALANCING FACTORS:\n\
         1. **Current Load**: How many active tasks does each agent have? What is their \
            token consumption rate?\n\
         2. **Estimated Effort**: How many tokens will this new task likely consume? \
            Use historical averages for similar skill categories.\n\
         3. **Agent Throughput**: Historical tokens-per-minute for each agent. Some agents \
            (Opus models) are slower but higher quality; others (Haiku) are faster.\n\
         4. **Role Fit**: Even under load, prefer the agent whose role matches the task. \
            Only overflow to a secondary agent if primary is > 80% loaded.\n\
         5. **Quality History**: If an agent consistently scores below threshold on a skill \
            type, prefer a different agent even if they are slightly more loaded.\n\n\
         DELEGATION ALGORITHM:\n\
         score(agent, task) = role_fit_weight * role_match \
                            + load_weight * (1.0 - current_load_pct) \
                            + quality_weight * avg_quality_score \
                            + throughput_weight * tokens_per_minute\n\
         Select the agent with the highest composite score.\n\n\
         OVERFLOW RULES:\n\
         - If all agents are > 90% loaded, queue the task rather than degrading quality.\n\
         - If a task has been queued for > 30 seconds, escalate to CTO for re-prioritization.\n\
         - Monitor agent should NEVER receive code generation tasks, even under extreme load.",
    )
    .with_quality_threshold(0.80)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 250,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 15. CrossCuttingConcernSync
// ---------------------------------------------------------------------------

fn cross_cutting_concern_sync() -> Skill {
    Skill::new(
        "coord_cross_cutting_concern_sync",
        "Cross-Cutting Concern Sync",
        "Synchronize cross-cutting concerns -- logging, error handling, \
         authentication, authorization, observability, and configuration -- \
         across all agents' outputs for consistency.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::DevOps,
            AgentRole::Security,
        ],
        OutputFormat::Code,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "You are synchronizing CROSS-CUTTING CONCERNS across all agents' code outputs.\n\n\
         CONCERNS TO SYNCHRONIZE:\n\
         1. **Logging**: Same log format, same severity levels, same correlation ID propagation. \
            Every agent's code must use the project's logging facade, not raw println/console.log.\n\
         2. **Error Handling**: Consistent error types, error propagation patterns, and user-facing \
            error messages. Backend and Frontend must agree on error response schema.\n\
         3. **Authentication**: Same token validation logic, same session management, same \
            header conventions (Authorization: Bearer). No agent should roll its own auth.\n\
         4. **Authorization**: RBAC/ABAC policies applied consistently. If Backend checks \
            permissions, Frontend must also gate UI elements accordingly.\n\
         5. **Observability**: Consistent metric names, tracing span conventions, health check \
            endpoints. DevOps and Monitor must be able to correlate across all services.\n\
         6. **Configuration**: Same config loading pattern, same environment variable naming, \
            same secret management approach. No hardcoded values.\n\n\
         SYNC PROCESS:\n\
         a) Extract the cross-cutting pattern from each agent's output.\n\
         b) Identify divergences from the established project patterns.\n\
         c) For each divergence, produce a specific diff showing what needs to change.\n\
         d) Verify that the corrections do not break the agent's core logic.\n\n\
         GOLDEN RULE: Cross-cutting concerns should be implemented ONCE in a shared module \
         and imported everywhere. Duplicated cross-cutting logic is a coordination failure.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 16. IncrementalIntegration
// ---------------------------------------------------------------------------

fn incremental_integration() -> Skill {
    Skill::new(
        "coord_incremental_integration",
        "Incremental Integration",
        "Continuously integrate agent outputs as they become available rather \
         than waiting for a big-bang merge at the end. Each increment is \
         validated before accepting the next.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::DevOps,
            AgentRole::Qa,
        ],
        OutputFormat::Report,
    )
    .with_estimated_tokens(6144)
    .with_system_prompt(
        "You are managing INCREMENTAL INTEGRATION of agent outputs.\n\n\
         INTEGRATION CADENCE:\n\
         1. As soon as an agent completes a subtask, attempt integration immediately.\n\
         2. Integration = merge the output into the accumulated project state.\n\
         3. After each integration, run validation checks before accepting.\n\n\
         VALIDATION CHECKS (per increment):\n\
         a) **Syntax**: Does the merged codebase parse/compile without errors?\n\
         b) **Type Safety**: Do all type references resolve? Are generics instantiated correctly?\n\
         c) **Import Resolution**: Do all imports point to existing modules/symbols?\n\
         d) **API Contract**: Do new endpoints/functions match their declared contracts?\n\
         e) **Test Regression**: Do existing tests still pass after the merge?\n\n\
         REJECTION PROTOCOL:\n\
         - If validation fails, reject the increment and return specific errors to the agent.\n\
         - The agent must fix issues before re-submitting.\n\
         - If an agent fails 3 consecutive integration attempts, escalate to CTO.\n\n\
         ADVANTAGES OVER BIG-BANG:\n\
         - Errors caught early when context is fresh.\n\
         - Each agent gets immediate feedback on integration compatibility.\n\
         - Final merge is trivial because all pieces have been pre-validated.\n\n\
         STATE MANAGEMENT: Maintain a running integration state with a changelog of \
         which agent contributed what, when, and the validation results.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 500,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 17. AgentDebateProtocol
// ---------------------------------------------------------------------------

fn agent_debate_protocol() -> Skill {
    Skill::new(
        "coord_agent_debate_protocol",
        "Agent Debate Protocol",
        "Structured debate between agents on contested decisions. Each side \
         presents evidence, rebuts the other, and a judge (CTO) renders a \
         binding decision with written rationale.",
        SkillCategory::Coordination,
        SkillComplexity::Orchestrated,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::Security,
        ],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(10240)
    .with_system_prompt(
        "You are in a STRUCTURED DEBATE between agents on a contested technical decision.\n\n\
         DEBATE FORMAT (3 rounds maximum):\n\
         **Round 1 -- Opening Statements**:\n\
         - Each debating agent states their position with supporting evidence.\n\
         - Evidence must be concrete: code examples, benchmark data, security advisories, \
           documentation references. Opinions without evidence carry zero weight.\n\n\
         **Round 2 -- Rebuttals**:\n\
         - Each agent responds to the other's evidence. Must address every point raised.\n\
         - New evidence is allowed. Ad hominem (\"Agent B always makes bad choices\") is NOT.\n\
         - Identify common ground: where do both sides agree?\n\n\
         **Round 3 -- Final Statements**:\n\
         - Each agent summarizes their position incorporating rebuttals.\n\
         - Must explicitly state: \"If the other side is chosen, the risks are X and \
           mitigations would be Y.\" This tests intellectual honesty.\n\n\
         **JUDGMENT (CTO)**:\n\
         - CTO reads all rounds and renders a binding decision.\n\
         - Decision must cite specific evidence from the debate.\n\
         - Decision must include: chosen approach, rejected alternative, risk mitigations \
           borrowed from the losing side, and implementation constraints.\n\n\
         DEBATE ETHICS:\n\
         - Prefer the option with better reversibility (easier to change later).\n\
         - Prefer the option with better observability (easier to debug in production).\n\
         - When evidence is evenly balanced, prefer simplicity.",
    )
    .with_quality_threshold(0.85)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 1000,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 18. EmergencyEscalation
// ---------------------------------------------------------------------------

fn emergency_escalation() -> Skill {
    Skill::new(
        "coord_emergency_escalation",
        "Emergency Escalation",
        "When agents are stuck (repeated failures, unresolvable conflicts, budget \
         exhaustion), escalate to the CTO agent with full context for human-like \
         strategic judgment.",
        SkillCategory::Coordination,
        SkillComplexity::Atomic,
        vec![AgentRole::Cto],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(8192)
    .with_system_prompt(
        "EMERGENCY ESCALATION has been triggered. You are the CTO agent receiving an escalation.\n\n\
         ESCALATION CONTEXT (provided to you):\n\
         - Which agent(s) are stuck and why.\n\
         - Number of failed attempts and error details.\n\
         - Current execution plan state (what has succeeded, what is blocked).\n\
         - Token budget consumed vs remaining.\n\
         - Time elapsed vs deadline.\n\n\
         YOUR RESPONSE MUST INCLUDE:\n\
         1. **Root Cause Analysis**: Why are agents stuck? Is it a task decomposition problem, \
            a missing capability, an external blocker, or a fundamental approach flaw?\n\
         2. **Strategic Decision**: Choose ONE of:\n\
            a) RETRY with modified approach (specify what changes)\n\
            b) SKIP the blocked task and work around it (specify workaround)\n\
            c) SIMPLIFY the overall plan to reduce scope (specify what to cut)\n\
            d) ABORT the current task and report inability (last resort)\n\
         3. **Revised Plan**: If not aborting, provide a concrete revised execution plan \
            with updated task assignments, modified skill parameters, or reduced scope.\n\
         4. **Prevention Note**: What should the system learn to avoid this escalation next time?\n\n\
         DECISION PRINCIPLES:\n\
         - Favor shipping something correct over shipping everything incomplete.\n\
         - If budget is > 70% consumed with < 30% progress, seriously consider simplification.\n\
         - Never throw away completed, validated work unless it is fundamentally wrong.\n\
         - Document the decision for the retrospective.",
    )
    .with_quality_threshold(0.90)
    .with_retry_strategy(RetryStrategy {
        max_retries: 0,
        backoff_ms: 0,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 19. PostTaskRetrospective
// ---------------------------------------------------------------------------

fn post_task_retrospective() -> Skill {
    Skill::new(
        "coord_post_task_retrospective",
        "Post-Task Retrospective",
        "After task completion, agents share what went well, what went poorly, \
         and specific improvements for future coordination. Feeds into the \
         shared memory bank as institutional knowledge.",
        SkillCategory::Coordination,
        SkillComplexity::Composite,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Backend,
            AgentRole::Frontend,
            AgentRole::DevOps,
            AgentRole::Qa,
            AgentRole::Security,
            AgentRole::Monitor,
        ],
        OutputFormat::Report,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are participating in a POST-TASK RETROSPECTIVE.\n\n\
         EACH AGENT REPORTS:\n\
         1. **What Went Well**: Specific coordination patterns, skill chains, or decisions \
            that led to good outcomes. Be concrete -- \"the blast radius analysis before \
            the schema change caught 3 breaking changes\" not \"things went okay\".\n\
         2. **What Went Poorly**: Coordination failures, wasted tokens, rework cycles, \
            conflicts that could have been avoided. No blame -- focus on systemic improvements.\n\
         3. **Metrics**: Tokens used vs estimated, iterations needed vs expected, \
            quality scores achieved vs threshold.\n\
         4. **Improvement Proposals**: Specific, actionable changes to coordination skills, \
            prompt templates, quality thresholds, or execution plan templates.\n\n\
         CTO SYNTHESIS:\n\
         After all agents report, the CTO synthesizes findings into:\n\
         a) **Pattern Updates**: Adjustments to coordination skill parameters.\n\
         b) **Memory Entries**: New shared knowledge items for the memory bank.\n\
         c) **Threshold Adjustments**: Raise or lower quality thresholds based on observed \
            quality vs effort trade-offs.\n\
         d) **Process Changes**: Modifications to the default execution plan structure.\n\n\
         RETROSPECTIVE BUDGET: This skill should consume < 5% of the total task token budget. \
         Be concise. Value density over volume.",
    )
    .with_quality_threshold(0.70)
    .with_retry_strategy(RetryStrategy {
        max_retries: 0,
        backoff_ms: 0,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// 20. ContextWindowOptimizer
// ---------------------------------------------------------------------------

fn context_window_optimizer() -> Skill {
    Skill::new(
        "coord_context_window_optimizer",
        "Context Window Optimizer",
        "Intelligently manage context window utilization across all agents. \
         Determines what each agent needs to see, compresses irrelevant context, \
         and shares only the relevant portions to maximize effective token usage.",
        SkillCategory::Coordination,
        SkillComplexity::Composite,
        vec![
            AgentRole::Cto,
            AgentRole::Architect,
            AgentRole::Monitor,
        ],
        OutputFormat::Plan,
    )
    .with_estimated_tokens(4096)
    .with_system_prompt(
        "You are the CONTEXT WINDOW OPTIMIZER managing token budgets across 8 agents.\n\n\
         OPTIMIZATION STRATEGIES:\n\
         1. **Relevance Filtering**: For each agent, determine which portions of the shared \
            context are relevant to their current task. Backend does not need Frontend's \
            CSS details. Security does not need DevOps's CI/CD pipeline config (unless \
            it contains secrets management).\n\
         2. **Progressive Summarization**: As context accumulates, compress older entries:\n\
            - Last 2 interactions: full detail.\n\
            - Interactions 3-5: key decisions and code signatures only.\n\
            - Interactions 6+: one-line summary per interaction.\n\
         3. **Deduplication**: Remove repeated information. If the same API schema appears \
            in 3 agent contexts, store it once and reference it.\n\
         4. **Priority Injection**: When context window is nearly full, prioritize:\n\
            a) Current task instructions (always full).\n\
            b) Relevant API contracts and schemas.\n\
            c) Recent review feedback.\n\
            d) Shared memory entries tagged as high-urgency.\n\
            e) Historical context (lowest priority, summarize aggressively).\n\n\
         BUDGET ALLOCATION:\n\
         - System prompt + skill prompt: ~20% of context window.\n\
         - Current task context: ~40%.\n\
         - Shared knowledge: ~25%.\n\
         - Historical context: ~15%.\n\n\
         METRICS TO TRACK:\n\
         - Context utilization per agent (% of window used).\n\
         - Relevance score of injected context (estimated by topic overlap).\n\
         - Wasted tokens (context injected but never referenced in output).",
    )
    .with_quality_threshold(0.80)
    .with_retry_strategy(RetryStrategy {
        max_retries: 1,
        backoff_ms: 250,
        fallback_skill: None,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_all_coordination_skills() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        assert_eq!(registry.by_category(SkillCategory::Coordination).len(), 20);
    }

    #[test]
    fn test_consensus_protocol_agents() {
        let skill = consensus_protocol();
        assert_eq!(skill.required_agents.len(), 8);
        assert!(skill.required_agents.contains(&AgentRole::Cto));
    }

    #[test]
    fn test_emergency_escalation_is_cto_only() {
        let skill = emergency_escalation();
        assert_eq!(skill.required_agents, vec![AgentRole::Cto]);
        assert_eq!(skill.retry_strategy.max_retries, 0);
    }

    #[test]
    fn test_all_coordination_skills_have_prompts() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        for skill in registry.by_category(SkillCategory::Coordination) {
            assert!(
                !skill.system_prompt_extension.is_empty(),
                "Skill {} has empty system prompt",
                skill.id
            );
            assert!(
                skill.system_prompt_extension.len() > 200,
                "Skill {} has suspiciously short system prompt ({} chars)",
                skill.id,
                skill.system_prompt_extension.len()
            );
        }
    }

    #[test]
    fn test_quality_thresholds_are_valid() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        for skill in registry.by_category(SkillCategory::Coordination) {
            assert!(
                skill.quality_threshold >= 0.0 && skill.quality_threshold <= 1.0,
                "Skill {} has invalid quality threshold: {}",
                skill.id,
                skill.quality_threshold
            );
        }
    }

    #[test]
    fn test_skill_ids_are_prefixed() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        for skill in registry.by_category(SkillCategory::Coordination) {
            assert!(
                skill.id.as_str().starts_with("coord_"),
                "Coordination skill {} should have coord_ prefix",
                skill.id
            );
        }
    }

    #[test]
    fn test_fallback_skills_exist() {
        let mut registry = SkillRegistry::new();
        register(&mut registry);
        for skill in registry.by_category(SkillCategory::Coordination) {
            if let Some(ref fallback) = skill.retry_strategy.fallback_skill {
                assert!(
                    registry.get(fallback).is_some(),
                    "Skill {} references non-existent fallback skill {}",
                    skill.id,
                    fallback
                );
            }
        }
    }
}
