#!/usr/bin/env bash
set -euo pipefail

# ─── mdviewer installer ───────────────────────────────────────────────────────
#
# Copies the built Markdown Viewer.app to ~/Applications, creates a symlink
# in ~/.local/bin/mdviewer, and registers file associations via LaunchServices.
#
# Usage:
#   ./bin/install.sh              # install (builds if needed)
#   ./bin/install.sh --no-build   # skip building, install from existing .app
#   ./bin/install.sh --uninstall  # remove symlink and file associations
#
# Prerequisites:
#   - A built app at src-tauri/target/release/bundle/macos/Markdown\ Viewer.app
#     (run `make bundle` or `cargo tauri build` first)
# ───────────────────────────────────────────────────────────────────────────────

APP_NAME="Markdown Viewer"
APP_BUNDLE="${APP_NAME}.app"
APPS_DIR="$HOME/Applications"
BIN_DIR="$HOME/.local/bin"
LAUNCHCTL_HELPER="app.mdviewer"

# Colour helpers
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[install]${NC} $*"; }
warn()  { echo -e "${YELLOW}[install]${NC} $*"; }
fail()  { echo -e "${RED}[install]${NC} $*" >&2; exit 1; }

# ── Uninstall ─────────────────────────────────────────────────────────────────

do_uninstall() {
    info "Removing symlink: ${BIN_DIR}/mdviewer"
    rm -f "${BIN_DIR}/mdviewer"

    info "Removing file associations for ${APP_NAME}"
    # Reset file associations by clearing LaunchServices defaults for our bundle ID
    for ext in md markdown txt; do
        /usr/libexec/PlistBuddy -c "Delete :${ext}" ~/Library/Preferences/com.apple.LaunchServices/com.apple.launchservices.secure.plist 2>/dev/null || true
    done

    # Re-compact LaunchServices so changes take effect
    /usr/bin/lsregister -f "$HOME/Applications/${APP_BUNDLE}" 2>/dev/null || true

    info "Uninstalled. You may manually remove ~/Applications/${APP_BUNDLE} if desired."
}

# ── Install ───────────────────────────────────────────────────────────────────

do_install() {
    local skip_build="${1:-false}"

    # 1. Build if needed
    if [ "$skip_build" = "false" ]; then
        info "Building app..."
        make bundle
    fi

    # 2. Find the .app bundle
    local bundle_path
    bundle_path=$(find src-tauri/target/release/bundle/macos -name "${APP_BUNDLE}" -type d 2>/dev/null | head -1)
    [ -n "$bundle_path" ] || fail "No ${APP_BUNDLE} found. Run 'make bundle' first."
    [ -d "$bundle_path" ] || fail "${bundle_path} is not a directory"

    info "Found bundle: ${bundle_path}"

    # 3. Copy to ~/Applications
    mkdir -p "$APPS_DIR"
    info "Copying to ${APPS_DIR}/"
    cp -Rf "$bundle_path" "$APPS_DIR/"

    # 4. Remove quarantine attribute (needed for macOS Gatekeeper)
    xattr -dr com.apple.quarantine "${APPS_DIR}/${APP_BUNDLE}" 2>/dev/null || true

    # 5. Create symlink in ~/.local/bin
    mkdir -p "$BIN_DIR"
    info "Creating symlink: ${BIN_DIR}/mdviewer -> ${APPS_DIR}/${APP_BUNDLE}"
    ln -sf "${APPS_DIR}/${APP_BUNDLE}" "${BIN_DIR}/mdviewer"

    # 6. Register file associations via LaunchServices
    info "Registering file associations (.md, .markdown, .txt)..."
    /usr/bin/lsregister -f "${APPS_DIR}/${APP_BUNDLE}" 2>/dev/null || true

    # 7. Print next steps
    echo ""
    info "Installation complete!"
    echo ""
    echo "  ${GREEN}mdviewer${NC} README.md          # open a file from any terminal"
    echo "  ${GREEN}mdviewer${NC} --help             # show usage"
    echo ""
    echo "Double-click .md/.markdown/.txt files in Finder — they'll open in ${APP_NAME}."
    echo ""
    echo "If 'mdviewer' is not found, add this to your shell config (~/.zshrc or ~/.bashrc):"
    echo ""
    echo "    export PATH=\"${BIN_DIR}:\$PATH\""
    echo ""
}

# ── Main ──────────────────────────────────────────────────────────────────────

case "${1:-}" in
    --uninstall)
        do_uninstall
        ;;
    --no-build)
        do_install "true"
        ;;
    --help|-h)
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  (none)       Build and install (default)"
        echo "  --no-build   Install from existing .app without building"
        echo "  --uninstall  Remove symlink and file associations"
        echo "  --help       Show this help"
        ;;
    *)
        do_install "false"
        ;;
esac
