//! Embedding generation using sentence-transformers.
//!
//! Two modes:
//! 1. HTTP API — calls an embedding server (e.g., TEI, or a custom Flask/FastAPI wrapper)
//! 2. Python subprocess — falls back to running sentence-transformers directly via Python
//!
//! Model: all-MiniLM-L6-v2 (384 dimensions, fast, good quality)

use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};

use crate::BrainError;

/// Embedding generator — wraps sentence-transformers model.
pub struct EmbeddingGenerator {
    /// HTTP client for API-based embedding
    client: reqwest::Client,
    /// Embedding server URL (if available)
    server_url: Option<String>,
    /// Model name for Python fallback
    model_name: String,
    /// Expected embedding dimensions
    dimensions: usize,
}

/// Response from the embedding HTTP API.
#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

/// Response from TEI-compatible API (Hugging Face Text Embeddings Inference).
/// TEI returns a flat array of arrays directly.
type TeiResponse = Vec<Vec<f32>>;

impl EmbeddingGenerator {
    /// Create a new embedding generator.
    pub fn new(model_name: &str, dimensions: usize, server_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            server_url,
            model_name: model_name.to_string(),
            dimensions,
        }
    }

    /// Generate embeddings for a batch of texts.
    #[instrument(skip(self, texts), fields(batch_size = texts.len()))]
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, BrainError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Try HTTP API first
        if let Some(ref url) = self.server_url {
            match self.embed_via_http(url, texts).await {
                Ok(embeddings) => return Ok(embeddings),
                Err(e) => {
                    warn!(error = %e, "HTTP embedding failed, falling back to Python subprocess");
                }
            }
        }

        // Fall back to Python subprocess
        self.embed_via_python(texts).await
    }

    /// Generate embedding for a single text.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, BrainError> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| BrainError::EmbeddingFailed("empty result".into()))
    }

    /// Generate embeddings via HTTP API (TEI-compatible).
    async fn embed_via_http(
        &self,
        url: &str,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, BrainError> {
        #[derive(Serialize)]
        struct TeiRequest<'a> {
            inputs: &'a [String],
        }

        let resp = self
            .client
            .post(format!("{}/embed", url.trim_end_matches('/')))
            .json(&TeiRequest { inputs: texts })
            .send()
            .await
            .map_err(|e| BrainError::EmbeddingServerUnavailable(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BrainError::EmbeddingFailed(format!(
                "HTTP {}: {}",
                "error", body
            )));
        }

        // Try TEI format first (direct array of arrays)
        let body = resp
            .text()
            .await
            .map_err(|e| BrainError::EmbeddingFailed(format!("response read error: {}", e)))?;

        // Try parsing as TEI response (Vec<Vec<f32>>)
        if let Ok(tei_resp) = serde_json::from_str::<TeiResponse>(&body) {
            self.validate_embeddings(&tei_resp)?;
            return Ok(tei_resp);
        }

        // Try custom format with "embeddings" key
        if let Ok(embed_resp) = serde_json::from_str::<EmbedResponse>(&body) {
            self.validate_embeddings(&embed_resp.embeddings)?;
            return Ok(embed_resp.embeddings);
        }

        Err(BrainError::EmbeddingFailed(
            "unrecognized embedding response format".into(),
        ))
    }

    /// Generate embeddings via Python subprocess (sentence-transformers).
    async fn embed_via_python(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, BrainError> {
        let texts_json =
            serde_json::to_string(texts).map_err(|e| BrainError::EmbeddingFailed(e.to_string()))?;

        // Python script that uses sentence-transformers to generate embeddings
        let python_script = format!(
            r#"
import sys, json
try:
    from sentence_transformers import SentenceTransformer
    model = SentenceTransformer("{model}")
    texts = json.loads(sys.argv[1])
    embeddings = model.encode(texts, convert_to_numpy=True).tolist()
    print(json.dumps(embeddings))
except Exception as e:
    print(json.dumps({{"error": str(e)}}), file=sys.stderr)
    sys.exit(1)
"#,
            model = self.model_name,
        );

        let output = tokio::process::Command::new("python3")
            .arg("-c")
            .arg(&python_script)
            .arg(&texts_json)
            .output()
            .await
            .map_err(|e| BrainError::PythonError(format!("failed to spawn python3: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrainError::PythonError(format!(
                "python embedding failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let embeddings: Vec<Vec<f32>> = serde_json::from_str(&stdout)
            .map_err(|e| BrainError::EmbeddingFailed(format!("parse error: {}", e)))?;

        self.validate_embeddings(&embeddings)?;
        debug!(count = embeddings.len(), "generated embeddings via Python");

        Ok(embeddings)
    }

    /// Validate that embeddings have the expected dimensions.
    fn validate_embeddings(&self, embeddings: &[Vec<f32>]) -> Result<(), BrainError> {
        for (i, emb) in embeddings.iter().enumerate() {
            if emb.len() != self.dimensions {
                return Err(BrainError::EmbeddingFailed(format!(
                    "embedding {} has {} dimensions, expected {}",
                    i,
                    emb.len(),
                    self.dimensions
                )));
            }
        }
        Ok(())
    }

    /// Check if the embedding server is available.
    pub async fn is_server_available(&self) -> bool {
        if let Some(ref url) = self.server_url {
            self.client
                .get(format!("{}/health", url.trim_end_matches('/')))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        } else {
            false
        }
    }

    /// Check if Python sentence-transformers is available.
    pub async fn is_python_available(&self) -> bool {
        let output = tokio::process::Command::new("python3")
            .arg("-c")
            .arg("import sentence_transformers; print('ok')")
            .output()
            .await;

        output.map(|o| o.status.success()).unwrap_or(false)
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_correct_dimensions() {
        let gen = EmbeddingGenerator::new("test-model", 384, None);
        let embeddings = vec![vec![0.0f32; 384], vec![0.0f32; 384]];
        assert!(gen.validate_embeddings(&embeddings).is_ok());
    }

    #[test]
    fn test_validate_wrong_dimensions() {
        let gen = EmbeddingGenerator::new("test-model", 384, None);
        let embeddings = vec![vec![0.0f32; 256]];
        assert!(gen.validate_embeddings(&embeddings).is_err());
    }

    #[test]
    fn test_validate_empty() {
        let gen = EmbeddingGenerator::new("test-model", 384, None);
        let embeddings: Vec<Vec<f32>> = vec![];
        assert!(gen.validate_embeddings(&embeddings).is_ok());
    }
}
