//! Ed25519 signing and verification for license keys.
//! License forgery is infeasible without the private signing key.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey, SECRET_KEY_LENGTH};
use rand::rngs::OsRng;
use zeroize::Zeroize;

use crate::CryptoError;

/// An Ed25519 keypair for license signing.
pub struct LicenseSigningKey {
    signing_key: SigningKey,
}

impl Drop for LicenseSigningKey {
    fn drop(&mut self) {
        let mut bytes = self.signing_key.to_bytes();
        bytes.zeroize();
    }
}

impl LicenseSigningKey {
    /// Generate a new random signing key.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Restore from raw secret key bytes (32 bytes).
    pub fn from_bytes(bytes: &[u8; SECRET_KEY_LENGTH]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        Self { signing_key }
    }

    /// Get the raw secret key bytes. Handle with care — zeroize after use.
    pub fn to_bytes(&self) -> [u8; SECRET_KEY_LENGTH] {
        self.signing_key.to_bytes()
    }

    /// Get the public verifying key.
    pub fn verifying_key(&self) -> LicenseVerifyingKey {
        LicenseVerifyingKey {
            verifying_key: self.signing_key.verifying_key(),
        }
    }

    /// Sign arbitrary data.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature = self.signing_key.sign(message);
        signature.to_bytes().to_vec()
    }
}

/// An Ed25519 public key for license verification.
/// Embedded in the Phantom binary.
#[derive(Clone)]
pub struct LicenseVerifyingKey {
    verifying_key: VerifyingKey,
}

impl LicenseVerifyingKey {
    /// Restore from raw public key bytes (32 bytes).
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        let verifying_key = VerifyingKey::from_bytes(bytes)
            .map_err(|e| CryptoError::InvalidLicense(e.to_string()))?;
        Ok(Self { verifying_key })
    }

    /// Get the raw public key bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Verify a signature against a message.
    pub fn verify(&self, message: &[u8], signature_bytes: &[u8]) -> Result<(), CryptoError> {
        let signature = Signature::from_slice(signature_bytes)
            .map_err(|_| CryptoError::SignatureVerificationFailed)?;
        self.verifying_key
            .verify(message, &signature)
            .map_err(|_| CryptoError::SignatureVerificationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let signing_key = LicenseSigningKey::generate();
        let verifying_key = signing_key.verifying_key();

        let message = b"phantom license payload";
        let signature = signing_key.sign(message);

        assert!(verifying_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_wrong_message_fails() {
        let signing_key = LicenseSigningKey::generate();
        let verifying_key = signing_key.verifying_key();

        let signature = signing_key.sign(b"real message");
        assert!(verifying_key
            .verify(b"tampered message", &signature)
            .is_err());
    }

    #[test]
    fn test_wrong_key_fails() {
        let signing_key1 = LicenseSigningKey::generate();
        let signing_key2 = LicenseSigningKey::generate();

        let message = b"phantom license payload";
        let signature = signing_key1.sign(message);

        let wrong_verifier = signing_key2.verifying_key();
        assert!(wrong_verifier.verify(message, &signature).is_err());
    }

    #[test]
    fn test_roundtrip_bytes() {
        let signing_key = LicenseSigningKey::generate();
        let bytes = signing_key.to_bytes();
        let restored = LicenseSigningKey::from_bytes(&bytes);

        let message = b"test roundtrip";
        let sig1 = signing_key.sign(message);
        let sig2 = restored.sign(message);
        assert_eq!(sig1, sig2);
    }
}
