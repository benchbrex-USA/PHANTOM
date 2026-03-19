//! AES-256-GCM authenticated encryption.
//! Used for all data-at-rest and data-in-transit encryption.
//! Zero-knowledge: servers store encrypted blobs they cannot decrypt.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    AeadCore, Aes256Gcm, Nonce,
};
use zeroize::Zeroize;

use crate::CryptoError;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

/// An AES-256-GCM encryption key. Zeroized on drop.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct EncryptionKey {
    bytes: [u8; KEY_LEN],
}

impl EncryptionKey {
    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self { bytes }
    }

    /// Generate a random encryption key.
    pub fn generate() -> Self {
        let mut bytes = [0u8; KEY_LEN];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.bytes
    }
}

/// Encrypt plaintext with AES-256-GCM.
/// Returns: nonce (12 bytes) || ciphertext || tag (16 bytes)
pub fn encrypt(key: &EncryptionKey, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(&key.bytes)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let mut output = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypt ciphertext produced by `encrypt`.
/// Input format: nonce (12 bytes) || ciphertext || tag (16 bytes)
pub fn decrypt(key: &EncryptionKey, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if data.len() < NONCE_LEN {
        return Err(CryptoError::DecryptionFailed(
            "data too short for nonce".into(),
        ));
    }

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key.bytes)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = EncryptionKey::generate();
        let plaintext = b"phantom zero-knowledge encrypted blob";

        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();

        assert_eq!(plaintext.as_slice(), &decrypted);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();

        let encrypted = encrypt(&key1, b"secret data").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = EncryptionKey::generate();
        let mut encrypted = encrypt(&key, b"integrity test").unwrap();

        // Flip a byte in the ciphertext
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;

        assert!(decrypt(&key, &encrypted).is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let key = EncryptionKey::generate();
        let encrypted = encrypt(&key, b"").unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert!(decrypted.is_empty());
    }
}
