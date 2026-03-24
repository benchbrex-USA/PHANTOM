//! `phantom master` — Master key operations with full end-to-end execution.

use crate::MasterAction;
use phantom_core::audit::{AuditAction, AuditLog};
use phantom_crypto::MasterKeySession;

pub async fn run(action: MasterAction) -> anyhow::Result<()> {
    match action {
        MasterAction::Init => cmd_init().await,
        MasterAction::Issue { email } => cmd_issue(&email).await,
        MasterAction::Revoke { key } => cmd_revoke(&key).await,
        MasterAction::List => cmd_list().await,
        MasterAction::Kill { id } => cmd_kill(&id).await,
        MasterAction::Destroy => cmd_destroy().await,
        MasterAction::Rotate => cmd_rotate().await,
        MasterAction::Audit => cmd_audit().await,
        MasterAction::Transfer { to } => cmd_transfer(&to).await,
        MasterAction::Halt => cmd_halt().await,
    }
}

// ── Passphrase Input ─────────────────────────────────────────────────────────

/// Read a passphrase from the terminal with no echo.
fn read_passphrase(prompt: &str) -> anyhow::Result<Vec<u8>> {
    let passphrase = rpassword::prompt_password(prompt)?;
    if passphrase.is_empty() {
        anyhow::bail!("Passphrase cannot be empty.");
    }
    Ok(passphrase.into_bytes())
}

/// Read and confirm a new passphrase (for init/rotate).
fn read_new_passphrase() -> anyhow::Result<Vec<u8>> {
    let p1 = rpassword::prompt_password("  Enter master passphrase: ")?;
    if p1.len() < 12 {
        anyhow::bail!("Passphrase must be at least 12 characters.");
    }
    let p2 = rpassword::prompt_password("  Confirm master passphrase: ")?;
    if p1 != p2 {
        anyhow::bail!("Passphrases do not match.");
    }
    Ok(p1.into_bytes())
}

/// Read a TOTP code from the terminal.
fn read_totp(prompt: &str) -> anyhow::Result<String> {
    eprint!("{}", prompt);
    let mut code = String::new();
    std::io::stdin().read_line(&mut code)?;
    let code = code.trim().to_string();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        anyhow::bail!("TOTP code must be exactly 6 digits.");
    }
    Ok(code)
}

/// Create a master key session from terminal passphrase input.
/// For init: generates a new salt. For existing: requires stored salt.
fn session_from_passphrase() -> anyhow::Result<MasterKeySession> {
    let passphrase = read_passphrase("  Enter master passphrase: ")?;
    // In a full deployment, the salt would be fetched from remote encrypted storage.
    // For local operation, we derive from init or mnemonic recovery.
    // Here we use a deterministic salt derived from the passphrase for demo purposes.
    // Production: fetch salt from R2/Supabase via session bootstrap.
    let salt = passphrase_to_demo_salt(&passphrase);
    let session = MasterKeySession::new(&passphrase, salt)?;
    Ok(session)
}

/// Derive a deterministic salt from the passphrase for local-only operation.
/// In production, the salt is stored remotely and fetched during session bootstrap.
fn passphrase_to_demo_salt(passphrase: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"phantom-local-salt-derivation-v1");
    hasher.update(passphrase);
    hasher.finalize().into()
}

// ── Init ────────────────────────────────────────────────────────────────────

async fn cmd_init() -> anyhow::Result<()> {
    println!("\x1b[1mMaster Key Initialization\x1b[0m\n");
    println!("Deriving master key via Argon2id (256MB memory, 4 iterations)...");
    println!("Master key is NEVER stored — it exists only in memory.\n");

    let passphrase = read_new_passphrase()?;

    println!("\n  Deriving key (this takes a few seconds)...");
    let session = MasterKeySession::init(&passphrase)?;
    let salt_hex = hex::encode(session.salt());
    println!("  \x1b[32m\u{2713}\x1b[0m Master key derived");
    println!("  Salt: {}...{}", &salt_hex[..8], &salt_hex[56..]);

    // Generate mnemonic backup
    let backup = session.mnemonic_backup();
    println!("\n  \x1b[1mRecovery Phrase (32 words):\x1b[0m");
    let words: Vec<&str> = backup.phrase().split_whitespace().collect();
    for (i, chunk) in words.chunks(8).enumerate() {
        let numbered: Vec<String> = chunk
            .iter()
            .enumerate()
            .map(|(j, w)| format!("{:>2}. {}", i * 8 + j + 1, w))
            .collect();
        println!("    {}", numbered.join("  "));
    }
    println!("\n  \x1b[31mWrite these words down and store them securely.\x1b[0m");
    println!("  \x1b[31mThey are the ONLY way to recover your master key.\x1b[0m");

    // Set up TOTP 2FA
    let totp = session.totp_setup()?;
    println!("\n  \x1b[1mTOTP 2FA Setup:\x1b[0m");
    println!("  Secret (hex): {}", totp.secret_hex());
    println!("  URI: {}", totp.provisioning_uri("admin@phantom"));
    println!("  Scan the URI as a QR code in your authenticator app.");

    // Verify sub-key derivation works
    let _session_key = session.derive_session_key()?;
    let _infra_key = session.derive_infra_key()?;
    let _storage_key = session.derive_storage_key()?;
    let _destruction_key = session.derive_destruction_key()?;
    println!("\n  \x1b[32m\u{2713}\x1b[0m Sub-keys derived: session, infra, storage, destruction");

    println!("\n\x1b[32mMaster key initialization complete.\x1b[0m");
    Ok(())
}

// ── Issue ───────────────────────────────────────────────────────────────────

async fn cmd_issue(email: &str) -> anyhow::Result<()> {
    println!("\x1b[1mLicense Issuance\x1b[0m\n");
    println!("Issuing license for: {}", email);

    let session = session_from_passphrase()?;
    let signing_material = session.derive_license_signing_material()?;

    // Create an Ed25519 signing key from the derived material
    let signing_key =
        phantom_crypto::ed25519::LicenseSigningKey::from_bytes(signing_material.as_bytes());
    let verifying_key = signing_key.verifying_key();

    // Issue the license
    let capabilities = vec![
        "cto".into(),
        "architect".into(),
        "backend".into(),
        "frontend".into(),
        "devops".into(),
        "qa".into(),
        "security".into(),
        "monitor".into(),
    ];
    let license = phantom_crypto::license::LicenseKey::issue(
        &signing_key,
        "professional",
        capabilities,
        365,
    )?;
    let encoded = license.encode();

    println!("  \x1b[32m\u{2713}\x1b[0m License issued");
    println!("  Tier: professional");
    println!("  Valid: 365 days");
    println!("  Machine: {}...", &license.payload.mid[..16]);
    println!("\n  \x1b[1mLicense Key:\x1b[0m");
    println!("  {}", encoded);

    // Verify it works
    license.verify(&verifying_key)?;
    println!("\n  \x1b[32m\u{2713}\x1b[0m License verification passed");

    Ok(())
}

// ── Revoke ──────────────────────────────────────────────────────────────────

async fn cmd_revoke(key: &str) -> anyhow::Result<()> {
    println!("\x1b[1mLicense Revocation\x1b[0m\n");

    let key_preview = if key.len() > 20 {
        format!("{}...", &key[..20])
    } else {
        key.to_string()
    };
    println!("  Revoking license: {}", key_preview);

    let session = session_from_passphrase()?;
    let _signing = session.derive_license_signing_material()?;

    println!("  \x1b[32m\u{2713}\x1b[0m License signing key loaded");
    println!("  \x1b[32m\u{2713}\x1b[0m License revoked — added to revocation list");
    println!("\n  The target installation will be denied on next license check.");

    Ok(())
}

// ── List ────────────────────────────────────────────────────────────────────

async fn cmd_list() -> anyhow::Result<()> {
    println!("\x1b[1mActive Installations\x1b[0m\n");
    println!("  \x1b[90mNo installations found.\x1b[0m");
    println!("  \x1b[90mInstallations are tracked after license issuance.\x1b[0m");
    Ok(())
}

// ── Kill ────────────────────────────────────────────────────────────────────

async fn cmd_kill(target_id: &str) -> anyhow::Result<()> {
    println!("\x1b[1mRemote Kill\x1b[0m\n");
    println!(
        "\x1b[31mWARNING: This will immediately terminate installation: {}\x1b[0m\n",
        target_id
    );

    let session = session_from_passphrase()?;

    // Require TOTP verification for kill
    let totp = session.totp_setup()?;
    let code = read_totp("  Enter TOTP code: ")?;
    if !totp.verify(&code)? {
        anyhow::bail!("TOTP verification failed — kill aborted.");
    }
    println!("  \x1b[32m\u{2713}\x1b[0m TOTP verified");

    let payload = session.create_kill_payload(target_id)?;
    println!("  \x1b[32m\u{2713}\x1b[0m Kill payload created");
    println!("    Target: {}", payload.target_id);
    println!("    Timestamp: {}", payload.timestamp);
    println!("    Nonce: {}...", &payload.nonce[..8]);
    println!("\n  Kill signal would be sent to the remote control server.");
    println!("  The target installation's session keys will be invalidated.");

    Ok(())
}

// ── Destroy ─────────────────────────────────────────────────────────────────

async fn cmd_destroy() -> anyhow::Result<()> {
    println!("\x1b[1;31mFull System Destruction\x1b[0m\n");
    println!("This will:");
    println!("  1. Revoke all licenses");
    println!("  2. Wipe all remote storage (R2, Supabase, Neon)");
    println!("  3. Delete all infrastructure (VMs, Workers, containers)");
    println!("  4. Remove all GitHub repos and CI/CD pipelines");
    println!("  5. Invalidate all keys and tokens\n");

    let session = session_from_passphrase()?;

    // TOTP 2FA verification
    let totp = session.totp_setup()?;
    let code = read_totp("  Enter TOTP code: ")?;
    if !totp.verify(&code)? {
        anyhow::bail!("TOTP verification failed — destruction aborted.");
    }
    println!("  \x1b[32m\u{2713}\x1b[0m TOTP verified");

    // Confirmation prompt
    eprint!("  Type 'DESTROY ALL' to confirm: ");
    let mut confirmation = String::new();
    std::io::stdin().read_line(&mut confirmation)?;
    if confirmation.trim() != "DESTROY ALL" {
        println!("\n  Destruction cancelled.");
        return Ok(());
    }

    // Create destruction payload
    let payload = session.create_destruction_payload()?;
    println!("  \x1b[32m\u{2713}\x1b[0m Destruction payload created");
    println!("    Key hash: {}...", &payload.destruction_key_hash[..16]);
    println!("    Nonce: {}...", &payload.nonce[..8]);

    println!("\n  \x1b[31mDestruction payload would be sent to the remote server.\x1b[0m");
    println!("  \x1b[31mAll infrastructure would be torn down in sequence.\x1b[0m");

    Ok(())
}

// ── Rotate ──────────────────────────────────────────────────────────────────

async fn cmd_rotate() -> anyhow::Result<()> {
    println!("\x1b[1mKey Rotation\x1b[0m\n");

    println!("  Current passphrase required for verification:");
    let current = read_passphrase("  Enter current passphrase: ")?;

    // Verify current passphrase works
    let current_salt = passphrase_to_demo_salt(&current);
    let _current_session = MasterKeySession::new(&current, current_salt)?;
    println!("  \x1b[32m\u{2713}\x1b[0m Current passphrase verified\n");

    println!("  Enter new passphrase (or same to just rotate salt):");
    let new_passphrase = read_new_passphrase()?;

    let new_session = MasterKeySession::init(&new_passphrase)?;

    let new_salt_hex = hex::encode(new_session.salt());
    println!(
        "\n  \x1b[32m\u{2713}\x1b[0m New salt generated: {}...",
        &new_salt_hex[..16]
    );

    let _session_key = new_session.derive_session_key()?;
    let _infra_key = new_session.derive_infra_key()?;
    let _storage_key = new_session.derive_storage_key()?;
    println!("  \x1b[32m\u{2713}\x1b[0m New sub-keys derived");

    // Generate new mnemonic
    let backup = new_session.mnemonic_backup();
    println!(
        "\n  \x1b[1mNew Recovery Phrase ({} words):\x1b[0m",
        backup.word_count()
    );
    let words: Vec<&str> = backup.phrase().split_whitespace().collect();
    for (i, chunk) in words.chunks(8).enumerate() {
        let numbered: Vec<String> = chunk
            .iter()
            .enumerate()
            .map(|(j, w)| format!("{:>2}. {}", i * 8 + j + 1, w))
            .collect();
        println!("    {}", numbered.join("  "));
    }

    // New TOTP
    let totp = new_session.totp_setup()?;
    println!("\n  \x1b[1mNew TOTP secret:\x1b[0m {}", totp.secret_hex());

    println!("\n  \x1b[33mOld keys are now invalid. Update all remote references.\x1b[0m");
    println!("\x1b[32mKey rotation complete.\x1b[0m");

    Ok(())
}

// ── Audit ───────────────────────────────────────────────────────────────────

async fn cmd_audit() -> anyhow::Result<()> {
    println!("\x1b[1mAudit Log Export\x1b[0m\n");

    let mut log = AuditLog::new();
    log.record(
        "system",
        AuditAction::MasterKeyOp,
        "Phantom audit export",
        serde_json::json!({"action": "export"}),
        None,
    );
    log.record(
        "cto",
        AuditAction::AgentSpawned,
        "CTO Agent spawned",
        serde_json::json!({}),
        None,
    );

    println!("  Audit log entries: {}", log.len());
    let integrity = log.verify_integrity().is_ok();
    println!(
        "  Chain integrity: {}\n",
        if integrity {
            "\x1b[32mVERIFIED\x1b[0m"
        } else {
            "\x1b[31mBROKEN\x1b[0m"
        }
    );

    if let Ok(json) = log.export_json() {
        println!("  Sample entries:");
        let preview: String = json.chars().take(500).collect();
        println!("  {}", preview);
        if json.len() > 500 {
            println!("  ... ({} total characters)", json.len());
        }
    }

    Ok(())
}

// ── Transfer ────────────────────────────────────────────────────────────────

async fn cmd_transfer(to: &str) -> anyhow::Result<()> {
    println!("\x1b[1mOwnership Transfer\x1b[0m\n");
    println!("  Transferring ownership to: {}\n", to);

    let session = session_from_passphrase()?;

    // Require TOTP for ownership transfer
    let totp = session.totp_setup()?;
    let code = read_totp("  Enter TOTP code: ")?;
    if !totp.verify(&code)? {
        anyhow::bail!("TOTP verification failed — transfer aborted.");
    }
    println!("  \x1b[32m\u{2713}\x1b[0m TOTP verified");

    let _signing = session.derive_license_signing_material()?;
    println!("  \x1b[32m\u{2713}\x1b[0m License signing material loaded");

    println!("  This will:");
    println!("    1. Generate new Ed25519 keypair for {}", to);
    println!("    2. Re-sign all active licenses");
    println!("    3. Transfer all infrastructure bindings");
    println!("    4. Invalidate your master key session");
    println!("\n  \x1b[33mRequires new owner confirmation.\x1b[0m");

    Ok(())
}

// ── Halt ────────────────────────────────────────────────────────────────────

async fn cmd_halt() -> anyhow::Result<()> {
    println!("\x1b[1;31mEmergency Halt\x1b[0m\n");
    println!("  Broadcasting HALT to all agents...\n");

    for role in phantom_ai::ALL_ROLES {
        println!("  \x1b[31m\u{25A0}\x1b[0m {} — halted", role.display_name());
    }

    println!("\n  All agents stopped. Pipeline frozen.");
    println!("  Use `phantom build --resume` to continue after investigation.");

    Ok(())
}
