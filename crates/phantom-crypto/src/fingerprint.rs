//! Machine fingerprinting for hardware-bound license keys.
//! HMAC-SHA256(salt, MAC || CPU_serial || disk_UUID || OS_install_UUID)

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::process::Command;

use crate::CryptoError;

type HmacSha256 = Hmac<Sha256>;

/// Collect machine-specific identifiers from macOS.
pub fn collect_machine_identifiers() -> MachineIdentifiers {
    MachineIdentifiers {
        mac_address: get_mac_address().unwrap_or_default(),
        cpu_serial: get_cpu_serial().unwrap_or_default(),
        disk_uuid: get_disk_uuid().unwrap_or_default(),
        os_install_uuid: get_os_install_uuid().unwrap_or_default(),
    }
}

/// Raw machine identifiers before hashing.
pub struct MachineIdentifiers {
    pub mac_address: String,
    pub cpu_serial: String,
    pub disk_uuid: String,
    pub os_install_uuid: String,
}

impl MachineIdentifiers {
    /// Compute HMAC-SHA256 fingerprint from collected identifiers.
    pub fn fingerprint(&self, salt: &[u8]) -> Result<[u8; 32], CryptoError> {
        let mut mac = HmacSha256::new_from_slice(salt)
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

        mac.update(self.mac_address.as_bytes());
        mac.update(b"||");
        mac.update(self.cpu_serial.as_bytes());
        mac.update(b"||");
        mac.update(self.disk_uuid.as_bytes());
        mac.update(b"||");
        mac.update(self.os_install_uuid.as_bytes());

        let result = mac.finalize().into_bytes();
        let mut fingerprint = [0u8; 32];
        fingerprint.copy_from_slice(&result);
        Ok(fingerprint)
    }
}

fn run_command(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

fn get_mac_address() -> Option<String> {
    // macOS: primary ethernet MAC
    run_command("ifconfig", &["en0"])
        .and_then(|output| {
            output
                .lines()
                .find(|l| l.contains("ether"))
                .map(|l| l.trim().to_string())
        })
}

fn get_cpu_serial() -> Option<String> {
    // macOS: hardware serial number
    run_command(
        "ioreg",
        &["-l", "-d", "2"],
    )
    .and_then(|output| {
        output
            .lines()
            .find(|l| l.contains("IOPlatformSerialNumber"))
            .and_then(|l| l.split('"').nth(3).map(|s| s.to_string()))
    })
}

fn get_disk_uuid() -> Option<String> {
    // macOS: boot volume UUID
    run_command("diskutil", &["info", "/"])
        .and_then(|output| {
            output
                .lines()
                .find(|l| l.contains("Volume UUID"))
                .and_then(|l| l.split(':').nth(1).map(|s| s.trim().to_string()))
        })
}

fn get_os_install_uuid() -> Option<String> {
    // macOS: hardware UUID
    run_command(
        "ioreg",
        &["-rd1", "-c", "IOPlatformExpertDevice"],
    )
    .and_then(|output| {
        output
            .lines()
            .find(|l| l.contains("IOPlatformUUID"))
            .and_then(|l| l.split('"').nth(3).map(|s| s.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_deterministic() {
        let ids = collect_machine_identifiers();
        let salt = b"test-license-salt";
        let fp1 = ids.fingerprint(salt).unwrap();
        let fp2 = ids.fingerprint(salt).unwrap();
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_different_salt_different_fingerprint() {
        let ids = collect_machine_identifiers();
        let fp1 = ids.fingerprint(b"salt-1").unwrap();
        let fp2 = ids.fingerprint(b"salt-2").unwrap();
        assert_ne!(fp1, fp2);
    }
}
