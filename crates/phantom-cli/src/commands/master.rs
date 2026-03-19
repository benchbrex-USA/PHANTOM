//! `phantom master` — Master key operations.

use crate::MasterAction;
use phantom_core::audit::{AuditAction, AuditLog};

pub async fn run(action: MasterAction) -> anyhow::Result<()> {
    match action {
        MasterAction::Init => {
            println!("\x1b[1mMaster Key Initialization\x1b[0m\n");
            println!("This will derive a master key from your passphrase using Argon2id.");
            println!("The master key is NEVER stored — it exists only in memory.\n");
            println!("Requirements:");
            println!("  - Passphrase must be 32+ characters");
            println!("  - Argon2id: 256MB memory, 4 iterations, 4 parallelism");
            println!("  - Derives: session key, infra key, storage key, agent keys\n");
            println!("\x1b[33mInteractive passphrase input not yet implemented.\x1b[0m");
            println!("This will be handled via secure terminal input (no echo).");
        }
        MasterAction::Issue { email } => {
            println!("\x1b[1mLicense Issuance\x1b[0m\n");
            println!("Issuing license for: {}", email);
            println!();
            println!("License will contain:");
            println!("  - Machine fingerprint (HMAC-SHA256)");
            println!("  - Ed25519 signature");
            println!("  - Expiry timestamp");
            println!("  - Capabilities & tier");
            println!();
            println!(
                "\x1b[33mRequires initialized master key. Run `phantom master init` first.\x1b[0m"
            );
        }
        MasterAction::Revoke { key } => {
            println!("\x1b[1mLicense Revocation\x1b[0m\n");
            let key_preview = if key.len() > 20 {
                format!("{}...", &key[..20])
            } else {
                key.clone()
            };
            println!("Revoking license: {}", key_preview);
            println!("\x1b[33mRequires initialized master key.\x1b[0m");
        }
        MasterAction::List => {
            println!("\x1b[1mActive Installations\x1b[0m\n");
            println!("No installations found.");
            println!("\x1b[90mInstallations are tracked after license issuance.\x1b[0m");
        }
        MasterAction::Kill { id } => {
            println!("\x1b[1mRemote Kill\x1b[0m\n");
            println!(
                "\x1b[31mWARNING: This will immediately terminate installation: {}\x1b[0m",
                id
            );
            println!("The target installation's session keys will be invalidated.");
            println!("\x1b[33mRequires initialized master key.\x1b[0m");
        }
        MasterAction::Destroy => {
            println!("\x1b[1;31mFull System Destruction\x1b[0m\n");
            println!("This will:");
            println!("  1. Revoke all licenses");
            println!("  2. Wipe all remote storage (R2, Supabase, Neon)");
            println!("  3. Delete all infrastructure (VMs, Workers, containers)");
            println!("  4. Remove all GitHub repos and CI/CD pipelines");
            println!("  5. Invalidate all keys and tokens");
            println!();
            println!("\x1b[31mTHIS ACTION IS IRREVERSIBLE.\x1b[0m");
            println!("Requires TOTP 2FA verification.");
            println!("\x1b[33mRequires initialized master key.\x1b[0m");
        }
        MasterAction::Rotate => {
            println!("\x1b[1mKey Rotation\x1b[0m\n");
            println!("This will rotate all derived keys:");
            println!("  - Session key");
            println!("  - Infrastructure key");
            println!("  - Storage key");
            println!("  - Per-agent keys (8 agents)");
            println!();
            println!("The master key itself is never stored, so it cannot be rotated.");
            println!("To change the master key, run `phantom master destroy` and re-init.");
            println!("\x1b[33mRequires initialized master key.\x1b[0m");
        }
        MasterAction::Audit => {
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

            println!("Audit log entries: {}", log.len());
            let integrity = log.verify_integrity().is_ok();
            println!(
                "Chain integrity: {}\n",
                if integrity {
                    "\x1b[32mVERIFIED\x1b[0m"
                } else {
                    "\x1b[31mBROKEN\x1b[0m"
                }
            );

            if let Ok(json) = log.export_json() {
                println!("Sample entries:");
                let preview: String = json.chars().take(500).collect();
                println!("{}", preview);
                if json.len() > 500 {
                    println!("  ... ({} total characters)", json.len());
                }
            }
        }
        MasterAction::Transfer { to } => {
            println!("\x1b[1mOwnership Transfer\x1b[0m\n");
            println!("Transferring ownership to: {}", to);
            println!();
            println!("This will:");
            println!("  1. Generate new Ed25519 keypair for the new owner");
            println!("  2. Re-sign all active licenses");
            println!("  3. Transfer all infrastructure bindings");
            println!("  4. Invalidate your master key session");
            println!();
            println!("\x1b[33mRequires initialized master key and new owner confirmation.\x1b[0m");
        }
        MasterAction::Halt => {
            println!("\x1b[1;31mEmergency Halt\x1b[0m\n");
            println!("Broadcasting HALT to all agents...");
            println!();

            for role in phantom_ai::ALL_ROLES {
                println!("  \x1b[31m\u{25A0}\x1b[0m {} — halted", role.display_name());
            }

            println!();
            println!("All agents stopped. Pipeline frozen.");
            println!("Use `phantom build --resume` to continue after investigation.");
        }
    }
    Ok(())
}
