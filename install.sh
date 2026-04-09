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
BASE_URL="https://phantom.benchbrex.com/releases/latest"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="phantom"
BINARY_PATH="${INSTALL_DIR}/${BINARY_NAME}"
TMP_DIR="$(mktemp -d)"

# ── Helpers ─────────────────────────────────────────────────────────────────
info()    { printf "  ${DIM}│${RESET}  ${DIM}→${RESET}  %s\n" "$1"; }
success() { printf "  ${DIM}│${RESET}  ${GREEN}✓${RESET}  %s\n" "$1"; }
warn()    { printf "  ${DIM}│${RESET}  ${YELLOW}!${RESET}  %s\n" "$1"; }
error()   { printf "  ${DIM}│${RESET}  ${RED}✗${RESET}  %s\n" "$1" >&2; exit 1; }
step()    { printf "\n  ${BOLD}%s${RESET} ${DIM}·${RESET} %s\n" "$1" "$2"; }

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

# ── Banner ───────────────────────────────────────────────────────────────────
printf "\n"
printf "  ${BOLD}${CYAN}phantom${RESET}\n"
printf "  ${DIM}autonomous AI engineering system${RESET}\n"
printf "  ${DIM}────────────────────────────────────────────${RESET}\n"

# ── 1. Platform Check ────────────────────────────────────────────────────────
step "Step 1" "Platform detection"

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

success "macOS ${MACOS_VERSION}"

# ── 2. Architecture Detection ────────────────────────────────────────────────
ARCH="$(uname -m)"
case "${ARCH}" in
  arm64)
    ARCH_SUFFIX="arm64"
    ARCH_LABEL="Apple Silicon"
    ;;
  x86_64)
    ARCH_SUFFIX="x64"
    ARCH_LABEL="Intel x86_64"
    ;;
  *)
    error "Unsupported architecture: ${ARCH}"
    ;;
esac

success "${ARCH_LABEL}"

# ── 3. Check Dependencies ────────────────────────────────────────────────────
step "Step 2" "Dependencies"

check_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    error "Required tool not found: $1"
  fi
}

check_cmd curl
check_cmd shasum

success "curl, shasum"

# ── 4. Download Binary ───────────────────────────────────────────────────────
DOWNLOAD_URL="${BASE_URL}/phantom-darwin-${ARCH_SUFFIX}.tar.gz"
CHECKSUM_URL="${BASE_URL}/phantom-darwin-${ARCH_SUFFIX}.tar.gz.sha256"

TMP_ARCHIVE="${TMP_DIR}/phantom.tar.gz"
TMP_BINARY="${TMP_DIR}/${BINARY_NAME}"
TMP_CHECKSUM="${TMP_DIR}/phantom.sha256"

step "Step 3" "Download"
info "${DIM}${DOWNLOAD_URL}${RESET}"

if ! curl -fsSL --progress-bar -L "${DOWNLOAD_URL}" -o "${TMP_ARCHIVE}"; then
  error "Failed to download from ${DOWNLOAD_URL}"
fi

# Extract binary from tar.gz
tar -xzf "${TMP_ARCHIVE}" -C "${TMP_DIR}"
# Find the phantom binary in extracted contents
if [ ! -f "${TMP_BINARY}" ]; then
  # Binary may be inside a subdirectory
  FOUND_BINARY="$(find "${TMP_DIR}" -name "${BINARY_NAME}" -type f | head -1)"
  if [ -n "${FOUND_BINARY}" ]; then
    mv "${FOUND_BINARY}" "${TMP_BINARY}"
  else
    error "Binary not found in archive"
  fi
fi

success "Downloaded $(du -sh "${TMP_BINARY}" | cut -f1 | tr -d ' ')"

# ── 5. SHA-256 Checksum Verification ────────────────────────────────────────
step "Step 4" "Integrity verification"

if curl -fsSL -L "${CHECKSUM_URL}" -o "${TMP_CHECKSUM}" 2>/dev/null; then
  EXPECTED_HASH="$(cat "${TMP_CHECKSUM}" | awk '{print $1}')"
  ACTUAL_HASH="$(shasum -a 256 "${TMP_ARCHIVE}" | awk '{print $1}')"

  if [ "${EXPECTED_HASH}" != "${ACTUAL_HASH}" ]; then
    error "SHA-256 checksum mismatch — binary may be corrupted or tampered with"
  fi
  success "SHA-256 verified"
else
  warn "Checksum file unavailable — skipping verification"
fi

info "Sourced from phantom.benchbrex.com"

# ── 6. Install Binary ────────────────────────────────────────────────────────
step "Step 5" "Install"

chmod +x "${TMP_BINARY}"

# Check if we can write without sudo
if [ -w "${INSTALL_DIR}" ]; then
  mv "${TMP_BINARY}" "${BINARY_PATH}"
else
  info "Requesting sudo to write to ${INSTALL_DIR}"
  sudo mv "${TMP_BINARY}" "${BINARY_PATH}"
fi

# Verify install
if ! command -v phantom >/dev/null 2>&1; then
  if [ ! -x "${BINARY_PATH}" ]; then
    error "Installation failed — binary not found at ${BINARY_PATH}"
  fi
fi

success "Installed to ${BINARY_PATH}"

# ── 7. Shell PATH Check ──────────────────────────────────────────────────────
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    ;;
  *)
    warn "${INSTALL_DIR} is not in your PATH"
    printf "\n"
    printf "  ${DIM}│${RESET}  Add to your shell profile:\n"
    printf "\n"
    printf "  ${DIM}│${RESET}  ${DIM}# zsh (default on macOS):${RESET}\n"
    printf "  ${DIM}│${RESET}  ${BOLD}echo 'export PATH=\"/usr/local/bin:\$PATH\"' >> ~/.zshrc${RESET}\n"
    printf "\n"
    ;;
esac

# ── 8. Version Check ─────────────────────────────────────────────────────────
INSTALLED_VERSION="$("${BINARY_PATH}" --version 2>/dev/null || echo 'unknown')"
success "Version: ${INSTALLED_VERSION}"

# ── 9. Success ──────────────────────────────────────────────────────────────
printf "\n"
printf "  ${DIM}────────────────────────────────────────────${RESET}\n"
printf "  ${GREEN}●${RESET} ${BOLD}Phantom installed successfully.${RESET}\n"
printf "\n"
printf "  ${BOLD}Next step${RESET} ${DIM}·${RESET} activate with your license key:\n"
printf "\n"
printf "  ${DIM}│${RESET}  ${CYAN}phantom activate --key PH1-xxxxx-xxxxx${RESET}\n"
printf "\n"
printf "  ${DIM}Don't have a key? Visit phantom.benchbrex.com${RESET}\n"
printf "\n"
printf "  ${BOLD}Commands${RESET}\n"
printf "  ${DIM}│${RESET}  phantom doctor     ${DIM}· system health check${RESET}\n"
printf "  ${DIM}│${RESET}  phantom --help     ${DIM}· all commands${RESET}\n"
printf "  ${DIM}│${RESET}  phantom status     ${DIM}· system overview${RESET}\n"
printf "\n"
