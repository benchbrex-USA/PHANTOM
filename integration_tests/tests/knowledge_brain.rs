//! Knowledge Brain integration tests — markdown chunking, heading context,
//! code block preservation, chunk sizing.

/// Generate a 200+ line sample markdown document with 3 major sections,
/// sub-sections, code blocks, and varied content.
fn sample_knowledge_md() -> String {
    let mut md = String::new();

    // Section 1: Architecture Patterns (~80 lines)
    md.push_str("# Architecture Patterns for Cloud-Native Applications\n\n");
    md.push_str("This document covers the core architecture patterns used in\n");
    md.push_str("modern cloud-native application development.\n\n");

    md.push_str("## 1.1 Microservices Architecture\n\n");
    md.push_str("Microservices decompose a monolithic application into small,\n");
    md.push_str("independently deployable services. Each service owns its data\n");
    md.push_str("and communicates via well-defined APIs.\n\n");
    md.push_str("Key principles:\n");
    md.push_str("- Single responsibility per service\n");
    md.push_str("- Independent deployment and scaling\n");
    md.push_str("- Decentralized data management\n");
    md.push_str("- Resilience through circuit breakers\n");
    md.push_str("- Observability via distributed tracing\n\n");
    for i in 0..10 {
        md.push_str(&format!(
            "Service {} handles its own domain logic and persists state independently.\n",
            i
        ));
    }
    md.push('\n');

    md.push_str("## 1.2 Event-Driven Architecture\n\n");
    md.push_str("Event-driven systems use asynchronous messaging to decouple producers\n");
    md.push_str("from consumers. This enables high throughput and loose coupling.\n\n");
    md.push_str("```rust\n");
    md.push_str("pub struct Event {\n");
    md.push_str("    pub id: String,\n");
    md.push_str("    pub event_type: String,\n");
    md.push_str("    pub payload: serde_json::Value,\n");
    md.push_str("    pub timestamp: chrono::DateTime<chrono::Utc>,\n");
    md.push_str("}\n\n");
    md.push_str("impl Event {\n");
    md.push_str("    pub fn new(event_type: &str, payload: serde_json::Value) -> Self {\n");
    md.push_str("        Self {\n");
    md.push_str("            id: uuid::Uuid::new_v4().to_string(),\n");
    md.push_str("            event_type: event_type.to_string(),\n");
    md.push_str("            payload,\n");
    md.push_str("            timestamp: chrono::Utc::now(),\n");
    md.push_str("        }\n");
    md.push_str("    }\n");
    md.push_str("}\n");
    md.push_str("```\n\n");
    for i in 0..8 {
        md.push_str(&format!(
            "Pattern {}: Use event sourcing to capture all changes as a sequence of events.\n",
            i
        ));
    }
    md.push('\n');

    // Section 2: Database Design (~80 lines)
    md.push_str("# Database Design Best Practices\n\n");
    md.push_str("Proper database design is critical for application performance,\n");
    md.push_str("maintainability, and data integrity.\n\n");

    md.push_str("## 2.1 Schema Design\n\n");
    md.push_str("Follow these principles when designing your database schema:\n\n");
    md.push_str("- Normalize to 3NF for transactional data\n");
    md.push_str("- Denormalize read-heavy queries with materialized views\n");
    md.push_str("- Use UUID primary keys for distributed systems\n");
    md.push_str("- Always include created_at and updated_at timestamps\n");
    md.push_str("- Add soft-delete (deleted_at) instead of hard deletes\n\n");
    md.push_str("```sql\n");
    md.push_str("CREATE TABLE users (\n");
    md.push_str("    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n");
    md.push_str("    email TEXT NOT NULL UNIQUE,\n");
    md.push_str("    name TEXT NOT NULL,\n");
    md.push_str("    role TEXT NOT NULL DEFAULT 'user',\n");
    md.push_str("    created_at TIMESTAMPTZ DEFAULT now(),\n");
    md.push_str("    updated_at TIMESTAMPTZ DEFAULT now(),\n");
    md.push_str("    deleted_at TIMESTAMPTZ\n");
    md.push_str(");\n\n");
    md.push_str("CREATE INDEX idx_users_email ON users(email);\n");
    md.push_str("CREATE INDEX idx_users_role ON users(role) WHERE deleted_at IS NULL;\n");
    md.push_str("```\n\n");

    md.push_str("## 2.2 Query Optimization\n\n");
    md.push_str("Optimize queries by:\n");
    md.push_str("- Adding composite indexes for frequent query patterns\n");
    md.push_str("- Using EXPLAIN ANALYZE to identify slow queries\n");
    md.push_str("- Partitioning large tables by date range\n");
    md.push_str("- Connection pooling with PgBouncer\n\n");
    for i in 0..15 {
        md.push_str(&format!(
            "Optimization rule {}: Always benchmark before and after index changes.\n",
            i
        ));
    }
    md.push('\n');

    // Section 3: Security Hardening (~60 lines)
    md.push_str("# Security Hardening Guidelines\n\n");
    md.push_str("Security must be built into every layer of the application.\n");
    md.push_str("Follow these guidelines to harden your deployment.\n\n");

    md.push_str("## 3.1 Authentication and Authorization\n\n");
    md.push_str("- Use OAuth 2.0 / OpenID Connect for authentication\n");
    md.push_str("- Implement RBAC (Role-Based Access Control)\n");
    md.push_str("- Enforce MFA for administrative accounts\n");
    md.push_str("- Use short-lived JWTs (15-minute expiry)\n");
    md.push_str("- Rotate refresh tokens on every use\n\n");
    for i in 0..10 {
        md.push_str(&format!(
            "Security control {}: Validate all inputs at the API boundary.\n",
            i
        ));
    }
    md.push('\n');

    md.push_str("## 3.2 Network Security\n\n");
    md.push_str("- TLS 1.3 for all external connections\n");
    md.push_str("- mTLS between internal services\n");
    md.push_str("- Network segmentation with VPC\n");
    md.push_str("- WAF rules for OWASP Top 10\n");
    md.push_str("- Rate limiting on all public endpoints\n\n");
    for i in 0..10 {
        md.push_str(&format!(
            "Network rule {}: Monitor all egress traffic for anomalies.\n",
            i
        ));
    }
    md.push('\n');

    // Section 4: Observability (~50 lines)
    md.push_str("# Observability and Monitoring\n\n");
    md.push_str("Comprehensive observability is critical for production systems.\n\n");
    md.push_str("## 4.1 Logging\n\n");
    md.push_str("- Structured JSON logging for all services\n");
    md.push_str("- Correlation IDs propagated across service boundaries\n");
    md.push_str("- Log levels: ERROR, WARN, INFO, DEBUG, TRACE\n");
    md.push_str("- Sensitive data redacted from all log output\n\n");
    for i in 0..15 {
        md.push_str(&format!(
            "Logging rule {}: Ensure all request handlers emit structured logs with latency metrics.\n",
            i
        ));
    }
    md.push('\n');
    md.push_str("## 4.2 Metrics and Alerting\n\n");
    md.push_str("- RED metrics (Rate, Errors, Duration) for all endpoints\n");
    md.push_str("- Custom business metrics for key flows\n");
    md.push_str("- Alert on error rate > 1% over 5-minute window\n");
    md.push_str("- Alert on p99 latency > 500ms\n\n");
    for i in 0..15 {
        md.push_str(&format!(
            "Metric {}: Track resource utilization and queue depth for capacity planning.\n",
            i
        ));
    }
    md.push('\n');

    md
}

// ═══════════════════════════════════════════════════════════════════════════
//  1. Chunk count verification
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_chunk_count_in_expected_range() {
    use phantom_brain::chunker::MarkdownChunker;

    let content = sample_knowledge_md();
    let line_count = content.lines().count();
    assert!(
        line_count >= 200,
        "sample must be 200+ lines, got {}",
        line_count
    );

    let chunker = MarkdownChunker::new(500);
    let chunks = chunker
        .chunk_file(
            "knowledge_test",
            &content,
            &["cto".to_string(), "backend".to_string()],
        )
        .unwrap();

    assert!(
        chunks.len() >= 4 && chunks.len() <= 15,
        "expected 4-15 chunks, got {}",
        chunks.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. Chunks maintain parent heading context
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_chunks_maintain_heading_context() {
    use phantom_brain::chunker::MarkdownChunker;

    let content = sample_knowledge_md();
    let chunker = MarkdownChunker::new(500);
    let chunks = chunker
        .chunk_file("knowledge_test", &content, &["cto".to_string()])
        .unwrap();

    // Every chunk must have a non-empty section heading
    for chunk in &chunks {
        assert!(
            !chunk.section_heading.is_empty(),
            "chunk starting at line {} has empty heading",
            chunk.line_start
        );
    }

    // Verify we see headings from all 3 major sections
    let headings: Vec<&str> = chunks.iter().map(|c| c.section_heading.as_str()).collect();

    let has_architecture = headings
        .iter()
        .any(|h| h.contains("Architecture") || h.contains("1."));
    let has_database = headings
        .iter()
        .any(|h| h.contains("Database") || h.contains("2."));
    let has_security = headings
        .iter()
        .any(|h| h.contains("Security") || h.contains("3."));

    assert!(
        has_architecture,
        "missing architecture section heading in chunks"
    );
    assert!(has_database, "missing database section heading in chunks");
    assert!(has_security, "missing security section heading in chunks");
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. Code blocks are not split across chunks
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_code_blocks_not_split() {
    use phantom_brain::chunker::MarkdownChunker;

    let content = sample_knowledge_md();
    let chunker = MarkdownChunker::new(500);
    let chunks = chunker
        .chunk_file("knowledge_test", &content, &["backend".to_string()])
        .unwrap();

    // Find chunks that contain code fences
    for chunk in &chunks {
        let open_count = chunk.content.matches("```").count();
        // Code fences must come in pairs (open + close) within the same chunk
        assert!(
            open_count % 2 == 0,
            "chunk '{}' (lines {}-{}) has {} unmatched code fences",
            chunk.section_heading,
            chunk.line_start,
            chunk.line_end,
            open_count
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. Chunk metadata
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_chunk_metadata_populated() {
    use phantom_brain::chunker::MarkdownChunker;

    let content = sample_knowledge_md();
    let chunker = MarkdownChunker::new(500);
    let tags = vec!["cto".to_string(), "backend".to_string()];
    let chunks = chunker
        .chunk_file("test_knowledge", &content, &tags)
        .unwrap();

    for chunk in &chunks {
        // Source file must match
        assert_eq!(chunk.source_file, "test_knowledge");

        // Line range must be valid
        assert!(chunk.line_start >= 1);
        assert!(chunk.line_end >= chunk.line_start);

        // Content must not be empty
        assert!(!chunk.content.trim().is_empty());

        // Agent tags must be propagated
        assert_eq!(chunk.agent_tags, tags);

        // Chunk ID must be stable and non-empty
        let id = chunk.chunk_id();
        assert!(!id.is_empty());
    }
}

#[test]
fn test_chunk_ids_are_stable() {
    use phantom_brain::chunker::MarkdownChunker;

    let content = sample_knowledge_md();
    let chunker = MarkdownChunker::new(500);
    let tags = vec!["cto".to_string()];

    let chunks1 = chunker.chunk_file("stable_test", &content, &tags).unwrap();
    let chunks2 = chunker.chunk_file("stable_test", &content, &tags).unwrap();

    assert_eq!(chunks1.len(), chunks2.len());
    for (c1, c2) in chunks1.iter().zip(chunks2.iter()) {
        assert_eq!(
            c1.chunk_id(),
            c2.chunk_id(),
            "chunk IDs must be deterministic"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  5. Token estimation
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_chunk_token_estimation() {
    use phantom_brain::chunker::MarkdownChunker;

    let content = sample_knowledge_md();
    let chunker = MarkdownChunker::new(500);
    let chunks = chunker
        .chunk_file("token_test", &content, &["cto".to_string()])
        .unwrap();

    for chunk in &chunks {
        let tokens = chunk.estimated_tokens();
        assert!(tokens > 0, "chunk must have non-zero estimated tokens");
        // Each chunk should be roughly within our max_tokens budget
        // (some may exceed slightly due to paragraph-boundary splitting)
        assert!(
            tokens <= 600,
            "chunk '{}' has {} estimated tokens, should be near 500 max",
            chunk.section_heading,
            tokens
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  6. Empty and edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_empty_content_produces_no_chunks() {
    use phantom_brain::chunker::MarkdownChunker;

    let chunker = MarkdownChunker::new(500);
    let chunks = chunker.chunk_file("empty", "", &[]).unwrap();
    assert!(chunks.is_empty());
}

#[test]
fn test_single_heading_produces_one_chunk() {
    use phantom_brain::chunker::MarkdownChunker;

    let content = "# Single Section\n\nJust one paragraph of content here.\n";
    let chunker = MarkdownChunker::new(500);
    let chunks = chunker
        .chunk_file("single", content, &["cto".to_string()])
        .unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].section_heading, "# Single Section");
}
