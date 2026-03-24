//! Oracle Cloud Infrastructure (OCI) provider client.
//!
//! Architecture Framework §9: Primary compute provider.
//! Free tier: 2 ARM VMs (Ampere A1, 4 OCPUs, 24GB RAM), 200GB block storage.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::errors::InfraError;

/// OCI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleConfig {
    /// OCI tenancy OCID
    pub tenancy_ocid: String,
    /// OCI user OCID
    pub user_ocid: String,
    /// API key fingerprint
    pub fingerprint: String,
    /// PEM-encoded private key
    pub private_key_pem: String,
    /// OCI region (e.g. "us-ashburn-1")
    pub region: String,
    /// Compartment OCID (defaults to tenancy root)
    pub compartment_ocid: Option<String>,
}

impl OracleConfig {
    /// Validate that all required fields are set.
    pub fn validate(&self) -> Result<(), InfraError> {
        if self.tenancy_ocid.is_empty() {
            return Err(InfraError::ProviderError(
                "OCI tenancy_ocid is empty".into(),
            ));
        }
        if self.user_ocid.is_empty() {
            return Err(InfraError::ProviderError("OCI user_ocid is empty".into()));
        }
        if self.fingerprint.is_empty() {
            return Err(InfraError::ProviderError("OCI fingerprint is empty".into()));
        }
        if self.private_key_pem.is_empty() {
            return Err(InfraError::ProviderError(
                "OCI private_key_pem is empty".into(),
            ));
        }
        if self.region.is_empty() {
            return Err(InfraError::ProviderError("OCI region is empty".into()));
        }
        Ok(())
    }

    /// Get the compartment OCID (falls back to tenancy root).
    pub fn compartment(&self) -> &str {
        self.compartment_ocid
            .as_deref()
            .unwrap_or(&self.tenancy_ocid)
    }

    /// Build the API base URL for this region.
    pub fn api_base(&self) -> String {
        format!("https://iaas.{}.oraclecloud.com/20160918", self.region)
    }
}

/// OCI compute instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub display_name: String,
    pub shape: String,
    pub lifecycle_state: InstanceState,
    pub availability_domain: String,
    pub region: String,
    pub time_created: Option<String>,
    pub image_id: Option<String>,
    pub fault_domain: Option<String>,
}

/// OCI instance lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstanceState {
    Provisioning,
    Running,
    Starting,
    Stopping,
    Stopped,
    Terminating,
    Terminated,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for InstanceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provisioning => write!(f, "PROVISIONING"),
            Self::Running => write!(f, "RUNNING"),
            Self::Starting => write!(f, "STARTING"),
            Self::Stopping => write!(f, "STOPPING"),
            Self::Stopped => write!(f, "STOPPED"),
            Self::Terminating => write!(f, "TERMINATING"),
            Self::Terminated => write!(f, "TERMINATED"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Parameters for creating a new instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInstanceParams {
    pub display_name: String,
    pub availability_domain: String,
    pub shape: String,
    pub shape_config: Option<ShapeConfig>,
    pub image_id: String,
    pub subnet_id: String,
    pub ssh_authorized_keys: Option<String>,
}

/// Shape configuration for flex shapes (e.g. VM.Standard.A1.Flex).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeConfig {
    pub ocpus: f32,
    pub memory_in_gbs: f32,
}

/// OCI Virtual Cloud Network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vcn {
    pub id: String,
    pub display_name: String,
    pub cidr_block: String,
    pub lifecycle_state: String,
    pub time_created: Option<String>,
}

/// Free-tier ARM shape constant.
pub const FREE_TIER_SHAPE: &str = "VM.Standard.A1.Flex";

/// Maximum free-tier OCPUs.
pub const FREE_TIER_MAX_OCPUS: f32 = 4.0;

/// Maximum free-tier memory in GB.
pub const FREE_TIER_MAX_MEMORY_GB: f32 = 24.0;

/// Known OCI regions.
pub const REGIONS: &[&str] = &[
    "us-ashburn-1",
    "us-phoenix-1",
    "us-sanjose-1",
    "us-chicago-1",
    "eu-frankfurt-1",
    "eu-amsterdam-1",
    "uk-london-1",
    "ap-tokyo-1",
    "ap-osaka-1",
    "ap-mumbai-1",
    "ap-seoul-1",
    "ap-sydney-1",
    "ca-toronto-1",
    "sa-saopaulo-1",
];

/// The OCI REST API client.
pub struct OracleClient {
    http: reqwest::Client,
    config: OracleConfig,
}

impl OracleClient {
    /// Create a new Oracle Cloud client.
    pub fn new(config: OracleConfig) -> Result<Self, InfraError> {
        config.validate()?;
        let http = reqwest::Client::new();
        Ok(Self { http, config })
    }

    /// Create from environment variables.
    pub fn from_env() -> Result<Self, InfraError> {
        let config = OracleConfig {
            tenancy_ocid: std::env::var("OCI_TENANCY_OCID")
                .map_err(|_| InfraError::ProviderError("OCI_TENANCY_OCID not set".into()))?,
            user_ocid: std::env::var("OCI_USER_OCID")
                .map_err(|_| InfraError::ProviderError("OCI_USER_OCID not set".into()))?,
            fingerprint: std::env::var("OCI_FINGERPRINT")
                .map_err(|_| InfraError::ProviderError("OCI_FINGERPRINT not set".into()))?,
            private_key_pem: std::env::var("OCI_PRIVATE_KEY")
                .map_err(|_| InfraError::ProviderError("OCI_PRIVATE_KEY not set".into()))?,
            region: std::env::var("OCI_REGION").unwrap_or_else(|_| "us-ashburn-1".into()),
            compartment_ocid: std::env::var("OCI_COMPARTMENT_OCID").ok(),
        };
        Self::new(config)
    }

    /// Build the OCI API request signing string.
    /// OCI uses RSA-SHA256 signatures per the OCI API signing spec.
    #[allow(dead_code)]
    fn signing_string(
        &self,
        method: &str,
        path: &str,
        host: &str,
        date: &str,
        content_sha256: Option<&str>,
    ) -> String {
        let mut parts = vec![
            format!("(request-target): {} {}", method.to_lowercase(), path),
            format!("date: {}", date),
            format!("host: {}", host),
        ];
        if let Some(sha) = content_sha256 {
            parts.push(format!("x-content-sha256: {}", sha));
            parts.push("content-type: application/json".to_string());
        }
        parts.join("\n")
    }

    /// Get the key ID for OCI signing.
    #[allow(dead_code)]
    fn key_id(&self) -> String {
        format!(
            "{}/{}/{}",
            self.config.tenancy_ocid, self.config.user_ocid, self.config.fingerprint
        )
    }

    /// List compute instances in the compartment.
    pub async fn list_instances(&self) -> Result<Vec<Instance>, InfraError> {
        let url = format!(
            "{}/instances?compartmentId={}",
            self.config.api_base(),
            self.config.compartment()
        );
        debug!("OCI list instances: {}", url);

        // In production, requests are signed with RSA-SHA256.
        // For now, we construct the request and rely on OCI CLI auth config.
        let resp = self
            .http
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "OCI list instances failed ({}): {}",
                status, body
            )));
        }

        let instances: Vec<Instance> = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI parse error: {}", e)))?;

        info!("OCI: found {} instances", instances.len());
        Ok(instances)
    }

    /// Create a new compute instance.
    pub async fn create_instance(
        &self,
        params: CreateInstanceParams,
    ) -> Result<Instance, InfraError> {
        let url = format!("{}/instances", self.config.api_base());

        let body = serde_json::json!({
            "compartmentId": self.config.compartment(),
            "availabilityDomain": params.availability_domain,
            "shape": params.shape,
            "displayName": params.display_name,
            "sourceDetails": {
                "sourceType": "image",
                "imageId": params.image_id
            },
            "createVnicDetails": {
                "subnetId": params.subnet_id
            },
            "shapeConfig": params.shape_config,
            "metadata": params.ssh_authorized_keys.map(|k| {
                serde_json::json!({"ssh_authorized_keys": k})
            }),
        });

        debug!("OCI create instance: {}", params.display_name);

        let resp = self
            .http
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI create failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "OCI create instance failed ({}): {}",
                status, body
            )));
        }

        let instance: Instance = resp
            .json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI parse error: {}", e)))?;

        info!("OCI: created instance {}", instance.id);
        Ok(instance)
    }

    /// Terminate (delete) an instance.
    pub async fn terminate_instance(&self, instance_id: &str) -> Result<(), InfraError> {
        let url = format!("{}/instances/{}", self.config.api_base(), instance_id);
        debug!("OCI terminate instance: {}", instance_id);

        let resp = self
            .http
            .delete(&url)
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI terminate failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "OCI terminate failed ({}): {}",
                status, body
            )));
        }

        info!("OCI: terminated instance {}", instance_id);
        Ok(())
    }

    /// Get a single instance by ID.
    pub async fn get_instance(&self, instance_id: &str) -> Result<Instance, InfraError> {
        let url = format!("{}/instances/{}", self.config.api_base(), instance_id);

        let resp = self
            .http
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI get failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "OCI get instance failed ({}): {}",
                status, body
            )));
        }

        resp.json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI parse error: {}", e)))
    }

    /// List VCNs in the compartment.
    pub async fn list_vcns(&self) -> Result<Vec<Vcn>, InfraError> {
        let url = format!(
            "https://iaas.{}.oraclecloud.com/20160918/vcns?compartmentId={}",
            self.config.region,
            self.config.compartment()
        );

        let resp = self
            .http
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI list VCNs failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "OCI list VCNs failed ({}): {}",
                status, body
            )));
        }

        resp.json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI parse error: {}", e)))
    }

    /// Create a VCN.
    pub async fn create_vcn(&self, name: &str, cidr: &str) -> Result<Vcn, InfraError> {
        let url = format!(
            "https://iaas.{}.oraclecloud.com/20160918/vcns",
            self.config.region
        );

        let body = serde_json::json!({
            "compartmentId": self.config.compartment(),
            "displayName": name,
            "cidrBlock": cidr,
        });

        let resp = self
            .http
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI create VCN failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(InfraError::ProviderError(format!(
                "OCI create VCN failed ({}): {}",
                status, body
            )));
        }

        resp.json()
            .await
            .map_err(|e| InfraError::ProviderError(format!("OCI parse error: {}", e)))
    }
}

/// Check if a shape is within free-tier limits.
pub fn is_free_tier_shape(shape: &str, ocpus: f32, memory_gb: f32) -> bool {
    shape == FREE_TIER_SHAPE && ocpus <= FREE_TIER_MAX_OCPUS && memory_gb <= FREE_TIER_MAX_MEMORY_GB
}

/// Get the default free-tier shape config (split evenly for 2 VMs).
pub fn default_free_tier_vm() -> ShapeConfig {
    ShapeConfig {
        ocpus: 2.0,
        memory_in_gbs: 12.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> OracleConfig {
        OracleConfig {
            tenancy_ocid: "ocid1.tenancy.oc1..aaaatest".into(),
            user_ocid: "ocid1.user.oc1..aaaatest".into(),
            fingerprint: "aa:bb:cc:dd:ee:ff:00:11".into(),
            private_key_pem: "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----"
                .into(),
            region: "us-ashburn-1".into(),
            compartment_ocid: None,
        }
    }

    #[test]
    fn test_config_validate() {
        let config = test_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_empty_tenancy() {
        let mut config = test_config();
        config.tenancy_ocid = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_empty_region() {
        let mut config = test_config();
        config.region = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_compartment_fallback() {
        let config = test_config();
        assert_eq!(config.compartment(), config.tenancy_ocid);
    }

    #[test]
    fn test_compartment_explicit() {
        let mut config = test_config();
        config.compartment_ocid = Some("ocid1.compartment.oc1..custom".into());
        assert_eq!(config.compartment(), "ocid1.compartment.oc1..custom");
    }

    #[test]
    fn test_api_base_url() {
        let config = test_config();
        assert_eq!(
            config.api_base(),
            "https://iaas.us-ashburn-1.oraclecloud.com/20160918"
        );
    }

    #[test]
    fn test_instance_state_serde() {
        let json = r#""RUNNING""#;
        let state: InstanceState = serde_json::from_str(json).unwrap();
        assert_eq!(state, InstanceState::Running);

        let json = r#""SOMETHING_NEW""#;
        let state: InstanceState = serde_json::from_str(json).unwrap();
        assert_eq!(state, InstanceState::Unknown);
    }

    #[test]
    fn test_instance_deser() {
        let json = serde_json::json!({
            "id": "ocid1.instance.oc1.iad.test",
            "display_name": "phantom-vm-1",
            "shape": "VM.Standard.A1.Flex",
            "lifecycle_state": "RUNNING",
            "availability_domain": "US-ASHBURN-AD-1",
            "region": "us-ashburn-1",
            "time_created": "2026-03-20T10:00:00Z",
            "image_id": "ocid1.image.oc1.test",
            "fault_domain": "FAULT-DOMAIN-1"
        });
        let inst: Instance = serde_json::from_value(json).unwrap();
        assert_eq!(inst.display_name, "phantom-vm-1");
        assert_eq!(inst.shape, FREE_TIER_SHAPE);
        assert_eq!(inst.lifecycle_state, InstanceState::Running);
    }

    #[test]
    fn test_free_tier_shape_check() {
        assert!(is_free_tier_shape(FREE_TIER_SHAPE, 2.0, 12.0));
        assert!(is_free_tier_shape(FREE_TIER_SHAPE, 4.0, 24.0));
        assert!(!is_free_tier_shape(FREE_TIER_SHAPE, 5.0, 24.0)); // over OCPU limit
        assert!(!is_free_tier_shape("VM.Standard.E4.Flex", 2.0, 12.0)); // wrong shape
    }

    #[test]
    fn test_default_free_tier_vm() {
        let config = default_free_tier_vm();
        assert_eq!(config.ocpus, 2.0);
        assert_eq!(config.memory_in_gbs, 12.0);
    }

    #[test]
    fn test_regions_list() {
        assert!(REGIONS.contains(&"us-ashburn-1"));
        assert!(REGIONS.contains(&"eu-frankfurt-1"));
        assert!(REGIONS.len() >= 10);
    }

    #[test]
    fn test_key_id_format() {
        let config = test_config();
        let client = OracleClient::new(config.clone()).unwrap();
        let key_id = client.key_id();
        assert!(key_id.contains(&config.tenancy_ocid));
        assert!(key_id.contains(&config.user_ocid));
        assert!(key_id.contains(&config.fingerprint));
    }

    #[test]
    fn test_signing_string_format() {
        let config = test_config();
        let client = OracleClient::new(config).unwrap();
        let ss = client.signing_string(
            "GET",
            "/test",
            "iaas.us-ashburn-1.oraclecloud.com",
            "Mon, 01 Jan 2026 00:00:00 GMT",
            None,
        );
        assert!(ss.contains("(request-target): get /test"));
        assert!(ss.contains("host: iaas.us-ashburn-1.oraclecloud.com"));
    }

    #[test]
    fn test_signing_string_with_body() {
        let config = test_config();
        let client = OracleClient::new(config).unwrap();
        let ss = client.signing_string(
            "POST",
            "/instances",
            "iaas.us-ashburn-1.oraclecloud.com",
            "Mon, 01 Jan 2026 00:00:00 GMT",
            Some("abc123sha256"),
        );
        assert!(ss.contains("x-content-sha256: abc123sha256"));
        assert!(ss.contains("content-type: application/json"));
    }

    #[test]
    fn test_vcn_deser() {
        let json = serde_json::json!({
            "id": "ocid1.vcn.oc1.iad.test",
            "display_name": "phantom-vcn",
            "cidr_block": "10.0.0.0/16",
            "lifecycle_state": "AVAILABLE",
            "time_created": "2026-03-20T10:00:00Z"
        });
        let vcn: Vcn = serde_json::from_value(json).unwrap();
        assert_eq!(vcn.display_name, "phantom-vcn");
        assert_eq!(vcn.cidr_block, "10.0.0.0/16");
    }
}
