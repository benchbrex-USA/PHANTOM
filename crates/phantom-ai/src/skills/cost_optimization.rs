//! Cost optimization skills for the Phantom autonomous AI engineering system.
//!
//! Covers cloud cost analysis, right-sizing, spot instances, reserved capacity,
//! serverless optimization, storage tiering, network costs, cost allocation,
//! budget alerts, and carbon footprint estimation.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all cost-optimization skills into the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(cloud_cost_analysis());
    registry.register(right_sizing());
    registry.register(spot_instance_strategy());
    registry.register(reserved_capacity_planning());
    registry.register(serverless_optimization());
    registry.register(storage_tiering());
    registry.register(network_cost_reduction());
    registry.register(cost_allocation());
    registry.register(budget_alert_system());
    registry.register(carbon_footprint());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn cloud_cost_analysis() -> Skill {
    Skill::new(
        "cloud_cost_analysis",
        "Cloud Cost Analysis",
        "Generates cloud spend analysis with breakdown by service, team, and \
         environment, trend detection, anomaly alerting, waste identification, \
         and savings opportunity ranking.",
        SkillCategory::CostOptimization,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Report,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build a cloud cost analysis system that ingests billing data from cloud \
         provider APIs (AWS Cost Explorer, GCP Billing, Azure Cost Management) and \
         normalizes it into a unified schema. Break down costs by service, resource, \
         team (via tags), environment (prod/staging/dev), and time period. Detect \
         trends using linear regression on daily spend and flag spend that deviates \
         more than two standard deviations from the 30-day rolling average as \
         anomalies. Identify waste: unused resources (zero-traffic load balancers, \
         unattached volumes, idle instances below 5% CPU for 7+ days), oversized \
         resources, and orphaned snapshots. Rank savings opportunities by estimated \
         monthly savings and implementation effort. Generate executive summaries \
         with month-over-month and year-over-year comparisons.",
    )
    .with_quality_threshold(0.80)
}

fn right_sizing() -> Skill {
    Skill::new(
        "right_sizing",
        "Resource Right-Sizing",
        "Implements resource right-sizing with utilization analysis across CPU, memory, \
         disk, and network, recommendation generation, savings projections, and safe \
         migration plans.",
        SkillCategory::CostOptimization,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Backend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Design a right-sizing system that collects resource utilization metrics \
         (CPU, memory, disk I/O, network) over a representative period (minimum 14 \
         days, ideally covering peak events). Analyze utilization distributions: \
         P50, P95, P99, and max. Recommend downsizing when P95 utilization is below \
         40% of allocated capacity, upsizing when P99 exceeds 80%, and instance \
         family changes when workload profiles better match a different type (e.g., \
         compute-optimized to memory-optimized). Project savings as monthly dollar \
         amounts based on current vs recommended pricing. Generate a migration plan \
         with ordered steps: create new instance, verify application health, shift \
         traffic, terminate old instance. Include a dry-run mode that simulates the \
         recommendation against historical metrics to validate it would not have \
         caused capacity issues.",
    )
    .with_quality_threshold(0.80)
}

fn spot_instance_strategy() -> Skill {
    Skill::new(
        "spot_instance_strategy",
        "Spot/Preemptible Instance Strategy",
        "Creates a spot instance management strategy with interruption handling, \
         automatic fallback to on-demand, diversified instance pools, and savings \
         tracking versus on-demand baseline.",
        SkillCategory::CostOptimization,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Implement a spot instance strategy that maximizes savings while maintaining \
         availability. Diversify across multiple instance types and availability zones \
         to reduce simultaneous interruption risk. Handle interruption warnings (2-min \
         on AWS, 30-sec on GCP) by draining in-flight work to persistent storage, \
         deregistering from load balancers, and triggering replacement provisioning. \
         Automatically fall back to on-demand instances when spot capacity is \
         unavailable, with automatic return to spot when capacity recovers. Configure \
         maximum spot price as a percentage of on-demand (typically 60-70%). Track \
         savings: compare actual spot spend against what on-demand would have cost. \
         Identify workloads suitable for spot (stateless, fault-tolerant, checkpointable) \
         versus workloads that should remain on-demand (stateful, latency-sensitive). \
         Include Kubernetes integration with node pools and pod disruption budgets.",
    )
    .with_quality_threshold(0.80)
}

fn reserved_capacity_planning() -> Skill {
    Skill::new(
        "reserved_capacity_planning",
        "Reserved Capacity Planning",
        "Generates reserved instance and savings plan recommendations with commitment \
         analysis, break-even calculations, coverage optimization, and renewal tracking.",
        SkillCategory::CostOptimization,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build a reserved capacity planner that analyzes historical usage patterns \
         to recommend optimal commitment purchases. Compute the steady-state baseline \
         by finding the minimum utilization over the analysis period (90+ days). \
         Recommend reservations to cover the baseline and savings plans for variable \
         workloads above it. Calculate break-even points: the number of months of \
         usage required to offset the upfront or committed spend compared to on-demand. \
         Optimize coverage by mixing 1-year and 3-year terms to balance savings depth \
         against flexibility. Model different payment options (all upfront, partial, \
         no upfront) with their effective hourly rates. Track existing reservation \
         utilization and flag underutilized reservations that should be exchanged \
         or sold on the marketplace. Generate renewal alerts 60 days before \
         expiration with updated recommendations based on current usage.",
    )
    .with_quality_threshold(0.80)
}

fn serverless_optimization() -> Skill {
    Skill::new(
        "serverless_optimization",
        "Serverless Cost Optimization",
        "Implements serverless function cost optimization including memory tuning, \
         cold start mitigation, provisioned concurrency analysis, batch invocation \
         patterns, and duration reduction.",
        SkillCategory::CostOptimization,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Backend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Design serverless cost optimization that tunes each function independently. \
         Memory tuning: profile functions at multiple memory settings (128MB to 3GB) \
         and find the cost-optimal point where (memory * duration) is minimized, \
         since higher memory also increases CPU allocation. Cold start mitigation: \
         analyze invocation patterns to identify functions that benefit from \
         provisioned concurrency (sustained traffic) vs those where cold starts are \
         acceptable (infrequent, latency-insensitive). For bursty workloads, use \
         warm-pool schedulers that send periodic keep-alive invocations. Batch \
         invocation patterns: group SQS/event messages into larger batches to reduce \
         invocation count while staying within timeout limits. Duration reduction: \
         identify functions spending time on I/O waits that could benefit from \
         connection pooling or caching. Track cost per invocation and per business \
         transaction for attribution.",
    )
    .with_quality_threshold(0.80)
}

fn storage_tiering() -> Skill {
    Skill::new(
        "storage_tiering",
        "Storage Tier Optimization",
        "Creates storage lifecycle policies with intelligent tiering between hot, \
         warm, cold, and archive tiers based on access patterns, with cost \
         projections and retrieval SLA guarantees.",
        SkillCategory::CostOptimization,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Backend],
        OutputFormat::Config,
    )
    .with_estimated_tokens(15_000)
    .with_system_prompt(
        "Build a storage tiering system that automatically moves data between tiers \
         based on access patterns. Analyze object access logs to classify data into \
         hot (accessed daily), warm (weekly), cold (monthly), and archive (rarely) \
         tiers. Generate lifecycle policies that transition objects between tiers \
         after configurable inactivity periods. Project cost savings comparing \
         current all-hot storage against the recommended tiered configuration. \
         Enforce retrieval SLA guarantees: hot tier returns in milliseconds, warm \
         in seconds, cold in minutes, archive in hours with expedited retrieval \
         option. Handle minimum storage duration charges to avoid premature tier \
         transitions that cost more than they save. Support intelligent tiering \
         services (S3 Intelligent-Tiering, GCS Autoclass) where available and \
         manual lifecycle rules where not. Include cost modeling for data retrieval \
         to prevent surprises when cold data is accessed.",
    )
    .with_quality_threshold(0.80)
}

fn network_cost_reduction() -> Skill {
    Skill::new(
        "network_cost_reduction",
        "Network Cost Reduction",
        "Implements data transfer cost optimization with CDN offloading, compression \
         strategies, regional routing optimization, VPC endpoints, and cross-AZ \
         traffic minimization.",
        SkillCategory::CostOptimization,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(15_000)
    .with_system_prompt(
        "Design network cost reduction by analyzing data transfer patterns. CDN \
         offloading: identify static and semi-static content served from origin that \
         could be cached at edge, estimating transfer savings and CDN costs. \
         Compression: ensure gzip/brotli compression on all text-based responses \
         and evaluate protobuf/messagepack for API payloads. Regional routing: \
         minimize cross-region transfers by co-locating communicating services in \
         the same region and using regional API endpoints for cloud services. VPC \
         endpoints: replace NAT Gateway-routed traffic to AWS services with VPC \
         endpoints that have no per-GB charge. Cross-AZ minimization: configure \
         service mesh locality-aware routing to prefer same-AZ communication. \
         Quantify each optimization with estimated monthly savings based on current \
         transfer volumes. Prioritize recommendations by savings-to-effort ratio.",
    )
    .with_quality_threshold(0.80)
}

fn cost_allocation() -> Skill {
    Skill::new(
        "cost_allocation",
        "Cost Allocation & Attribution",
        "Creates a cost allocation system with resource tagging enforcement, \
         showback/chargeback models, per-customer cost attribution, shared resource \
         apportionment, and team budget dashboards.",
        SkillCategory::CostOptimization,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Report,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build a cost allocation system that attributes every dollar of cloud spend \
         to a business owner. Enforce mandatory tagging (team, service, environment, \
         cost-center) at provisioning time via policy-as-code that blocks untagged \
         resource creation. For shared resources (databases, Kubernetes clusters, \
         networking), apportion costs using usage-weighted splits: CPU-seconds for \
         compute, storage bytes for databases, request counts for shared APIs. \
         Per-customer attribution tracks resource consumption per tenant in multi-\
         tenant systems using application-level metering. Support showback (visibility \
         without billing) and chargeback (internal invoicing) models with configurable \
         allocation rules. Generate team-level budget dashboards showing allocated \
         vs actual spend, burn rate, and projected month-end. Alert team leads when \
         their projected spend exceeds the allocated budget by configurable thresholds.",
    )
    .with_quality_threshold(0.80)
}

fn budget_alert_system() -> Skill {
    Skill::new(
        "budget_alert_system",
        "Budget Alert & Forecasting System",
        "Implements budget alerting with spend forecasting, anomaly detection, \
         threshold-based notifications, and automated remediation actions for \
         runaway costs.",
        SkillCategory::CostOptimization,
        SkillComplexity::Composite,
        vec![AgentRole::DevOps, AgentRole::Monitor],
        OutputFormat::Code,
    )
    .with_estimated_tokens(15_000)
    .with_system_prompt(
        "Design a budget alert system that forecasts end-of-month spend using linear \
         extrapolation from month-to-date actuals, adjusted for known patterns \
         (weekday/weekend, beginning/end of month). Set tiered alert thresholds at \
         50%, 75%, 90%, and 100% of budget with configurable notification channels \
         (email, Slack, PagerDuty). Anomaly detection flags daily spend that exceeds \
         the trailing 7-day average by more than a configurable multiple (default 2x). \
         Automated remediation actions trigger at critical thresholds: scale down \
         non-production environments, terminate idle dev instances, reduce provisioned \
         capacity to minimum. All automated actions require a confirmation step \
         (Slack approval) unless the budget overage exceeds an emergency threshold. \
         Track budget vs actual over time with variance analysis for finance reviews.",
    )
    .with_quality_threshold(0.80)
}

fn carbon_footprint() -> Skill {
    Skill::new(
        "carbon_footprint",
        "Infrastructure Carbon Footprint",
        "Estimates infrastructure carbon impact using provider emissions data, \
         identifies green region alternatives, and recommends workload placement \
         optimizations to reduce carbon intensity.",
        SkillCategory::CostOptimization,
        SkillComplexity::Atomic,
        vec![AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Report,
    )
    .with_estimated_tokens(15_000)
    .with_system_prompt(
        "Build a carbon footprint estimator that calculates the CO2 equivalent \
         emissions of cloud infrastructure. Use provider-published carbon intensity \
         data per region (gCO2eq/kWh) combined with resource utilization to estimate \
         emissions. Break down by service type: compute (CPU-hours * regional carbon \
         intensity * PUE), storage (TB-months * energy per TB * regional intensity), \
         networking (GB transferred * energy per GB). Identify green region \
         alternatives where workloads could run with lower carbon intensity without \
         violating latency or data residency requirements. Recommend time-shifting \
         flexible workloads (batch jobs, CI/CD, backups) to hours when grid carbon \
         intensity is lowest. Track carbon footprint over time with trend reporting \
         and provide scope 1/2/3 categorization for ESG reporting. Compare estimated \
         emissions against industry benchmarks per unit of compute.",
    )
    .with_quality_threshold(0.75)
}
