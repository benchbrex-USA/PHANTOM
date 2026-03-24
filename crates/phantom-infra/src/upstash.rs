//! Upstash Redis REST API client.
//!
//! Provides programmatic access to Upstash's management API for creating
//! and managing serverless Redis databases, plus command execution via
//! the per-database REST endpoint.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::errors::InfraError;

/// Upstash management API base URL.
const API_BASE: &str = "https://api.upstash.com/v2";

/// Supported Upstash regions.
const VALID_REGIONS: &[&str] = &[
    "us-east-1",
    "us-west-1",
    "us-west-2",
    "eu-west-1",
    "eu-central-1",
    "ap-southeast-1",
    "ap-northeast-1",
    "global",
];

/// Upstash Redis REST API client.
#[derive(Debug, Clone)]
pub struct UpstashClient {
    client: reqwest::Client,
    api_key: String,
    email: String,
}

/// An Upstash Redis database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisDatabase {
    pub database_id: String,
    pub database_name: String,
    pub endpoint: String,
    pub password: String,
    pub port: u16,
    pub region: String,
    #[serde(default)]
    pub created_at: Option<u64>,
    #[serde(default)]
    pub tls: Option<bool>,
    #[serde(default)]
    pub rest_token: Option<String>,
    #[serde(default)]
    pub read_only_rest_token: Option<String>,
}

impl UpstashClient {
    /// Create a new Upstash client.
    pub fn new(api_key: String, email: String) -> Self {
        let client = reqwest::Client::builder()
            .build()
            .expect("failed to build reqwest client");
        Self {
            client,
            api_key,
            email,
        }
    }

    fn basic_auth(&self) -> String {
        use std::io::Write;
        let mut buf = Vec::new();
        write!(buf, "{}:{}", self.email, self.api_key).unwrap();
        let encoded = base64_encode(&buf);
        format!("Basic {}", encoded)
    }

    /// Create a new Redis database.
    pub async fn create_database(
        &self,
        name: &str,
        region: &str,
    ) -> Result<RedisDatabase, InfraError> {
        if !is_valid_region(region) {
            return Err(InfraError::ProvisioningFailed {
                resource: format!("upstash-db/{}", name),
                reason: format!(
                    "invalid region '{}', must be one of: {}",
                    region,
                    VALID_REGIONS.join(", ")
                ),
            });
        }

        info!(name = %name, region = %region, "creating Upstash Redis database");

        let url = format!("{}/redis/database", API_BASE);
        let body = serde_json::json!({
            "name": name,
            "region": region,
            "tls": true,
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.basic_auth())
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProvisioningFailed {
                resource: format!("upstash-db/{}", name),
                reason: format!("HTTP {} — {}", status, text),
            });
        }

        let db: RedisDatabase = resp.json().await?;
        debug!(db_id = %db.database_id, endpoint = %db.endpoint, "database created");
        Ok(db)
    }

    /// Delete a Redis database by ID.
    pub async fn delete_database(&self, db_id: &str) -> Result<(), InfraError> {
        info!(db_id = %db_id, "deleting Upstash Redis database");

        let url = format!("{}/redis/database/{}", API_BASE, db_id);
        let resp = self
            .client
            .delete(&url)
            .header("Authorization", self.basic_auth())
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProvisioningFailed {
                resource: format!("upstash-db/{}", db_id),
                reason: format!("HTTP {} — {}", status, text),
            });
        }

        Ok(())
    }

    /// List all Redis databases.
    pub async fn list_databases(&self) -> Result<Vec<RedisDatabase>, InfraError> {
        debug!("listing Upstash Redis databases");

        let url = format!("{}/redis/databases", API_BASE);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.basic_auth())
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::Http(format!("HTTP {} — {}", status, text)));
        }

        let dbs: Vec<RedisDatabase> = resp.json().await?;
        Ok(dbs)
    }

    /// Execute a Redis command via the per-database REST API.
    ///
    /// The `endpoint` and `token` come from the created database's
    /// `endpoint` and `rest_token` fields.
    pub async fn execute_command(
        &self,
        endpoint: &str,
        token: &str,
        cmd: &[&str],
    ) -> Result<serde_json::Value, InfraError> {
        if cmd.is_empty() {
            return Err(InfraError::ProvisioningFailed {
                resource: "upstash-command".into(),
                reason: "command must not be empty".into(),
            });
        }

        debug!(endpoint = %endpoint, cmd = ?cmd, "executing Redis command");

        let url = format!("https://{}", endpoint);
        let body = cmd;

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(InfraError::Http(format!(
                "redis command failed: HTTP {} — {}",
                status, text
            )));
        }

        let result: serde_json::Value = resp.json().await?;
        Ok(result)
    }
}

/// Check if a region string is valid for Upstash.
fn is_valid_region(region: &str) -> bool {
    VALID_REGIONS.contains(&region)
}

/// Simple base64 encoding (no padding issues in auth).
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let chunks = data.chunks(3);
    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

// ═══════════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = UpstashClient::new("api_key_123".into(), "user@example.com".into());
        assert_eq!(client.api_key, "api_key_123");
        assert_eq!(client.email, "user@example.com");
    }

    #[test]
    fn test_basic_auth_format() {
        let client = UpstashClient::new("key".into(), "me@test.com".into());
        let auth = client.basic_auth();
        assert!(auth.starts_with("Basic "));
        assert!(auth.len() > 6);
    }

    #[test]
    fn test_redis_database_deserialize() {
        let json = r#"{
            "database_id": "db-123abc",
            "database_name": "phantom-cache",
            "endpoint": "usw1-fast-panda-12345.upstash.io",
            "password": "secret_pwd",
            "port": 6379,
            "region": "us-west-1",
            "created_at": 1700000000,
            "tls": true,
            "rest_token": "AXyz..."
        }"#;
        let db: RedisDatabase = serde_json::from_str(json).unwrap();
        assert_eq!(db.database_id, "db-123abc");
        assert_eq!(db.database_name, "phantom-cache");
        assert_eq!(db.port, 6379);
        assert_eq!(db.region, "us-west-1");
        assert!(db.tls.unwrap());
    }

    #[test]
    fn test_redis_database_deserialize_minimal() {
        let json = r#"{
            "database_id": "db-1",
            "database_name": "test",
            "endpoint": "host.upstash.io",
            "password": "pw",
            "port": 6380,
            "region": "eu-west-1"
        }"#;
        let db: RedisDatabase = serde_json::from_str(json).unwrap();
        assert_eq!(db.database_id, "db-1");
        assert!(db.created_at.is_none());
        assert!(db.rest_token.is_none());
    }

    #[test]
    fn test_command_serialization() {
        let cmd: &[&str] = &["SET", "key", "value"];
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(json, r#"["SET","key","value"]"#);
    }

    #[test]
    fn test_region_validation_valid() {
        assert!(is_valid_region("us-east-1"));
        assert!(is_valid_region("eu-west-1"));
        assert!(is_valid_region("global"));
        assert!(is_valid_region("ap-northeast-1"));
    }

    #[test]
    fn test_region_validation_invalid() {
        assert!(!is_valid_region("us-east-99"));
        assert!(!is_valid_region("moon-base-1"));
        assert!(!is_valid_region(""));
        assert!(!is_valid_region("US-EAST-1")); // case-sensitive
    }

    #[test]
    fn test_api_base_url() {
        assert_eq!(API_BASE, "https://api.upstash.com/v2");
        let db_url = format!("{}/redis/database", API_BASE);
        assert_eq!(db_url, "https://api.upstash.com/v2/redis/database");
    }
}
