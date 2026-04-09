//! Business logic skills for the Phantom autonomous AI engineering system.
//!
//! Covers subscription billing, e-commerce, CRM integration, analytics,
//! onboarding, feedback, CMS, marketplaces, scheduling, and reporting.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all business-logic skills into the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(subscription_billing());
    registry.register(ecommerce_engine());
    registry.register(crm_integration());
    registry.register(analytics_dashboard());
    registry.register(onboarding_flow());
    registry.register(feedback_system());
    registry.register(content_management_system());
    registry.register(marketplace_engine());
    registry.register(scheduling_system());
    registry.register(reporting_engine());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn subscription_billing() -> Skill {
    Skill::new(
        "subscription_billing",
        "Subscription & Billing Engine",
        "Generates a complete SaaS billing system with plan management, free trials, \
         usage-based pricing, metered billing, proration on plan changes, automated \
         invoicing, dunning for failed payments, and revenue recognition.",
        SkillCategory::Business,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(35_000)
    .with_system_prompt(
        "Build a billing engine where plans are versioned configurations with flat, \
         tiered, and usage-based pricing components. Trials have configurable duration \
         and auto-convert with explicit opt-in. Usage meters aggregate events in \
         real-time via an append-only event log with periodic snapshotting. Proration \
         calculates credit/debit at second granularity when customers change plans \
         mid-cycle. Invoices are generated as immutable documents with line items, \
         taxes (integrate tax calculation API), and PDF rendering. Dunning retries \
         failed charges on an exponential schedule (1, 3, 7, 14 days) with email \
         notifications at each step, and suspends the account after final failure. \
         All monetary operations must use integer cents to avoid floating-point \
         errors. Include webhook handlers for payment processor events and an \
         idempotency layer to handle duplicate deliveries.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.90)
}

fn ecommerce_engine() -> Skill {
    Skill::new(
        "ecommerce_engine",
        "E-Commerce Engine",
        "Creates a full e-commerce backend with product catalog, variant management, \
         shopping cart, checkout flow, inventory tracking, order lifecycle management, \
         and fulfillment integration.",
        SkillCategory::Business,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(35_000)
    .with_system_prompt(
        "Design an e-commerce engine with a product catalog supporting variants (size, \
         color), option combinations with per-variant pricing and SKUs, and rich media \
         attachments. The cart must be persistent (database-backed for authenticated \
         users, cookie-based for guests) with real-time inventory reservation that \
         expires after configurable TTL. Checkout orchestrates address validation, \
         shipping rate calculation, tax computation, payment authorization, and order \
         creation in a saga with compensating actions on partial failure. Inventory \
         uses optimistic locking with a separate reserved vs available count. Order \
         lifecycle tracks states (pending, paid, processing, shipped, delivered, \
         returned) via a state machine with webhook notifications at each transition. \
         Fulfillment integration pushes orders to warehouse APIs and polls for \
         tracking updates.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn crm_integration() -> Skill {
    Skill::new(
        "crm_integration",
        "CRM Integration Layer",
        "Implements CRM integration with bidirectional contact syncing, deal/pipeline \
         management, activity tracking, custom field mapping, and reporting \
         aggregation across CRM providers.",
        SkillCategory::Business,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build a CRM integration layer behind a provider-agnostic interface \
         (Salesforce, HubSpot, Pipedrive). Bidirectional contact sync uses \
         conflict resolution based on last-modified timestamp with field-level \
         merge rules (e.g., prefer CRM for sales fields, prefer app for usage \
         fields). Deal management maps internal opportunity stages to CRM pipeline \
         stages with configurable mappings per provider. Activity tracking \
         automatically logs emails sent, meetings booked, and feature usage as CRM \
         activities. Custom field mapping uses a schema translator that maps internal \
         field names and types to each provider's custom field API. Sync runs \
         incrementally via change-data-capture or polling with high-water marks. \
         Include rate-limit-aware API clients with automatic backoff and quota \
         reservation.",
    )
    .with_quality_threshold(0.80)
}

fn analytics_dashboard() -> Skill {
    Skill::new(
        "analytics_dashboard",
        "Business Analytics Dashboard",
        "Creates a business analytics system with KPI tracking, cohort analysis, \
         funnel visualization, retention curves, revenue metrics (MRR/ARR/churn), \
         and self-serve exploration.",
        SkillCategory::Business,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Design an analytics dashboard backed by pre-computed metric tables. KPIs \
         (MRR, ARR, churn rate, LTV, CAC) are calculated by nightly batch jobs and \
         stored as time-series for trend charts. Cohort analysis groups users by \
         signup week/month and tracks retention, revenue, and engagement over time. \
         Funnel tracking records ordered step completions per user session and \
         computes conversion rates with statistical confidence intervals. Retention \
         curves show day-N retention with rolling averages. The frontend must render \
         charts server-side for email reports and client-side for interactive \
         exploration. Support dimension filtering (plan, country, acquisition \
         channel) on all metrics. Use materialized views for sub-second dashboard \
         loads and include a cache invalidation strategy tied to the batch job \
         completion.",
    )
    .with_quality_threshold(0.80)
}

fn onboarding_flow() -> Skill {
    Skill::new(
        "onboarding_flow",
        "User Onboarding Flow",
        "Generates a user onboarding system with guided product tours, task \
         checklists, progressive feature disclosure, activation metric tracking, \
         and personalized onboarding paths based on user role or intent.",
        SkillCategory::Business,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend, AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build an onboarding system composed of steps, checklists, and tooltips \
         stored as a declarative configuration. Steps have preconditions (previous \
         step completed, feature flag enabled) and actions (highlight UI element, \
         show modal, trigger API call). Checklists track completion state per user \
         with progress persistence. Progressive disclosure gates advanced features \
         behind activation milestones (e.g., first project created, team member \
         invited). Track activation metrics: time-to-first-value, checklist \
         completion rate, drop-off per step. Support personalized paths by branching \
         the onboarding flow based on signup survey answers or detected user role. \
         The frontend SDK must be framework-agnostic (vanilla JS with React/Vue \
         wrappers) and render non-intrusively with dismiss/snooze options.",
    )
    .with_quality_threshold(0.80)
}

fn feedback_system() -> Skill {
    Skill::new(
        "feedback_system",
        "User Feedback Collection System",
        "Implements a user feedback platform with NPS surveys, CSAT scoring, in-app \
         contextual surveys, feature request tracking with voting, and sentiment \
         analysis on free-text responses.",
        SkillCategory::Business,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Design a feedback system with survey types (NPS, CSAT, CES, custom) \
         triggered by events (post-purchase, after support ticket, time-based). \
         Surveys target specific user segments using rule-based filters (plan, \
         tenure, activity). Responses are stored with context metadata (page, \
         feature, session). Feature requests allow free-text submission with \
         automatic deduplication via semantic similarity, voting, and status \
         tracking (under review, planned, shipped). Free-text responses are \
         analyzed for sentiment and topic extraction using an LLM classifier. \
         Dashboard aggregates NPS score over time with segment breakdowns. \
         Rate-limit survey delivery so users see at most one survey per \
         configurable cool-down period to avoid fatigue.",
    )
    .with_quality_threshold(0.80)
}

fn content_management_system() -> Skill {
    Skill::new(
        "content_management_system",
        "Headless CMS",
        "Creates a headless content management system with flexible content modeling, \
         draft/publish workflow, versioning with diff, preview environments, webhook \
         notifications, and multi-locale support.",
        SkillCategory::Business,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Frontend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Build a headless CMS where content types are user-defined schemas with \
         typed fields (text, rich text, media, reference, enum, JSON). Content \
         entries have draft and published versions with full revision history \
         and visual diffs. The publishing workflow supports review/approval stages \
         with role-based permissions. Preview generates a short-lived URL rendering \
         draft content against the frontend template. Deliver content via a REST \
         and GraphQL API with CDN-friendly cache headers and webhook notifications \
         on publish events for cache invalidation. Multi-locale support stores \
         translations as parallel entries linked by a content group ID, with \
         fallback chains (e.g., fr-CA -> fr -> en). Media assets are stored in \
         object storage with on-the-fly image transforms (resize, crop, format).",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 3_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn marketplace_engine() -> Skill {
    Skill::new(
        "marketplace_engine",
        "Multi-Vendor Marketplace Engine",
        "Generates a multi-vendor marketplace with seller onboarding, product \
         listings, order routing, escrow payments, review/rating system, commission \
         calculation, and dispute resolution.",
        SkillCategory::Business,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(35_000)
    .with_system_prompt(
        "Design a marketplace where vendors self-onboard through a verification \
         flow (identity, bank account, tax info). Product listings belong to a \
         vendor and go through moderation before publishing. Orders containing items \
         from multiple vendors are split into sub-orders routed to each vendor. \
         Payments are held in escrow: the platform collects payment at checkout, \
         holds funds during fulfillment, and releases to vendor after delivery \
         confirmation minus the platform commission. Commission rates are \
         configurable per category and vendor tier. Reviews are buyer-verified \
         (only purchasers can review) with text, rating, and photo. Include a \
         dispute resolution workflow with escalation from vendor to platform \
         support. All financial operations must be audit-logged with double-entry \
         bookkeeping.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn scheduling_system() -> Skill {
    Skill::new(
        "scheduling_system",
        "Calendar & Booking System",
        "Implements a scheduling system with availability management, timezone-aware \
         booking, conflict detection, buffer times, reminder notifications, and \
         calendar provider sync (Google, Outlook).",
        SkillCategory::Business,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Build a scheduling system where providers define availability as recurring \
         weekly rules with override exceptions (holidays, time off). All times are \
         stored in UTC and converted to the viewer's timezone for display. Booking \
         requests atomically check availability, reserve the slot (optimistic lock), \
         and create the event. Conflict detection considers buffer time before/after \
         each booking and cross-references linked calendars. Reminders are sent via \
         email and push notification at configurable intervals (24h, 1h, 15min). \
         Bidirectional sync with Google Calendar and Outlook uses webhook \
         subscriptions for real-time updates and periodic full-sync as fallback. \
         Cancellation and rescheduling enforce per-provider policies (minimum \
         notice, refund eligibility). Include a booking page generator that \
         renders available slots in the booker's timezone.",
    )
    .with_quality_threshold(0.85)
}

fn reporting_engine() -> Skill {
    Skill::new(
        "reporting_engine",
        "Report Generation Engine",
        "Creates a report generation system with parameterized templates, scheduled \
         delivery, PDF and Excel export, interactive drill-down, sharing with \
         access control, and embedded chart rendering.",
        SkillCategory::Business,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Frontend],
        OutputFormat::Report,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Design a reporting engine where reports are defined as parameterized \
         templates with SQL queries, chart configurations, and layout rules. \
         Parameters (date range, filters, groupings) are user-configurable with \
         type validation and default values. The rendering pipeline executes \
         queries, transforms results into chart data, and produces output in the \
         requested format. PDF export uses a headless browser to render the HTML \
         report with charts to a pixel-perfect document. Excel export maps data to \
         worksheets with formatted headers, data types, and embedded charts. \
         Scheduled reports run on cron and deliver via email with the rendered \
         attachment. Interactive mode supports drill-down: clicking a chart segment \
         re-queries with additional filters. Sharing generates a time-limited signed \
         URL with optional password protection.",
    )
    .with_quality_threshold(0.80)
}
