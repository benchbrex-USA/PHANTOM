//! Compliance skills for the Phantom autonomous AI engineering system.
//!
//! Covers GDPR, SOC2, HIPAA, PCI-DSS, accessibility, license compliance,
//! data residency, privacy by design, audit readiness, and regulatory reporting.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all compliance skills into the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(gdpr_compliance());
    registry.register(soc2_controls());
    registry.register(hipaa_compliance());
    registry.register(pci_dss_compliance());
    registry.register(accessibility_compliance());
    registry.register(license_compliance());
    registry.register(data_residency());
    registry.register(privacy_by_design());
    registry.register(audit_readiness());
    registry.register(regulatory_reporting());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn gdpr_compliance() -> Skill {
    Skill::new(
        "gdpr_compliance",
        "GDPR Compliance Implementation",
        "Implements GDPR requirements including consent management, data subject \
         rights (access, erasure, portability), data processing agreements, breach \
         notification workflows, and data protection impact assessments.",
        SkillCategory::Compliance,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Security, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Implement GDPR compliance as code. Consent management must record granular \
         per-purpose consent with timestamp, version of privacy policy accepted, and \
         withdrawal capability. Data subject access requests (DSAR) trigger an \
         automated pipeline that collects all personal data across services, compiles \
         it into a portable format (JSON + CSV), and delivers within the 30-day \
         deadline with progress tracking. Right to erasure performs cascading deletion \
         across all data stores with a verification step and audit log of what was \
         deleted. Data portability exports in machine-readable format. Breach \
         notification workflow detects potential breaches, assesses severity, and \
         generates the supervisory authority notification within 72 hours with all \
         required fields. DPIA templates assess processing activities for risk and \
         document mitigations. All GDPR operations produce immutable audit records.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.90)
}

fn soc2_controls() -> Skill {
    Skill::new(
        "soc2_controls",
        "SOC2 Type II Controls",
        "Generates SOC2 Type II control implementations covering access control, \
         encryption at rest and in transit, continuous monitoring, incident response \
         procedures, and vendor management policies.",
        SkillCategory::Compliance,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Security, AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Implement SOC2 trust service criteria as enforceable code controls. Access \
         control: RBAC with least-privilege defaults, MFA enforcement, access reviews \
         on a quarterly schedule with automated stale-access detection. Encryption: \
         AES-256 at rest with key rotation, TLS 1.3 in transit with certificate \
         management. Monitoring: centralized logging with tamper-proof storage, \
         anomaly detection on login patterns and data access volumes, and 90-day \
         retention. Incident response: runbook-driven workflow from detection through \
         containment, eradication, recovery, and post-mortem with SLA timers at each \
         stage. Vendor management: third-party risk assessments with questionnaires, \
         SLA tracking, and access scope documentation. Generate evidence artifacts \
         (screenshots, query results, config exports) that map directly to SOC2 \
         control IDs for auditor consumption.",
    )
    .with_quality_threshold(0.90)
}

fn hipaa_compliance() -> Skill {
    Skill::new(
        "hipaa_compliance",
        "HIPAA Compliance Safeguards",
        "Creates HIPAA technical safeguards for PHI handling with access controls, \
         audit trails, encryption, Business Associate Agreements, and minimum \
         necessary data access enforcement.",
        SkillCategory::Compliance,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Security, AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Implement HIPAA technical safeguards for systems handling Protected Health \
         Information (PHI). Access controls enforce unique user identification, \
         emergency access procedures, automatic logoff, and role-based authorization \
         scoped to the minimum necessary PHI for each role. Audit trails log every \
         PHI access (read, create, update, delete) with user identity, timestamp, \
         accessed records, and action, stored in tamper-evident append-only logs \
         retained for six years. Encryption uses AES-256 for PHI at rest and TLS 1.3 \
         in transit with no exceptions. Generate BAA templates with required clauses \
         and track execution status per business associate. Implement integrity \
         controls with checksums on PHI records and alerts on unexpected modifications. \
         The transmission security layer ensures PHI is never sent over unencrypted \
         channels and logs all transmission events.",
    )
    .with_quality_threshold(0.90)
}

fn pci_dss_compliance() -> Skill {
    Skill::new(
        "pci_dss_compliance",
        "PCI-DSS Compliance",
        "Implements PCI-DSS requirements including cardholder data protection, network \
         segmentation, vulnerability management, access restriction, and logging \
         with regular penetration testing support.",
        SkillCategory::Compliance,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Security, AgentRole::Backend, AgentRole::DevOps],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Implement PCI-DSS controls to protect cardholder data. Never store full \
         track data, CVV, or PIN; if card numbers must be stored, use tokenization \
         via the payment processor or encrypt with dedicated keys in an HSM. Network \
         segmentation isolates the cardholder data environment (CDE) from the rest \
         of the infrastructure using firewall rules and VPC boundaries. Restrict \
         access to CDE to only personnel whose job requires it, with named accounts \
         and MFA. Vulnerability management includes automated dependency scanning \
         (CVE checks), container image scanning, and quarterly ASV scans. Log all \
         access to cardholder data and network resources in the CDE with centralized, \
         tamper-proof storage. Generate self-assessment questionnaire (SAQ) evidence \
         artifacts. Include a penetration test scope definition and finding tracker.",
    )
    .with_quality_threshold(0.90)
}

fn accessibility_compliance() -> Skill {
    Skill::new(
        "accessibility_compliance",
        "WCAG Accessibility Compliance",
        "Generates accessibility compliance tooling for WCAG 2.1/2.2 at AA/AAA \
         levels with automated scanning, manual audit checklists, remediation \
         guidance, and continuous regression testing.",
        SkillCategory::Compliance,
        SkillComplexity::Composite,
        vec![AgentRole::Frontend, AgentRole::Qa],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build accessibility compliance tooling that combines automated scanning \
         with manual audit workflows. Automated scanning runs axe-core or similar \
         engines against rendered pages, categorizing violations by WCAG criterion, \
         severity, and affected element with remediation snippets. Manual audit \
         checklists cover criteria that cannot be automated: meaningful alt text, \
         logical reading order, keyboard navigation flow, screen reader \
         announcements. Generate a per-page compliance report mapping each WCAG \
         success criterion to its status (pass, fail, not applicable, needs manual \
         review). Remediation guidance provides code fixes for common violations \
         (missing labels, low contrast, missing landmarks). Integrate into CI as a \
         regression gate that fails the build if new violations are introduced. \
         Track compliance score over time with trend dashboards.",
    )
    .with_quality_threshold(0.85)
}

fn license_compliance() -> Skill {
    Skill::new(
        "license_compliance",
        "Open Source License Compliance",
        "Creates an OSS license compliance system with dependency license detection, \
         compatibility analysis, SBOM generation in SPDX/CycloneDX format, and \
         obligation tracking per license type.",
        SkillCategory::Compliance,
        SkillComplexity::Composite,
        vec![AgentRole::Security, AgentRole::Backend],
        OutputFormat::Analysis,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build a license compliance system that scans all project dependencies \
         (direct and transitive) and identifies their licenses via package metadata, \
         LICENSE files, and SPDX identifiers. Analyze compatibility: flag copyleft \
         licenses (GPL, AGPL) in proprietary codebases, detect dual-licensed \
         dependencies where commercial licenses are available, and warn on unknown \
         or custom licenses requiring legal review. Generate Software Bill of \
         Materials (SBOM) in both SPDX and CycloneDX formats with complete \
         dependency trees. Track obligations per license: attribution notices, \
         source code disclosure, patent grants. Integrate into CI to block merges \
         that introduce incompatible licenses. Maintain a curated allow/deny list \
         of licenses configurable per organization policy.",
    )
    .with_quality_threshold(0.85)
}

fn data_residency() -> Skill {
    Skill::new(
        "data_residency",
        "Data Residency Enforcement",
        "Implements data residency controls with region-based routing, storage \
         locality enforcement, cross-border transfer restrictions, and compliance \
         proof generation.",
        SkillCategory::Compliance,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::DevOps, AgentRole::Security],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Design data residency enforcement that ensures data stays within \
         designated regions. Route user requests to the nearest compliant region \
         based on the user's data residency configuration (not just geographic \
         proximity). Storage locality enforces that databases, object stores, and \
         caches for region-tagged data exist only in allowed regions, with \
         provisioning-time validation. Block cross-border data transfers unless \
         an approved mechanism (SCCs, adequacy decision, explicit consent) is in \
         place, enforced at the API gateway layer. Implement data residency labels \
         as metadata on every record, propagated through the processing pipeline \
         so downstream services inherit the constraint. Generate compliance proofs: \
         infrastructure audit reports showing resource locations, data flow diagrams \
         with region annotations, and periodic attestation of storage locality.",
    )
    .with_quality_threshold(0.90)
}

fn privacy_by_design() -> Skill {
    Skill::new(
        "privacy_by_design",
        "Privacy-by-Design Implementation",
        "Generates privacy-by-design patterns including data minimization, purpose \
         limitation, storage limitation with TTL, consent-gated processing, and \
         privacy-preserving analytics.",
        SkillCategory::Compliance,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend, AgentRole::Security],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Implement privacy-by-design principles as reusable patterns. Data \
         minimization: collect only fields explicitly required for each feature, \
         validated at the API schema level so over-collection is structurally \
         impossible. Purpose limitation: tag every data field with its processing \
         purpose(s) and enforce at query time that access is scoped to the stated \
         purpose. Storage limitation: attach TTL policies to personal data with \
         automatic deletion jobs that run on schedule, producing deletion \
         certificates. Consent-gated processing: wrap data access in consent \
         checks that verify the user consented to the specific purpose before \
         returning data. Privacy-preserving analytics: aggregate and anonymize \
         before analysis using k-anonymity thresholds, differential privacy noise, \
         or federated computation. Include a privacy impact scoring function that \
         rates new features by data sensitivity and processing scope.",
    )
    .with_quality_threshold(0.85)
}

fn audit_readiness() -> Skill {
    Skill::new(
        "audit_readiness",
        "Audit Readiness Platform",
        "Creates an audit preparation system with automated evidence collection, \
         control-to-evidence mapping, gap analysis, remediation tracking, and \
         auditor-ready report generation.",
        SkillCategory::Compliance,
        SkillComplexity::Composite,
        vec![AgentRole::Security, AgentRole::DevOps],
        OutputFormat::Report,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Build an audit readiness platform that continuously collects evidence for \
         compliance controls. Evidence collectors pull data from infrastructure \
         (IAM policies, encryption settings, network configs), application logs \
         (access patterns, change history), and process tools (Jira tickets, PR \
         reviews, incident reports). Map each piece of evidence to the specific \
         control IDs it satisfies across frameworks (SOC2, ISO 27001, HIPAA). Gap \
         analysis compares collected evidence against required controls and flags \
         missing or stale evidence. Remediation tracking assigns gaps as tasks with \
         owners, deadlines, and status. Generate auditor-ready reports with a \
         control matrix showing each control, its evidence, last validation date, \
         and status. Evidence must be timestamped and integrity-protected with \
         checksums to prove it was not modified after collection.",
    )
    .with_quality_threshold(0.85)
}

fn regulatory_reporting() -> Skill {
    Skill::new(
        "regulatory_reporting",
        "Automated Regulatory Reporting",
        "Implements automated regulatory reporting with data extraction from \
         operational systems, format transformation to regulatory schemas, \
         validation, and submission tracking.",
        SkillCategory::Compliance,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::Security],
        OutputFormat::Report,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Design an automated regulatory reporting pipeline that extracts data from \
         operational databases and APIs, transforms it into the required regulatory \
         format (XML, XBRL, CSV with fixed schemas), validates against the \
         regulator's published schema and business rules, and tracks submission \
         status. Data extraction uses read replicas to avoid production impact and \
         snapshots at the reporting period boundary for consistency. Transformation \
         maps internal field names and formats to regulatory field definitions with \
         explicit rules per report type. Validation runs the regulator's published \
         validation rules locally before submission to catch errors early. \
         Submission tracking records each filing with timestamp, status \
         (draft, submitted, accepted, rejected), and any regulator feedback. \
         Include a reconciliation step that compares reported figures against \
         internal records to detect discrepancies. Retain all reports and \
         supporting data for the legally required retention period.",
    )
    .with_quality_threshold(0.90)
}
