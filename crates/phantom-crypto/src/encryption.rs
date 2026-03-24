//! High-level encryption API with EncryptedBlob, AAD support, and JSON serialization.
//! All data-at-rest uses this module.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng, Payload},
    AeadCore, Aes256Gcm, Nonce,
};
use serde::{Deserialize, Serialize};

use crate::CryptoError;

// ── EncryptedBlob ──────────────────────────────────────────────────────────

/// An encrypted blob with nonce and ciphertext (tag appended by AES-GCM).
/// Nonce serializes as hex, ciphertext as base64 for JSON compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlob {
    /// 12-byte AES-GCM nonce (hex-encoded in JSON)
    #[serde(with = "hex_bytes")]
    pub nonce: [u8; 12],
    /// Ciphertext + authentication tag (base64 in JSON)
    #[serde(with = "base64_bytes")]
    pub ciphertext: Vec<u8>,
}

/// Encrypt plaintext with AES-256-GCM and associated data (AAD).
/// AAD is authenticated but not encrypted — prevents blob-swapping attacks.
pub fn encrypt(plaintext: &[u8], key: &[u8; 32], aad: &[u8]) -> Result<EncryptedBlob, CryptoError> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let nonce_bytes: [u8; 12] = nonce.into();

    let payload = Payload {
        msg: plaintext,
        aad,
    };

    let ciphertext = cipher
        .encrypt(&nonce, payload)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    Ok(EncryptedBlob {
        nonce: nonce_bytes,
        ciphertext,
    })
}

/// Decrypt an EncryptedBlob with AES-256-GCM and verify AAD.
pub fn decrypt(blob: &EncryptedBlob, key: &[u8; 32], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    let nonce = Nonce::from_slice(&blob.nonce);

    let payload = Payload {
        msg: &blob.ciphertext,
        aad,
    };

    cipher
        .decrypt(nonce, payload)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
}

/// Encrypt to a JSON string.
pub fn encrypt_to_json(
    plaintext: &[u8],
    key: &[u8; 32],
    aad: &[u8],
) -> Result<String, CryptoError> {
    let blob = encrypt(plaintext, key, aad)?;
    serde_json::to_string(&blob).map_err(|e| CryptoError::Serialization(e.to_string()))
}

/// Decrypt from a JSON string.
pub fn decrypt_from_json(json: &str, key: &[u8; 32], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let blob: EncryptedBlob =
        serde_json::from_str(json).map_err(|e| CryptoError::Serialization(e.to_string()))?;
    decrypt(&blob, key, aad)
}

// ── Serde helpers ──────────────────────────────────────────────────────────

mod hex_bytes {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 12], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 12], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 12 bytes for nonce"))
    }
}

mod base64_bytes {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let plaintext = b"hello phantom world";
        let aad = b"vault/github/api_key";

        let blob = encrypt(plaintext, &key, aad).unwrap();
        let decrypted = decrypt(&blob, &key, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = test_key();
        let key2 = test_key();

        let blob = encrypt(b"secret", &key1, b"aad").unwrap();
        let result = decrypt(&blob, &key2, b"aad");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_aad_fails() {
        let key = test_key();
        let blob = encrypt(b"secret", &key, b"correct-path").unwrap();
        let result = decrypt(&blob, &key, b"wrong-path");
        assert!(result.is_err());
    }

    #[test]
    fn test_json_roundtrip() {
        let key = test_key();
        let plaintext = b"test json roundtrip data";
        let aad = b"test-key";

        let json = encrypt_to_json(plaintext, &key, aad).unwrap();
        assert!(json.contains("nonce"));
        assert!(json.contains("ciphertext"));

        let decrypted = decrypt_from_json(&json, &key, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypted_blob_serde() {
        let key = test_key();
        let blob = encrypt(b"test", &key, b"aad").unwrap();

        let json = serde_json::to_string(&blob).unwrap();
        let restored: EncryptedBlob = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.nonce, blob.nonce);
        assert_eq!(restored.ciphertext, blob.ciphertext);
    }

    #[test]
    fn test_empty_plaintext() {
        let key = test_key();
        let blob = encrypt(b"", &key, b"").unwrap();
        let decrypted = decrypt(&blob, &key, b"").unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_large_plaintext() {
        let key = test_key();
        let plaintext = vec![0xABu8; 100_000];
        let blob = encrypt(&plaintext, &key, b"big-file").unwrap();
        let decrypted = decrypt(&blob, &key, b"big-file").unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = test_key();
        let mut blob = encrypt(b"test", &key, b"aad").unwrap();
        if !blob.ciphertext.is_empty() {
            blob.ciphertext[0] ^= 0xFF;
        }
        assert!(decrypt(&blob, &key, b"aad").is_err());
    }
}
