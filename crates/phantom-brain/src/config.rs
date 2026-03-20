//! Brain configuration — ChromaDB endpoint, embedding model, chunk parameters.

use serde::{Deserialize, Serialize};

/// Configuration for the Knowledge Brain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainConfig {
    /// ChromaDB server URL (e.g., "http://localhost:8000")
    pub chromadb_url: String,

    /// ChromaDB collection name for knowledge chunks
    pub collection_name: String,

    /// Embedding model name (sentence-transformers model ID)
    pub embedding_model: String,

    /// Embedding dimensions (384 for all-MiniLM-L6-v2)
    pub embedding_dimensions: usize,

    /// Maximum tokens per chunk when splitting markdown
    pub max_chunk_tokens: usize,

    /// Number of results to return per query (top-K)
    pub top_k: usize,

    /// Minimum relevance score threshold (0.0 - 1.0)
    pub min_score: f32,

    /// Embedding server URL (for HTTP-based embedding generation)
    /// If None, falls back to Python subprocess
    pub embedding_server_url: Option<String>,
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            chromadb_url: "http://localhost:8000".to_string(),
            collection_name: "phantom_knowledge".to_string(),
            embedding_model: "all-MiniLM-L6-v2".to_string(),
            embedding_dimensions: 384,
            max_chunk_tokens: 500,
            top_k: 5,
            min_score: 0.3,
            embedding_server_url: None,
        }
    }
}
