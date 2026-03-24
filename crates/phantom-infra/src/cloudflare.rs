//! Cloudflare provider client: Workers, R2 object storage, DNS management.
//!
//! Covers the Cloudflare API v4 for:
//! - Workers (serverless functions on the edge)
//! - R2 (S3-compatible object storage, 10GB free tier)
//! - DNS zone and record management

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::errors::InfraError;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration required to initialize the Cloudflare client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareConfig {
    /// Cloudflare API token (scoped bearer token).
    pub api_token: String,
    /// Cloudflare account ID (hex string, 32 chars).
    pub account_id: String,
}

impl CloudflareConfig {
    /// Validate the configuration fields.
    pub fn validate(&self) -> Result<(), InfraError> {
        if self.api_token.is_empty() {
            return Err(InfraError::AuthRequired {
                provider: "cloudflare".into(),
            });
        }
        if self.account_id.is_empty() {
            return Err(InfraError::ProviderUnavailable {
                provider: "cloudflare".into(),
                reason: "account_id is required".into(),
            });
        }
        if !self.account_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(InfraError::ProviderUnavailable {
                provider: "cloudflare".into(),
                reason: "account_id must be a hex string".into(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// API response envelope
// ---------------------------------------------------------------------------

/// Standard Cloudflare API v4 response wrapper.
#[derive(Debug, Deserialize)]
pub struct CfApiResponse<T> {
    pub success: bool,
    pub errors: Vec<CfApiError>,
    pub messages: Vec<serde_json::Value>,
    pub result: Option<T>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CfApiError {
    pub code: i64,
    pub message: String,
}

/// Paginated list response.
#[derive(Debug, Deserialize)]
pub struct CfListResponse<T> {
    pub success: bool,
    pub errors: Vec<CfApiError>,
    pub result: Vec<T>,
}

// ---------------------------------------------------------------------------
// Domain models
// ---------------------------------------------------------------------------

/// A Cloudflare Worker script.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_on: Option<String>,
    #[serde(default)]
    pub modified_on: Option<String>,
}

/// An R2 bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Bucket {
    pub name: String,
    #[serde(default)]
    pub creation_date: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
}

/// A DNS zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZone {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub name_servers: Vec<String>,
}

/// DNS record type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DnsRecordType {
    A,
    Aaaa,
    Cname,
    Txt,
    Mx,
    Ns,
    Srv,
}

impl std::fmt::Display for DnsRecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::Aaaa => write!(f, "AAAA"),
            Self::Cname => write!(f, "CNAME"),
            Self::Txt => write!(f, "TXT"),
            Self::Mx => write!(f, "MX"),
            Self::Ns => write!(f, "NS"),
            Self::Srv => write!(f, "SRV"),
        }
    }
}

/// A DNS record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    /// Record ID (set by Cloudflare on creation, optional in requests).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Record type (A, AAAA, CNAME, etc.).
    #[serde(rename = "type")]
    pub record_type: DnsRecordType,
    /// DNS name (e.g. "example.com").
    pub name: String,
    /// Record content / value.
    pub content: String,
    /// TTL in seconds (1 = automatic).
    #[serde(default = "default_ttl")]
    pub ttl: u32,
    /// Whether the record is proxied through Cloudflare.
    #[serde(default)]
    pub proxied: bool,
    /// Priority (for MX records).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u16>,
}

fn default_ttl() -> u32 {
    1
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// Cloudflare API v4 client covering Workers, R2, and DNS.
pub struct CloudflareClient {
    http: reqwest::Client,
    api_token: String,
    account_id: String,
}

impl CloudflareClient {
    /// Create a new Cloudflare client from the given config.
    ///
    /// The config is validated before constructing the client.
    pub fn new(config: CloudflareConfig) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_token: config.api_token,
            account_id: config.account_id,
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_token)
    }

    fn account_url(&self, path: &str) -> String {
        format!("{}/accounts/{}{}", CF_API_BASE, self.account_id, path)
    }

    fn zone_url(&self, path: &str) -> String {
        format!("{}{}", CF_API_BASE, path)
    }

    /// Parse a Cloudflare API response, converting errors into `InfraError`.
    fn parse_response<T: serde::de::DeserializeOwned>(
        status: reqwest::StatusCode,
        body: &str,
        context: &str,
    ) -> Result<T, InfraError> {
        let resp: CfApiResponse<T> = serde_json::from_str(body).map_err(|e| {
            InfraError::Http(format!(
                "cloudflare {context}: failed to parse response: {e}"
            ))
        })?;

        if !resp.success {
            let msg = resp
                .errors
                .iter()
                .map(|e| format!("[{}] {}", e.code, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(InfraError::ProvisioningFailed {
                resource: format!("cloudflare/{context}"),
                reason: msg,
            });
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(InfraError::ResourceNotFound {
                resource: format!("cloudflare/{context}"),
            });
        }

        resp.result.ok_or_else(|| {
            InfraError::Http(format!(
                "cloudflare {context}: response success=true but result is null"
            ))
        })
    }

    /// Parse a Cloudflare list response.
    fn parse_list_response<T: serde::de::DeserializeOwned>(
        body: &str,
        context: &str,
    ) -> Result<Vec<T>, InfraError> {
        let resp: CfListResponse<T> = serde_json::from_str(body).map_err(|e| {
            InfraError::Http(format!(
                "cloudflare {context}: failed to parse response: {e}"
            ))
        })?;

        if !resp.success {
            let msg = resp
                .errors
                .iter()
                .map(|e| format!("[{}] {}", e.code, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(InfraError::ProvisioningFailed {
                resource: format!("cloudflare/{context}"),
                reason: msg,
            });
        }

        Ok(resp.result)
    }

    // -----------------------------------------------------------------------
    // Workers
    // -----------------------------------------------------------------------

    /// List all Workers scripts in the account.
    pub async fn list_workers(&self) -> Result<Vec<Worker>, InfraError> {
        let url = self.account_url("/workers/scripts");
        debug!(url = %url, "listing cloudflare workers");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        let body = resp.text().await?;
        Self::parse_list_response::<Worker>(&body, "list_workers")
    }

    /// Deploy (create or update) a Worker script.
    pub async fn deploy_worker(&self, name: &str, script: &str) -> Result<Worker, InfraError> {
        let url = self.account_url(&format!("/workers/scripts/{name}"));
        info!(name = %name, "deploying cloudflare worker");

        let resp = self
            .http
            .put(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/javascript")
            .body(script.to_owned())
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        Self::parse_response::<Worker>(status, &body, "deploy_worker")
    }

    /// Delete a Worker script by name.
    pub async fn delete_worker(&self, name: &str) -> Result<(), InfraError> {
        let url = self.account_url(&format!("/workers/scripts/{name}"));
        info!(name = %name, "deleting cloudflare worker");

        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = resp.text().await?;
            warn!(name = %name, status = %status, "worker deletion failed");
            Err(InfraError::ProvisioningFailed {
                resource: format!("cloudflare/worker/{name}"),
                reason: body,
            })
        }
    }

    // -----------------------------------------------------------------------
    // R2 (S3-compatible object storage)
    // -----------------------------------------------------------------------

    /// Validate an R2 bucket name according to Cloudflare rules.
    pub fn validate_r2_bucket_name(name: &str) -> Result<(), InfraError> {
        if name.len() < 3 {
            return Err(InfraError::ProvisioningFailed {
                resource: "cloudflare/r2".into(),
                reason: "bucket name must be at least 3 characters".into(),
            });
        }
        if name.len() > 63 {
            return Err(InfraError::ProvisioningFailed {
                resource: "cloudflare/r2".into(),
                reason: "bucket name must be at most 63 characters".into(),
            });
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(InfraError::ProvisioningFailed {
                resource: "cloudflare/r2".into(),
                reason: "bucket name may only contain lowercase letters, digits, and hyphens"
                    .into(),
            });
        }
        if name.starts_with('-') || name.ends_with('-') {
            return Err(InfraError::ProvisioningFailed {
                resource: "cloudflare/r2".into(),
                reason: "bucket name must not start or end with a hyphen".into(),
            });
        }
        Ok(())
    }

    /// List all R2 buckets in the account.
    pub async fn list_r2_buckets(&self) -> Result<Vec<R2Bucket>, InfraError> {
        let url = self.account_url("/r2/buckets");
        debug!(url = %url, "listing r2 buckets");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        let body = resp.text().await?;

        // R2 list endpoint wraps in { buckets: [...] }
        #[derive(Deserialize)]
        struct R2ListResult {
            buckets: Vec<R2Bucket>,
        }

        let parsed: CfApiResponse<R2ListResult> = serde_json::from_str(&body).map_err(|e| {
            InfraError::Http(format!("cloudflare list_r2_buckets: parse error: {e}"))
        })?;

        if !parsed.success {
            let msg = parsed
                .errors
                .iter()
                .map(|e| format!("[{}] {}", e.code, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(InfraError::ProvisioningFailed {
                resource: "cloudflare/r2".into(),
                reason: msg,
            });
        }

        Ok(parsed.result.map(|r| r.buckets).unwrap_or_default())
    }

    /// Create a new R2 bucket.
    pub async fn create_r2_bucket(&self, name: &str) -> Result<R2Bucket, InfraError> {
        Self::validate_r2_bucket_name(name)?;

        let url = self.account_url("/r2/buckets");
        info!(name = %name, "creating r2 bucket");

        #[derive(Serialize)]
        struct CreateBucket<'a> {
            name: &'a str,
        }

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&CreateBucket { name })
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        Self::parse_response::<R2Bucket>(status, &body, "create_r2_bucket")
    }

    /// Delete an R2 bucket by name.
    pub async fn delete_r2_bucket(&self, name: &str) -> Result<(), InfraError> {
        let url = self.account_url(&format!("/r2/buckets/{name}"));
        info!(name = %name, "deleting r2 bucket");

        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = resp.text().await?;
            warn!(name = %name, status = %status, "r2 bucket deletion failed");
            Err(InfraError::ProvisioningFailed {
                resource: format!("cloudflare/r2/{name}"),
                reason: body,
            })
        }
    }

    // -----------------------------------------------------------------------
    // DNS
    // -----------------------------------------------------------------------

    /// List all DNS zones accessible to this account.
    pub async fn list_zones(&self) -> Result<Vec<DnsZone>, InfraError> {
        let url = self.zone_url("/zones");
        debug!(url = %url, "listing dns zones");

        let resp = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        let body = resp.text().await?;
        Self::parse_list_response::<DnsZone>(&body, "list_zones")
    }

    /// Create a DNS record in the given zone.
    pub async fn create_dns_record(
        &self,
        zone_id: &str,
        record: DnsRecord,
    ) -> Result<DnsRecord, InfraError> {
        let url = self.zone_url(&format!("/zones/{zone_id}/dns_records"));
        info!(
            zone_id = %zone_id,
            record_type = %record.record_type,
            name = %record.name,
            "creating dns record"
        );

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&record)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        Self::parse_response::<DnsRecord>(status, &body, "create_dns_record")
    }

    /// Delete a DNS record from the given zone.
    pub async fn delete_dns_record(
        &self,
        zone_id: &str,
        record_id: &str,
    ) -> Result<(), InfraError> {
        let url = self.zone_url(&format!("/zones/{zone_id}/dns_records/{record_id}"));
        info!(
            zone_id = %zone_id,
            record_id = %record_id,
            "deleting dns record"
        );

        let resp = self
            .http
            .delete(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            let body = resp.text().await?;
            warn!(zone_id = %zone_id, record_id = %record_id, status = %status, "dns record deletion failed");
            Err(InfraError::ProvisioningFailed {
                resource: format!("cloudflare/dns/{zone_id}/{record_id}"),
                reason: body,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Config validation --------------------------------------------------

    #[test]
    fn test_config_validate_success() {
        let cfg = CloudflareConfig {
            api_token: "test-token-abc123".into(),
            account_id: "abcdef0123456789abcdef0123456789".into(),
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_token() {
        let cfg = CloudflareConfig {
            api_token: "".into(),
            account_id: "abcdef0123456789abcdef0123456789".into(),
        };
        let err = cfg.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("authentication required"), "got: {msg}");
    }

    #[test]
    fn test_config_validate_empty_account_id() {
        let cfg = CloudflareConfig {
            api_token: "some-token".into(),
            account_id: "".into(),
        };
        let err = cfg.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("account_id is required"), "got: {msg}");
    }

    #[test]
    fn test_config_validate_non_hex_account_id() {
        let cfg = CloudflareConfig {
            api_token: "some-token".into(),
            account_id: "not-a-hex-string!!!".into(),
        };
        let err = cfg.validate().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("hex"), "got: {msg}");
    }

    // -- API response deserialization ---------------------------------------

    #[test]
    fn test_deserialize_worker_response() {
        let json = r#"{
            "success": true,
            "errors": [],
            "messages": [],
            "result": {
                "id": "worker-123",
                "name": "my-worker",
                "created_on": "2024-01-01T00:00:00Z",
                "modified_on": "2024-06-15T12:30:00Z"
            }
        }"#;
        let resp: CfApiResponse<Worker> = serde_json::from_str(json).unwrap();
        assert!(resp.success);
        let worker = resp.result.unwrap();
        assert_eq!(worker.id, "worker-123");
        assert_eq!(worker.name, "my-worker");
        assert_eq!(worker.created_on.as_deref(), Some("2024-01-01T00:00:00Z"));
    }

    #[test]
    fn test_deserialize_error_response() {
        let json = r#"{
            "success": false,
            "errors": [{"code": 10000, "message": "Authentication error"}],
            "messages": [],
            "result": null
        }"#;
        let resp: CfApiResponse<Worker> = serde_json::from_str(json).unwrap();
        assert!(!resp.success);
        assert_eq!(resp.errors.len(), 1);
        assert_eq!(resp.errors[0].code, 10000);
        assert_eq!(resp.errors[0].message, "Authentication error");
    }

    #[test]
    fn test_parse_response_api_error() {
        let json = r#"{
            "success": false,
            "errors": [{"code": 7003, "message": "Could not route to /bad/path"}],
            "messages": [],
            "result": null
        }"#;
        let result = CloudflareClient::parse_response::<Worker>(
            reqwest::StatusCode::BAD_REQUEST,
            json,
            "test",
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("7003"), "got: {msg}");
        assert!(msg.contains("Could not route"), "got: {msg}");
    }

    // -- Worker script handling ---------------------------------------------

    #[test]
    fn test_worker_serde_roundtrip() {
        let worker = Worker {
            id: "w-abc".into(),
            name: "handler".into(),
            created_on: Some("2025-01-01T00:00:00Z".into()),
            modified_on: None,
        };
        let json = serde_json::to_string(&worker).unwrap();
        let decoded: Worker = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "w-abc");
        assert_eq!(decoded.name, "handler");
        assert!(decoded.modified_on.is_none());
    }

    // -- R2 bucket naming validation ----------------------------------------

    #[test]
    fn test_r2_bucket_name_valid() {
        assert!(CloudflareClient::validate_r2_bucket_name("my-bucket-123").is_ok());
        assert!(CloudflareClient::validate_r2_bucket_name("abc").is_ok());
        assert!(CloudflareClient::validate_r2_bucket_name("a0b").is_ok());
    }

    #[test]
    fn test_r2_bucket_name_too_short() {
        let err = CloudflareClient::validate_r2_bucket_name("ab").unwrap_err();
        assert!(err.to_string().contains("at least 3"), "got: {err}");
    }

    #[test]
    fn test_r2_bucket_name_too_long() {
        let long = "a".repeat(64);
        let err = CloudflareClient::validate_r2_bucket_name(&long).unwrap_err();
        assert!(err.to_string().contains("at most 63"), "got: {err}");
    }

    #[test]
    fn test_r2_bucket_name_invalid_chars() {
        let err = CloudflareClient::validate_r2_bucket_name("My_Bucket").unwrap_err();
        assert!(err.to_string().contains("lowercase"), "got: {err}");
    }

    #[test]
    fn test_r2_bucket_name_hyphen_edges() {
        let err = CloudflareClient::validate_r2_bucket_name("-bucket").unwrap_err();
        assert!(err.to_string().contains("hyphen"), "got: {err}");
        let err2 = CloudflareClient::validate_r2_bucket_name("bucket-").unwrap_err();
        assert!(err2.to_string().contains("hyphen"), "got: {err2}");
    }

    // -- DNS record types ---------------------------------------------------

    #[test]
    fn test_dns_record_type_display() {
        assert_eq!(DnsRecordType::A.to_string(), "A");
        assert_eq!(DnsRecordType::Aaaa.to_string(), "AAAA");
        assert_eq!(DnsRecordType::Cname.to_string(), "CNAME");
        assert_eq!(DnsRecordType::Txt.to_string(), "TXT");
        assert_eq!(DnsRecordType::Mx.to_string(), "MX");
    }

    #[test]
    fn test_dns_record_serde() {
        let record = DnsRecord {
            id: None,
            record_type: DnsRecordType::A,
            name: "example.com".into(),
            content: "93.184.216.34".into(),
            ttl: 300,
            proxied: true,
            priority: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"type\":\"A\""), "got: {json}");
        assert!(!json.contains("\"id\""), "id should be skipped: {json}");
        assert!(
            !json.contains("\"priority\""),
            "priority should be skipped: {json}"
        );

        let decoded: DnsRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.record_type, DnsRecordType::A);
        assert_eq!(decoded.name, "example.com");
        assert_eq!(decoded.ttl, 300);
        assert!(decoded.proxied);
    }

    #[test]
    fn test_dns_record_mx_with_priority() {
        let record = DnsRecord {
            id: Some("rec-123".into()),
            record_type: DnsRecordType::Mx,
            name: "example.com".into(),
            content: "mail.example.com".into(),
            ttl: 1,
            proxied: false,
            priority: Some(10),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"priority\":10"), "got: {json}");
        assert!(json.contains("\"MX\""), "got: {json}");

        let decoded: DnsRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.priority, Some(10));
    }

    #[test]
    fn test_dns_zone_deserialization() {
        let json = r#"{
            "id": "zone-abc",
            "name": "example.com",
            "status": "active",
            "name_servers": ["ns1.cloudflare.com", "ns2.cloudflare.com"]
        }"#;
        let zone: DnsZone = serde_json::from_str(json).unwrap();
        assert_eq!(zone.id, "zone-abc");
        assert_eq!(zone.name, "example.com");
        assert_eq!(zone.status, "active");
        assert_eq!(zone.name_servers.len(), 2);
    }

    #[test]
    fn test_r2_bucket_deserialization() {
        let json = r#"{
            "name": "my-data-bucket",
            "creation_date": "2024-03-01T10:00:00Z",
            "location": "WNAM"
        }"#;
        let bucket: R2Bucket = serde_json::from_str(json).unwrap();
        assert_eq!(bucket.name, "my-data-bucket");
        assert_eq!(
            bucket.creation_date.as_deref(),
            Some("2024-03-01T10:00:00Z")
        );
        assert_eq!(bucket.location.as_deref(), Some("WNAM"));
    }

    #[test]
    fn test_client_url_construction() {
        let client = CloudflareClient::new(CloudflareConfig {
            api_token: "tok".into(),
            account_id: "abc123".into(),
        });
        assert_eq!(
            client.account_url("/workers/scripts"),
            "https://api.cloudflare.com/client/v4/accounts/abc123/workers/scripts"
        );
        assert_eq!(
            client.zone_url("/zones"),
            "https://api.cloudflare.com/client/v4/zones"
        );
    }
}
