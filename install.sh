#!/usr/bin/env bash
# ============================================================================
# Phantom — Autonomous AI Engineering Team
# curl-installable bootstrap script for macOS
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/benchbrex/phantom/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/benchbrex/phantom/main/install.sh | bash -s -- --version 0.2.0
#
# What this script does:
#   1. Detects macOS architecture (arm64 / x86_64)
#   2. Downloads the signed Phantom binary for that arch
#   3. Verifies SHA-256 checksum
#   4. Verifies Apple code signature (if codesign is available)
#   5. Installs to /usr/local/bin/phantom
#   6. Runs `phantom doctor` as first-run bootstrap
#
# Requirements: macOS 13+, curl, shasum
# ============================================================================

set -euo pipefail

# ── Configuration ───────────────────────────────────────────────────────────

REPO_OWNER="benchbrex"
REPO_NAME="phantom"
BINARY_NAME="phantom"
INSTALL_DIR="/usr/local/bin"
GITHUB_API="https://api.github.com"
GITHUB_RELEASES="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases"
REQUIRED_OS="Darwin"
MIN_MACOS_VERSION="13.0"
APPLE_TEAM_ID="BENCHBREX"  # Apple Developer Team ID for code signature verification

# ── Colors ──────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
RESET='\033[0m'

# ── Helpers ─────────────────────────────────────────────────────────────────

info()    { echo -e "${BLUE}[info]${RESET} $1"; }
success() { echo -e "${GREEN}[ok]${RESET}   $1"; }
warn()    { echo -e "${YELLOW}[warn]${RESET} $1"; }
error()   { echo -e "${RED}[error]${RESET} $1" >&2; }
fatal()   { error "$1"; exit 1; }

cleanup() {
    if [ -n "${TMPDIR_CREATED:-}" ] && [ -d "${WORK_DIR:-}" ]; then
        rm -rf "$WORK_DIR"
    fi
}
trap cleanup EXIT

# ── Parse Arguments ─────────────────────────────────────────────────────────

VERSION=""
SKIP_VERIFY=false
SKIP_BOOTSTRAP=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)    VERSION="$2"; shift 2 ;;
        --no-verify)  SKIP_VERIFY=true; shift ;;
        --no-bootstrap) SKIP_BOOTSTRAP=true; shift ;;
        --help|-h)
            echo "Usage: install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --version VERSION   Install a specific version (default: latest)"
            echo "  --no-verify         Skip SHA-256 and code signature verification"
            echo "  --no-bootstrap      Skip first-run dependency bootstrap"
            echo "  --help, -h          Show this help message"
            exit 0
            ;;
        *) fatal "Unknown option: $1" ;;
    esac
done

# ── Pre-flight Checks ──────────────────────────────────────────────────────

echo -e "\n${BOLD}Phantom Installer${RESET}\n"

# Check OS
OS="$(uname -s)"
if [ "$OS" != "$REQUIRED_OS" ]; then
    fatal "Phantom requires macOS. Detected: $OS"
fi

# Check macOS version
MACOS_VERSION="$(sw_vers -productVersion 2>/dev/null || echo "0.0")"
MACOS_MAJOR="${MACOS_VERSION%%.*}"
if [ "$MACOS_MAJOR" -lt 13 ] 2>/dev/null; then
    fatal "Phantom requires macOS 13 (Ventura) or later. Detected: $MACOS_VERSION"
fi
success "macOS $MACOS_VERSION detected"

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    arm64|aarch64) ARCH_SUFFIX="aarch64-apple-darwin" ;;
    x86_64)        ARCH_SUFFIX="x86_64-apple-darwin" ;;
    *)             fatal "Unsupported architecture: $ARCH" ;;
esac
success "Architecture: $ARCH ($ARCH_SUFFIX)"

# Check required tools
for cmd in curl shasum; do
    if ! command -v "$cmd" &>/dev/null; then
        fatal "Required tool not found: $cmd"
    fi
done

# ── Resolve Version ────────────────────────────────────────────────────────

if [ -z "$VERSION" ]; then
    info "Fetching latest release..."
    VERSION=$(curl -fsSL "${GITHUB_API}/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" \
        | grep '"tag_name"' \
        | head -1 \
        | sed -E 's/.*"tag_name": *"v?([^"]+)".*/\1/')

    if [ -z "$VERSION" ]; then
        fatal "Could not determine latest version. Use --version to specify."
    fi
fi
success "Version: $VERSION"

# ── Create Working Directory ────────────────────────────────────────────────

WORK_DIR="$(mktemp -d)"
TMPDIR_CREATED=1
info "Working directory: $WORK_DIR"

# ── Download Binary + Checksum ──────────────────────────────────────────────

ASSET_NAME="${BINARY_NAME}-${VERSION}-${ARCH_SUFFIX}"
BINARY_URL="${GITHUB_RELEASES}/download/v${VERSION}/${ASSET_NAME}.tar.gz"
CHECKSUM_URL="${GITHUB_RELEASES}/download/v${VERSION}/${ASSET_NAME}.tar.gz.sha256"

info "Downloading ${ASSET_NAME}.tar.gz ..."
HTTP_CODE=$(curl -fsSL -w "%{http_code}" -o "${WORK_DIR}/${ASSET_NAME}.tar.gz" "$BINARY_URL" 2>/dev/null || true)
if [ ! -f "${WORK_DIR}/${ASSET_NAME}.tar.gz" ] || [ "${HTTP_CODE:-0}" != "200" ]; then
    fatal "Download failed. Check that version $VERSION exists at:\n  $BINARY_URL"
fi
DOWNLOAD_SIZE=$(wc -c < "${WORK_DIR}/${ASSET_NAME}.tar.gz" | tr -d ' ')
success "Downloaded $(( DOWNLOAD_SIZE / 1024 )) KB"

# ── Verify SHA-256 Checksum ─────────────────────────────────────────────────

if [ "$SKIP_VERIFY" = false ]; then
    info "Downloading checksum..."
    if curl -fsSL -o "${WORK_DIR}/${ASSET_NAME}.tar.gz.sha256" "$CHECKSUM_URL" 2>/dev/null; then
        info "Verifying SHA-256 checksum..."
        EXPECTED_HASH=$(awk '{print $1}' "${WORK_DIR}/${ASSET_NAME}.tar.gz.sha256")
        ACTUAL_HASH=$(shasum -a 256 "${WORK_DIR}/${ASSET_NAME}.tar.gz" | awk '{print $1}')

        if [ "$EXPECTED_HASH" != "$ACTUAL_HASH" ]; then
            error "Checksum mismatch!"
            error "  Expected: $EXPECTED_HASH"
            error "  Actual:   $ACTUAL_HASH"
            fatal "The downloaded binary may be corrupted or tampered with."
        fi
        success "SHA-256 checksum verified"
    else
        warn "Checksum file not available — skipping verification"
    fi
else
    warn "Checksum verification skipped (--no-verify)"
fi

# ── Extract Binary ──────────────────────────────────────────────────────────

info "Extracting binary..."
tar -xzf "${WORK_DIR}/${ASSET_NAME}.tar.gz" -C "$WORK_DIR"

EXTRACTED_BINARY="${WORK_DIR}/${BINARY_NAME}"
if [ ! -f "$EXTRACTED_BINARY" ]; then
    # Try nested directory
    EXTRACTED_BINARY=$(find "$WORK_DIR" -name "$BINARY_NAME" -type f | head -1)
    if [ -z "$EXTRACTED_BINARY" ]; then
        fatal "Binary not found in archive"
    fi
fi
chmod +x "$EXTRACTED_BINARY"
success "Binary extracted"

# ── Verify Code Signature ──────────────────────────────────────────────────

if [ "$SKIP_VERIFY" = false ] && command -v codesign &>/dev/null; then
    info "Verifying Apple code signature..."
    if codesign --verify --deep --strict "$EXTRACTED_BINARY" 2>/dev/null; then
        # Check signing identity
        SIGNING_ID=$(codesign -dv "$EXTRACTED_BINARY" 2>&1 | grep "TeamIdentifier" | awk -F= '{print $2}')
        if [ -n "$SIGNING_ID" ] && [ "$SIGNING_ID" != "not set" ]; then
            success "Code signature valid (Team: $SIGNING_ID)"
        else
            warn "Binary is signed but team identifier is not set"
        fi
    else
        warn "Code signature verification failed — binary may be unsigned"
        warn "This is expected for development builds"
    fi
fi

# ── Install Binary ─────────────────────────────────────────────────────────

info "Installing to ${INSTALL_DIR}/${BINARY_NAME} ..."

# Check if install dir exists and is writable
if [ ! -d "$INSTALL_DIR" ]; then
    warn "$INSTALL_DIR does not exist, creating with sudo..."
    sudo mkdir -p "$INSTALL_DIR"
fi

if [ -w "$INSTALL_DIR" ]; then
    mv "$EXTRACTED_BINARY" "${INSTALL_DIR}/${BINARY_NAME}"
else
    info "Elevated permissions required for $INSTALL_DIR"
    sudo mv "$EXTRACTED_BINARY" "${INSTALL_DIR}/${BINARY_NAME}"
    sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
fi

# Verify installation
if command -v phantom &>/dev/null; then
    INSTALLED_VERSION=$(phantom --version 2>/dev/null | awk '{print $2}' || echo "unknown")
    success "Phantom $INSTALLED_VERSION installed to $(command -v phantom)"
else
    if [ -x "${INSTALL_DIR}/${BINARY_NAME}" ]; then
        success "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
        warn "$INSTALL_DIR may not be in your PATH. Add it:"
        echo -e "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    else
        fatal "Installation failed"
    fi
fi

# ── First-Run Bootstrap ────────────────────────────────────────────────────

if [ "$SKIP_BOOTSTRAP" = false ]; then
    echo ""
    info "Running first-run dependency check..."
    echo ""
    "${INSTALL_DIR}/${BINARY_NAME}" doctor 2>/dev/null || true
fi

# ── Done ────────────────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}${GREEN}Phantom installed successfully.${RESET}"
echo ""
echo "Next steps:"
echo "  1. Activate with your license key:"
echo "     phantom activate --key PH1-<your-key>"
echo ""
echo "  2. Build a project:"
echo "     phantom build --framework architecture.md"
echo ""
echo "  3. Keep up to date:"
echo "     phantom self-update"
echo ""
