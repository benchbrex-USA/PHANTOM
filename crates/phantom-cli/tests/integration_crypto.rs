//! Integration tests: full cryptographic pipeline.
//!
//! Tests the end-to-end flow: master key → sub-keys → encryption → license.

use phantom_crypto::{
    aes256gcm::{decrypt, encrypt, EncryptionKey},
    argon2id,
    ed25519::LicenseSigningKey,
    license::LicenseKey,
    master_key::MasterKeySession,
};

#[test]
fn test_full_key_derivation_pipeline() {
    // 1. Derive master key from passphrase
    let passphrase = b"this-is-a-very-secure-passphrase-for-testing-32+";
    let salt = [0x42u8; 32];
    let derived = argon2id::derive_key(passphrase, &salt).unwrap();
    assert_eq!(derived.as_bytes().len(), 32);

    // 2. Create master key session (derives all sub-keys)
    let session = MasterKeySession::new(passphrase, salt).unwrap();
    let session_key = session.derive_session_key().unwrap();
    let infra_key = session.derive_infra_key().unwrap();
    let storage_key = session.derive_storage_key().unwrap();

    // 3. All sub-keys should be different
    assert_ne!(session_key.as_bytes(), infra_key.as_bytes());
    assert_ne!(infra_key.as_bytes(), storage_key.as_bytes());
    assert_ne!(session_key.as_bytes(), storage_key.as_bytes());

    // 4. Sub-keys should be deterministic (same input → same output)
    let session2 = MasterKeySession::new(passphrase, salt).unwrap();
    assert_eq!(
        session.derive_session_key().unwrap().as_bytes(),
        session2.derive_session_key().unwrap().as_bytes()
    );
    assert_eq!(
        session.derive_infra_key().unwrap().as_bytes(),
        session2.derive_infra_key().unwrap().as_bytes()
    );
}

#[test]
fn test_encrypt_decrypt_roundtrip_with_derived_key() {
    // Derive key from passphrase
    let passphrase = b"test-passphrase-for-aes-encryption!!";
    let salt = [0x43u8; 32];
    let session = MasterKeySession::new(passphrase, salt).unwrap();

    // Use storage key for encryption
    let key = session.derive_storage_key().unwrap();

    // Encrypt some data
    let plaintext = b"sensitive API token: ghp_abc123xyz789";
    let ciphertext = encrypt(&key, plaintext).expect("encryption failed");

    // Ciphertext should be different from plaintext
    assert_ne!(&ciphertext, plaintext);
    assert!(ciphertext.len() > plaintext.len()); // nonce + tag overhead

    // Decrypt
    let decrypted = decrypt(&key, &ciphertext).expect("decryption failed");
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_wrong_key_cannot_decrypt() {
    let key1 = EncryptionKey::from_bytes([0x11u8; 32]);
    let key2 = EncryptionKey::from_bytes([0x22u8; 32]);

    let plaintext = b"secret data";
    let ciphertext = encrypt(&key1, plaintext).unwrap();

    // Decrypting with wrong key should fail
    let result = decrypt(&key2, &ciphertext);
    assert!(result.is_err());
}

#[test]
fn test_license_sign_verify_roundtrip() {
    // Generate signing keypair
    let signing_key = LicenseSigningKey::generate();
    let verifying_key = signing_key.verifying_key();

    // Issue (sign) the license for the current machine
    let caps = vec!["build".into(), "deploy".into()];
    let license = LicenseKey::issue(&signing_key, "pro", caps, 365).unwrap();

    // Encode to string
    let encoded = license.encode();
    assert!(encoded.starts_with("PH1-"));

    // Decode from string
    let decoded = LicenseKey::decode(&encoded).expect("decode failed");
    assert_eq!(decoded.payload.tier, "pro");
    assert_eq!(decoded.payload.cap, vec!["build", "deploy"]);

    // Verify signature (includes machine fingerprint check)
    assert!(decoded.verify(&verifying_key).is_ok());
}

#[test]
fn test_license_tampered_signature_fails() {
    let signing_key = LicenseSigningKey::generate();
    let verifying_key = signing_key.verifying_key();

    let license = LicenseKey::issue(&signing_key, "enterprise", vec!["all".into()], 365).unwrap();
    let encoded = license.encode();

    // Tamper with the encoded license (change a character in the signature)
    let mut chars: Vec<char> = encoded.chars().collect();
    if let Some(last) = chars.last_mut() {
        *last = if *last == 'A' { 'B' } else { 'A' };
    }
    let tampered: String = chars.into_iter().collect();

    // Should either fail to decode or fail signature verification
    match LicenseKey::decode(&tampered) {
        Ok(decoded) => {
            assert!(decoded.verify(&verifying_key).is_err());
        }
        Err(_) => {
            // Also acceptable — corrupted encoding
        }
    }
}

#[test]
fn test_agent_scoped_keys_are_unique() {
    let passphrase = b"agent-key-test-passphrase-minimum-32chars!!";
    let salt = [0x44u8; 32];
    let session = MasterKeySession::new(passphrase, salt).unwrap();

    let agents = [
        "cto",
        "architect",
        "backend",
        "frontend",
        "devops",
        "qa",
        "security",
        "monitor",
    ];
    let mut keys: Vec<[u8; 32]> = Vec::new();

    for agent in &agents {
        let key = session.derive_agent_key(agent, "default-task").unwrap();
        let key_bytes = *key.as_bytes();
        // Each agent key should be unique
        assert!(
            !keys.contains(&key_bytes),
            "duplicate key for agent: {}",
            agent
        );
        keys.push(key_bytes);
    }

    assert_eq!(keys.len(), 8);
}
