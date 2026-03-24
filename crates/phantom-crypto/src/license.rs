//! License key creation and verification.
//! Format: PH1-<base62_payload>-<base62_signature>
//! Core Law 1: No installation without a valid license key.
//!
//! The embedded public key is compiled into the binary. Only the master key
//! holder has the corresponding private key and can issue valid licenses.

use chrono::Utc;
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};

use crate::ed25519::{LicenseSigningKey, LicenseVerifyingKey};
use crate::fingerprint::{collect_machine_identifiers, MachineIdentifiers};
use crate::CryptoError;

// ── Embedded Public Key ────────────────────────────────────────────────────

/// The Ed25519 public key embedded in every Phantom binary.
/// Only licenses signed by the corresponding private key are accepted.
const EMBEDDED_PUBLIC_KEY_PEM: &str = "-----BEGIN PUBLIC KEY-----\n\
MCowBQYDK2VwAyEAY22rvLzkzW4d9QmAAGfMC8zunmkJ82K/klQK8a5M+Rg=\n\
-----END PUBLIC KEY-----";

/// Parse the embedded PEM public key into an Ed25519 verifying key.
fn embedded_verifying_key() -> Result<LicenseVerifyingKey, CryptoError> {
    // The PEM wraps a DER-encoded SubjectPublicKeyInfo.
    // For Ed25519, the raw 32-byte key starts at byte 12 of the DER.
    let b64_line = EMBEDDED_PUBLIC_KEY_PEM
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<String>();

    let der = base64::engine::general_purpose::STANDARD
        .decode(&b64_line)
        .map_err(|e| CryptoError::InvalidLicense(format!("embedded key decode: {e}")))?;

    // SubjectPublicKeyInfo for Ed25519: 12-byte header + 32-byte key
    if der.len() < 44 {
        return Err(CryptoError::InvalidLicense("embedded key too short".into()));
    }
    let raw_key: [u8; 32] = der[12..44]
        .try_into()
        .map_err(|_| CryptoError::InvalidLicense("embedded key extraction failed".into()))?;

    let vk = VerifyingKey::from_bytes(&raw_key)
        .map_err(|e| CryptoError::InvalidLicense(format!("embedded key invalid: {e}")))?;

    Ok(LicenseVerifyingKey::from_dalek(vk))
}

use base64::Engine;

// ── Base62 codec for arbitrary bytes ───────────────────────────────────────

const BASE62_ALPHABET: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Encode arbitrary bytes as a base62 string.
/// Uses big-endian byte-to-digit conversion.
fn base62_encode(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }
    // Work on a mutable copy, repeatedly divide by 62
    let mut digits: Vec<u8> = Vec::new();
    let mut source = data.to_vec();

    while !source.is_empty() {
        let mut remainder = 0u32;
        let mut next = Vec::new();
        for &byte in &source {
            let acc = (remainder << 8) | byte as u32;
            let quotient = acc / 62;
            remainder = acc % 62;
            if !next.is_empty() || quotient > 0 {
                next.push(quotient as u8);
            }
        }
        digits.push(remainder as u8);
        source = next;
    }

    digits.reverse();
    digits
        .into_iter()
        .map(|d| BASE62_ALPHABET[d as usize] as char)
        .collect()
}

/// Decode a base62 string back to bytes.
fn base62_decode(encoded: &str) -> Result<Vec<u8>, CryptoError> {
    if encoded.is_empty() {
        return Ok(Vec::new());
    }

    let mut digits: Vec<u8> = Vec::with_capacity(encoded.len());
    for ch in encoded.bytes() {
        let val = match ch {
            b'0'..=b'9' => ch - b'0',
            b'A'..=b'Z' => ch - b'A' + 10,
            b'a'..=b'z' => ch - b'a' + 36,
            _ => {
                return Err(CryptoError::InvalidLicense(format!(
                    "invalid base62 character: {}",
                    ch as char
                )));
            }
        };
        digits.push(val);
    }

    // Convert from base62 digits to bytes
    let mut bytes: Vec<u8> = Vec::new();
    let mut source = digits;

    while !source.is_empty() {
        let mut remainder = 0u32;
        let mut next = Vec::new();
        for &digit in &source {
            let acc = remainder * 62 + digit as u32;
            let quotient = acc / 256;
            remainder = acc % 256;
            if !next.is_empty() || quotient > 0 {
                next.push(quotient as u8);
            }
        }
        bytes.push(remainder as u8);
        source = next;
    }

    bytes.reverse();
    Ok(bytes)
}

// ── License types ──────────────────────────────────────────────────────────

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
    /// Installation ID (hex-encoded random bytes)
    pub iid: String,
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

        // Generate a random installation ID
        let mut iid_bytes = [0u8; 16];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut iid_bytes);
        let iid = hex::encode(iid_bytes);

        let payload = LicensePayload {
            v: 1,
            mid,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            cap: capabilities,
            tier: tier.to_string(),
            iid,
        };

        // Serialize with sorted keys for deterministic signatures
        let payload_bytes =
            serde_json::to_vec(&payload).map_err(|e| CryptoError::Serialization(e.to_string()))?;

        let signature = signing_key.sign(&payload_bytes);

        Ok(Self {
            payload,
            payload_bytes,
            signature,
        })
    }

    /// Encode as the wire format: PH1-<base62_payload>-<base62_signature>
    pub fn encode(&self) -> String {
        let payload_enc = base62_encode(&self.payload_bytes);
        let sig_enc = base62_encode(&self.signature);
        format!("PH1-{}-{}", payload_enc, sig_enc)
    }

    /// Decode from wire format: PH1-<base62_payload>-<base62_signature>
    pub fn decode(encoded: &str) -> Result<Self, CryptoError> {
        let parts: Vec<&str> = encoded.splitn(3, '-').collect();
        if parts.len() != 3 || parts[0] != "PH1" {
            return Err(CryptoError::InvalidLicense(
                "invalid format: must start with PH1-".into(),
            ));
        }

        let payload_bytes = base62_decode(parts[1])?;
        let signature = base62_decode(parts[2])?;

        let payload: LicensePayload = serde_json::from_slice(&payload_bytes)
            .map_err(|e| CryptoError::InvalidLicense(format!("payload parse: {e}")))?;

        Ok(Self {
            payload,
            payload_bytes,
            signature,
        })
    }

    /// Verify the license using the embedded public key.
    /// Checks: signature validity + machine fingerprint + expiration.
    pub fn verify_with_embedded_key(&self) -> Result<(), CryptoError> {
        let verifying_key = embedded_verifying_key()?;
        self.verify(&verifying_key)
    }

    /// Verify the license against a specific verifying key.
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

    /// Verify only the signature (no fingerprint or expiry check).
    /// Useful for checking a license destined for another machine.
    pub fn verify_signature_only(
        &self,
        verifying_key: &LicenseVerifyingKey,
    ) -> Result<(), CryptoError> {
        verifying_key.verify(&self.payload_bytes, &self.signature)
    }
}

/// Check if a license payload has expired.
pub fn check_expiry(payload: &LicensePayload) -> bool {
    let now = Utc::now().timestamp();
    now <= payload.exp
}

/// Check if a license payload grants a specific capability.
pub fn check_capabilities(payload: &LicensePayload, required: &str) -> bool {
    payload.cap.iter().any(|c| c == required)
}

/// Convenience: decode and verify a license token string with the embedded key.
pub fn verify_license(token: &str) -> Result<LicensePayload, CryptoError> {
    let license = LicenseKey::decode(token)?;
    license.verify_with_embedded_key()?;
    Ok(license.payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ed25519::LicenseSigningKey;

    #[test]
    fn test_base62_roundtrip() {
        let data = b"hello world license payload test data 1234567890";
        let encoded = base62_encode(data);
        let decoded = base62_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_base62_empty() {
        assert_eq!(base62_encode(b""), "");
        assert_eq!(base62_decode("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn test_base62_single_byte() {
        for b in 0..=255u8 {
            let encoded = base62_encode(&[b]);
            let decoded = base62_decode(&encoded).unwrap();
            assert_eq!(decoded, vec![b], "failed roundtrip for byte {b}");
        }
    }

    #[test]
    fn test_issue_encode_decode() {
        let signing_key = LicenseSigningKey::generate();
        let verifying_key = signing_key.verifying_key();

        let caps = vec!["cto".into(), "backend".into(), "frontend".into()];

        let license = LicenseKey::issue(&signing_key, "founder", caps, 365).expect("issue failed");
        let encoded = license.encode();

        assert!(encoded.starts_with("PH1-"));
        // Ensure base62 characters only (plus PH1- prefix)
        let body = &encoded[4..]; // skip "PH1-"
        for ch in body.chars() {
            assert!(
                ch.is_ascii_alphanumeric() || ch == '-',
                "unexpected char: {ch}"
            );
        }

        let decoded = LicenseKey::decode(&encoded).expect("decode failed");
        assert_eq!(decoded.payload.tier, "founder");
        assert_eq!(decoded.payload.v, 1);
        assert!(!decoded.payload.iid.is_empty());

        // Verify should pass on same machine with correct key
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

    #[test]
    fn test_check_expiry() {
        let payload = LicensePayload {
            v: 1,
            mid: "test".into(),
            iat: Utc::now().timestamp(),
            exp: Utc::now().timestamp() + 86400,
            cap: vec![],
            tier: "founder".into(),
            iid: "abc123".into(),
        };
        assert!(check_expiry(&payload));

        let expired = LicensePayload {
            exp: 0,
            ..payload.clone()
        };
        assert!(!check_expiry(&expired));
    }

    #[test]
    fn test_check_capabilities() {
        let payload = LicensePayload {
            v: 1,
            mid: "test".into(),
            iat: 0,
            exp: i64::MAX,
            cap: vec!["cto".into(), "backend".into()],
            tier: "founder".into(),
            iid: "abc".into(),
        };
        assert!(check_capabilities(&payload, "cto"));
        assert!(check_capabilities(&payload, "backend"));
        assert!(!check_capabilities(&payload, "frontend"));
    }

    #[test]
    fn test_embedded_key_parses() {
        let vk = embedded_verifying_key();
        assert!(vk.is_ok(), "embedded key should parse: {:?}", vk.err());
    }

    #[test]
    fn test_payload_has_iid() {
        let signing_key = LicenseSigningKey::generate();
        let license = LicenseKey::issue(&signing_key, "founder", vec!["cto".into()], 365).unwrap();
        assert_eq!(license.payload.iid.len(), 32); // 16 bytes = 32 hex chars
    }
}
