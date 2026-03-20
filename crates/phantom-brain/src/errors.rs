use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrainError {
    #[error("ChromaDB connection failed: {0}")]
    ConnectionFailed(String),

    #[error("ChromaDB request failed: {0}")]
    RequestFailed(String),

    #[error("collection not found: {0}")]
    CollectionNotFound(String),

    #[error("embedding generation failed: {0}")]
    EmbeddingFailed(String),

    #[error("embedding server not available: {0}")]
    EmbeddingServerUnavailable(String),

    #[error("knowledge file not found: {0}")]
    FileNotFound(String),

    #[error("knowledge file read error: {0}")]
    FileReadError(String),

    #[error("query failed: {0}")]
    QueryFailed(String),

    #[error("chunking failed: {0}")]
    ChunkingFailed(String),

    #[error("no results found for query")]
    NoResults,

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("Python subprocess failed: {0}")]
    PythonError(String),
}

impl From<reqwest::Error> for BrainError {
    fn from(e: reqwest::Error) -> Self {
        BrainError::RequestFailed(e.to_string())
    }
}

impl From<serde_json::Error> for BrainError {
    fn from(e: serde_json::Error) -> Self {
        BrainError::Serialization(e.to_string())
    }
}

impl From<std::io::Error> for BrainError {
    fn from(e: std::io::Error) -> Self {
        BrainError::FileReadError(e.to_string())
    }
}
