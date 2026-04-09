//! AI/ML skills for the Phantom autonomous AI engineering system.
//!
//! Covers model serving, RAG pipelines, prompt engineering, embeddings, LLM
//! gateways, guardrails, fine-tuning, agent frameworks, semantic search, and
//! AI observability.

use super::{
    OutputFormat, RetryStrategy, Skill, SkillCategory, SkillComplexity, SkillId, SkillRegistry,
};
use crate::agents::AgentRole;

/// Register all AI/ML skills into the given registry.
pub fn register(registry: &mut SkillRegistry) {
    registry.register(model_serving_pipeline());
    registry.register(rag_pipeline());
    registry.register(prompt_engineering());
    registry.register(embedding_pipeline());
    registry.register(llm_gateway());
    registry.register(ai_guardrails());
    registry.register(finetuning_pipeline());
    registry.register(ai_agent_framework());
    registry.register(semantic_search());
    registry.register(ai_observability());
}

// ---------------------------------------------------------------------------
// Skill constructors
// ---------------------------------------------------------------------------

fn model_serving_pipeline() -> Skill {
    Skill::new(
        "model_serving_pipeline",
        "ML Model Serving Pipeline",
        "Generates production ML model serving infrastructure with A/B testing, \
         shadow mode evaluation, canary rollouts, model version management, instant \
         rollback, and autoscaling based on inference latency.",
        SkillCategory::AiMl,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::DevOps, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(35_000)
    .with_system_prompt(
        "Build a model serving pipeline that loads versioned model artifacts from an \
         object store, wraps them behind a uniform prediction API (REST + gRPC), and \
         routes traffic using configurable strategies. A/B tests must use sticky \
         assignment (hash of user ID) with statistical significance checks before \
         promotion. Shadow mode duplicates requests to a candidate model without \
         affecting responses, logging predictions for offline comparison. Canary \
         rollouts ramp traffic from 1% to 100% with automatic rollback if error rate \
         or latency exceed thresholds. Model versions are immutable artifacts with \
         metadata (training run, metrics, schema). Autoscale replicas based on P99 \
         inference latency with scale-to-zero for infrequently used models.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn rag_pipeline() -> Skill {
    Skill::new(
        "rag_pipeline",
        "RAG Pipeline",
        "Implements retrieval-augmented generation with document ingestion, chunking \
         strategies, embedding generation, vector store indexing, hybrid search, \
         reranking, and context window assembly with citation tracking.",
        SkillCategory::AiMl,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Design a RAG pipeline where documents are ingested through format-specific \
         parsers (PDF, HTML, Markdown, DOCX) into a unified text representation. \
         Chunk using recursive character splitting with overlap, respecting semantic \
         boundaries (paragraphs, sections, code blocks). Generate embeddings via a \
         configurable model (OpenAI, Cohere, local) with batching and retry. Index \
         chunks in a vector store with metadata filters (source, date, type). \
         Retrieval must combine dense vector search with sparse BM25 via reciprocal \
         rank fusion. Apply a cross-encoder reranker to the top-k candidates. \
         Assemble the final context window by fitting ranked chunks within the token \
         budget, deduplicating overlapping content, and attaching source citations \
         that map each chunk to its origin document and page.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 3_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn prompt_engineering() -> Skill {
    Skill::new(
        "prompt_engineering",
        "Prompt Engineering & Optimization",
        "Generates optimized prompts with chain-of-thought reasoning, few-shot \
         exemplars, structured output schemas, input/output guardrails, and \
         systematic prompt versioning with A/B evaluation.",
        SkillCategory::AiMl,
        SkillComplexity::Composite,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Engineer prompts as versioned, testable artifacts. Use chain-of-thought \
         with explicit reasoning steps when accuracy matters more than latency. \
         Select few-shot exemplars that cover edge cases and failure modes, not \
         just happy paths. Define output schemas as JSON Schema or Pydantic models \
         and instruct the model to conform strictly. Add input guardrails that \
         validate user input before it reaches the model, and output guardrails \
         that parse, validate, and retry on malformed responses. Store prompts in \
         a registry with semantic versioning so changes are auditable. Include an \
         evaluation harness that runs prompt versions against a golden dataset and \
         reports accuracy, latency, and token cost regressions.",
    )
    .with_quality_threshold(0.80)
}

fn embedding_pipeline() -> Skill {
    Skill::new(
        "embedding_pipeline",
        "Embedding Generation Pipeline",
        "Builds an embedding generation pipeline with model selection, batched \
         inference, result caching, dimensionality reduction, and incremental \
         re-embedding on source changes.",
        SkillCategory::AiMl,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(20_000)
    .with_system_prompt(
        "Implement an embedding pipeline that accepts text, images, or multimodal \
         inputs and produces dense vector representations. Support pluggable models \
         (OpenAI text-embedding-3, Cohere embed-v3, local sentence-transformers) \
         behind a unified interface. Batch inputs to maximize GPU/API throughput \
         while respecting rate limits. Cache embeddings keyed on content hash to \
         avoid redundant computation. Optionally apply dimensionality reduction \
         (PCA, Matryoshka) with configurable target dimensions. Track which source \
         documents have changed since last embedding via content hashing and only \
         re-embed diffs. Emit metrics on embedding latency, cache hit rate, and \
         model-specific token consumption.",
    )
    .with_quality_threshold(0.80)
}

fn llm_gateway() -> Skill {
    Skill::new(
        "llm_gateway",
        "LLM Gateway & Router",
        "Creates a centralized LLM gateway with intelligent model routing, rate \
         limiting, cost tracking per team/feature, prompt caching, provider failover, \
         and request/response logging.",
        SkillCategory::AiMl,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Backend, AgentRole::Architect, AgentRole::DevOps],
        OutputFormat::Code,
    )
    .with_estimated_tokens(30_000)
    .with_system_prompt(
        "Build an LLM gateway that sits between application code and model providers. \
         Route requests to the optimal model based on task complexity, latency budget, \
         and cost constraints (e.g., simple classification to Haiku, complex reasoning \
         to Opus). Enforce per-team and per-feature rate limits with token bucket \
         algorithms. Track cost attribution by tagging every request with team, \
         feature, and environment. Implement semantic prompt caching that returns \
         cached responses for semantically identical prompts within a TTL window. \
         Failover transparently to backup providers when the primary returns 5xx or \
         exceeds latency SLA. Log all request/response pairs (with PII redaction) \
         for debugging and compliance. Expose a unified API that normalizes provider \
         differences in streaming, tool calling, and structured output.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 3,
        backoff_ms: 2_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn ai_guardrails() -> Skill {
    Skill::new(
        "ai_guardrails",
        "AI Input/Output Guardrails",
        "Implements comprehensive AI guardrails with toxicity detection, PII \
         filtering, topic restriction enforcement, hallucination detection via \
         grounding checks, and content policy enforcement.",
        SkillCategory::AiMl,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Security],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Build a guardrails layer that wraps LLM calls with configurable input and \
         output validators. Input guardrails: detect and redact PII (SSN, email, \
         phone, credit card) using regex + NER, reject prompt injection attempts \
         via classifier, enforce topic allow/deny lists. Output guardrails: score \
         toxicity with a lightweight classifier and block above threshold, verify \
         factual claims against provided source documents for hallucination detection, \
         validate structured outputs against their schema, check for data leakage of \
         training data. Each guardrail is independently toggleable with per-use-case \
         policy files. Failed checks must return structured error responses with the \
         specific violation and a sanitized fallback. Log all violations for \
         security review without exposing the violating content in application logs.",
    )
    .with_quality_threshold(0.90)
}

fn finetuning_pipeline() -> Skill {
    Skill::new(
        "finetuning_pipeline",
        "Model Fine-Tuning Pipeline",
        "Generates an end-to-end fine-tuning pipeline with dataset preparation, \
         train/eval splits, hyperparameter configuration, training orchestration, \
         evaluation against baselines, and deployment promotion.",
        SkillCategory::AiMl,
        SkillComplexity::Pipeline,
        vec![AgentRole::Backend, AgentRole::Architect],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Implement a fine-tuning pipeline where datasets are prepared from raw \
         labeled data into the provider's required format (JSONL with system/user/ \
         assistant turns for chat models). Automatically split into train/validation/ \
         test sets with stratification. Validate data quality: check for duplicates, \
         empty fields, token length violations, and class imbalance. Configure \
         hyperparameters (learning rate, epochs, batch size) with sensible defaults \
         and optional sweep. Orchestrate training via provider API (OpenAI, \
         Anthropic, or local) with progress polling and cost estimation. Evaluate \
         the fine-tuned model against the base model on the held-out test set using \
         task-specific metrics (accuracy, F1, BLEU, human preference). Promote to \
         serving only if metrics exceed the baseline by a configurable threshold.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 10_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn ai_agent_framework() -> Skill {
    Skill::new(
        "ai_agent_framework",
        "AI Agent Framework",
        "Creates an AI agent framework with tool use orchestration, persistent memory, \
         planning/reflection loops, multi-agent coordination, sandboxed execution, \
         and conversation state management.",
        SkillCategory::AiMl,
        SkillComplexity::Orchestrated,
        vec![AgentRole::Architect, AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(35_000)
    .with_system_prompt(
        "Design an agent framework where agents are composed of a planner, executor, \
         and reflector. The planner decomposes goals into sub-tasks using chain-of-\
         thought. The executor dispatches sub-tasks to registered tools with typed \
         input/output schemas and sandboxed execution (timeout, memory limit, no \
         network unless whitelisted). The reflector evaluates tool outputs against \
         the plan and decides to continue, retry, or re-plan. Persistent memory \
         stores conversation history, tool results, and learned facts in a vector \
         store for retrieval across sessions. Multi-agent coordination uses a \
         message bus where agents publish observations and subscribe to relevant \
         topics. Include a supervision layer that enforces token budgets, detects \
         infinite loops via repeated action patterns, and escalates to a human \
         when confidence drops below threshold.",
    )
    .with_retry_strategy(RetryStrategy {
        max_retries: 2,
        backoff_ms: 5_000,
        fallback_skill: None,
    })
    .with_quality_threshold(0.85)
}

fn semantic_search() -> Skill {
    Skill::new(
        "semantic_search",
        "Semantic Search Engine",
        "Builds hybrid semantic search combining BM25 sparse retrieval with dense \
         vector search, query expansion, faceted filtering, relevance tuning, and \
         search analytics.",
        SkillCategory::AiMl,
        SkillComplexity::Composite,
        vec![AgentRole::Backend],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Implement a hybrid search system that fuses BM25 lexical scores with dense \
         vector cosine similarity via reciprocal rank fusion or learned score \
         combination. Query expansion rewrites user queries using synonyms, acronym \
         expansion, and LLM-generated sub-queries for recall improvement. Support \
         faceted filtering (category, date range, author) applied as pre-filters \
         on metadata before scoring. Relevance tuning exposes per-field boost weights \
         and fusion alpha as configurable parameters. Implement search analytics that \
         track query volume, click-through rate, zero-result rate, and mean \
         reciprocal rank from implicit feedback. Include an offline evaluation \
         harness that scores nDCG and recall@k against a labeled relevance dataset.",
    )
    .with_quality_threshold(0.85)
}

fn ai_observability() -> Skill {
    Skill::new(
        "ai_observability",
        "AI/LLM Observability",
        "Creates an AI observability platform with token usage tracking, latency \
         profiling, output quality scoring, cost attribution, model drift detection, \
         and prompt performance dashboards.",
        SkillCategory::AiMl,
        SkillComplexity::Composite,
        vec![AgentRole::Backend, AgentRole::Monitor],
        OutputFormat::Code,
    )
    .with_estimated_tokens(25_000)
    .with_system_prompt(
        "Build an observability layer for LLM-powered features that instruments \
         every model call. Track token consumption (input, output, cached) per \
         request with attribution to team, feature, and user tier. Profile latency \
         broken down by queue time, inference time, and post-processing. Score output \
         quality using automated evaluators (relevance, faithfulness, coherence) and \
         human feedback loops. Attribute costs by multiplying token counts by \
         per-model pricing, aggregated into dashboards by team, feature, and time \
         period. Detect model drift by comparing output distribution metrics (average \
         length, refusal rate, tool call frequency) against a rolling baseline with \
         statistical alerting. Expose all metrics via OpenTelemetry spans and a \
         dedicated LLM trace UI that shows the full chain of prompts, tool calls, \
         and responses for each user interaction.",
    )
    .with_quality_threshold(0.80)
}
