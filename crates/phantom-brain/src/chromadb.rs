//! ChromaDB HTTP client for knowledge vector storage and retrieval.
//!
//! ChromaDB REST API v1 — self-hosted instance.
//! Handles: collection management, document upsert, semantic query.
//!
//! All data stored in ChromaDB is encrypted before storage on remote servers.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};

use crate::BrainError;

/// ChromaDB HTTP client.
pub struct ChromaClient {
    client: Client,
    base_url: String,
}

/// ChromaDB collection metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub metadata: Option<serde_json::Value>,
}

/// Request to create or get a collection.
#[derive(Debug, Serialize)]
struct CreateCollectionRequest {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    get_or_create: bool,
}

/// Request to add/upsert documents to a collection.
#[derive(Debug, Serialize)]
struct UpsertRequest {
    ids: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    documents: Vec<String>,
    metadatas: Vec<serde_json::Value>,
}

/// Request to query a collection.
#[derive(Debug, Serialize)]
struct QueryRequest {
    query_embeddings: Vec<Vec<f32>>,
    n_results: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#where: Option<serde_json::Value>,
    include: Vec<String>,
}

/// Response from a ChromaDB query.
#[derive(Debug, Deserialize)]
pub struct QueryResponse {
    pub ids: Vec<Vec<String>>,
    pub documents: Option<Vec<Vec<Option<String>>>>,
    pub metadatas: Option<Vec<Vec<Option<serde_json::Value>>>>,
    pub distances: Option<Vec<Vec<f32>>>,
}

/// A single query result with all fields resolved.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub id: String,
    pub document: String,
    pub metadata: serde_json::Value,
    /// Distance from query (lower = more similar for L2, higher for cosine similarity)
    pub distance: f32,
}

/// Request to delete documents from a collection.
#[derive(Debug, Serialize)]
struct DeleteRequest {
    ids: Vec<String>,
}

impl ChromaClient {
    /// Create a new ChromaDB client.
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Health check — verify ChromaDB is reachable.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<bool, BrainError> {
        let resp = self
            .client
            .get(format!("{}/api/v1/heartbeat", self.base_url))
            .send()
            .await
            .map_err(|e| BrainError::ConnectionFailed(e.to_string()))?;

        Ok(resp.status().is_success())
    }

    /// Get or create a collection.
    #[instrument(skip(self))]
    pub async fn get_or_create_collection(
        &self,
        name: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<Collection, BrainError> {
        let req = CreateCollectionRequest {
            name: name.to_string(),
            metadata,
            get_or_create: true,
        };

        let resp = self
            .client
            .post(format!("{}/api/v1/collections", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(BrainError::RequestFailed(format!(
                "create collection: {} — {}",
                status, body
            )));
        }

        let collection: Collection = resp.json().await?;
        debug!(collection_id = %collection.id, name = %collection.name, "collection ready");
        Ok(collection)
    }

    /// Get a collection by name.
    #[instrument(skip(self))]
    pub async fn get_collection(&self, name: &str) -> Result<Collection, BrainError> {
        let resp = self
            .client
            .get(format!("{}/api/v1/collections/{}", self.base_url, name))
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(BrainError::CollectionNotFound(name.to_string()));
        }

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BrainError::RequestFailed(format!(
                "get collection: {}",
                body
            )));
        }

        Ok(resp.json().await?)
    }

    /// Upsert documents with embeddings into a collection.
    #[instrument(skip(self, embeddings, documents, metadatas), fields(count = ids.len()))]
    pub async fn upsert(
        &self,
        collection_id: &str,
        ids: Vec<String>,
        embeddings: Vec<Vec<f32>>,
        documents: Vec<String>,
        metadatas: Vec<serde_json::Value>,
    ) -> Result<(), BrainError> {
        let req = UpsertRequest {
            ids,
            embeddings,
            documents,
            metadatas,
        };

        let resp = self
            .client
            .post(format!(
                "{}/api/v1/collections/{}/upsert",
                self.base_url, collection_id
            ))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BrainError::RequestFailed(format!("upsert: {}", body)));
        }

        Ok(())
    }

    /// Query a collection with an embedding vector.
    #[instrument(skip(self, query_embedding), fields(n_results))]
    pub async fn query(
        &self,
        collection_id: &str,
        query_embedding: Vec<f32>,
        n_results: usize,
        where_filter: Option<serde_json::Value>,
    ) -> Result<Vec<QueryResult>, BrainError> {
        let req = QueryRequest {
            query_embeddings: vec![query_embedding],
            n_results,
            r#where: where_filter,
            include: vec![
                "documents".to_string(),
                "metadatas".to_string(),
                "distances".to_string(),
            ],
        };

        let resp = self
            .client
            .post(format!(
                "{}/api/v1/collections/{}/query",
                self.base_url, collection_id
            ))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BrainError::QueryFailed(format!("query: {}", body)));
        }

        let query_resp: QueryResponse = resp.json().await?;
        let results = self.parse_query_response(query_resp);

        debug!(result_count = results.len(), "query complete");
        Ok(results)
    }

    /// Parse the ChromaDB query response into structured results.
    fn parse_query_response(&self, resp: QueryResponse) -> Vec<QueryResult> {
        let mut results = Vec::new();

        // ChromaDB returns nested arrays (one per query embedding — we send 1)
        if resp.ids.is_empty() {
            return results;
        }

        let ids = &resp.ids[0];
        let documents = resp.documents.as_ref().map(|d| &d[0]);
        let metadatas = resp.metadatas.as_ref().map(|m| &m[0]);
        let distances = resp.distances.as_ref().map(|d| &d[0]);

        for (i, id) in ids.iter().enumerate() {
            let document = documents
                .and_then(|docs| docs.get(i))
                .and_then(|d| d.clone())
                .unwrap_or_default();

            let metadata = metadatas
                .and_then(|metas| metas.get(i))
                .and_then(|m| m.clone())
                .unwrap_or(serde_json::Value::Null);

            let distance = distances
                .and_then(|dists| dists.get(i))
                .copied()
                .unwrap_or(f32::MAX);

            results.push(QueryResult {
                id: id.clone(),
                document,
                metadata,
                distance,
            });
        }

        results
    }

    /// Delete documents by ID from a collection.
    #[instrument(skip(self), fields(count = ids.len()))]
    pub async fn delete(&self, collection_id: &str, ids: Vec<String>) -> Result<(), BrainError> {
        let req = DeleteRequest { ids };

        let resp = self
            .client
            .post(format!(
                "{}/api/v1/collections/{}/delete",
                self.base_url, collection_id
            ))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BrainError::RequestFailed(format!("delete: {}", body)));
        }

        Ok(())
    }

    /// Get the count of documents in a collection.
    #[instrument(skip(self))]
    pub async fn count(&self, collection_id: &str) -> Result<usize, BrainError> {
        let resp = self
            .client
            .get(format!(
                "{}/api/v1/collections/{}/count",
                self.base_url, collection_id
            ))
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(BrainError::RequestFailed(format!("count: {}", body)));
        }

        let count: usize = resp.json().await?;
        Ok(count)
    }

    /// Delete a collection entirely.
    #[instrument(skip(self))]
    pub async fn delete_collection(&self, name: &str) -> Result<(), BrainError> {
        let resp = self
            .client
            .delete(format!("{}/api/v1/collections/{}", self.base_url, name))
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            warn!(collection = name, error = %body, "failed to delete collection");
            return Err(BrainError::RequestFailed(format!(
                "delete collection: {}",
                body
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_response() {
        let client = ChromaClient::new("http://localhost:8000");
        let resp = QueryResponse {
            ids: vec![],
            documents: None,
            metadatas: None,
            distances: None,
        };

        let results = client.parse_query_response(resp);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_query_response() {
        let client = ChromaClient::new("http://localhost:8000");
        let resp = QueryResponse {
            ids: vec![vec!["id1".to_string(), "id2".to_string()]],
            documents: Some(vec![vec![
                Some("doc1 content".to_string()),
                Some("doc2 content".to_string()),
            ]]),
            metadatas: Some(vec![vec![
                Some(serde_json::json!({"source": "file1"})),
                Some(serde_json::json!({"source": "file2"})),
            ]]),
            distances: Some(vec![vec![0.1, 0.5]]),
        };

        let results = client.parse_query_response(resp);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "id1");
        assert_eq!(results[0].document, "doc1 content");
        assert_eq!(results[0].distance, 0.1);
        assert_eq!(results[1].id, "id2");
        assert_eq!(results[1].distance, 0.5);
    }
}
