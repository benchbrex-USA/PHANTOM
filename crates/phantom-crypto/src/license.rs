//! License key creation and verification.
//! Format: PH1-<base64url_payload>-<base64url_signature>
//! Core Law 1: No installation without a valid license key.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::ed25519::{LicenseSigningKey, LicenseVerifyingKey};
use crate::fingerprint::{collect_machine_identifiers, MachineIdentifiers};
use crate::CryptoError;

/// License payload — signed by Ed25519, verified at every launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    /// Version
    pub v: u32,
    /// Machine fingerprint (hex-encoded HMAC-SHA256)
    pub mid: String,
    /// Issued at (unix timestamp)
    pub iat: i64,
    /// Expires at (unix timestamp)
    pub exp: i64,
    /// Capabilities (agent roles this license enables)
    pub cap: Vec<String>,
    /// License tier
    pub tier: String,
}

/// A complete license key: payload + signature.
pub struct LicenseKey {
    pub payload: LicensePayload,
    pub payload_bytes: Vec<u8>,
    pub signature: Vec<u8>,
}

/// License salt used for machine fingerprinting.
const LICENSE_SALT: &[u8] = b"phantom-license-fingerprint-salt-v1";

impl LicenseKey {
    /// Issue a new license for the current machine.
    pub fn issue(
        signing_key: &LicenseSigningKey,
        tier: &str,
        capabilities: Vec<String>,
        valid_days: u32,
    ) -> Result<Self, CryptoError> {
        let ids = collect_machine_identifiers();
        Self::issue_for_machine(signing_key, &ids, tier, capabilities, valid_days)
    }

    /// Issue a license for a specific machine (used by master key holder).
    pub fn issue_for_machine(
        signing_key: &LicenseSigningKey,
        machine: &MachineIdentifiers,
        tier: &str,
        capabilities: Vec<String>,
        valid_days: u32,
    ) -> Result<Self, CryptoError> {
        let fingerprint = machine.fingerprint(LICENSE_SALT)?;
        let mid = hex::encode(fingerprint);

        let now = Utc::now();
        let exp = now + chrono::Duration::days(valid_days as i64);

        let payload = LicensePayload {
            v: 1,
            mid,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            cap: capabilities,
            tier: tier.to_string(),
        };

        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|e| CryptoError::Serialization(e.to_string()))?;

        let signature = signing_key.sign(&payload_bytes);

        Ok(Self {
            payload,
            payload_bytes,
            signature,
        })
    }

    /// Encode as the wire format: PH1-<base64url_payload>-<base64url_signature>
    pub fn encode(&self) -> String {
        let payload_enc = URL_SAFE_NO_PAD.encode(&self.payload_bytes);
        let sig_enc = URL_SAFE_NO_PAD.encode(&self.signature);
        format!("PH1-{}-{}", payload_enc, sig_enc)
    }

    /// Decode from wire format.
    pub fn decode(encoded: &str) -> Result<Self, CryptoError> {
        let parts: Vec<&str> = encoded.splitn(3, '-').collect();
        if parts.len() != 3 || parts[0] != "PH1" {
            return Err(CryptoError::InvalidLicense("invalid format".into()));
        }

        let payload_bytes = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| CryptoError::InvalidLicense(format!("payload decode: {e}")))?;

        let signature = URL_SAFE_NO_PAD
            .decode(parts[2])
            .map_err(|e| CryptoError::InvalidLicense(format!("signature decode: {e}")))?;

        let payload: LicensePayload = serde_json::from_slice(&payload_bytes)
            .map_err(|e| CryptoError::InvalidLicense(format!("payload parse: {e}")))?;

        Ok(Self {
            payload,
            payload_bytes,
            signature,
        })
    }

    /// Verify the license: signature + fingerprint + expiration.
    pub fn verify(&self, verifying_key: &LicenseVerifyingKey) -> Result<(), CryptoError> {
        // Step 1: Verify Ed25519 signature
        verifying_key.verify(&self.payload_bytes, &self.signature)?;

        // Step 2: Verify machine fingerprint
        let ids = collect_machine_identifiers();
        let current_fingerprint = ids.fingerprint(LICENSE_SALT)?;
        let current_mid = hex::encode(current_fingerprint);

        if current_mid != self.payload.mid {
            return Err(CryptoError::FingerprintMismatch);
        }

        // Step 3: Check expiration
        let now = Utc::now().timestamp();
        if now > self.payload.exp {
            return Err(CryptoError::LicenseExpired);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ed25519::LicenseSigningKey;

    #[test]
    fn test_issue_encode_decode_verify() {
        let signing_key = LicenseSigningKey::generate();
        let verifying_key = signing_key.verifying_key();

        let caps = vec![
            "cto".into(),
            "backend".into(),
            "frontend".into(),
        ];

        let license = LicenseKey::issue(&signing_key, "founder", caps, 365).unwrap();
        let encoded = license.encode();

        assert!(encoded.starts_with("PH1-"));

        let decoded = LicenseKey::decode(&encoded).unwrap();
        assert_eq!(decoded.payload.tier, "founder");
        assert_eq!(decoded.payload.v, 1);

        // Verify should pass on same machine
        assert!(decoded.verify(&verifying_key).is_ok());
    }

    #[test]
    fn test_tampered_signature_fails() {
        let signing_key = LicenseSigningKey::generate();
        let wrong_key = LicenseSigningKey::generate();
        let wrong_verifier = wrong_key.verifying_key();

        let license = LicenseKey::issue(&signing_key, "trial", vec!["cto".into()], 30).unwrap();
        let decoded = LicenseKey::decode(&license.encode()).unwrap();

        assert!(decoded.verify(&wrong_verifier).is_err());
    }
}
