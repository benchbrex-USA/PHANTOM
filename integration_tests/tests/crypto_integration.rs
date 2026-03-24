//! Crypto integration tests — full key hierarchy, license lifecycle,
//! fingerprinting, encryption roundtrips, session + agent keys.

// ═══════════════════════════════════════════════════════════════════════════
//  1. Key Hierarchy: Passphrase → Argon2id → MasterKey → HKDF → sub-keys
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_full_key_hierarchy_passphrase_to_subkeys() {
    use phantom_crypto::master_key::MasterKeySession;

    let passphrase = b"integration-test-passphrase-32!!";
    let salt = [42u8; 32];

    let session = MasterKeySession::new(passphrase, salt).unwrap();

    // Derive all sub-keys from master key via HKDF
    let session_key = session.derive_session_key().unwrap();
    let infra_key = session.derive_infra_key().unwrap();
    let storage_key = session.derive_storage_key().unwrap();
    let agent_key = session.derive_agent_key("backend", "task-1").unwrap();

    // All sub-keys should be 32 bytes
    assert_eq!(session_key.as_bytes().len(), 32);
    assert_eq!(infra_key.as_bytes().len(), 32);
    assert_eq!(storage_key.as_bytes().len(), 32);
    assert_eq!(agent_key.as_bytes().len(), 32);

    // All sub-keys must be distinct
    assert_ne!(session_key.as_bytes(), infra_key.as_bytes());
    assert_ne!(session_key.as_bytes(), storage_key.as_bytes());
    assert_ne!(infra_key.as_bytes(), storage_key.as_bytes());
    assert_ne!(session_key.as_bytes(), agent_key.as_bytes());
}

#[test]
fn test_key_derivation_is_deterministic() {
    use phantom_crypto::master_key::MasterKeySession;

    let passphrase = b"deterministic-key-test-phrase!!!!";
    let salt = [99u8; 32];

    let s1 = MasterKeySession::new(passphrase, salt).unwrap();
    let s2 = MasterKeySession::new(passphrase, salt).unwrap();

    assert_eq!(
        s1.derive_session_key().unwrap().as_bytes(),
        s2.derive_session_key().unwrap().as_bytes(),
    );
    assert_eq!(
        s1.derive_infra_key().unwrap().as_bytes(),
        s2.derive_infra_key().unwrap().as_bytes(),
    );
    assert_eq!(
        s1.derive_agent_key("cto", "plan").unwrap().as_bytes(),
        s2.derive_agent_key("cto", "plan").unwrap().as_bytes(),
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  2. License: generate keypair → sign → encode → decode → verify
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_license_full_lifecycle() {
    use phantom_crypto::ed25519::LicenseSigningKey;
    use phantom_crypto::license::{check_capabilities, check_expiry, LicenseKey};

    // Generate Ed25519 keypair
    let signing_key = LicenseSigningKey::generate();
    let verifying_key = signing_key.verifying_key();

    // Issue a license
    let caps = vec![
        "cto".into(),
        "backend".into(),
        "frontend".into(),
        "devops".into(),
    ];
    let license = LicenseKey::issue(&signing_key, "founder", caps, 365).unwrap();

    // Encode to wire format
    let encoded = license.encode();
    assert!(encoded.starts_with("PH1-"));

    // Decode from wire format
    let decoded = LicenseKey::decode(&encoded).unwrap();
    assert_eq!(decoded.payload.tier, "founder");
    assert_eq!(decoded.payload.v, 1);
    assert_eq!(decoded.payload.cap.len(), 4);

    // Verify signature
    assert!(decoded.verify_signature_only(&verifying_key).is_ok());

    // Check expiry (should be valid — 365 days)
    assert!(check_expiry(&decoded.payload));

    // Check capabilities
    assert!(check_capabilities(&decoded.payload, "cto"));
    assert!(check_capabilities(&decoded.payload, "backend"));
    assert!(!check_capabilities(&decoded.payload, "security"));
}

#[test]
fn test_license_tampered_signature_rejected() {
    use phantom_crypto::ed25519::LicenseSigningKey;
    use phantom_crypto::license::LicenseKey;

    let signing_key = LicenseSigningKey::generate();
    let wrong_key = LicenseSigningKey::generate();
    let wrong_verifier = wrong_key.verifying_key();

    let license = LicenseKey::issue(&signing_key, "trial", vec!["cto".into()], 30).unwrap();
    let decoded = LicenseKey::decode(&license.encode()).unwrap();

    // Verification with wrong key must fail
    assert!(decoded.verify_signature_only(&wrong_verifier).is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
//  3. Fingerprint: deterministic on same machine
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_fingerprint_deterministic_same_machine() {
    use phantom_crypto::fingerprint::collect_machine_identifiers;

    let ids = collect_machine_identifiers();
    let salt = b"phantom-integration-test-salt-v1";

    let fp1 = ids.fingerprint(salt).unwrap();
    let fp2 = ids.fingerprint(salt).unwrap();

    assert_eq!(
        fp1, fp2,
        "fingerprint must be identical on the same machine"
    );
    assert_eq!(fp1.len(), 32, "fingerprint must be 32 bytes (HMAC-SHA256)");
}

#[test]
fn test_fingerprint_different_salts_produce_different_hashes() {
    use phantom_crypto::fingerprint::collect_machine_identifiers;

    let ids = collect_machine_identifiers();
    let fp1 = ids.fingerprint(b"salt-alpha").unwrap();
    let fp2 = ids.fingerprint(b"salt-beta").unwrap();

    assert_ne!(
        fp1, fp2,
        "different salts must produce different fingerprints"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  4. Encryption roundtrip via phantom_crypto::encryption
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_encryption_roundtrip_with_aad() {
    use phantom_crypto::encryption::{decrypt, encrypt};

    let mut key = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut key);

    let plaintext = b"phantom zero-footprint encrypted secret credential";
    let aad = b"vault/github/api_token";

    let blob = encrypt(plaintext, &key, aad).unwrap();
    let decrypted = decrypt(&blob, &key, aad).unwrap();

    assert_eq!(&decrypted, plaintext);
}

#[test]
fn test_encryption_wrong_key_fails() {
    use phantom_crypto::encryption::{decrypt, encrypt};

    let mut key1 = [0u8; 32];
    let mut key2 = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut key1);
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut key2);

    let blob = encrypt(b"secret", &key1, b"aad").unwrap();
    assert!(
        decrypt(&blob, &key2, b"aad").is_err(),
        "decryption with wrong key must fail"
    );
}

#[test]
fn test_encryption_wrong_aad_fails() {
    use phantom_crypto::encryption::{decrypt, encrypt};

    let mut key = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut key);

    let blob = encrypt(b"secret", &key, b"vault/correct/path").unwrap();
    assert!(
        decrypt(&blob, &key, b"vault/wrong/path").is_err(),
        "decryption with wrong AAD must fail (prevents blob-swapping)"
    );
}

#[test]
fn test_encryption_json_roundtrip() {
    use phantom_crypto::encryption::{decrypt_from_json, encrypt_to_json};

    let mut key = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut key);

    let plaintext = b"JSON-serialized encrypted blob test";
    let aad = b"state/session";

    let json = encrypt_to_json(plaintext, &key, aad).unwrap();
    assert!(json.contains("nonce"));
    assert!(json.contains("ciphertext"));

    let decrypted = decrypt_from_json(&json, &key, aad).unwrap();
    assert_eq!(&decrypted, plaintext);
}

// ═══════════════════════════════════════════════════════════════════════════
//  5. Session key → agent keys: different agents get different keys
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_session_to_agent_keys_unique_per_agent() {
    use phantom_crypto::session::{AgentPermissions, SessionKey};

    let license_bytes = b"PH1-integration-test-license-material";
    let timestamp = 1700000000u64;

    let session = SessionKey::new(license_bytes, timestamp).unwrap();

    let cto_key = session
        .derive_agent_key("cto", "planning", AgentPermissions::ALL)
        .unwrap();
    let backend_key = session
        .derive_agent_key("backend", "planning", AgentPermissions::READ_CODE)
        .unwrap();
    let frontend_key = session
        .derive_agent_key("frontend", "planning", AgentPermissions::READ_CODE)
        .unwrap();

    // Different agents must get different keys
    assert_ne!(cto_key.as_bytes(), backend_key.as_bytes());
    assert_ne!(cto_key.as_bytes(), frontend_key.as_bytes());
    assert_ne!(backend_key.as_bytes(), frontend_key.as_bytes());
}

#[test]
fn test_session_to_agent_keys_unique_per_task() {
    use phantom_crypto::session::{AgentPermissions, SessionKey};

    let session = SessionKey::new(b"PH1-license-bytes", 1000).unwrap();

    let k1 = session
        .derive_agent_key("backend", "build-api", AgentPermissions::READ_CODE)
        .unwrap();
    let k2 = session
        .derive_agent_key("backend", "write-tests", AgentPermissions::READ_CODE)
        .unwrap();

    assert_ne!(
        k1.as_bytes(),
        k2.as_bytes(),
        "same agent, different tasks must produce different keys"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
//  6. SessionKey + AgentKey with permissions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_agent_key_permissions_enforcement() {
    use phantom_crypto::session::{permissions_for_role, AgentPermissions, SessionKey};

    let session = SessionKey::new(b"PH1-perm-test", 5000).unwrap();

    // Backend agent: should have READ_CODE, WRITE_CODE, DB_QUERY but NOT DEPLOY
    let backend_perms = permissions_for_role("backend");
    let backend_key = session
        .derive_agent_key("backend", "task-1", backend_perms)
        .unwrap();

    assert!(backend_key.has_permission(AgentPermissions::READ_CODE));
    assert!(backend_key.has_permission(AgentPermissions::WRITE_CODE));
    assert!(backend_key.has_permission(AgentPermissions::DB_QUERY));
    assert!(backend_key.has_permission(AgentPermissions::SHELL_EXEC));
    assert!(!backend_key.has_permission(AgentPermissions::DEPLOY));
    assert!(!backend_key.has_permission(AgentPermissions::DESTROY));

    // CTO agent: should have ALL permissions
    let cto_perms = permissions_for_role("cto");
    let cto_key = session
        .derive_agent_key("cto", "task-1", cto_perms)
        .unwrap();

    assert!(cto_key.has_permission(AgentPermissions::ALL));
    assert!(cto_key.has_permission(AgentPermissions::DEPLOY));
    assert!(cto_key.has_permission(AgentPermissions::DESTROY));

    // Unknown role: should have NONE
    let unknown_perms = permissions_for_role("unknown");
    let unknown_key = session
        .derive_agent_key("unknown", "task-1", unknown_perms)
        .unwrap();

    assert!(!unknown_key.has_permission(AgentPermissions::READ_CODE));
    assert_eq!(unknown_key.permissions.bits(), 0);
}

#[test]
fn test_master_key_agent_keys_isolation() {
    use phantom_crypto::master_key::MasterKeySession;

    // Two different master key sessions (different salts) produce different agent keys
    let s1 = MasterKeySession::init(b"isolation-test-passphrase!!!!!!!").unwrap();
    let s2 = MasterKeySession::init(b"isolation-test-passphrase!!!!!!!").unwrap();

    let k1 = s1.derive_agent_key("backend", "task-1").unwrap();
    let k2 = s2.derive_agent_key("backend", "task-1").unwrap();

    // Different random salts → different master keys → different agent keys
    assert_ne!(
        k1.as_bytes(),
        k2.as_bytes(),
        "different sessions must produce different agent keys"
    );
}
