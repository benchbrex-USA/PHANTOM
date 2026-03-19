//! Phantom Knowledge Brain: ChromaDB client, embedding pipeline, semantic query.
//!
//! Core Law 4: The Knowledge Brain is the source of truth.
//!
//! Architecture:
//!   10 expert knowledge files (~25,000 lines)
//!     → Markdown chunker (split by heading, ~500 tokens each)
//!     → Embedding generator (sentence-transformers, all-MiniLM-L6-v2, 384-dim)
//!     → ChromaDB vector storage (self-hosted)
//!     → Semantic query by every agent before every decision
//!
//! Total chunks: ~500 across all 10 files
//! Query latency target: <50ms

pub mod chromadb;
pub mod chunker;
pub mod config;
pub mod embeddings;
pub mod errors;
pub mod knowledge;

pub use chunker::MarkdownChunker;
pub use config::BrainConfig;
pub use errors::BrainError;
pub use knowledge::{KnowledgeBrain, KnowledgeChunk, KnowledgeQuery};
