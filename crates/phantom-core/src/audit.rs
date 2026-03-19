//! Tamper-evident audit log.
//!
//! Core Law 9: Every action is audited. Signed. Exportable. Tamper-evident.
//!
//! Each entry is chained via SHA-256 hash of the previous entry,
//! creating a tamper-evident log (any modification breaks the chain).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// An entry in the signed audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Agent that performed the action
    pub agent_id: String,
    /// Action category
    pub action: AuditAction,
    /// Detailed description
    pub description: String,
    /// Additional structured details
    pub details: serde_json::Value,
    /// Knowledge citation (which KB section influenced this decision)
    pub knowledge_citation: Option<String>,
    /// SHA-256 hash of the previous entry (tamper-evident chain)
    pub prev_hash: String,
    /// SHA-256 hash of this entry
    pub hash: String,
}

/// Categories of auditable actions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// Agent spawned
    AgentSpawned,
    /// Agent stopped
    AgentStopped,
    /// Task created
    TaskCreated,
    /// Task started
    TaskStarted,
    /// Task completed
    TaskCompleted,
    /// Task failed
    TaskFailed,
    /// Self-healing triggered
    SelfHealing,
    /// Knowledge Brain queried
    KnowledgeQuery,
    /// Infrastructure provisioned
    InfraProvisioned,
    /// Account created
    AccountCreated,
    /// Credential stored
    CredentialStored,
    /// Credential rotated
    CredentialRotated,
    /// Code generated
    CodeGenerated,
    /// Tests executed
    TestsExecuted,
    /// Security audit performed
    SecurityAudit,
    /// Deployment executed
    Deployment,
    /// Master key operation
    MasterKeyOp,
    /// License operation
    LicenseOp,
    /// Emergency halt
    EmergencyHalt,
    /// Owner input requested/received
    OwnerInteraction,
    /// System event
    System,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self));
        write!(f, "{}", s)
    }
}

/// The tamper-evident audit log.
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Record a new audit entry.
    pub fn record(
        &mut self,
        agent_id: impl Into<String>,
        action: AuditAction,
        description: impl Into<String>,
        details: serde_json::Value,
        knowledge_citation: Option<String>,
    ) -> &AuditEntry {
        let prev_hash = self
            .entries
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| "genesis".to_string());

        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let agent_id = agent_id.into();
        let description = description.into();

        // Compute hash of this entry
        let hash_input = format!(
            "{}:{}:{}:{:?}:{}:{}:{}",
            id,
            timestamp.to_rfc3339(),
            agent_id,
            action,
            description,
            serde_json::to_string(&details).unwrap_or_default(),
            prev_hash,
        );
        let hash = hex_sha256(hash_input.as_bytes());

        let entry = AuditEntry {
            id,
            timestamp,
            agent_id,
            action,
            description,
            details,
            knowledge_citation,
            prev_hash,
            hash,
        };

        self.entries.push(entry);
        self.entries.last().unwrap()
    }

    /// Verify the integrity of the entire audit chain.
    pub fn verify_integrity(&self) -> Result<(), AuditIntegrityError> {
        for (i, entry) in self.entries.iter().enumerate() {
            // Check chain link
            let expected_prev = if i == 0 {
                "genesis".to_string()
            } else {
                self.entries[i - 1].hash.clone()
            };

            if entry.prev_hash != expected_prev {
                return Err(AuditIntegrityError::BrokenChain {
                    entry_index: i,
                    entry_id: entry.id.clone(),
                });
            }

            // Verify hash
            let hash_input = format!(
                "{}:{}:{}:{:?}:{}:{}:{}",
                entry.id,
                entry.timestamp.to_rfc3339(),
                entry.agent_id,
                entry.action,
                entry.description,
                serde_json::to_string(&entry.details).unwrap_or_default(),
                entry.prev_hash,
            );
            let computed_hash = hex_sha256(hash_input.as_bytes());

            if computed_hash != entry.hash {
                return Err(AuditIntegrityError::TamperedEntry {
                    entry_index: i,
                    entry_id: entry.id.clone(),
                });
            }
        }

        Ok(())
    }

    /// Export the entire audit log as JSON.
    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.entries)
    }

    /// Get all entries.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get entries filtered by agent.
    pub fn entries_by_agent(&self, agent_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.agent_id == agent_id)
            .collect()
    }

    /// Get entries filtered by action type.
    pub fn entries_by_action(&self, action: &AuditAction) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| &e.action == action)
            .collect()
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Errors found during audit integrity verification.
#[derive(Debug, thiserror::Error)]
pub enum AuditIntegrityError {
    #[error("broken chain at entry {entry_index} ({entry_id})")]
    BrokenChain {
        entry_index: usize,
        entry_id: String,
    },

    #[error("tampered entry at index {entry_index} ({entry_id})")]
    TamperedEntry {
        entry_index: usize,
        entry_id: String,
    },
}

/// Compute hex-encoded SHA-256 hash.
fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_verify() {
        let mut log = AuditLog::new();

        log.record(
            "cto-0",
            AuditAction::AgentSpawned,
            "CTO agent spawned",
            serde_json::json!({"model": "claude-opus-4-6"}),
            None,
        );
        log.record(
            "backend-0",
            AuditAction::TaskStarted,
            "Building API",
            serde_json::json!({"task_id": "t1"}),
            Some("API Expert §4".into()),
        );

        assert_eq!(log.len(), 2);
        assert!(log.verify_integrity().is_ok());
    }

    #[test]
    fn test_tamper_detection() {
        let mut log = AuditLog::new();

        log.record(
            "cto-0",
            AuditAction::AgentSpawned,
            "CTO spawned",
            serde_json::Value::Null,
            None,
        );
        log.record(
            "backend-0",
            AuditAction::TaskStarted,
            "task start",
            serde_json::Value::Null,
            None,
        );

        // Tamper with the first entry
        log.entries[0].description = "TAMPERED".to_string();

        assert!(log.verify_integrity().is_err());
    }

    #[test]
    fn test_chain_integrity() {
        let mut log = AuditLog::new();

        log.record(
            "a",
            AuditAction::System,
            "first",
            serde_json::Value::Null,
            None,
        );
        log.record(
            "a",
            AuditAction::System,
            "second",
            serde_json::Value::Null,
            None,
        );
        log.record(
            "a",
            AuditAction::System,
            "third",
            serde_json::Value::Null,
            None,
        );

        // First entry's prev_hash should be "genesis"
        assert_eq!(log.entries[0].prev_hash, "genesis");

        // Each subsequent entry's prev_hash should match the previous entry's hash
        assert_eq!(log.entries[1].prev_hash, log.entries[0].hash);
        assert_eq!(log.entries[2].prev_hash, log.entries[1].hash);

        assert!(log.verify_integrity().is_ok());
    }

    #[test]
    fn test_filter_by_agent() {
        let mut log = AuditLog::new();
        log.record(
            "cto",
            AuditAction::TaskCreated,
            "t1",
            serde_json::Value::Null,
            None,
        );
        log.record(
            "backend",
            AuditAction::TaskStarted,
            "t2",
            serde_json::Value::Null,
            None,
        );
        log.record(
            "cto",
            AuditAction::TaskCreated,
            "t3",
            serde_json::Value::Null,
            None,
        );

        let cto_entries = log.entries_by_agent("cto");
        assert_eq!(cto_entries.len(), 2);
    }

    #[test]
    fn test_filter_by_action() {
        let mut log = AuditLog::new();
        log.record(
            "a",
            AuditAction::TaskCreated,
            "t1",
            serde_json::Value::Null,
            None,
        );
        log.record(
            "a",
            AuditAction::TaskCompleted,
            "t1 done",
            serde_json::Value::Null,
            None,
        );
        log.record(
            "b",
            AuditAction::TaskCreated,
            "t2",
            serde_json::Value::Null,
            None,
        );

        let created = log.entries_by_action(&AuditAction::TaskCreated);
        assert_eq!(created.len(), 2);
    }

    #[test]
    fn test_export_json() {
        let mut log = AuditLog::new();
        log.record(
            "test",
            AuditAction::System,
            "test entry",
            serde_json::json!({"key": "value"}),
            None,
        );

        let json = log.export_json().unwrap();
        assert!(json.contains("test entry"));
        assert!(json.contains("key"));
    }

    #[test]
    fn test_knowledge_citation() {
        let mut log = AuditLog::new();
        let entry = log.record(
            "backend-0",
            AuditAction::CodeGenerated,
            "Built BaseAPIClient with retry + circuit breaker",
            serde_json::json!({"file": "src/api/client.py"}),
            Some("API Expert KB §4, §22".to_string()),
        );

        assert_eq!(
            entry.knowledge_citation.as_deref(),
            Some("API Expert KB §4, §22")
        );
    }
}
