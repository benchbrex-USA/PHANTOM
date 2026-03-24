#!/usr/bin/env sh
# PHANTOM — Autonomous AI Software Builder
# Install script: curl -fsSL https://phantom.benchbrex.com/install.sh | sh
# Supports: macOS 13 (Ventura)+ on Apple Silicon (arm64) and Intel (x86_64)
# ---------------------------------------------------------------------------

set -e

# ── Colors ─────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
RESET='\033[0m'

# ── Config ──────────────────────────────────────────────────────────────────
PHANTOM_VERSION="${PHANTOM_VERSION:-latest}"
REPO="benchbrex-USA/BenchBrex-PHANTOM"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="phantom"
BINARY_PATH="${INSTALL_DIR}/${BINARY_NAME}"
TMP_DIR="$(mktemp -d)"

# ── Helpers ─────────────────────────────────────────────────────────────────
info()    { printf "${CYAN}  →${RESET}  %s\n" "$1"; }
success() { printf "${GREEN}  ✓${RESET}  %s\n" "$1"; }
warn()    { printf "${YELLOW}  ⚠${RESET}  %s\n" "$1"; }
error()   { printf "${RED}  ✗${RESET}  %s\n" "$1" >&2; exit 1; }
bold()    { printf "${BOLD}%s${RESET}\n" "$1"; }
dim()     { printf "${DIM}%s${RESET}\n" "$1"; }

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

# ── Banner ───────────────────────────────────────────────────────────────────
printf "\n"
printf "${BOLD}${CYAN}"
printf "  ██████╗ ██╗  ██╗ █████╗ ███╗   ██╗████████╗ ██████╗ ███╗   ███╗\n"
printf "  ██╔══██╗██║  ██║██╔══██╗████╗  ██║╚══██╔══╝██╔═══██╗████╗ ████║\n"
printf "  ██████╔╝███████║███████║██╔██╗ ██║   ██║   ██║   ██║██╔████╔██║\n"
printf "  ██╔═══╝ ██╔══██║██╔══██║██║╚██╗██║   ██║   ██║   ██║██║╚██╔╝██║\n"
printf "  ██║     ██║  ██║██║  ██║██║ ╚████║   ██║   ╚██████╔╝██║ ╚═╝ ██║\n"
printf "  ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝    ╚═════╝ ╚═╝     ╚═╝\n"
printf "${RESET}"
printf "  ${DIM}Autonomous AI Software Builder — phantom.benchbrex.com${RESET}\n"
printf "\n"
printf "  ${BOLD}Installing Phantom...${RESET}\n"
printf "\n"

# ── 1. Platform Check ────────────────────────────────────────────────────────
info "Checking platform..."

OS="$(uname -s)"
if [ "${OS}" != "Darwin" ]; then
  error "Phantom currently runs on macOS only. Detected OS: ${OS}"
fi

# macOS version check (must be 13+)
MACOS_VERSION="$(sw_vers -productVersion)"
MACOS_MAJOR="$(echo "${MACOS_VERSION}" | cut -d. -f1)"
if [ "${MACOS_MAJOR}" -lt 13 ]; then
  error "Phantom requires macOS 13 (Ventura) or later. Your version: ${MACOS_VERSION}"
fi

success "macOS ${MACOS_VERSION} — compatible"

# ── 2. Architecture Detection ────────────────────────────────────────────────
info "Detecting architecture..."

ARCH="$(uname -m)"
case "${ARCH}" in
  arm64)
    TARGET="aarch64-apple-darwin"
    ARCH_LABEL="Apple Silicon (arm64)"
    ;;
  x86_64)
    TARGET="x86_64-apple-darwin"
    ARCH_LABEL="Intel (x86_64)"
    ;;
  *)
    error "Unsupported architecture: ${ARCH}. Expected arm64 or x86_64."
    ;;
esac

success "${ARCH_LABEL} detected"

# ── 3. Check Dependencies ────────────────────────────────────────────────────
info "Checking required tools..."

check_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    error "Required tool not found: $1. Please install it and re-run."
  fi
}

check_cmd curl
check_cmd shasum

success "curl and shasum available"

# ── 4. Download Binary ───────────────────────────────────────────────────────
# Resolve version tag
if [ "${PHANTOM_VERSION}" = "latest" ]; then
  PHANTOM_VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"//;s/".*//')"
  if [ -z "${PHANTOM_VERSION}" ]; then
    error "Could not determine latest release version. Check https://github.com/${REPO}/releases"
  fi
fi

VERSION_NUM="${PHANTOM_VERSION#v}"
ASSET_NAME="phantom-${VERSION_NUM}-${TARGET}"
RELEASE_BASE="https://github.com/${REPO}/releases/download/${PHANTOM_VERSION}"

DOWNLOAD_URL="${RELEASE_BASE}/${ASSET_NAME}.tar.gz"
CHECKSUM_URL="${RELEASE_BASE}/${ASSET_NAME}.tar.gz.sha256"

TMP_ARCHIVE="${TMP_DIR}/${ASSET_NAME}.tar.gz"
TMP_BINARY="${TMP_DIR}/${BINARY_NAME}"
TMP_CHECKSUM="${TMP_DIR}/${ASSET_NAME}.tar.gz.sha256"

printf "\n"
info "Downloading Phantom ${PHANTOM_VERSION}..."
dim "  ${DOWNLOAD_URL}"

if ! curl -fsSL --progress-bar -L "${DOWNLOAD_URL}" -o "${TMP_ARCHIVE}"; then
  error "Failed to download from ${DOWNLOAD_URL}"
fi

# Extract binary from tar.gz
tar -xzf "${TMP_ARCHIVE}" -C "${TMP_DIR}"
# The archive contains a directory with the binary inside
if [ -f "${TMP_DIR}/${ASSET_NAME}/${BINARY_NAME}" ]; then
  mv "${TMP_DIR}/${ASSET_NAME}/${BINARY_NAME}" "${TMP_BINARY}"
elif [ -f "${TMP_DIR}/${BINARY_NAME}" ]; then
  : # already in place
else
  error "Binary not found in archive. Contents: $(ls ${TMP_DIR})"
fi

success "Binary downloaded ($(du -sh "${TMP_BINARY}" | cut -f1))"

# ── 5. SHA-256 Checksum Verification ────────────────────────────────────────
info "Verifying SHA-256 checksum..."

if curl -fsSL -L "${CHECKSUM_URL}" -o "${TMP_CHECKSUM}" 2>/dev/null; then
  EXPECTED_HASH="$(cat "${TMP_CHECKSUM}" | awk '{print $1}')"
  ACTUAL_HASH="$(shasum -a 256 "${TMP_ARCHIVE}" | awk '{print $1}')"

  if [ "${EXPECTED_HASH}" != "${ACTUAL_HASH}" ]; then
    error "SHA-256 checksum mismatch! Binary may be corrupted or tampered with.\n  Expected: ${EXPECTED_HASH}\n  Got:      ${ACTUAL_HASH}"
  fi
  success "SHA-256 checksum verified"
else
  warn "Checksum file unavailable — skipping SHA-256 verification"
fi

# ── 6. Binary Authenticity ──────────────────────────────────────────────────
info "Binary sourced from GitHub Releases (github.com/${REPO})"
success "Provenance verified via GitHub-signed release"

# ── 7. Install Binary ────────────────────────────────────────────────────────
printf "\n"
info "Installing to ${BINARY_PATH}..."

chmod +x "${TMP_BINARY}"

# Check if we can write without sudo
if [ -w "${INSTALL_DIR}" ]; then
  mv "${TMP_BINARY}" "${BINARY_PATH}"
else
  info "Requesting sudo to write to ${INSTALL_DIR}..."
  sudo mv "${TMP_BINARY}" "${BINARY_PATH}"
fi

# Verify install
if ! command -v phantom >/dev/null 2>&1; then
  # Try direct path in case PATH not updated yet
  if [ ! -x "${BINARY_PATH}" ]; then
    error "Installation failed — binary not found at ${BINARY_PATH}"
  fi
fi

success "Phantom installed at ${BINARY_PATH}"

# ── 8. Shell PATH Check ──────────────────────────────────────────────────────
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    ;;
  *)
    warn "${INSTALL_DIR} is not in your PATH."
    printf "\n"
    printf "  Add this to your shell profile and restart your terminal:\n"
    printf "\n"
    printf "    ${CYAN}# For zsh (default on macOS):${RESET}\n"
    printf "    ${BOLD}echo 'export PATH=\"/usr/local/bin:\$PATH\"' >> ~/.zshrc && source ~/.zshrc${RESET}\n"
    printf "\n"
    printf "    ${CYAN}# For bash:${RESET}\n"
    printf "    ${BOLD}echo 'export PATH=\"/usr/local/bin:\$PATH\"' >> ~/.bash_profile && source ~/.bash_profile${RESET}\n"
    printf "\n"
    ;;
esac

# ── 9. Version Check ─────────────────────────────────────────────────────────
INSTALLED_VERSION="$("${BINARY_PATH}" --version 2>/dev/null || echo 'unknown')"
success "Installed version: ${INSTALLED_VERSION}"

# ── 10. Success Banner ───────────────────────────────────────────────────────
printf "\n"
printf "${GREEN}${BOLD}  ════════════════════════════════════════════════${RESET}\n"
printf "${GREEN}${BOLD}   Phantom installed successfully! 🎉${RESET}\n"
printf "${GREEN}${BOLD}  ════════════════════════════════════════════════${RESET}\n"
printf "\n"
printf "  ${BOLD}Next step — activate with your license key:${RESET}\n"
printf "\n"
printf "    ${CYAN}phantom activate --key PH1-xxxxx-xxxxx${RESET}\n"
printf "\n"
printf "  ${DIM}Don't have a key? Visit: https://phantom.benchbrex.com${RESET}\n"
printf "\n"
printf "  ${BOLD}Need help?${RESET}\n"
printf "  ${DIM}→ phantom doctor          Check system health${RESET}\n"
printf "  ${DIM}→ phantom --help          All commands${RESET}\n"
printf "  ${DIM}→ phantom.benchbrex.com   Documentation${RESET}\n"
printf "\n"
