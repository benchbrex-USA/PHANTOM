#!/usr/bin/env sh
# PHANTOM — Autonomous AI Software Builder
# Uninstall script: curl -fsSL https://raw.githubusercontent.com/benchbrex-USA/PHANTOM/main/uninstall.sh | sh
# Completely removes PHANTOM and all associated data from your system.
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
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="phantom"
BINARY_PATH="${INSTALL_DIR}/${BINARY_NAME}"
PHANTOM_DATA_DIR="${HOME}/.phantom"
LAUNCH_AGENTS_DIR="${HOME}/Library/LaunchAgents"

# ── Helpers ─────────────────────────────────────────────────────────────────
info()    { printf "${CYAN}  →${RESET}  %s\n" "$1"; }
success() { printf "${GREEN}  ✓${RESET}  %s\n" "$1"; }
warn()    { printf "${YELLOW}  ⚠${RESET}  %s\n" "$1"; }
error()   { printf "${RED}  ✗${RESET}  %s\n" "$1" >&2; exit 1; }
bold()    { printf "${BOLD}%s${RESET}\n" "$1"; }
dim()     { printf "${DIM}%s${RESET}\n" "$1"; }

removed_count=0
track() { removed_count=$((removed_count + 1)); }

# ── Banner ───────────────────────────────────────────────────────────────────
printf "\n"
printf "${BOLD}${RED}"
printf "  ██████╗ ██╗  ██╗ █████╗ ███╗   ██╗████████╗ ██████╗ ███╗   ███╗\n"
printf "  ██╔══██╗██║  ██║██╔══██╗████╗  ██║╚══██╔══╝██╔═══██╗████╗ ████║\n"
printf "  ██████╔╝███████║███████║██╔██╗ ██║   ██║   ██║   ██║██╔████╔██║\n"
printf "  ██╔═══╝ ██╔══██║██╔══██║██║╚██╗██║   ██║   ██║   ██║██║╚██╔╝██║\n"
printf "  ██║     ██║  ██║██║  ██║██║ ╚████║   ██║   ╚██████╔╝██║ ╚═╝ ██║\n"
printf "  ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝    ╚═════╝ ╚═╝     ╚═╝\n"
printf "${RESET}"
printf "  ${DIM}Autonomous AI Software Builder — phantom.benchbrex.com${RESET}\n"
printf "\n"
printf "  ${BOLD}${RED}Uninstalling Phantom...${RESET}\n"
printf "\n"

# ── Confirmation ─────────────────────────────────────────────────────────────
# Skip confirmation if --yes flag is passed (for scripted uninstalls)
SKIP_CONFIRM=false
for arg in "$@"; do
  case "$arg" in
    --yes|-y) SKIP_CONFIRM=true ;;
  esac
done

if [ "$SKIP_CONFIRM" = false ]; then
  printf "  ${YELLOW}${BOLD}WARNING: This will completely remove Phantom and all its data.${RESET}\n"
  printf "  ${DIM}This includes: binary, config, state, daemons, keychain entries, and session artifacts.${RESET}\n"
  printf "\n"
  printf "  ${BOLD}What would you like to remove?${RESET}\n"
  printf "\n"
  printf "    ${CYAN}1)${RESET}  ${BOLD}Everything${RESET}       — Complete removal (binary + all data + daemons + keychain)\n"
  printf "    ${CYAN}2)${RESET}  ${BOLD}Binary only${RESET}      — Remove just the phantom binary, keep config & data\n"
  printf "    ${CYAN}3)${RESET}  ${BOLD}Data only${RESET}        — Remove config & data, keep the binary\n"
  printf "    ${CYAN}4)${RESET}  ${BOLD}Cancel${RESET}           — Abort uninstall\n"
  printf "\n"
  printf "  ${BOLD}Enter choice [1-4]:${RESET} "
  read -r CHOICE < /dev/tty

  case "$CHOICE" in
    1) REMOVE_BINARY=true;  REMOVE_DATA=true  ;;
    2) REMOVE_BINARY=true;  REMOVE_DATA=false ;;
    3) REMOVE_BINARY=false; REMOVE_DATA=true  ;;
    4|"")
      printf "\n"
      success "Uninstall cancelled. Phantom remains installed."
      printf "\n"
      exit 0
      ;;
    *)
      error "Invalid choice. Aborting."
      ;;
  esac

  printf "\n"
  printf "  ${YELLOW}Are you sure? This cannot be undone. [y/N]:${RESET} "
  read -r CONFIRM < /dev/tty
  case "$CONFIRM" in
    y|Y|yes|YES) ;;
    *)
      printf "\n"
      success "Uninstall cancelled. Phantom remains installed."
      printf "\n"
      exit 0
      ;;
  esac
else
  REMOVE_BINARY=true
  REMOVE_DATA=true
fi

printf "\n"

# ── 1. Stop Running Phantom Processes ────────────────────────────────────────
info "Stopping any running Phantom processes..."

PHANTOM_PIDS="$(pgrep -f "${BINARY_NAME}" 2>/dev/null || true)"
if [ -n "${PHANTOM_PIDS}" ]; then
  for pid in ${PHANTOM_PIDS}; do
    # Don't kill ourselves
    if [ "$pid" != "$$" ]; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  # Wait briefly for graceful shutdown
  sleep 1
  # Force kill any remaining
  for pid in ${PHANTOM_PIDS}; do
    if [ "$pid" != "$$" ]; then
      kill -9 "$pid" 2>/dev/null || true
    fi
  done
  success "Phantom processes stopped"
  track
else
  dim "    No running Phantom processes found"
fi

# ── 2. Unload & Remove LaunchAgent Daemons ──────────────────────────────────
if [ "$REMOVE_DATA" = true ]; then
  info "Removing Phantom LaunchAgent daemons..."

  if [ -d "${LAUNCH_AGENTS_DIR}" ]; then
    FOUND_PLISTS=false
    for plist in "${LAUNCH_AGENTS_DIR}"/com.phantom.*.plist; do
      [ -f "$plist" ] || continue
      FOUND_PLISTS=true
      LABEL="$(basename "$plist" .plist)"

      # Unload the daemon first
      launchctl unload "$plist" 2>/dev/null || true
      launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true

      # Remove the plist file
      rm -f "$plist"
      success "Removed daemon: ${LABEL}"
      track
    done

    if [ "$FOUND_PLISTS" = false ]; then
      dim "    No Phantom LaunchAgent plists found"
    fi
  else
    dim "    LaunchAgents directory not found"
  fi
fi

# ── 3. Remove macOS Keychain Entries ─────────────────────────────────────────
if [ "$REMOVE_DATA" = true ]; then
  info "Removing Phantom Keychain entries..."

  KEYCHAIN_CLEANED=false
  # Remove all com.phantom.* service entries from the login keychain
  while true; do
    # Try to find and delete a phantom keychain entry
    if security find-generic-password -s "com.phantom" 2>/dev/null | grep -q "com.phantom"; then
      SERVICE_NAME="$(security find-generic-password -s "com.phantom" 2>/dev/null | grep "svce" | head -1 | sed 's/.*<blob>="//' | sed 's/".*//' 2>/dev/null || true)"
      if [ -n "$SERVICE_NAME" ]; then
        security delete-generic-password -s "$SERVICE_NAME" 2>/dev/null || true
        KEYCHAIN_CLEANED=true
        track
      else
        break
      fi
    else
      break
    fi
  done

  # Also try known phantom keychain services directly
  for svc in "com.phantom.license" "com.phantom.master-key" "com.phantom.credentials" "com.phantom.agent-service" "com.phantom.api-keys"; do
    security delete-generic-password -s "$svc" 2>/dev/null && { KEYCHAIN_CLEANED=true; track; } || true
  done

  if [ "$KEYCHAIN_CLEANED" = true ]; then
    success "Phantom Keychain entries removed"
  else
    dim "    No Phantom Keychain entries found"
  fi
fi

# ── 4. Remove ~/.phantom/ Data Directory ────────────────────────────────────
if [ "$REMOVE_DATA" = true ]; then
  info "Removing Phantom data directory (~/.phantom/)..."

  if [ -d "${PHANTOM_DATA_DIR}" ]; then
    # List what's being removed for transparency
    FILE_COUNT="$(find "${PHANTOM_DATA_DIR}" -type f 2>/dev/null | wc -l | tr -d ' ')"
    rm -rf "${PHANTOM_DATA_DIR}"
    success "Removed ${PHANTOM_DATA_DIR} (${FILE_COUNT} files)"
    track
  else
    dim "    ${PHANTOM_DATA_DIR} not found"
  fi
fi

# ── 5. Remove Zero-Footprint Session Artifacts ──────────────────────────────
if [ "$REMOVE_DATA" = true ]; then
  info "Scanning for zero-footprint session artifacts..."

  ZFP_COUNT=0
  # Search common project locations for phantom artifacts
  for artifact_name in ".phantom-credentials" ".phantom-env" ".phantom-session" ".phantom-state" ".phantom.lock"; do
    # Search home directory (1 level deep) and common project dirs
    for search_dir in "${HOME}" "${HOME}/Documents" "${HOME}/Projects" "${HOME}/Developer" "${HOME}/Desktop" "${HOME}/Code" "${HOME}/repos" "${HOME}/src" "${HOME}/workspace"; do
      if [ -d "$search_dir" ]; then
        find "$search_dir" -maxdepth 3 -name "$artifact_name" -type f 2>/dev/null | while read -r artifact; do
          rm -f "$artifact"
          dim "    Removed: ${artifact}"
          ZFP_COUNT=$((ZFP_COUNT + 1))
        done
      fi
    done
  done

  # Also check /tmp for any phantom temp files
  find /tmp -maxdepth 2 -name "phantom*" -user "$(whoami)" 2>/dev/null | while read -r tmpfile; do
    rm -rf "$tmpfile"
    dim "    Removed temp: ${tmpfile}"
  done

  success "Zero-footprint artifact scan complete"
  track
fi

# ── 6. Remove Phantom Binary ────────────────────────────────────────────────
if [ "$REMOVE_BINARY" = true ]; then
  info "Removing Phantom binary..."

  if [ -f "${BINARY_PATH}" ]; then
    if [ -w "${INSTALL_DIR}" ]; then
      rm -f "${BINARY_PATH}"
    else
      info "Requesting sudo to remove ${BINARY_PATH}..."
      sudo rm -f "${BINARY_PATH}"
    fi
    success "Removed ${BINARY_PATH}"
    track
  else
    dim "    Binary not found at ${BINARY_PATH}"
    # Check if it's somewhere else in PATH
    ALT_PATH="$(command -v phantom 2>/dev/null || true)"
    if [ -n "$ALT_PATH" ]; then
      warn "Found phantom at alternative location: ${ALT_PATH}"
      printf "  ${YELLOW}Remove it manually:${RESET} sudo rm -f ${ALT_PATH}\n"
    fi
  fi
fi

# ── 7. Clean Shell History References (Optional) ────────────────────────────
if [ "$REMOVE_DATA" = true ]; then
  info "Checking for Phantom environment variables in shell profiles..."

  SHELL_CLEANED=false
  for profile in "${HOME}/.zshrc" "${HOME}/.bashrc" "${HOME}/.bash_profile" "${HOME}/.zprofile" "${HOME}/.profile"; do
    if [ -f "$profile" ]; then
      if grep -q "PHANTOM" "$profile" 2>/dev/null; then
        # Create backup before modifying
        cp "$profile" "${profile}.phantom-backup"
        # Remove lines containing PHANTOM env vars
        awk '/github\.com/ { print; next } /PHANTOM_|phantom_|# Phantom|# PHANTOM/ { next } { print }' "$profile" > "${profile}.tmp" 2>/dev/null || true
        mv "${profile}.tmp" "$profile"
        success "Cleaned Phantom references from $(basename "$profile") (backup: $(basename "$profile").phantom-backup)"
        SHELL_CLEANED=true
        track
      fi
    fi
  done

  if [ "$SHELL_CLEANED" = false ]; then
    dim "    No Phantom environment variables found in shell profiles"
  fi
fi

# ── 8. Remove Application Support & Caches ──────────────────────────────────
if [ "$REMOVE_DATA" = true ]; then
  info "Removing Application Support & Cache data..."

  for dir in \
    "${HOME}/Library/Application Support/phantom" \
    "${HOME}/Library/Application Support/Phantom" \
    "${HOME}/Library/Caches/phantom" \
    "${HOME}/Library/Caches/Phantom" \
    "${HOME}/Library/Logs/phantom" \
    "${HOME}/Library/Logs/Phantom" \
    "${HOME}/Library/Preferences/com.phantom.plist" \
    "${HOME}/Library/Preferences/com.benchbrex.phantom.plist"; do
    if [ -e "$dir" ]; then
      rm -rf "$dir"
      success "Removed: ${dir}"
      track
    fi
  done

  dim "    Application Support & Cache cleanup complete"
fi

# ── 9. Verify Complete Removal ──────────────────────────────────────────────
printf "\n"
info "Verifying removal..."

REMNANTS=false

if [ "$REMOVE_BINARY" = true ] && command -v phantom >/dev/null 2>&1; then
  warn "phantom binary still found in PATH: $(command -v phantom)"
  REMNANTS=true
fi

if [ "$REMOVE_DATA" = true ] && [ -d "${PHANTOM_DATA_DIR}" ]; then
  warn "Data directory still exists: ${PHANTOM_DATA_DIR}"
  REMNANTS=true
fi

if [ "$REMOVE_DATA" = true ] && ls "${LAUNCH_AGENTS_DIR}"/com.phantom.*.plist >/dev/null 2>&1; then
  warn "LaunchAgent plists still found"
  REMNANTS=true
fi

if [ "$REMNANTS" = false ]; then
  success "Verification passed — no Phantom remnants detected"
fi

# ── 10. Final Banner ────────────────────────────────────────────────────────
printf "\n"
if [ "$REMOVE_BINARY" = true ] && [ "$REMOVE_DATA" = true ]; then
  printf "${GREEN}${BOLD}  ════════════════════════════════════════════════${RESET}\n"
  printf "${GREEN}${BOLD}   Phantom completely uninstalled.${RESET}\n"
  printf "${GREEN}${BOLD}  ════════════════════════════════════════════════${RESET}\n"
  printf "\n"
  printf "  ${DIM}Removed: binary, config, state, daemons, keychain entries,${RESET}\n"
  printf "  ${DIM}session artifacts, caches, and environment variables.${RESET}\n"
elif [ "$REMOVE_BINARY" = true ]; then
  printf "${GREEN}${BOLD}  ════════════════════════════════════════════════${RESET}\n"
  printf "${GREEN}${BOLD}   Phantom binary removed.${RESET}\n"
  printf "${GREEN}${BOLD}  ════════════════════════════════════════════════${RESET}\n"
  printf "\n"
  printf "  ${DIM}Config and data preserved at ~/.phantom/${RESET}\n"
  printf "  ${DIM}Reinstall: curl -fsSL https://raw.githubusercontent.com/benchbrex-USA/PHANTOM/main/install.sh | sh${RESET}\n"
elif [ "$REMOVE_DATA" = true ]; then
  printf "${GREEN}${BOLD}  ════════════════════════════════════════════════${RESET}\n"
  printf "${GREEN}${BOLD}   Phantom data removed.${RESET}\n"
  printf "${GREEN}${BOLD}  ════════════════════════════════════════════════${RESET}\n"
  printf "\n"
  printf "  ${DIM}Binary preserved at ${BINARY_PATH}${RESET}\n"
  printf "  ${DIM}Re-activate: phantom activate --key PH1-xxxxx-xxxxx${RESET}\n"
fi

printf "\n"
printf "  ${DIM}Want to reinstall? → curl -fsSL https://raw.githubusercontent.com/benchbrex-USA/PHANTOM/main/install.sh | sh${RESET}\n"
printf "  ${DIM}Feedback?          → https://phantom.benchbrex.com/feedback${RESET}\n"
printf "\n"
