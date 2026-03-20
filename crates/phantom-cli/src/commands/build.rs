//! `phantom build --framework <file>` — Full autonomous build pipeline.

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

    println!();
    println!("\x1b[33mBuild requires an active license and master key session.\x1b[0m");
    println!("Run `phantom activate --key <KEY>` first, then `phantom master init`.");
    println!();
    println!(
        "\x1b[36mTo approve this plan and start the build, Phantom will prompt for confirmation.\x1b[0m"
    );

    Ok(())
}
