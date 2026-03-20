//! `phantom self-update` — Fetch and install the latest Phantom release.
//!
//! Workflow:
//!   1. Query GitHub Releases API for the latest version
//!   2. Compare with current version (skip if already up-to-date)
//!   3. Download the binary + SHA-256 checksum for the current arch
//!   4. Verify checksum
//!   5. Replace the current binary atomically
//!   6. Run `phantom doctor` as post-update bootstrap

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

/// GitHub repository coordinates.
const REPO_OWNER: &str = "benchbrex";
const REPO_NAME: &str = "phantom";
const GITHUB_API: &str = "https://api.github.com";

/// Current binary version (injected by Cargo at compile time).
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ── GitHub API Types ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
    body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

// ── Public API ─────────────────────────────────────────────────────────────

pub async fn run(force: bool) -> anyhow::Result<()> {
    println!("\x1b[1mPhantom Self-Update\x1b[0m\n");

    // Step 1: Detect current arch
    let arch_suffix = detect_arch_suffix()?;
    println!("  Current version: {}", CURRENT_VERSION);
    println!("  Architecture:    {}", arch_suffix);

    // Step 2: Fetch latest release from GitHub
    println!("\n  Checking for updates...");
    let release = fetch_latest_release().await?;
    let latest_version = release.tag_name.trim_start_matches('v');
    println!("  Latest version:  {}", latest_version);

    // Step 3: Compare versions
    if !force && latest_version == CURRENT_VERSION {
        println!("\n  \x1b[32m\u{2713}\x1b[0m Already up to date.");
        return Ok(());
    }

    if !force && !is_newer(latest_version, CURRENT_VERSION) {
        println!("\n  \x1b[32m\u{2713}\x1b[0m Current version is newer than latest release.");
        return Ok(());
    }

    println!(
        "\n  \x1b[33mUpdate available: {} → {}\x1b[0m",
        CURRENT_VERSION, latest_version
    );

    // Show release notes if available
    if let Some(ref body) = release.body {
        let preview: String = body.lines().take(5).collect::<Vec<_>>().join("\n    ");
        if !preview.is_empty() {
            println!("\n  Release notes:\n    {}", preview);
        }
    }

    // Step 4: Find the asset for our architecture
    let asset_name = format!("phantom-{}-{}.tar.gz", latest_version, arch_suffix);
    let checksum_name = format!("{}.sha256", asset_name);

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No release asset found for {}. Available: {}",
                asset_name,
                release
                    .assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    println!(
        "\n  Downloading {} ({:.1} MB)...",
        asset_name,
        asset.size as f64 / 1_048_576.0
    );

    // Step 5: Download to temp directory
    let work_dir = tempfile::tempdir()?;
    let archive_path = work_dir.path().join(&asset_name);
    let checksum_path = work_dir.path().join(&checksum_name);

    download_file(&asset.browser_download_url, &archive_path).await?;
    println!("  \x1b[32m\u{2713}\x1b[0m Downloaded");

    // Step 6: Download and verify checksum
    let checksum_asset = release.assets.iter().find(|a| a.name == checksum_name);
    if let Some(cs_asset) = checksum_asset {
        println!("  Verifying SHA-256 checksum...");
        download_file(&cs_asset.browser_download_url, &checksum_path).await?;
        verify_sha256(&archive_path, &checksum_path)?;
        println!("  \x1b[32m\u{2713}\x1b[0m Checksum verified");
    } else {
        println!("  \x1b[33m!\x1b[0m Checksum file not available — skipping verification");
    }

    // Step 7: Extract binary
    println!("  Extracting...");
    let binary_path = extract_binary(&archive_path, work_dir.path())?;
    println!("  \x1b[32m\u{2713}\x1b[0m Extracted");

    // Step 8: Replace current binary atomically
    let current_exe = env::current_exe()?;
    println!("  Installing to {}...", current_exe.display());
    replace_binary(&binary_path, &current_exe)?;
    println!("  \x1b[32m\u{2713}\x1b[0m Installed");

    // Step 9: Post-update bootstrap
    println!("\n  Running post-update dependency check...");
    let doctor_status = Command::new(&current_exe).arg("doctor").status();
    match doctor_status {
        Ok(s) if s.success() => println!("  \x1b[32m\u{2713}\x1b[0m Dependencies OK"),
        _ => println!("  \x1b[33m!\x1b[0m Run `phantom doctor` to check dependencies"),
    }

    println!("\n\x1b[32mPhantom updated to {}.\x1b[0m", latest_version);

    Ok(())
}

// ── Architecture Detection ─────────────────────────────────────────────────

fn detect_arch_suffix() -> anyhow::Result<&'static str> {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;

    if os != "macos" {
        anyhow::bail!("Self-update is only supported on macOS. Detected: {}", os);
    }

    match arch {
        "aarch64" => Ok("aarch64-apple-darwin"),
        "x86_64" => Ok("x86_64-apple-darwin"),
        other => anyhow::bail!("Unsupported architecture: {}", other),
    }
}

// ── GitHub API ─────────────────────────────────────────────────────────────

async fn fetch_latest_release() -> anyhow::Result<GitHubRelease> {
    let url = format!(
        "{}/repos/{}/{}/releases/latest",
        GITHUB_API, REPO_OWNER, REPO_NAME
    );

    let client = reqwest::Client::builder()
        .user_agent(format!("phantom-self-update/{}", CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!(
            "GitHub API returned {}: {}",
            status,
            body.chars().take(200).collect::<String>()
        );
    }

    Ok(response.json().await?)
}

// ── Download ───────────────────────────────────────────────────────────────

async fn download_file(url: &str, dest: &std::path::Path) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(format!("phantom-self-update/{}", CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(300))
        .build()?;

    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", response.status());
    }

    let bytes = response.bytes().await?;
    let mut file = fs::File::create(dest)?;
    file.write_all(&bytes)?;

    Ok(())
}

// ── Checksum Verification ──────────────────────────────────────────────────

fn verify_sha256(
    archive_path: &std::path::Path,
    checksum_path: &std::path::Path,
) -> anyhow::Result<()> {
    let checksum_content = fs::read_to_string(checksum_path)?;
    let expected_hash = checksum_content
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid checksum file format"))?
        .to_lowercase();

    // Use shasum -a 256 for verification (available on all macOS)
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(archive_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("shasum command failed");
    }

    let actual_hash = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();

    if expected_hash != actual_hash {
        anyhow::bail!(
            "SHA-256 checksum mismatch!\n  Expected: {}\n  Actual:   {}",
            expected_hash,
            actual_hash
        );
    }

    Ok(())
}

// ── Archive Extraction ─────────────────────────────────────────────────────

fn extract_binary(
    archive_path: &std::path::Path,
    work_dir: &std::path::Path,
) -> anyhow::Result<PathBuf> {
    let output = Command::new("tar")
        .args(["-xzf"])
        .arg(archive_path)
        .arg("-C")
        .arg(work_dir)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "tar extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Find the phantom binary in the extracted files
    let binary_path = work_dir.join("phantom");
    if binary_path.exists() {
        return Ok(binary_path);
    }

    // Search recursively
    for entry in fs::read_dir(work_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let nested = path.join("phantom");
            if nested.exists() {
                return Ok(nested);
            }
        }
    }

    anyhow::bail!("phantom binary not found in archive")
}

// ── Binary Replacement ─────────────────────────────────────────────────────

fn replace_binary(
    new_binary: &std::path::Path,
    current_exe: &std::path::Path,
) -> anyhow::Result<()> {
    // Make new binary executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(new_binary, fs::Permissions::from_mode(0o755))?;
    }

    // Atomic replacement: rename new binary over the old one.
    // On macOS, rename(2) is atomic within the same filesystem.
    // If the current binary is on a different mount, we fall back to copy.
    let backup_path = current_exe.with_extension("old");

    // Back up current binary
    if current_exe.exists() {
        fs::rename(current_exe, &backup_path)
            .or_else(|_| fs::copy(current_exe, &backup_path).map(|_| ()))?;
    }

    // Install new binary
    match fs::rename(new_binary, current_exe) {
        Ok(()) => {}
        Err(_) => {
            // Cross-device: fall back to copy
            fs::copy(new_binary, current_exe)?;
        }
    }

    // Remove backup
    let _ = fs::remove_file(&backup_path);

    Ok(())
}

// ── Version Comparison ─────────────────────────────────────────────────────

/// Simple semver comparison: returns true if `new` is newer than `current`.
fn is_newer(new: &str, current: &str) -> bool {
    let parse = |v: &str| -> (u64, u64, u64) {
        let parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };

    parse(new) > parse(current)
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
    }

    #[test]
    fn test_is_newer_missing_parts() {
        assert!(is_newer("1.0", "0.9.9"));
        assert!(is_newer("2", "1.9.9"));
    }

    #[test]
    fn test_current_version_set() {
        assert!(!CURRENT_VERSION.is_empty());
    }

    #[test]
    fn test_detect_arch() {
        // On macOS this should succeed; on other platforms it will fail gracefully
        let result = detect_arch_suffix();
        if cfg!(target_os = "macos") {
            assert!(result.is_ok());
            let suffix = result.unwrap();
            assert!(suffix == "aarch64-apple-darwin" || suffix == "x86_64-apple-darwin");
        }
    }

    #[test]
    fn test_github_release_deser() {
        let json = r#"{
            "tag_name": "v0.2.0",
            "assets": [
                {
                    "name": "phantom-0.2.0-aarch64-apple-darwin.tar.gz",
                    "browser_download_url": "https://example.com/phantom.tar.gz",
                    "size": 12345678
                },
                {
                    "name": "phantom-0.2.0-aarch64-apple-darwin.tar.gz.sha256",
                    "browser_download_url": "https://example.com/phantom.tar.gz.sha256",
                    "size": 128
                }
            ],
            "body": "Release notes here"
        }"#;

        let release: GitHubRelease = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v0.2.0");
        assert_eq!(release.assets.len(), 2);
        assert!(release.assets[0].name.contains("aarch64"));
        assert_eq!(release.assets[1].size, 128);
        assert!(release.body.unwrap().contains("Release notes"));
    }

    #[test]
    fn test_github_release_no_body() {
        let json = r#"{
            "tag_name": "v0.1.0",
            "assets": [],
            "body": null
        }"#;

        let release: GitHubRelease = serde_json::from_str(json).unwrap();
        assert!(release.body.is_none());
        assert!(release.assets.is_empty());
    }

    #[test]
    fn test_asset_name_format() {
        let version = "0.2.0";
        let arch = "aarch64-apple-darwin";
        let name = format!("phantom-{}-{}.tar.gz", version, arch);
        assert_eq!(name, "phantom-0.2.0-aarch64-apple-darwin.tar.gz");
    }

    #[test]
    fn test_version_tag_strip() {
        let tag = "v0.2.0";
        let version = tag.trim_start_matches('v');
        assert_eq!(version, "0.2.0");

        // Already stripped
        let tag2 = "0.2.0";
        assert_eq!(tag2.trim_start_matches('v'), "0.2.0");
    }
}
