//! Session keys and agent-scoped keys.
//! Session keys are derived from license material and are ephemeral (in-memory only).
//! Agent keys are further derived from session keys with per-agent, per-task scoping.

use crate::hkdf_keys;
use crate::CryptoError;
use zeroize::Zeroize;

// ── Agent Permissions (bitflags) ───────────────────────────────────────────

/// Permissions that control what an agent can do.
/// Enforced at tool-execution time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentPermissions(u32);

impl AgentPermissions {
    pub const READ_CODE: Self = Self(1 << 0);
    pub const WRITE_CODE: Self = Self(1 << 1);
    pub const READ_CREDENTIALS: Self = Self(1 << 2);
    pub const WRITE_CREDENTIALS: Self = Self(1 << 3);
    pub const DEPLOY: Self = Self(1 << 4);
    pub const PROVISION: Self = Self(1 << 5);
    pub const SPAWN_AGENTS: Self = Self(1 << 6);
    pub const SHELL_EXEC: Self = Self(1 << 7);
    pub const HTTP_REQUEST: Self = Self(1 << 8);
    pub const DB_QUERY: Self = Self(1 << 9);
    pub const AUDIT_READ: Self = Self(1 << 10);
    pub const AUDIT_WRITE: Self = Self(1 << 11);
    pub const KNOWLEDGE_READ: Self = Self(1 << 12);
    pub const KNOWLEDGE_WRITE: Self = Self(1 << 13);
    pub const HALT_AGENTS: Self = Self(1 << 14);
    pub const DESTROY: Self = Self(1 << 15);

    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(0xFFFF);

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn bits(self) -> u32 {
        self.0
    }

    pub fn from_bits(bits: u32) -> Self {
        Self(bits)
    }
}

/// Predefined permission sets per agent role (Section 8.2).
pub fn permissions_for_role(role: &str) -> AgentPermissions {
    match role {
        "cto" => AgentPermissions::ALL,
        "architect" => AgentPermissions::READ_CODE
            .union(AgentPermissions::WRITE_CODE)
            .union(AgentPermissions::KNOWLEDGE_READ)
            .union(AgentPermissions::SPAWN_AGENTS)
            .union(AgentPermissions::AUDIT_WRITE),
        "backend" => AgentPermissions::READ_CODE
            .union(AgentPermissions::WRITE_CODE)
            .union(AgentPermissions::SHELL_EXEC)
            .union(AgentPermissions::DB_QUERY)
            .union(AgentPermissions::HTTP_REQUEST)
            .union(AgentPermissions::KNOWLEDGE_READ)
            .union(AgentPermissions::AUDIT_WRITE),
        "frontend" => AgentPermissions::READ_CODE
            .union(AgentPermissions::WRITE_CODE)
            .union(AgentPermissions::SHELL_EXEC)
            .union(AgentPermissions::HTTP_REQUEST)
            .union(AgentPermissions::KNOWLEDGE_READ)
            .union(AgentPermissions::AUDIT_WRITE),
        "devops" => AgentPermissions::READ_CODE
            .union(AgentPermissions::WRITE_CODE)
            .union(AgentPermissions::SHELL_EXEC)
            .union(AgentPermissions::DEPLOY)
            .union(AgentPermissions::PROVISION)
            .union(AgentPermissions::READ_CREDENTIALS)
            .union(AgentPermissions::HTTP_REQUEST)
            .union(AgentPermissions::KNOWLEDGE_READ)
            .union(AgentPermissions::AUDIT_WRITE),
        "qa" => AgentPermissions::READ_CODE
            .union(AgentPermissions::WRITE_CODE)
            .union(AgentPermissions::SHELL_EXEC)
            .union(AgentPermissions::DB_QUERY)
            .union(AgentPermissions::KNOWLEDGE_READ)
            .union(AgentPermissions::AUDIT_WRITE),
        "security" => AgentPermissions::READ_CODE
            .union(AgentPermissions::READ_CREDENTIALS)
            .union(AgentPermissions::SHELL_EXEC)
            .union(AgentPermissions::HTTP_REQUEST)
            .union(AgentPermissions::KNOWLEDGE_READ)
            .union(AgentPermissions::AUDIT_WRITE)
            .union(AgentPermissions::AUDIT_READ),
        "monitor" => AgentPermissions::READ_CODE
            .union(AgentPermissions::SHELL_EXEC)
            .union(AgentPermissions::HTTP_REQUEST)
            .union(AgentPermissions::KNOWLEDGE_READ)
            .union(AgentPermissions::AUDIT_READ)
            .union(AgentPermissions::HALT_AGENTS),
        _ => AgentPermissions::NONE,
    }
}

// ── Session Key ────────────────────────────────────────────────────────────

/// An ephemeral session key, derived from license material.
/// Lives only in memory for the duration of the session.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SessionKey {
    bytes: [u8; 32],
}

impl SessionKey {
    /// Derive a session key from license bytes and a timestamp.
    /// HKDF-SHA256: salt=b"phantom-session-v1", info=timestamp as LE bytes.
    pub fn new(license_bytes: &[u8], timestamp: u64) -> Result<Self, CryptoError> {
        let info = timestamp.to_le_bytes();
        let derived = hkdf_keys::derive_subkey(license_bytes, Some(b"phantom-session-v1"), &info)?;
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(derived.as_bytes());
        Ok(Self { bytes })
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Derive an agent-scoped key from this session key.
    pub fn derive_agent_key(
        &self,
        agent: &str,
        task_id: &str,
        permissions: AgentPermissions,
    ) -> Result<AgentKey, CryptoError> {
        let info = format!("agent-{}-{}", agent, task_id);
        let derived =
            hkdf_keys::derive_subkey(&self.bytes, Some(b"phantom-agent-v1"), info.as_bytes())?;
        let mut key = [0u8; 32];
        key.copy_from_slice(derived.as_bytes());
        Ok(AgentKey { key, permissions })
    }
}

impl std::fmt::Debug for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED SessionKey]")
    }
}

// ── Agent Key ──────────────────────────────────────────────────────────────

/// A key scoped to a specific agent and task, with attached permissions.
pub struct AgentKey {
    #[allow(dead_code)]
    key: [u8; 32],
    pub permissions: AgentPermissions,
}

impl AgentKey {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.key
    }

    /// Check if this agent key has the required permission.
    pub fn has_permission(&self, required: AgentPermissions) -> bool {
        self.permissions.contains(required)
    }
}

impl Drop for AgentKey {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl std::fmt::Debug for AgentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[REDACTED AgentKey perms=0x{:04x}]",
            self.permissions.bits()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_key_derivation() {
        let license = b"PH1-test-license-bytes";
        let sk1 = SessionKey::new(license, 1000).unwrap();
        let sk2 = SessionKey::new(license, 1000).unwrap();
        assert_eq!(sk1.as_bytes(), sk2.as_bytes(), "deterministic");
    }

    #[test]
    fn test_different_timestamps_different_keys() {
        let license = b"PH1-test-license-bytes";
        let sk1 = SessionKey::new(license, 1000).unwrap();
        let sk2 = SessionKey::new(license, 2000).unwrap();
        assert_ne!(sk1.as_bytes(), sk2.as_bytes());
    }

    #[test]
    fn test_agent_key_derivation() {
        let license = b"PH1-test-license-bytes";
        let session = SessionKey::new(license, 1000).unwrap();

        let ak1 = session
            .derive_agent_key("cto", "task-1", AgentPermissions::ALL)
            .unwrap();
        let ak2 = session
            .derive_agent_key("backend", "task-1", AgentPermissions::READ_CODE)
            .unwrap();

        assert_ne!(
            ak1.as_bytes(),
            ak2.as_bytes(),
            "different agents = different keys"
        );
    }

    #[test]
    fn test_agent_key_different_tasks() {
        let license = b"PH1-test-license-bytes";
        let session = SessionKey::new(license, 1000).unwrap();

        let ak1 = session
            .derive_agent_key("cto", "task-1", AgentPermissions::ALL)
            .unwrap();
        let ak2 = session
            .derive_agent_key("cto", "task-2", AgentPermissions::ALL)
            .unwrap();

        assert_ne!(
            ak1.as_bytes(),
            ak2.as_bytes(),
            "different tasks = different keys"
        );
    }

    #[test]
    fn test_permissions_bitflags() {
        let perms = AgentPermissions::READ_CODE.union(AgentPermissions::WRITE_CODE);
        assert!(perms.contains(AgentPermissions::READ_CODE));
        assert!(perms.contains(AgentPermissions::WRITE_CODE));
        assert!(!perms.contains(AgentPermissions::DEPLOY));
    }

    #[test]
    fn test_permissions_for_roles() {
        let cto = permissions_for_role("cto");
        assert!(cto.contains(AgentPermissions::ALL));

        let backend = permissions_for_role("backend");
        assert!(backend.contains(AgentPermissions::READ_CODE));
        assert!(backend.contains(AgentPermissions::DB_QUERY));
        assert!(!backend.contains(AgentPermissions::DEPLOY));

        let unknown = permissions_for_role("unknown");
        assert_eq!(unknown.bits(), 0);
    }

    #[test]
    fn test_session_key_debug_redacted() {
        let sk = SessionKey::new(b"test", 0).unwrap();
        let debug = format!("{:?}", sk);
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("test"));
    }

    #[test]
    fn test_agent_key_permission_check() {
        let license = b"test";
        let session = SessionKey::new(license, 0).unwrap();
        let agent = session
            .derive_agent_key("backend", "t1", permissions_for_role("backend"))
            .unwrap();

        assert!(agent.has_permission(AgentPermissions::READ_CODE));
        assert!(agent.has_permission(AgentPermissions::SHELL_EXEC));
        assert!(!agent.has_permission(AgentPermissions::DEPLOY));
        assert!(!agent.has_permission(AgentPermissions::DESTROY));
    }
}
