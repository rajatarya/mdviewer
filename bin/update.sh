#!/usr/bin/env bash
set -euo pipefail

# ─── mdviewer updater ─────────────────────────────────────────────────────────
#
# Checks GitHub Releases for a newer version, downloads and installs the .dmg.
#
# Usage:
#   ./bin/update.sh              # check and install latest
#   ./bin/update.sh --check      # only check, don't install
#   ./bin/update.sh --help       # show usage
#
# Requires:
#   - curl (pre-installed on macOS)
#   - jq (install via: brew install jq)
#   - hdiutil (pre-installed on macOS)
#   - A GitHub token if rate-limited (sets GITHUB_TOKEN env var)
#
# ───────────────────────────────────────────────────────────────────────────────

REPO="rajatarya/mdviewer"
APP_NAME="Markdown Viewer"
APP_BUNDLE="${APP_NAME}.app"
APPS_DIR="$HOME/Applications"
BIN_DIR="$HOME/.local/bin"
WRAPPER="${BIN_DIR}/mdviewer"

# Colour helpers
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[update]${NC} $*"; }
warn() { echo -e "${YELLOW}[update]${NC} $*"; }
fail() {
  echo -e "${RED}[update]${NC} $*" >&2
  exit 1
}

# ── Get local version ────────────────────────────────────────────────────────

get_local_version() {
  # Prefer the installed app's bundle version over Cargo.toml
  local info_plist="${APPS_DIR}/${APP_BUNDLE}/Contents/Info.plist"
  if [ -f "$info_plist" ]; then
    local ver
    ver=$(defaults read "$info_plist" CFBundleShortVersionString 2>/dev/null) || true
    if [ -n "$ver" ]; then
      echo "$ver"
      return
    fi
  fi
  # Fallback: read from Cargo.toml (for running from source tree)
  if [ -f "src-tauri/Cargo.toml" ]; then
    sed -n 's/^version = "\(.*\)"/\1/p' src-tauri/Cargo.toml | head -1
  else
    echo "0.0.0"
  fi
}

# ── Get latest release from GitHub ───────────────────────────────────────────

get_latest_release() {
  local url="https://api.github.com/repos/${REPO}/releases/latest"
  local headers=""

  if [ -n "${GITHUB_TOKEN:-}" ]; then
    headers="-H 'Authorization: Bearer ${GITHUB_TOKEN}'"
  fi

  local response
  response=$(curl -s -f ${headers} "$url" 2>/dev/null) || {
    fail "Failed to fetch latest release from GitHub.\n\
      Is the repo public? Do you need to set GITHUB_TOKEN?"
  }

  local tag
  tag=$(echo "$response" | jq -r '.tag_name') || {
    fail "Could not parse release tag from GitHub API response."
  }

  local dmg_url
  dmg_url=$(echo "$response" | jq -r '.assets[] | select(.name | endswith(".dmg")) | .browser_download_url' | head -1) || dmg_url=""

  if [ -z "$dmg_url" ]; then
    fail "No .dmg asset found in the latest release."
  fi

  echo "${tag#v}|${dmg_url}"
}

# ── Compare versions ────────────────────────────────────────────────────────

version_gt() {
  # Returns 0 (true) if $1 > $2
  local v1="${1#v}" v2="${2#v}"
  [ "$v1" = "$v2" ] && return 1

  IFS='.' read -ra p1 <<< "$v1"
  IFS='.' read -ra p2 <<< "$v2"

  local max=${#p1[@]}
  [ ${#p2[@]} -gt "$max" ] && max=${#p2[@]}

  for ((i = 0; i < max; i++)); do
    local a="${p1[$i]:-0}" b="${p2[$i]:-0}"
    if [ "$a" -gt "$b" ] 2>/dev/null; then return 0; fi
    if [ "$a" -lt "$b" ] 2>/dev/null; then return 1; fi
  done
  return 1
}

# ── Download and mount .dmg ──────────────────────────────────────────────────

download_and_mount() {
  local dmg_url="$1"
  local tmp_dir
  tmp_dir=$(mktemp -d /tmp/mdviewer-update.XXXXXX)

  trap "rm -rf '$tmp_dir'" EXIT

  info "Downloading latest .dmg..."
  curl -fSL -o "${tmp_dir}/${APP_BUNDLE%.app}.dmg" "$dmg_url"

  info "Mounting .dmg..."
  local mount_point
  mount_point=$(hdiutil attach -nobrowse -noautoopen "${tmp_dir}/${APP_BUNDLE%.app}.dmg" 2>/dev/null | grep '^/Volumes' | head -1)
  [ -n "$mount_point" ] || fail "Failed to mount .dmg"

  echo "$mount_point"
}

# ── Install the app ──────────────────────────────────────────────────────────

install_app() {
  local dmg_mount="$1"
  local dmg_app="${dmg_mount}/${APP_BUNDLE}"

  # 1. Copy to ~/Applications
  mkdir -p "$APPS_DIR"
  info "Installing to ${APPS_DIR}/"
  rm -rf "${APPS_DIR}/${APP_BUNDLE}"  # remove old version
  cp -R "${dmg_app}" "${APPS_DIR}/"

  # 2. Remove quarantine
  info "Removing quarantine..."
  xattr -dr com.apple.quarantine "${APPS_DIR}/${APP_BUNDLE}" 2>/dev/null || true

  # 3. Re-sign
  info "Re-signing app bundle..."
  codesign --sign - --force --deep "${APPS_DIR}/${APP_BUNDLE}" 2>/dev/null || true

  # 4. Update wrapper script if it exists
  if [ -f "$WRAPPER" ]; then
    sed -i '' "s|${APPS_DIR}/${APP_BUNDLE}|${APPS_DIR}/${APP_BUNDLE}|" "$WRAPPER" 2>/dev/null || true
  fi

  # 5. Re-register file associations
  /usr/bin/lsregister -f "${APPS_DIR}/${APP_BUNDLE}" 2>/dev/null || true

  info "Installation complete!"
}

# ── Unmount .dmg ─────────────────────────────────────────────────────────────

unmount_dmg() {
  local dmg_mount="$1"
  hdiutil detach "$dmg_mount" -quiet 2>/dev/null || true
}

# ── Main ─────────────────────────────────────────────────────────────────────

case "${1:-}" in
  --check)
    local_ver=$(get_local_version)
    latest_info=$(get_latest_release)
    latest_tag="${latest_info%%|*}"
    latest_ver="${latest_info%%|*}"
    echo "Local:   ${local_ver}"
    echo "Latest:  ${latest_tag}"
    if version_gt "$latest_ver" "$local_ver"; then
      info "New version available: ${latest_tag}"
      echo "  Run './bin/update.sh' to install."
      exit 0
    else
      info "Already up to date (v${local_ver})"
      exit 0
    fi
    ;;
  --help | -h)
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "  (none)       Check for updates and install if available"
    echo "  --check      Only check, don't install"
    echo "  --help       Show this help"
    echo ""
    echo "Environment:"
    echo "  GITHUB_TOKEN  Personal access token (needed if rate-limited)"
    ;;
  *)
    # Normal update flow
    local_ver=$(get_local_version)
    info "Current version: v${local_ver}"

    latest_info=$(get_latest_release)
    latest_tag="${latest_info%%|*}"
    latest_ver="${latest_info%%|*}"
    dmg_url="${latest_info#*|}"

    echo "  Latest:  ${latest_tag}"
    echo ""

    if [ "$latest_ver" = "$local_ver" ]; then
      info "Already up to date! (v${local_ver})"
      exit 0
    fi

    if version_gt "$latest_ver" "$local_ver"; then
      warn "New version available: ${latest_tag}"
      printf "Install %s? (y/n): " "$latest_tag"
      read -r confirm
      if [[ "$confirm" != [yY]* ]]; then
        info "Aborted."
        exit 0
      fi
    else
      fail "Local version (${local_ver}) is newer than latest release (${latest_tag})."
    fi

    dmg_mount=$(download_and_mount "$dmg_url")
    install_app "$dmg_mount"
    unmount_dmg "$dmg_mount"

    echo ""
    info "Updated to ${latest_tag}! 🎉"
    echo ""
    echo "  ${GREEN}mdviewer${NC} README.md          # open a file"
    echo "  ${GREEN}mdviewer${NC} --help             # show usage"
    ;;
esac
