//! Argon2id key derivation for master key.
//! Master passphrase → Argon2id → 256-bit key. Never stored.

use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use zeroize::Zeroize;

use crate::CryptoError;

/// Argon2id parameters — tuned for security on modern hardware.
const MEMORY_COST_KIB: u32 = 256 * 1024; // 256 MB
const TIME_COST: u32 = 4;
const PARALLELISM: u32 = 4;
const OUTPUT_LEN: usize = 32; // 256-bit key
const SALT_LEN: usize = 32;

/// A 256-bit derived key. Zeroized on drop.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct DerivedKey {
    bytes: [u8; OUTPUT_LEN],
}

impl DerivedKey {
    pub fn as_bytes(&self) -> &[u8; OUTPUT_LEN] {
        &self.bytes
    }
}

/// Generate a random salt for Argon2id.
pub fn generate_salt() -> Result<[u8; SALT_LEN], CryptoError> {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng()
        .try_fill_bytes(&mut salt)
        .map_err(|_| CryptoError::RngFailed)?;
    Ok(salt)
}

/// Derive a 256-bit key from a passphrase using Argon2id.
pub fn derive_key(passphrase: &[u8], salt: &[u8]) -> Result<DerivedKey, CryptoError> {
    let params = Params::new(MEMORY_COST_KIB, TIME_COST, PARALLELISM, Some(OUTPUT_LEN))
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut output = [0u8; OUTPUT_LEN];
    argon2
        .hash_password_into(passphrase, salt, &mut output)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

    Ok(DerivedKey { bytes: output })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_deterministic() {
        let salt = [42u8; SALT_LEN];
        let key1 = derive_key(b"test-passphrase", &salt).unwrap();
        let key2 = derive_key(b"test-passphrase", &salt).unwrap();
        assert_eq!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_different_passphrase_different_key() {
        let salt = [42u8; SALT_LEN];
        let key1 = derive_key(b"passphrase-1", &salt).unwrap();
        let key2 = derive_key(b"passphrase-2", &salt).unwrap();
        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_different_salt_different_key() {
        let salt1 = [1u8; SALT_LEN];
        let salt2 = [2u8; SALT_LEN];
        let key1 = derive_key(b"same-passphrase", &salt1).unwrap();
        let key2 = derive_key(b"same-passphrase", &salt2).unwrap();
        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }
}
