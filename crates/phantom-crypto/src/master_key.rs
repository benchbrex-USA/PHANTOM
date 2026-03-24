//! Master key management.
//! Core Law 2: No ownership without the master key.
//! Passphrase → Argon2id → 256-bit master key → HKDF → sub-keys.
//! Master key is NEVER stored. Derived in-memory, zeroized on drop.
//!
//! Extended with:
//! - TOTP 2FA for destructive operations (HMAC-SHA256 based)
//! - Mnemonic recovery phrase (256-word encoding of the salt)
//! - Remote destroy/kill payload generation

use crate::aes256gcm::EncryptionKey;
use crate::argon2id::{self, DerivedKey};
use crate::hkdf_keys::{self, info, DerivedSubKey};
use crate::CryptoError;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::Zeroize;

type HmacSha256 = Hmac<Sha256>;

// ── TOTP Constants ──────────────────────────────────────────────────────────

const TOTP_INFO: &[u8] = b"phantom-totp-secret-v1";
const TOTP_TIME_STEP: u64 = 30;
const TOTP_DIGITS: u32 = 6;
const TOTP_SKEW: u64 = 1; // Accept codes ±1 step

// ── MasterKeySession ────────────────────────────────────────────────────────

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
        let sub =
            hkdf_keys::derive_subkey(self.master_key.as_bytes(), None, info::INFRASTRUCTURE_KEY)?;
        Ok(EncryptionKey::from_bytes(*sub.as_bytes()))
    }

    /// Derive the storage encryption key.
    pub fn derive_storage_key(&self) -> Result<EncryptionKey, CryptoError> {
        let sub = hkdf_keys::derive_subkey(self.master_key.as_bytes(), None, info::STORAGE_KEY)?;
        Ok(EncryptionKey::from_bytes(*sub.as_bytes()))
    }

    /// Derive the license signing key material.
    pub fn derive_license_signing_material(&self) -> Result<DerivedSubKey, CryptoError> {
        hkdf_keys::derive_subkey(self.master_key.as_bytes(), None, info::LICENSE_SIGNING_KEY)
    }

    /// Derive an agent-scoped key for a specific agent + task.
    pub fn derive_agent_key(
        &self,
        agent_id: &str,
        task_id: &str,
    ) -> Result<EncryptionKey, CryptoError> {
        let info = format!("phantom-agent-key-v1:{}:{}", agent_id, task_id);
        let sub = hkdf_keys::derive_subkey(self.master_key.as_bytes(), None, info.as_bytes())?;
        Ok(EncryptionKey::from_bytes(*sub.as_bytes()))
    }

    /// Derive the destruction key for system wipe operations.
    pub fn derive_destruction_key(&self) -> Result<EncryptionKey, CryptoError> {
        let sub =
            hkdf_keys::derive_subkey(self.master_key.as_bytes(), None, info::DESTRUCTION_KEY)?;
        Ok(EncryptionKey::from_bytes(*sub.as_bytes()))
    }

    /// Set up TOTP 2FA — derives a stable secret from the master key.
    pub fn totp_setup(&self) -> Result<TotpConfig, CryptoError> {
        let sub = hkdf_keys::derive_subkey(self.master_key.as_bytes(), None, TOTP_INFO)?;
        Ok(TotpConfig {
            secret: *sub.as_bytes(),
        })
    }

    /// Generate a mnemonic backup phrase from this session's salt.
    pub fn mnemonic_backup(&self) -> MnemonicBackup {
        MnemonicBackup::from_salt(self.salt)
    }

    /// Create a signed destruction payload for remote wipe.
    pub fn create_destruction_payload(&self) -> Result<DestructionPayload, CryptoError> {
        let dest_key = self.derive_destruction_key()?;
        let hash = hex::encode(sha256_hash(dest_key.as_bytes()));
        let nonce = generate_nonce()?;
        Ok(DestructionPayload {
            timestamp: chrono::Utc::now().timestamp(),
            destruction_key_hash: hash,
            nonce,
        })
    }

    /// Create a kill payload to remotely terminate a specific installation.
    pub fn create_kill_payload(&self, target_id: &str) -> Result<RemoteKillPayload, CryptoError> {
        let session_key = self.derive_session_key()?;
        let hash = hex::encode(sha256_hash(session_key.as_bytes()));
        let nonce = generate_nonce()?;
        Ok(RemoteKillPayload {
            target_id: target_id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            session_invalidation_hash: hash,
            nonce,
        })
    }

    /// Rotate all derived keys by generating a new salt and re-deriving.
    /// Returns the new session with a fresh salt.
    pub fn rotate(passphrase: &[u8]) -> Result<Self, CryptoError> {
        Self::init(passphrase)
    }
}

impl Drop for MasterKeySession {
    fn drop(&mut self) {
        // master_key (DerivedKey) auto-zeroizes via its own Drop impl.
        // We must also zeroize the salt to prevent residual key-derivation material.
        self.salt.zeroize();
    }
}

impl std::fmt::Debug for MasterKeySession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED MasterKeySession]")
    }
}

// ── TOTP 2FA ────────────────────────────────────────────────────────────────

/// TOTP configuration derived from the master key.
/// Uses HMAC-SHA256 with 6-digit codes and 30-second time steps.
pub struct TotpConfig {
    secret: [u8; 32],
}

impl Drop for TotpConfig {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

impl std::fmt::Debug for TotpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED TotpConfig]")
    }
}

impl TotpConfig {
    /// Generate the current TOTP code.
    pub fn generate(&self) -> Result<String, CryptoError> {
        self.generate_at(current_timestamp()?)
    }

    /// Generate a TOTP code for a specific unix timestamp.
    pub fn generate_at(&self, timestamp: u64) -> Result<String, CryptoError> {
        let counter = timestamp / TOTP_TIME_STEP;
        totp_hmac_sha256(&self.secret, counter)
    }

    /// Verify a TOTP code (accepts ±1 time step for clock skew).
    pub fn verify(&self, code: &str) -> Result<bool, CryptoError> {
        self.verify_at(code, current_timestamp()?)
    }

    /// Verify a TOTP code at a specific unix timestamp.
    pub fn verify_at(&self, code: &str, timestamp: u64) -> Result<bool, CryptoError> {
        let counter = timestamp / TOTP_TIME_STEP;
        for offset in 0..=TOTP_SKEW {
            for c in [counter.wrapping_sub(offset), counter.wrapping_add(offset)] {
                let generated = totp_hmac_sha256(&self.secret, c)?;
                if constant_time_eq(code.as_bytes(), generated.as_bytes()) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Hex-encoded secret for provisioning to an authenticator app.
    pub fn secret_hex(&self) -> String {
        hex::encode(self.secret)
    }

    /// Base32-encoded secret (standard for OTP URIs).
    pub fn secret_base32(&self) -> String {
        base32_encode(&self.secret)
    }

    /// Generate an otpauth:// URI for QR code provisioning.
    pub fn provisioning_uri(&self, account: &str) -> String {
        format!(
            "otpauth://totp/Phantom:{}?secret={}&issuer=Phantom&algorithm=SHA256&digits={}&period={}",
            account,
            self.secret_base32(),
            TOTP_DIGITS,
            TOTP_TIME_STEP
        )
    }
}

fn current_timestamp() -> Result<u64, CryptoError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| CryptoError::KeyDerivationFailed("system time error".into()))
}

fn totp_hmac_sha256(secret: &[u8], counter: u64) -> Result<String, CryptoError> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();

    // Dynamic truncation (RFC 4226 adapted for SHA-256 — 32 bytes)
    let offset = (result[31] & 0x0f) as usize;
    let code = u32::from_be_bytes([
        result[offset] & 0x7f,
        result[offset + 1],
        result[offset + 2],
        result[offset + 3],
    ]) % 10u32.pow(TOTP_DIGITS);

    Ok(format!("{:0>width$}", code, width = TOTP_DIGITS as usize))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn base32_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::new();
    let mut buffer: u64 = 0;
    let mut bits_left = 0;
    for &byte in data {
        buffer = (buffer << 8) | byte as u64;
        bits_left += 8;
        while bits_left >= 5 {
            bits_left -= 5;
            result.push(ALPHABET[((buffer >> bits_left) & 0x1f) as usize] as char);
        }
    }
    if bits_left > 0 {
        result.push(ALPHABET[((buffer << (5 - bits_left)) & 0x1f) as usize] as char);
    }
    result
}

// ── Mnemonic Recovery ───────────────────────────────────────────────────────

/// A mnemonic backup phrase encoding the master key salt.
/// Uses a 256-word list (8 bits per word, 32 words for 256-bit salt).
pub struct MnemonicBackup {
    phrase: String,
    salt: [u8; 32],
}

impl MnemonicBackup {
    /// Generate a mnemonic phrase from a salt.
    pub fn from_salt(salt: [u8; 32]) -> Self {
        let phrase = encode_mnemonic(&salt);
        Self { phrase, salt }
    }

    /// Restore the salt from a mnemonic phrase.
    pub fn restore(phrase: &str) -> Result<[u8; 32], CryptoError> {
        decode_mnemonic(phrase)
    }

    /// Restore a full MasterKeySession from a mnemonic phrase and passphrase.
    pub fn restore_session(
        phrase: &str,
        passphrase: &[u8],
    ) -> Result<MasterKeySession, CryptoError> {
        let salt = Self::restore(phrase)?;
        MasterKeySession::new(passphrase, salt)
    }

    pub fn phrase(&self) -> &str {
        &self.phrase
    }

    pub fn salt(&self) -> &[u8; 32] {
        &self.salt
    }

    /// Verify that a phrase is valid (correct word count and all words in wordlist).
    pub fn verify(phrase: &str) -> bool {
        decode_mnemonic(phrase).is_ok()
    }

    pub fn word_count(&self) -> usize {
        self.phrase.split_whitespace().count()
    }
}

impl Drop for MnemonicBackup {
    fn drop(&mut self) {
        self.salt.zeroize();
        // Zeroize the phrase string's backing allocation
        // SAFETY: phrase is a String; we zeroize its bytes then clear.
        unsafe {
            let bytes = self.phrase.as_mut_vec();
            bytes.zeroize();
        }
    }
}

impl std::fmt::Debug for MnemonicBackup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED MnemonicBackup]")
    }
}

fn encode_mnemonic(bytes: &[u8; 32]) -> String {
    bytes
        .iter()
        .map(|&b| WORDLIST[b as usize])
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_mnemonic(phrase: &str) -> Result<[u8; 32], CryptoError> {
    let words: Vec<&str> = phrase.split_whitespace().collect();
    if words.len() != 32 {
        return Err(CryptoError::InvalidMnemonic(format!(
            "expected 32 words, got {}",
            words.len()
        )));
    }

    let mut bytes = [0u8; 32];
    for (i, word) in words.iter().enumerate() {
        let idx = WORDLIST
            .iter()
            .position(|&w| w == *word)
            .ok_or_else(|| CryptoError::InvalidMnemonic(format!("unknown word: {}", word)))?;
        bytes[i] = idx as u8;
    }

    Ok(bytes)
}

// ── Remote Operation Payloads ───────────────────────────────────────────────

/// Signed payload for `phantom destroy` — full system wipe.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DestructionPayload {
    pub timestamp: i64,
    pub destruction_key_hash: String,
    pub nonce: String,
}

/// Signed payload for `phantom kill` — remote termination of one installation.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RemoteKillPayload {
    pub target_id: String,
    pub timestamp: i64,
    pub session_invalidation_hash: String,
    pub nonce: String,
}

impl DestructionPayload {
    /// Verify the payload hasn't expired (5-minute window).
    pub fn is_valid_timestamp(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        (now - self.timestamp).abs() < 300
    }
}

impl RemoteKillPayload {
    /// Verify the payload hasn't expired (5-minute window).
    pub fn is_valid_timestamp(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        (now - self.timestamp).abs() < 300
    }

    /// Serialize to JSON for transmission.
    pub fn to_json(&self) -> Result<String, CryptoError> {
        serde_json::to_string(self).map_err(|e| CryptoError::Serialization(e.to_string()))
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn sha256_hash(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

fn generate_nonce() -> Result<String, CryptoError> {
    let mut nonce_bytes = [0u8; 16];
    rand::RngCore::try_fill_bytes(&mut rand::thread_rng(), &mut nonce_bytes)
        .map_err(|_| CryptoError::RngFailed)?;
    Ok(hex::encode(nonce_bytes))
}

// ── Mnemonic Wordlist (first 256 words of BIP-39 English) ───────────────────

const WORDLIST: [&str; 256] = [
    "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd",
    "abuse", "access", "accident", "account", "accuse", "achieve", "acid", "acoustic", "acquire",
    "across", "act", "action", "actual", "adapt", "add", "addict", "address", "adjust", "admit",
    "adult", "advance", "advice", "aerobic", "affair", "afford", "afraid", "again", "age", "agent",
    "agree", "ahead", "aim", "air", "airport", "aisle", "alarm", "album", "alcohol", "alert",
    "alien", "all", "alley", "allow", "almost", "alone", "alpha", "already", "also", "alter",
    "always", "amateur", "amazing", "among", "amount", "amused", "analyst", "anchor", "ancient",
    "anger", "angle", "angry", "animal", "ankle", "announce", "annual", "another", "answer",
    "antenna", "antique", "anxiety", "any", "apart", "apology", "appear", "apple", "approve",
    "april", "arch", "arctic", "area", "arena", "argue", "arm", "armed", "armor", "army", "around",
    "arrange", "arrest", "arrive", "arrow", "art", "artefact", "artist", "artwork", "ask",
    "aspect", "assault", "asset", "assist", "assume", "asthma", "athlete", "atom", "attack",
    "attend", "attitude", "attract", "auction", "audit", "august", "aunt", "author", "auto",
    "autumn", "average", "avocado", "avoid", "awake", "aware", "awesome", "awful", "awkward",
    "axis", "baby", "bachelor", "bacon", "badge", "bag", "balance", "balcony", "ball", "bamboo",
    "banana", "banner", "bar", "barely", "bargain", "barrel", "base", "basic", "basket", "battle",
    "beach", "bean", "beauty", "because", "become", "beef", "before", "begin", "behave", "behind",
    "believe", "below", "belt", "bench", "benefit", "best", "betray", "better", "between",
    "beyond", "bicycle", "bid", "bike", "bind", "biology", "bird", "birth", "bitter", "black",
    "blade", "blame", "blanket", "blast", "bleak", "bless", "blind", "blood", "blossom", "blow",
    "blue", "blur", "blush", "board", "boat", "body", "boil", "bomb", "bone", "bonus", "book",
    "boost", "border", "boring", "borrow", "boss", "bottom", "bounce", "box", "boy", "bracket",
    "brain", "brand", "brass", "brave", "bread", "breeze", "brick", "bridge", "brief", "bright",
    "bring", "brisk", "broccoli", "broken", "bronze", "broom", "brother", "brown", "brush",
    "bubble", "buddy", "budget", "buffalo", "build", "bulb", "bulk", "bullet", "bundle", "bunny",
    "burden", "burger", "burst", "bus", "business", "busy", "butter", "buyer", "buzz", "cabbage",
    "cabin", "cable", "cactus", "cage", "cake",
];

// ── Tests ───────────────────────────────────────────────────────────────────

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

    // ── TOTP Tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_totp_setup_deterministic() {
        let salt = [42u8; 32];
        let s1 = MasterKeySession::new(b"totp-test-passphrase-32-chars!!!", salt).unwrap();
        let s2 = MasterKeySession::new(b"totp-test-passphrase-32-chars!!!", salt).unwrap();

        let t1 = s1.totp_setup().unwrap();
        let t2 = s2.totp_setup().unwrap();

        assert_eq!(t1.secret, t2.secret);
    }

    #[test]
    fn test_totp_generate_and_verify() {
        let session =
            MasterKeySession::new(b"totp-verify-test-passphrase!!!!!", [77u8; 32]).unwrap();
        let totp = session.totp_setup().unwrap();

        let timestamp = 1700000000u64; // fixed timestamp
        let code = totp.generate_at(timestamp).unwrap();

        assert_eq!(code.len(), 6);
        assert!(totp.verify_at(&code, timestamp).unwrap());
    }

    #[test]
    fn test_totp_verify_with_skew() {
        let session =
            MasterKeySession::new(b"totp-skew-test-passphrase!!!!!!!", [88u8; 32]).unwrap();
        let totp = session.totp_setup().unwrap();

        let timestamp = 1700000000u64;
        let code = totp.generate_at(timestamp).unwrap();

        // Should verify at +30s (one step forward)
        assert!(totp.verify_at(&code, timestamp + 30).unwrap());
    }

    #[test]
    fn test_totp_reject_wrong_code() {
        let session =
            MasterKeySession::new(b"totp-reject-test-passphrase!!!!!", [66u8; 32]).unwrap();
        let totp = session.totp_setup().unwrap();

        assert!(!totp.verify_at("000000", 1700000000).unwrap());
    }

    #[test]
    fn test_totp_different_sessions_different_codes() {
        let s1 = MasterKeySession::new(b"totp-diff-1-passphrase!!!!!!!!!!", [11u8; 32]).unwrap();
        let s2 = MasterKeySession::new(b"totp-diff-2-passphrase!!!!!!!!!!", [22u8; 32]).unwrap();

        let c1 = s1.totp_setup().unwrap().generate_at(1700000000).unwrap();
        let c2 = s2.totp_setup().unwrap().generate_at(1700000000).unwrap();

        assert_ne!(c1, c2);
    }

    #[test]
    fn test_totp_provisioning_uri() {
        let session =
            MasterKeySession::new(b"totp-uri-test-passphrase!!!!!!!!", [55u8; 32]).unwrap();
        let totp = session.totp_setup().unwrap();

        let uri = totp.provisioning_uri("parth@benchbrex.com");
        assert!(uri.starts_with("otpauth://totp/Phantom:"));
        assert!(uri.contains("parth@benchbrex.com"));
        assert!(uri.contains("algorithm=SHA256"));
    }

    // ── Mnemonic Tests ──────────────────────────────────────────────────────

    #[test]
    fn test_mnemonic_roundtrip() {
        let salt = [42u8; 32];
        let session = MasterKeySession::new(b"mnemonic-test-passphrase!!!!!!!!", salt).unwrap();

        let backup = session.mnemonic_backup();
        assert_eq!(backup.word_count(), 32);

        let restored_salt = MnemonicBackup::restore(backup.phrase()).unwrap();
        assert_eq!(&restored_salt, &salt);
    }

    #[test]
    fn test_mnemonic_restore_session() {
        let passphrase = b"mnemonic-session-restore-test!!!";
        let salt = [7u8; 32];
        let original = MasterKeySession::new(passphrase, salt).unwrap();
        let phrase = original.mnemonic_backup().phrase().to_string();

        let restored = MnemonicBackup::restore_session(&phrase, passphrase).unwrap();

        let k1 = original.derive_session_key().unwrap();
        let k2 = restored.derive_session_key().unwrap();
        assert_eq!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn test_mnemonic_verify_valid() {
        let backup = MnemonicBackup::from_salt([0u8; 32]);
        assert!(MnemonicBackup::verify(backup.phrase()));
    }

    #[test]
    fn test_mnemonic_verify_invalid_word() {
        assert!(!MnemonicBackup::verify("notaword abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon"));
    }

    #[test]
    fn test_mnemonic_verify_wrong_count() {
        assert!(!MnemonicBackup::verify("abandon ability able"));
    }

    #[test]
    fn test_mnemonic_all_zero_salt() {
        let backup = MnemonicBackup::from_salt([0u8; 32]);
        // All zeros → first word repeated 32 times
        let words: Vec<&str> = backup.phrase().split_whitespace().collect();
        assert!(words.iter().all(|&w| w == "abandon"));
    }

    // ── Destruction/Kill Payload Tests ──────────────────────────────────────

    #[test]
    fn test_destruction_key_differs_from_session() {
        let session =
            MasterKeySession::new(b"destruction-test-passphrase!!!!!", [33u8; 32]).unwrap();

        let dest = session.derive_destruction_key().unwrap();
        let sess = session.derive_session_key().unwrap();
        assert_ne!(dest.as_bytes(), sess.as_bytes());
    }

    #[test]
    fn test_destruction_payload_creation() {
        let session =
            MasterKeySession::new(b"payload-test-passphrase!!!!!!!!!!", [44u8; 32]).unwrap();

        let payload = session.create_destruction_payload().unwrap();
        assert!(payload.is_valid_timestamp());
        assert!(!payload.destruction_key_hash.is_empty());
        assert_eq!(payload.nonce.len(), 32); // 16 bytes hex-encoded
    }

    #[test]
    fn test_kill_payload_creation() {
        let session =
            MasterKeySession::new(b"kill-payload-test-passphrase!!!!", [55u8; 32]).unwrap();

        let payload = session.create_kill_payload("inst-abc-123").unwrap();
        assert!(payload.is_valid_timestamp());
        assert_eq!(payload.target_id, "inst-abc-123");
        assert!(!payload.session_invalidation_hash.is_empty());
    }

    #[test]
    fn test_kill_payload_serialization() {
        let session =
            MasterKeySession::new(b"kill-serial-test-passphrase!!!!!", [66u8; 32]).unwrap();

        let payload = session.create_kill_payload("target-x").unwrap();
        let json = payload.to_json().unwrap();
        let parsed: RemoteKillPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.target_id, "target-x");
    }

    #[test]
    fn test_key_rotation() {
        let s1 = MasterKeySession::rotate(b"rotation-test-passphrase!!!!!!!!");
        let s2 = MasterKeySession::rotate(b"rotation-test-passphrase!!!!!!!!");

        // Different salts → different keys (salts are random)
        let s1 = s1.unwrap();
        let s2 = s2.unwrap();
        assert_ne!(s1.salt(), s2.salt());
        assert_ne!(
            s1.derive_session_key().unwrap().as_bytes(),
            s2.derive_session_key().unwrap().as_bytes()
        );
    }

    // ── Helper Tests ────────────────────────────────────────────────────────

    #[test]
    fn test_base32_encode() {
        assert_eq!(base32_encode(b""), "");
        assert_eq!(base32_encode(b"f"), "MY");
        assert_eq!(base32_encode(b"fo"), "MZXQ");
        assert_eq!(base32_encode(b"foo"), "MZXW6");
        assert_eq!(base32_encode(b"foob"), "MZXW6YQ");
        assert_eq!(base32_encode(b"fooba"), "MZXW6YTB");
        assert_eq!(base32_encode(b"foobar"), "MZXW6YTBOI");
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"123456", b"123456"));
        assert!(!constant_time_eq(b"123456", b"654321"));
        assert!(!constant_time_eq(b"12345", b"123456"));
    }

    #[test]
    fn test_wordlist_has_256_entries() {
        assert_eq!(WORDLIST.len(), 256);
    }

    #[test]
    fn test_wordlist_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for word in &WORDLIST {
            assert!(seen.insert(word), "duplicate word: {}", word);
        }
    }
}
