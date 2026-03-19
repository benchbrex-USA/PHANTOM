//! Master key management.
//! Core Law 2: No ownership without the master key.
//! Passphrase → Argon2id → 256-bit master key → HKDF → sub-keys.
//! Master key is NEVER stored. Derived in-memory, zeroized on drop.

use crate::argon2id::{self, DerivedKey};
use crate::hkdf_keys::{self, info, DerivedSubKey};
use crate::aes256gcm::EncryptionKey;
use crate::CryptoError;

/// The live master key session. Exists only in memory.
/// Created from passphrase, produces all sub-keys, zeroized on drop.
pub struct MasterKeySession {
    master_key: DerivedKey,
    salt: [u8; 32],
}

impl MasterKeySession {
    /// Initialize a new master key from a passphrase and salt.
    /// The salt must be stored (encrypted) remotely for future derivation.
    pub fn new(passphrase: &[u8], salt: [u8; 32]) -> Result<Self, CryptoError> {
        let master_key = argon2id::derive_key(passphrase, &salt)?;
        Ok(Self { master_key, salt })
    }

    /// First-time initialization: generate a random salt, derive the master key.
    pub fn init(passphrase: &[u8]) -> Result<Self, CryptoError> {
        let salt = argon2id::generate_salt()?;
        Self::new(passphrase, salt)
    }

    /// Get the salt (needed for future key re-derivation).
    pub fn salt(&self) -> &[u8; 32] {
        &self.salt
    }

    /// Derive a session key (ephemeral, per-session).
    pub fn derive_session_key(&self) -> Result<EncryptionKey, CryptoError> {
        let sub = hkdf_keys::derive_subkey(self.master_key.as_bytes(), None, info::SESSION_KEY)?;
        Ok(EncryptionKey::from_bytes(*sub.as_bytes()))
    }

    /// Derive the infrastructure encryption key.
    pub fn derive_infra_key(&self) -> Result<EncryptionKey, CryptoError> {
        let sub = hkdf_keys::derive_subkey(
            self.master_key.as_bytes(),
            None,
            info::INFRASTRUCTURE_KEY,
        )?;
        Ok(EncryptionKey::from_bytes(*sub.as_bytes()))
    }

    /// Derive the storage encryption key.
    pub fn derive_storage_key(&self) -> Result<EncryptionKey, CryptoError> {
        let sub = hkdf_keys::derive_subkey(self.master_key.as_bytes(), None, info::STORAGE_KEY)?;
        Ok(EncryptionKey::from_bytes(*sub.as_bytes()))
    }

    /// Derive the license signing key material.
    pub fn derive_license_signing_material(&self) -> Result<DerivedSubKey, CryptoError> {
        hkdf_keys::derive_subkey(
            self.master_key.as_bytes(),
            None,
            info::LICENSE_SIGNING_KEY,
        )
    }

    /// Derive an agent-scoped key for a specific agent + task.
    pub fn derive_agent_key(&self, agent_id: &str, task_id: &str) -> Result<EncryptionKey, CryptoError> {
        let info = format!("phantom-agent-key-v1:{}:{}", agent_id, task_id);
        let sub = hkdf_keys::derive_subkey(self.master_key.as_bytes(), None, info.as_bytes())?;
        Ok(EncryptionKey::from_bytes(*sub.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_key_init_and_derive() {
        let session = MasterKeySession::init(b"my-secure-32-char-passphrase!!!!").unwrap();

        let session_key = session.derive_session_key().unwrap();
        let infra_key = session.derive_infra_key().unwrap();

        // Different sub-keys should be different
        assert_ne!(session_key.as_bytes(), infra_key.as_bytes());
    }

    #[test]
    fn test_master_key_deterministic_with_same_salt() {
        let passphrase = b"deterministic-passphrase-test!!!";
        let salt = [99u8; 32];

        let s1 = MasterKeySession::new(passphrase, salt).unwrap();
        let s2 = MasterKeySession::new(passphrase, salt).unwrap();

        let k1 = s1.derive_session_key().unwrap();
        let k2 = s2.derive_session_key().unwrap();
        assert_eq!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn test_agent_keys_are_unique() {
        let session = MasterKeySession::init(b"agent-key-test-passphrase!!!!!!!").unwrap();

        let k1 = session.derive_agent_key("backend", "task-1").unwrap();
        let k2 = session.derive_agent_key("backend", "task-2").unwrap();
        let k3 = session.derive_agent_key("frontend", "task-1").unwrap();

        assert_ne!(k1.as_bytes(), k2.as_bytes());
        assert_ne!(k1.as_bytes(), k3.as_bytes());
        assert_ne!(k2.as_bytes(), k3.as_bytes());
    }
}
