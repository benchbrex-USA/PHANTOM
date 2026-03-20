//! Knowledge query interface — the main API agents use to query the brain.
//!
//! Every agent queries the Knowledge Brain BEFORE making a decision:
//!   1. Agent generates semantic query from task description
//!   2. Brain returns top-K relevant knowledge chunks
//!   3. Chunks injected into agent's context as KNOWLEDGE REFERENCE
//!   4. Agent's output CITES which section it used

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use crate::chromadb::{ChromaClient, Collection};
use crate::chunker::{self, MarkdownChunker};
use crate::config::BrainConfig;
use crate::embeddings::EmbeddingGenerator;
use crate::BrainError;

/// A knowledge chunk returned from a brain query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeChunk {
    /// Source knowledge file name
    pub source_file: String,
    /// Section heading within the file
    pub section: String,
    /// The actual content
    pub content: String,
    /// Relevance score (0.0 - 1.0, higher = more relevant)
    pub score: f32,
    /// Which agent roles this chunk is tagged for
    pub agent_tags: Vec<String>,
    /// Line range in source file
    pub line_start: usize,
    pub line_end: usize,
}

impl KnowledgeChunk {
    /// Format this chunk as a context reference for injection into agent prompts.
    pub fn as_context_reference(&self) -> String {
        format!(
            "KNOWLEDGE REFERENCE (source: {}, section: {}, relevance: {:.2}):\n{}",
            self.source_file, self.section, self.score, self.content
        )
    }
}

/// A query to the Knowledge Brain.
#[derive(Debug, Clone)]
pub struct KnowledgeQuery {
    /// The semantic query text
    pub query: String,
    /// Optional: filter by agent role
    pub agent_role: Option<String>,
    /// Number of results to return (overrides config default)
    pub top_k: Option<usize>,
    /// Minimum relevance score (overrides config default)
    pub min_score: Option<f32>,
}

impl KnowledgeQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            agent_role: None,
            top_k: None,
            min_score: None,
        }
    }

    pub fn with_agent_role(mut self, role: impl Into<String>) -> Self {
        self.agent_role = Some(role.into());
        self
    }

    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = Some(k);
        self
    }

    pub fn with_min_score(mut self, score: f32) -> Self {
        self.min_score = Some(score);
        self
    }
}

/// The Knowledge Brain — the central knowledge query system.
///
/// Agents use this to ground their decisions in the 10 expert knowledge files.
pub struct KnowledgeBrain {
    config: BrainConfig,
    chroma: ChromaClient,
    embedder: EmbeddingGenerator,
    collection: Option<Collection>,
}

impl KnowledgeBrain {
    /// Create a new Knowledge Brain with the given configuration.
    pub fn new(config: BrainConfig) -> Self {
        let chroma = ChromaClient::new(&config.chromadb_url);
        let embedder = EmbeddingGenerator::new(
            &config.embedding_model,
            config.embedding_dimensions,
            config.embedding_server_url.clone(),
        );

        Self {
            config,
            chroma,
            embedder,
            collection: None,
        }
    }

    /// Initialize the brain — connect to ChromaDB, create/get collection.
    #[instrument(skip(self))]
    pub async fn initialize(&mut self) -> Result<(), BrainError> {
        // Health check ChromaDB
        let healthy = self.chroma.health_check().await.unwrap_or(false);
        if !healthy {
            return Err(BrainError::ConnectionFailed(
                "ChromaDB health check failed".into(),
            ));
        }

        // Get or create the knowledge collection
        let collection = self
            .chroma
            .get_or_create_collection(
                &self.config.collection_name,
                Some(serde_json::json!({
                    "description": "Phantom Knowledge Brain — 10 expert knowledge files",
                    "hnsw:space": "cosine"
                })),
            )
            .await?;

        info!(
            collection_id = %collection.id,
            name = %collection.name,
            "Knowledge Brain initialized"
        );

        self.collection = Some(collection);
        Ok(())
    }

    /// Ingest a knowledge file — chunk it, embed it, store in ChromaDB.
    #[instrument(skip(self, content), fields(filename))]
    pub async fn ingest_file(&self, filename: &str, content: &str) -> Result<usize, BrainError> {
        let collection = self
            .collection
            .as_ref()
            .ok_or_else(|| BrainError::ConnectionFailed("brain not initialized".into()))?;

        // Step 1: Determine agent tags for this file
        let agent_tags = chunker::default_agent_tags(filename);

        // Step 2: Chunk the file
        let chunker = MarkdownChunker::new(self.config.max_chunk_tokens);
        let chunks = chunker.chunk_file(filename, content, &agent_tags)?;

        if chunks.is_empty() {
            warn!(filename, "no chunks extracted from file");
            return Ok(0);
        }

        info!(
            filename,
            chunk_count = chunks.len(),
            "chunked knowledge file"
        );

        // Step 3: Generate embeddings for all chunks
        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = self.embedder.embed_batch(&texts).await?;

        // Step 4: Prepare metadata and upsert into ChromaDB
        let ids: Vec<String> = chunks.iter().map(|c| c.chunk_id()).collect();
        let documents: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let metadatas: Vec<serde_json::Value> = chunks
            .iter()
            .map(|c| {
                serde_json::json!({
                    "source_file": c.source_file,
                    "section_heading": c.section_heading,
                    "line_start": c.line_start,
                    "line_end": c.line_end,
                    "agent_tags": c.agent_tags.join(","),
                    "estimated_tokens": c.estimated_tokens(),
                })
            })
            .collect();

        self.chroma
            .upsert(&collection.id, ids, embeddings, documents, metadatas)
            .await?;

        info!(
            filename,
            chunk_count = chunks.len(),
            "ingested knowledge file into ChromaDB"
        );

        Ok(chunks.len())
    }

    /// Ingest a knowledge file from disk.
    pub async fn ingest_file_from_path(&self, path: &Path) -> Result<usize, BrainError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| BrainError::FileReadError(format!("{}: {}", path.display(), e)))?;

        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        self.ingest_file(filename, &content).await
    }

    /// Ingest all knowledge files from a directory.
    #[instrument(skip(self))]
    pub async fn ingest_directory(&self, dir: &Path) -> Result<usize, BrainError> {
        let mut total_chunks = 0;

        let entries = std::fs::read_dir(dir)
            .map_err(|e| BrainError::FileReadError(format!("{}: {}", dir.display(), e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| BrainError::FileReadError(e.to_string()))?;
            let path = entry.path();

            if path.extension().map(|e| e == "md").unwrap_or(false) {
                match self.ingest_file_from_path(&path).await {
                    Ok(count) => total_chunks += count,
                    Err(e) => {
                        warn!(
                            file = %path.display(),
                            error = %e,
                            "failed to ingest knowledge file, continuing"
                        );
                    }
                }
            }
        }

        info!(total_chunks, "knowledge directory ingestion complete");
        Ok(total_chunks)
    }

    /// Query the Knowledge Brain — the primary API for agents.
    ///
    /// Returns knowledge chunks relevant to the query, optionally filtered by agent role.
    #[instrument(skip(self))]
    pub async fn query(&self, query: &KnowledgeQuery) -> Result<Vec<KnowledgeChunk>, BrainError> {
        let collection = self
            .collection
            .as_ref()
            .ok_or_else(|| BrainError::ConnectionFailed("brain not initialized".into()))?;

        // Step 1: Generate embedding for the query
        let query_embedding = self.embedder.embed(&query.query).await?;

        // Step 2: Build optional where filter for agent role
        let where_filter = query.agent_role.as_ref().map(|role| {
            serde_json::json!({
                "agent_tags": {"$contains": role}
            })
        });

        // Step 3: Query ChromaDB
        let top_k = query.top_k.unwrap_or(self.config.top_k);
        let results = self
            .chroma
            .query(&collection.id, query_embedding, top_k, where_filter)
            .await?;

        // Step 4: Convert to KnowledgeChunks with scores
        let min_score = query.min_score.unwrap_or(self.config.min_score);
        let chunks: Vec<KnowledgeChunk> = results
            .into_iter()
            .filter_map(|r| {
                // ChromaDB returns cosine distance (0 = identical, 2 = opposite)
                // Convert to similarity score: 1 - (distance / 2)
                let score = 1.0 - (r.distance / 2.0);

                if score < min_score {
                    return None;
                }

                let metadata = &r.metadata;
                Some(KnowledgeChunk {
                    source_file: metadata
                        .get("source_file")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    section: metadata
                        .get("section_heading")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    content: r.document,
                    score,
                    agent_tags: metadata
                        .get("agent_tags")
                        .and_then(|v| v.as_str())
                        .map(|s| s.split(',').map(String::from).collect())
                        .unwrap_or_default(),
                    line_start: metadata
                        .get("line_start")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                    line_end: metadata
                        .get("line_end")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                })
            })
            .collect();

        debug!(
            query = %query.query,
            results = chunks.len(),
            "knowledge query complete"
        );

        Ok(chunks)
    }

    /// Format query results as context for injection into an agent's prompt.
    pub fn format_as_context(chunks: &[KnowledgeChunk]) -> String {
        if chunks.is_empty() {
            return "No relevant knowledge found.".to_string();
        }

        chunks
            .iter()
            .enumerate()
            .map(|(i, chunk)| format!("[Reference {}] {}\n", i + 1, chunk.as_context_reference()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get the number of chunks stored in the brain.
    pub async fn chunk_count(&self) -> Result<usize, BrainError> {
        let collection = self
            .collection
            .as_ref()
            .ok_or_else(|| BrainError::ConnectionFailed("brain not initialized".into()))?;

        self.chroma.count(&collection.id).await
    }

    /// Check if the brain's dependencies are available.
    pub async fn check_dependencies(&self) -> BrainDependencyStatus {
        let chromadb_available = self.chroma.health_check().await.unwrap_or(false);
        let embedding_server_available = self.embedder.is_server_available().await;
        let python_available = self.embedder.is_python_available().await;

        BrainDependencyStatus {
            chromadb_available,
            embedding_server_available,
            python_available,
            embedding_available: embedding_server_available || python_available,
        }
    }
}

/// Status of the brain's external dependencies.
#[derive(Debug, Clone)]
pub struct BrainDependencyStatus {
    pub chromadb_available: bool,
    pub embedding_server_available: bool,
    pub python_available: bool,
    pub embedding_available: bool,
}

impl std::fmt::Display for BrainDependencyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Knowledge Brain Dependencies:")?;
        writeln!(
            f,
            "  ChromaDB:         {}",
            if self.chromadb_available {
                "OK"
            } else {
                "NOT AVAILABLE"
            }
        )?;
        writeln!(
            f,
            "  Embedding Server: {}",
            if self.embedding_server_available {
                "OK"
            } else {
                "NOT AVAILABLE"
            }
        )?;
        writeln!(
            f,
            "  Python (fallback): {}",
            if self.python_available {
                "OK"
            } else {
                "NOT AVAILABLE"
            }
        )?;
        writeln!(
            f,
            "  Embedding Ready:  {}",
            if self.embedding_available {
                "OK"
            } else {
                "NOT AVAILABLE"
            }
        )
    }
}

/// The 10 knowledge files that make up Phantom's brain.
pub const KNOWLEDGE_FILES: &[&str] = &[
    "The_CTO_Architecture_Framework",
    "The_CTO_s_Complete_Technology_Knowledge",
    "The_Complete_Multi-Agent_Autonomous_System",
    "Build_Once._Launch_Directly",
    "The_Complete_Full-Stack_Software_Blueprint",
    "Every_Technology_Used_to_Build_Software",
    "The_Complete_Design_Expert_Knowledge_Base",
    "The_Complete_AI_ML_Expert_Knowledge_Base",
    "The_Complete_API_Expert_Knowledge_Base",
    "AI_Code_GitHub_Errors_Fixes",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_query_builder() {
        let q = KnowledgeQuery::new("API design REST authentication")
            .with_agent_role("backend")
            .with_top_k(3)
            .with_min_score(0.5);

        assert_eq!(q.query, "API design REST authentication");
        assert_eq!(q.agent_role.as_deref(), Some("backend"));
        assert_eq!(q.top_k, Some(3));
        assert_eq!(q.min_score, Some(0.5));
    }

    #[test]
    fn test_knowledge_chunk_context_format() {
        let chunk = KnowledgeChunk {
            source_file: "API_Expert".to_string(),
            section: "## 4. BaseAPIClient Architecture".to_string(),
            content: "The BaseAPIClient pattern provides retry + circuit breaker...".to_string(),
            score: 0.92,
            agent_tags: vec!["backend".to_string()],
            line_start: 100,
            line_end: 150,
        };

        let ctx = chunk.as_context_reference();
        assert!(ctx.contains("API_Expert"));
        assert!(ctx.contains("BaseAPIClient"));
        assert!(ctx.contains("0.92"));
    }

    #[test]
    fn test_format_empty_context() {
        let result = KnowledgeBrain::format_as_context(&[]);
        assert_eq!(result, "No relevant knowledge found.");
    }

    #[test]
    fn test_format_multiple_chunks() {
        let chunks = vec![
            KnowledgeChunk {
                source_file: "file1".to_string(),
                section: "## Section A".to_string(),
                content: "Content A".to_string(),
                score: 0.9,
                agent_tags: vec![],
                line_start: 1,
                line_end: 10,
            },
            KnowledgeChunk {
                source_file: "file2".to_string(),
                section: "## Section B".to_string(),
                content: "Content B".to_string(),
                score: 0.7,
                agent_tags: vec![],
                line_start: 1,
                line_end: 10,
            },
        ];

        let result = KnowledgeBrain::format_as_context(&chunks);
        assert!(result.contains("[Reference 1]"));
        assert!(result.contains("[Reference 2]"));
        assert!(result.contains("Content A"));
        assert!(result.contains("Content B"));
    }

    #[test]
    fn test_knowledge_files_count() {
        assert_eq!(KNOWLEDGE_FILES.len(), 10);
    }

    #[test]
    fn test_brain_dependency_status_display() {
        let status = BrainDependencyStatus {
            chromadb_available: true,
            embedding_server_available: false,
            python_available: true,
            embedding_available: true,
        };

        let display = format!("{}", status);
        assert!(display.contains("ChromaDB:         OK"));
        assert!(display.contains("Embedding Server: NOT AVAILABLE"));
        assert!(display.contains("Python (fallback): OK"));
    }
}
