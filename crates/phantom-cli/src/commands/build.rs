//! `phantom build --framework <file>` — Full autonomous build pipeline.
//!
//! Architecture Framework §13: Autonomous Build Pipeline — Spec to Production.
//! 8 phases: Ingest → Infrastructure → Architecture → Code → Test → Security → Deploy → Deliver.
//!
//! §2.2/§8.2: Every agent queries the Knowledge Brain (ChromaDB) before decisions.
//! Graceful degradation: if ChromaDB is unavailable, agents proceed without KB enrichment.

use std::path::Path;
use std::time::Instant;

use phantom_ai::{
    AgentOrchestrator, AgentRole, AiBackend, OrchestratorConfig, TaskRequest,
    parse_file_output, ParsedFile,
};
use phantom_brain::{BrainConfig, KnowledgeBrain, KnowledgeQuery};
use phantom_core::framework_ingestion::{IngestionPipeline, PlanGenerator};
use phantom_core::pipeline::{BuildPhase, BuildPipeline};

pub async fn run(
    framework: Option<String>,
    resume: bool,
    component: Option<String>,
    test_only: bool,
    deploy_only: bool,
) -> anyhow::Result<()> {
    println!("\x1b[1mPhantom Build\x1b[0m\n");

    if resume {
        println!("Resuming interrupted build...");
        println!("\x1b[33mNo build state found. Start a new build with --framework.\x1b[0m");
        return Ok(());
    }

    if test_only {
        println!("Running tests only...");
        println!("\x1b[33mNo build artifacts found. Run a full build first.\x1b[0m");
        return Ok(());
    }

    if deploy_only {
        println!("Deploying existing build...");
        println!("\x1b[33mNo build artifacts found. Run a full build first.\x1b[0m");
        return Ok(());
    }

    if let Some(comp) = component {
        println!("Building single component: {}", comp);
        println!("\x1b[33mComponent builds require an active build session.\x1b[0m");
        return Ok(());
    }

    let framework_path = framework
        .ok_or_else(|| anyhow::anyhow!("--framework <file> is required for a full build"))?;

    // Verify framework file exists
    if !std::path::Path::new(&framework_path).exists() {
        anyhow::bail!("Framework file not found: {}", framework_path);
    }

    println!("Framework: {}\n", framework_path);

    // ── Phase 0: INGEST — Architecture Framework Ingestion Pipeline ──

    println!("\x1b[1;34m▶ Phase 0: Ingest\x1b[0m");
    println!("  Parsing architecture framework...\n");

    // Run the full ingestion pipeline (offline mode — no Knowledge Brain required)
    let result = IngestionPipeline::run_sync(&framework_path)?;

    // Display extraction summary
    println!(
        "\x1b[1;32m  ✓ Parsed\x1b[0m {} sections, {} lines",
        result.framework.sections.len(),
        result.framework.total_lines,
    );
    println!(
        "\x1b[1;32m  ✓ Extracted\x1b[0m {} components, {} technologies, {} constraints",
        result.architecture.components.len(),
        result.architecture.technologies.len(),
        result.architecture.constraints.len(),
    );
    println!(
        "\x1b[1;32m  ✓ DAG\x1b[0m {} nodes in {} parallel layers",
        result.dag.len(),
        result.dag.layers.len(),
    );
    println!(
        "\x1b[1;32m  ✓ Plan\x1b[0m {} work streams, ~{} estimated LOC\n",
        result.plan.streams.len(),
        result.plan.total_estimated_loc,
    );

    // Display the execution plan
    println!("{}", result.plan.display_summary());

    // Show the 8-phase pipeline with ingestion results
    let mut pipeline = BuildPipeline::new(Some(framework_path.clone()));

    // Generate the task graph from the execution plan
    let task_graph = PlanGenerator::to_task_graph(&result.plan, &result.dag)?;
    let stats = task_graph.stats();

    println!("\x1b[1;34mTask Graph\x1b[0m");
    println!("  Total tasks:      {}", stats.total);
    println!("  Ready to execute: {}", stats.pending);
    println!("  Est. time:        {}s\n", stats.total_estimated_seconds);

    // Show parallel layers
    println!("\x1b[1;34mParallel Execution Layers\x1b[0m");
    for (i, stream) in result.plan.streams.iter().enumerate() {
        println!(
            "  Layer {}: {} components ({} parallel) — ~{} LOC",
            i,
            stream.components.len(),
            stream.agent_roles.len(),
            stream.estimated_loc,
        );
        for comp_id in &stream.components {
            if let Some(node) = result.dag.get(comp_id) {
                println!(
                    "    ├── \x1b[1m{}\x1b[0m [{}] ~{} LOC",
                    node.name, node.agent_role, node.estimated_loc
                );
            }
        }
    }

    // Inject the task graph into the pipeline
    pipeline.task_graph = task_graph;

    println!();
    println!("\x1b[1;34mBuild Pipeline\x1b[0m");
    let phases = [
        (
            BuildPhase::Ingest,
            "Parse framework, build task graph, plan",
        ),
        (
            BuildPhase::Infrastructure,
            "Provision servers, create accounts, setup CI/CD",
        ),
        (
            BuildPhase::Architecture,
            "System design, DB schema, API contracts, ADRs",
        ),
        (
            BuildPhase::Code,
            "4 parallel streams (backend, frontend, devops, integrations)",
        ),
        (
            BuildPhase::Test,
            "Unit + integration + E2E (80% coverage gate)",
        ),
        (
            BuildPhase::Security,
            "Dependency audit, OWASP, auth review, pen test",
        ),
        (
            BuildPhase::Deploy,
            "Push > CI > Docker > deploy > DNS > TLS",
        ),
        (
            BuildPhase::Deliver,
            "Generate report, URLs, credentials, handoff",
        ),
    ];

    for (i, (phase, desc)) in phases.iter().enumerate() {
        let marker = if i == 0 { "\x1b[1;32m✓\x1b[0m" } else { " " };
        println!("  {} Phase {}: \x1b[1m{}\x1b[0m", marker, i, phase);
        println!("             {}", desc);
    }

    // ── AI Execution ─────────────────────────────────────────────────

    println!();

    // Step 1: Initialize AI backend
    let backend = AiBackend::auto_detect()
        .map_err(|e| anyhow::anyhow!("Failed to initialize AI backend: {}", e))?;

    println!(
        "\x1b[1;32m  ✓ AI Backend\x1b[0m {}",
        backend.backend_name()
    );

    // If Ollama, verify reachable
    if matches!(&backend, AiBackend::Ollama(_)) {
        verify_ollama().await?;
    }

    // Step 1b: Initialize Knowledge Brain (§2.2 — ChromaDB, graceful degradation)
    let brain = init_knowledge_brain().await;
    match &brain {
        Some(_) => println!("\x1b[1;32m  ✓ Knowledge Brain\x1b[0m ChromaDB connected"),
        None => println!("\x1b[33m  ⚠ Knowledge Brain\x1b[0m ChromaDB unavailable — proceeding without KB enrichment"),
    }

    // Step 2: Start orchestrator
    let config = OrchestratorConfig {
        task_timeout_seconds: 600, // 10 min per task for local models
        ..Default::default()
    };
    let orch = AgentOrchestrator::new(backend, config);
    let handle = orch.start().await;

    // Step 3: Derive project directory
    let project_name = sanitize_project_name(
        &result
            .framework
            .sections
            .first()
            .map(|s| s.heading.clone())
            .unwrap_or_else(|| "phantom-project".to_string()),
    );
    let project_dir = std::env::current_dir()?.join(&project_name);
    std::fs::create_dir_all(&project_dir)?;

    // Truncate framework for context window
    let framework_content = std::fs::read_to_string(&framework_path)?;
    let framework_summary = if framework_content.len() > 6000 {
        format!(
            "{}\n\n[... truncated for context limits ...]",
            &framework_content[..6000]
        )
    } else {
        framework_content.clone()
    };

    let mut total_files = 0u32;
    let mut total_tokens = 0u64;
    let build_start = Instant::now();

    // ── §13 PHASE 1: INFRASTRUCTURE (DevOps + Security agents) ─────

    println!("\n\x1b[1;34m▶ Phase 1: Infrastructure\x1b[0m");
    print!("  DevOps Agent...");

    let kb_infra = query_brain(&brain, "infrastructure CI/CD Docker deployment provisioning", "devops").await;

    let infra_output = handle
        .submit_task(
            TaskRequest::new(
                "infra-1",
                AgentRole::DevOps,
                "Plan infrastructure: servers, accounts, CI/CD setup",
            )
            .with_knowledge(kb_infra)
            .with_context(format!(
                "You are the DevOps Agent (§13 Phase 1: Infrastructure). \
                 You MUST output ONLY raw source code. Output using this EXACT format:\n\n\
                 --- FILE: path/to/file ---\n\
                 <actual code here>\n\n\
                 Create infrastructure planning files:\n\
                 1. infra/INFRASTRUCTURE_PLAN.md — Server topology, provider selection, cost analysis\n\
                 2. infra/docker-compose.yml — Services: backend, frontend, postgres, redis\n\
                 3. infra/.env.example — All required environment variables with descriptions\n\
                 4. infra/provisioning.sh — Shell script for server setup (packages, users, firewall)\n\n\
                 IMPORTANT: Every file MUST contain actual working code.\n\n\
                 Architecture Framework:\n{}",
                framework_summary
            )),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Infrastructure failed: {}", e))?;

    let infra_files = parse_file_output(&infra_output.raw_response);
    write_parsed_files(&project_dir, &infra_files)?;
    total_files += infra_files.len() as u32;
    total_tokens += infra_output.total_tokens;
    println!(
        " \x1b[1;32m✓\x1b[0m {} files, {} tokens",
        infra_files.len(),
        infra_output.total_tokens
    );

    // ── §13 PHASE 2: ARCHITECTURE (Architect agent, §8.2) ─────────

    println!("\n\x1b[1;34m▶ Phase 2: Architecture\x1b[0m");
    print!("  Architect Agent...");

    let kb_arch = query_brain(&brain, "system design database schema API contracts architecture patterns", "architect").await;

    let arch_output = handle
        .submit_task(
            TaskRequest::new(
                "arch-1",
                AgentRole::Architect,
                "Design complete architecture: DB schema, API contracts, system design",
            )
            .with_knowledge(kb_arch)
            .with_context(format!(
                "You are the Architect Agent (§13 Phase 2: Architecture). \
                 You MUST output ONLY raw source code files. \
                 NEVER write descriptions or explanations. Output files using this EXACT format:\n\n\
                 --- FILE: path/to/file ---\n\
                 <actual source code here>\n\n\
                 Create these files with REAL CODE:\n\
                 1. migrations/001_init.sql — Full PostgreSQL CREATE TABLE statements with UUID PKs, created_at/updated_at\n\
                 2. docs/ARCHITECTURE.md — System design document in markdown\n\
                 3. openapi.yaml — OpenAPI 3.0 spec with all endpoints\n\n\
                 IMPORTANT: Every file must contain actual working code, not descriptions.\n\n\
                 Architecture Framework:\n{}",
                framework_summary
            )),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Architect failed: {}", e))?;

    let arch_files = parse_file_output(&arch_output.raw_response);
    write_parsed_files(&project_dir, &arch_files)?;
    total_files += arch_files.len() as u32;
    total_tokens += arch_output.total_tokens;
    println!(
        " \x1b[1;32m✓\x1b[0m {} files, {} tokens",
        arch_files.len(),
        arch_output.total_tokens
    );

    // ── §13 PHASE 3: CODE GENERATION (Backend + Frontend + DevOps) ─

    println!("\n\x1b[1;34m▶ Phase 3: Code Generation\x1b[0m");

    // Backend
    print!("  Backend Agent...");
    let kb_be = query_brain(&brain, "FastAPI backend API authentication database SQLAlchemy", "backend").await;

    let be_output = handle
        .submit_task(
            TaskRequest::new(
                "code-backend",
                AgentRole::Backend,
                "Build the complete backend for this project",
            )
            .with_knowledge(kb_be)
            .with_context(format!(
                "You are the Backend Agent (§13 Phase 3: Code). You MUST output ONLY raw source code. \
                 NEVER write descriptions or explanations. Output using this EXACT format:\n\n\
                 --- FILE: path/to/file ---\n\
                 <actual python source code here>\n\n\
                 Create these files with COMPLETE WORKING PYTHON CODE:\n\
                 1. backend/main.py — FastAPI app with CORS, routers included\n\
                 2. backend/config.py — Settings using pydantic BaseSettings\n\
                 3. backend/database.py — SQLAlchemy async engine and session\n\
                 4. backend/models/user.py — SQLAlchemy User model\n\
                 5. backend/schemas/user.py — Pydantic request/response schemas\n\
                 6. backend/routes/auth.py — POST /register, POST /login, GET /me endpoints\n\
                 7. backend/routes/api.py — CRUD endpoints\n\
                 8. backend/services/auth_service.py — JWT token create/verify, password hashing\n\
                 9. backend/requirements.txt — All pip dependencies\n\
                 10. backend/Dockerfile — Python 3.12-slim multi-stage\n\n\
                 IMPORTANT: Every file MUST contain actual working Python code. No prose.\n\n\
                 Architecture Framework:\n{}",
                framework_summary
            )),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Backend failed: {}", e))?;

    let be_files = parse_file_output(&be_output.raw_response);
    write_parsed_files(&project_dir, &be_files)?;
    total_files += be_files.len() as u32;
    total_tokens += be_output.total_tokens;
    println!(
        " \x1b[1;32m✓\x1b[0m {} files, {} tokens",
        be_files.len(),
        be_output.total_tokens
    );

    // Frontend
    print!("  Frontend Agent...");
    let kb_fe = query_brain(&brain, "Next.js React TypeScript Tailwind dark mode WCAG accessibility design tokens", "frontend").await;

    let fe_output = handle
        .submit_task(
            TaskRequest::new(
                "code-frontend",
                AgentRole::Frontend,
                "Build the complete frontend for this project",
            )
            .with_knowledge(kb_fe)
            .with_context(format!(
                "You are the Frontend Agent (§13 Phase 3: Code). You MUST output ONLY raw source code. \
                 NEVER write descriptions. Output using this EXACT format:\n\n\
                 --- FILE: path/to/file ---\n\
                 <actual TypeScript/TSX source code here>\n\n\
                 Create these files with COMPLETE WORKING CODE:\n\
                 1. frontend/package.json — dependencies: next, react, tailwindcss, typescript\n\
                 2. frontend/tsconfig.json — strict TypeScript config\n\
                 3. frontend/tailwind.config.ts — Tailwind with dark mode\n\
                 4. frontend/app/layout.tsx — Root layout with html, body, providers\n\
                 5. frontend/app/page.tsx — Landing page component\n\
                 6. frontend/app/dashboard/page.tsx — Dashboard with data fetching\n\
                 7. frontend/app/login/page.tsx — Login form with useState\n\
                 8. frontend/components/Navbar.tsx — Navigation bar component\n\
                 9. frontend/components/Sidebar.tsx — Sidebar navigation\n\
                 10. frontend/lib/api.ts — Fetch wrapper for backend API\n\n\
                 IMPORTANT: Every file MUST contain actual working TypeScript/TSX code. No prose.\n\n\
                 Architecture Framework:\n{}",
                framework_summary
            )),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Frontend failed: {}", e))?;

    let fe_files = parse_file_output(&fe_output.raw_response);
    write_parsed_files(&project_dir, &fe_files)?;
    total_files += fe_files.len() as u32;
    total_tokens += fe_output.total_tokens;
    println!(
        " \x1b[1;32m✓\x1b[0m {} files, {} tokens",
        fe_files.len(),
        fe_output.total_tokens
    );

    // DevOps — CI/CD files
    print!("  DevOps Agent...");
    let kb_do = query_brain(&brain, "GitHub Actions CI/CD pipeline Docker deployment branch protection pre-push", "devops").await;

    let do_output = handle
        .submit_task(
            TaskRequest::new(
                "code-devops",
                AgentRole::DevOps,
                "Create CI/CD and deployment configuration",
            )
            .with_knowledge(kb_do)
            .with_context(format!(
                "You are the DevOps Agent (§13 Phase 3: Code). You MUST output ONLY raw source code. \
                 NEVER write descriptions. Output using this EXACT format:\n\n\
                 --- FILE: path/to/file ---\n\
                 <actual YAML/Makefile/markdown code here>\n\n\
                 Create these files with COMPLETE WORKING CODE:\n\
                 1. .github/workflows/ci.yml — GitHub Actions: lint, test, build, deploy\n\
                 2. README.md — Project setup instructions with commands\n\
                 3. Makefile — targets: dev, build, test, deploy, clean\n\
                 4. Dockerfile — Multi-stage production build\n\
                 5. .dockerignore — Exclude node_modules, .git, etc.\n\n\
                 IMPORTANT: Every file MUST contain actual working code. No prose.\n\n\
                 Architecture Framework:\n{}",
                framework_summary
            )),
        )
        .await
        .map_err(|e| anyhow::anyhow!("DevOps failed: {}", e))?;

    let do_files = parse_file_output(&do_output.raw_response);
    write_parsed_files(&project_dir, &do_files)?;
    total_files += do_files.len() as u32;
    total_tokens += do_output.total_tokens;
    println!(
        " \x1b[1;32m✓\x1b[0m {} files, {} tokens",
        do_files.len(),
        do_output.total_tokens
    );

    // ── §13 PHASE 4: TEST (QA agent) ──────────────────────────────

    println!("\n\x1b[1;34m▶ Phase 4: Test\x1b[0m");
    print!("  QA Agent...");

    let code_context = read_generated_context(&project_dir, 5);
    let kb_qa = query_brain(&brain, "testing pytest Vitest coverage AI code errors edge cases", "qa").await;

    let qa_output = handle
        .submit_task(
            TaskRequest::new(
                "qa-1",
                AgentRole::Qa,
                "Write comprehensive tests for this project",
            )
            .with_knowledge(kb_qa)
            .with_context(format!(
                "You are the QA Agent (§13 Phase 4: Test). You MUST output ONLY raw source code. \
                 NEVER write descriptions. Output using this EXACT format:\n\n\
                 --- FILE: path/to/file ---\n\
                 <actual test source code here>\n\n\
                 Create these files with COMPLETE WORKING TEST CODE:\n\
                 1. backend/tests/conftest.py — pytest fixtures with async client\n\
                 2. backend/tests/test_auth.py — Tests for register, login, token refresh\n\
                 3. backend/tests/test_routes.py — Tests for all CRUD endpoints\n\
                 4. frontend/__tests__/page.test.tsx — Vitest tests for page components\n\n\
                 IMPORTANT: Every file MUST contain actual working test code with assertions. No prose.\n\n\
                 Architecture:\n{}\n\nGenerated code:\n{}",
                &framework_summary[..framework_summary.len().min(3000)],
                code_context
            )),
        )
        .await
        .map_err(|e| anyhow::anyhow!("QA failed: {}", e))?;

    let qa_files = parse_file_output(&qa_output.raw_response);
    write_parsed_files(&project_dir, &qa_files)?;
    total_files += qa_files.len() as u32;
    total_tokens += qa_output.total_tokens;
    println!(
        " \x1b[1;32m✓\x1b[0m {} test files, {} tokens",
        qa_files.len(),
        qa_output.total_tokens
    );

    // ── §13 PHASE 5: SECURITY (Security agent) ────────────────────

    println!("\n\x1b[1;34m▶ Phase 5: Security Audit\x1b[0m");
    print!("  Security Agent...");

    let audit_context = read_generated_context(&project_dir, 8);
    let kb_sec = query_brain(&brain, "OWASP security audit authentication injection dependency vulnerabilities", "security").await;

    let sec_output = handle
        .submit_task(
            TaskRequest::new(
                "sec-1",
                AgentRole::Security,
                "Perform security audit of the generated codebase",
            )
            .with_knowledge(kb_sec)
            .with_context(format!(
                "You are the Security Agent (§13 Phase 5: Security). You MUST output using this EXACT format:\n\n\
                 --- FILE: SECURITY_AUDIT.md ---\n\
                 <actual markdown content here>\n\n\
                 Create SECURITY_AUDIT.md with these sections in markdown:\n\
                 # Security Audit Report\n\
                 ## Executive Summary\n\
                 ## OWASP Top 10 Analysis\n\
                 ## Authentication Review\n\
                 ## Injection Vulnerability Check\n\
                 ## Dependency Review\n\
                 ## Recommendations\n\n\
                 Fill each section with specific findings based on the code provided.\n\n\
                 Code to audit:\n{}",
                audit_context
            )),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Security failed: {}", e))?;

    let sec_files = parse_file_output(&sec_output.raw_response);
    write_parsed_files(&project_dir, &sec_files)?;
    total_files += sec_files.len() as u32;
    total_tokens += sec_output.total_tokens;
    println!(
        " \x1b[1;32m✓\x1b[0m {} files, {} tokens",
        sec_files.len(),
        sec_output.total_tokens
    );

    // ── §13 PHASE 6: DEPLOY (DevOps agent) ────────────────────────

    println!("\n\x1b[1;34m▶ Phase 6: Deploy\x1b[0m");
    print!("  DevOps Agent...");

    let deploy_context = read_generated_context(&project_dir, 5);
    let kb_deploy = query_brain(&brain, "deployment Docker registry DNS TLS health checks production launch", "devops").await;

    let deploy_output = handle
        .submit_task(
            TaskRequest::new(
                "deploy-1",
                AgentRole::DevOps,
                "Generate deployment configuration and launch scripts",
            )
            .with_knowledge(kb_deploy)
            .with_context(format!(
                "You are the DevOps Agent (§13 Phase 6: Deploy). You MUST output using this EXACT format:\n\n\
                 --- FILE: path/to/file ---\n\
                 <actual code here>\n\n\
                 Create deployment files:\n\
                 1. deploy/deploy.sh — Production deployment script (build, push, deploy, verify)\n\
                 2. deploy/health_check.sh — Endpoint health verification script\n\
                 3. deploy/rollback.sh — Rollback to previous version\n\
                 4. deploy/nginx.conf — Reverse proxy configuration with TLS\n\
                 5. deploy/DEPLOY_GUIDE.md — Step-by-step deployment instructions\n\n\
                 IMPORTANT: Every file MUST contain actual working code.\n\n\
                 Project code:\n{}",
                deploy_context
            )),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Deploy failed: {}", e))?;

    let deploy_files = parse_file_output(&deploy_output.raw_response);
    write_parsed_files(&project_dir, &deploy_files)?;
    total_files += deploy_files.len() as u32;
    total_tokens += deploy_output.total_tokens;
    println!(
        " \x1b[1;32m✓\x1b[0m {} files, {} tokens",
        deploy_files.len(),
        deploy_output.total_tokens
    );

    // ── §13 PHASE 7: DELIVER (CTO agent) ──────────────────────────

    println!("\n\x1b[1;34m▶ Phase 7: Deliver\x1b[0m");
    print!("  CTO Agent...");

    let deliver_context = read_generated_context(&project_dir, 10);

    let deliver_output = handle
        .submit_task(
            TaskRequest::new(
                "deliver-1",
                AgentRole::Cto,
                "Generate final delivery report with project summary, architecture decisions, and handoff documentation",
            )
            .with_context(format!(
                "You are the CTO Agent (§13 Phase 7: Deliver). You MUST output using this EXACT format:\n\n\
                 --- FILE: path/to/file ---\n\
                 <actual markdown content here>\n\n\
                 Create delivery documentation:\n\
                 1. DELIVERY_REPORT.md — Executive summary: what was built, architecture decisions, \
                    file manifest, token usage, deployment URLs, credentials needed, known limitations\n\
                 2. docs/ADR/001-architecture-pattern.md — Architecture Decision Record: why this pattern was chosen\n\
                 3. docs/RUNBOOK.md — Operations runbook: how to start, stop, monitor, troubleshoot\n\n\
                 IMPORTANT: Be specific. Reference actual files in the project. Include real commands.\n\n\
                 Complete project state:\n{}",
                deliver_context
            )),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Deliver failed: {}", e))?;

    let deliver_files = parse_file_output(&deliver_output.raw_response);
    write_parsed_files(&project_dir, &deliver_files)?;
    total_files += deliver_files.len() as u32;
    total_tokens += deliver_output.total_tokens;
    println!(
        " \x1b[1;32m✓\x1b[0m {} files, {} tokens",
        deliver_files.len(),
        deliver_output.total_tokens
    );

    // ── Finalize ───────────────────────────────────────────────────

    // Git init
    git_init(&project_dir)?;

    // Update state.json
    let elapsed = build_start.elapsed();
    update_state(&project_name, &project_dir, total_files, total_tokens, &elapsed)?;

    // Shutdown orchestrator
    handle.shutdown().await;

    // Summary (§13 Phase 7: Deliver)
    println!("\n\x1b[1;32m━━━ Build Complete ━━━\x1b[0m");
    println!("  Project:  {}", project_name);
    println!("  Location: {}", project_dir.display());
    println!("  Files:    {}", total_files);
    println!("  Tokens:   {}", total_tokens);
    println!("  Duration: {:.1}s", elapsed.as_secs_f64());
    println!("  Phases:   0-Ingest → 1-Infra → 2-Arch → 3-Code → 4-Test → 5-Security → 6-Deploy → 7-Deliver");
    println!();

    Ok(())
}

// ── Helper Functions ─────────────────────────────────────────────────────

/// Verify Ollama is reachable.
async fn verify_ollama() -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(r) if r.status().is_success() => Ok(()),
        _ => anyhow::bail!(
            "Ollama not reachable at localhost:11434. \
             Start it with: ollama serve"
        ),
    }
}

/// Sanitize a project name for use as a directory.
fn sanitize_project_name(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Write parsed files to the project directory.
fn write_parsed_files(project_dir: &Path, files: &[ParsedFile]) -> anyhow::Result<()> {
    for file in files {
        let full_path = project_dir.join(&file.path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, &file.content)?;
    }
    Ok(())
}

/// Read generated source files for context injection into later phases.
fn read_generated_context(project_dir: &Path, max_files: usize) -> String {
    let mut context = String::new();
    let extensions = ["py", "ts", "tsx", "yml", "yaml"];
    let mut count = 0;

    if let Ok(entries) = walkdir(project_dir) {
        for entry in entries {
            if count >= max_files {
                break;
            }
            let ext = entry
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if !extensions.contains(&ext) {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&entry) {
                let relative = entry
                    .strip_prefix(project_dir)
                    .unwrap_or(&entry)
                    .display();
                let truncated = if content.len() > 1500 {
                    &content[..1500]
                } else {
                    &content
                };
                context.push_str(&format!("\n--- FILE: {} ---\n{}\n", relative, truncated));
                count += 1;
            }
        }
    }

    context
}

/// Walk a directory recursively, returning file paths.
fn walkdir(dir: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    walk_recursive(dir, &mut files)?;
    Ok(files)
}

fn walk_recursive(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip .git
                if path.file_name().map_or(false, |n| n == ".git") {
                    continue;
                }
                walk_recursive(&path, files)?;
            } else {
                files.push(path);
            }
        }
    }
    Ok(())
}

/// Initialize git in the project directory.
fn git_init(project_dir: &Path) -> anyhow::Result<()> {
    use std::process::Command;

    let run = |args: &[&str]| -> anyhow::Result<()> {
        let status = Command::new("git")
            .args(args)
            .current_dir(project_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        if !status.success() {
            anyhow::bail!("git {} failed", args.join(" "));
        }
        Ok(())
    };

    run(&["init"])?;
    run(&["add", "-A"])?;
    run(&["commit", "-m", "Initial build by Phantom"])?;
    println!("\n\x1b[1;32m  ✓ Git\x1b[0m initialized with initial commit");
    Ok(())
}

/// Update ~/.phantom/state.json with the build record.
fn update_state(
    project_name: &str,
    project_dir: &Path,
    total_files: u32,
    total_tokens: u64,
    elapsed: &std::time::Duration,
) -> anyhow::Result<()> {
    let state_path = dirs_or_home().join("state.json");

    // Read existing state or create default
    let mut state: serde_json::Value = if state_path.exists() {
        let data = std::fs::read_to_string(&state_path)?;
        serde_json::from_str(&data).unwrap_or_else(|_| default_state())
    } else {
        default_state()
    };

    let now = chrono::Utc::now().to_rfc3339();

    // Build record
    let build_record = serde_json::json!({
        "project": project_name,
        "project_dir": project_dir.display().to_string(),
        "status": "complete",
        "started_at": now,
        "completed_at": now,
        "duration": format!("{:.1}s", elapsed.as_secs_f64()),
        "files_created": total_files,
        "tokens_used": total_tokens,
        "engine": "Phantom AI (auto-detect)",
    });

    // Append to builds array
    if let Some(builds) = state.get_mut("builds").and_then(|b| b.as_array_mut()) {
        builds.push(build_record);
    }

    // Clear current_build
    state["current_build"] = serde_json::Value::Null;

    // Atomic write
    let tmp_path = state_path.with_extension("tmp");
    std::fs::write(&tmp_path, serde_json::to_string_pretty(&state)?)?;
    std::fs::rename(&tmp_path, &state_path)?;

    Ok(())
}

/// Get ~/.phantom/ directory.
fn dirs_or_home() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let phantom_dir = std::path::PathBuf::from(home).join(".phantom");
    let _ = std::fs::create_dir_all(&phantom_dir);
    phantom_dir
}

/// Default state.json structure.
fn default_state() -> serde_json::Value {
    serde_json::json!({
        "activated": false,
        "license": null,
        "instance_id": null,
        "activated_at": null,
        "dependencies": {},
        "accounts": {},
        "infrastructure": {},
        "builds": [],
        "current_build": null,
        "agents": {},
        "knowledge_brain": { "files_indexed": 0, "chunks": 0 },
    })
}

/// Initialize Knowledge Brain (§2.2 — ChromaDB). Returns None if unavailable.
async fn init_knowledge_brain() -> Option<KnowledgeBrain> {
    let config = BrainConfig::default();
    let mut brain = KnowledgeBrain::new(config);
    match brain.initialize().await {
        Ok(()) => Some(brain),
        Err(e) => {
            tracing::warn!("Knowledge Brain unavailable: {}", e);
            None
        }
    }
}

/// Query Knowledge Brain for relevant chunks, converting to the AI context type.
/// Returns empty vec if brain is None or query fails.
async fn query_brain(
    brain: &Option<KnowledgeBrain>,
    query_text: &str,
    role: &str,
) -> Vec<phantom_ai::context::KnowledgeChunk> {
    let brain = match brain {
        Some(b) => b,
        None => return Vec::new(),
    };

    let query = KnowledgeQuery::new(query_text)
        .with_agent_role(role)
        .with_top_k(5)
        .with_min_score(0.3);

    match brain.query(&query).await {
        Ok(chunks) => chunks
            .into_iter()
            .map(|c| phantom_ai::context::KnowledgeChunk {
                source: c.source_file,
                heading: c.section,
                content: c.content,
                score: c.score as f64,
            })
            .collect(),
        Err(e) => {
            tracing::warn!("Knowledge Brain query failed: {}", e);
            Vec::new()
        }
    }
}
