//! HKDF-SHA256 key derivation for deriving sub-keys from master key.
//! Used to derive: session keys, agent keys, infrastructure keys, destruction key.

use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroize;

use crate::CryptoError;

const DERIVED_KEY_LEN: usize = 32;

/// Derive a 256-bit sub-key from input keying material using HKDF-SHA256.
///
/// - `ikm`: Input keying material (e.g., master key bytes)
/// - `salt`: Optional salt (use `None` for default)
/// - `info`: Context string identifying the derived key purpose
///
/// Returns a 32-byte derived key that is zeroized on drop.
pub fn derive_subkey(
    ikm: &[u8],
    salt: Option<&[u8]>,
    info: &[u8],
) -> Result<DerivedSubKey, CryptoError> {
    let hk = Hkdf::<Sha256>::new(salt, ikm);
    let mut okm = [0u8; DERIVED_KEY_LEN];
    hk.expand(info, &mut okm)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;
    Ok(DerivedSubKey { bytes: okm })
}

/// A 256-bit derived sub-key. Zeroized on drop.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct DerivedSubKey {
    bytes: [u8; DERIVED_KEY_LEN],
}

impl DerivedSubKey {
    pub fn as_bytes(&self) -> &[u8; DERIVED_KEY_LEN] {
        &self.bytes
    }
}

/// Well-known info strings for Phantom's key hierarchy.
pub mod info {
    pub const SESSION_KEY: &[u8] = b"phantom-session-key-v1";
    pub const AGENT_KEY: &[u8] = b"phantom-agent-key-v1";
    pub const INFRASTRUCTURE_KEY: &[u8] = b"phantom-infra-key-v1";
    pub const DESTRUCTION_KEY: &[u8] = b"phantom-destruction-key-v1";
    pub const LICENSE_SIGNING_KEY: &[u8] = b"phantom-license-signing-key-v1";
    pub const STORAGE_KEY: &[u8] = b"phantom-storage-key-v1";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_subkey_deterministic() {
        let ikm = b"master-key-material";
        let k1 = derive_subkey(ikm, None, info::SESSION_KEY).unwrap();
        let k2 = derive_subkey(ikm, None, info::SESSION_KEY).unwrap();
        assert_eq!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn test_different_info_different_key() {
        let ikm = b"master-key-material";
        let k1 = derive_subkey(ikm, None, info::SESSION_KEY).unwrap();
        let k2 = derive_subkey(ikm, None, info::AGENT_KEY).unwrap();
        assert_ne!(k1.as_bytes(), k2.as_bytes());
    }
}
