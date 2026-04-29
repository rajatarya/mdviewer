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
    info "Removing wrapper: ${BIN_DIR}/mdviewer"
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

    # 2. Find the .app bundle (Tauri builds to workspace root, not src-tauri/)
    local bundle_path
    bundle_path=$(find target/release/bundle/macos -name "${APP_BUNDLE}" -type d 2>/dev/null | head -1)
    [ -n "$bundle_path" ] || fail "No ${APP_BUNDLE} found. Run 'make bundle' first."

    info "Found bundle: ${bundle_path}"

    # 3. Copy to ~/Applications
    mkdir -p "$APPS_DIR"
    info "Copying to ${APPS_DIR}/"
    cp -Rf "$bundle_path" "$APPS_DIR/"

    # 4. Remove quarantine attribute (needed for macOS Gatekeeper)
    xattr -dr com.apple.quarantine "${APPS_DIR}/${APP_BUNDLE}" 2>/dev/null || true

    # 5. Create wrapper script in ~/.local/bin
    # .app bundles are directories, so we use a small wrapper that calls 'open'.
    # Intercepts --help/-h so it prints help without launching the app.
    mkdir -p "$BIN_DIR"
    local wrapper="${BIN_DIR}/mdviewer"
    info "Creating wrapper: ${wrapper}"
    cat > "$wrapper" <<'WRAPPER'
#!/usr/bin/env bash
if [[ "$1" == "--help" || "$1" == "-h" ]]; then
    cat <<'HELP'
Markdown Viewer — A lightweight Markdown viewer for macOS

USAGE:
    mdviewer [FLAGS] [FILES...]

FLAGS:
    -h, --help       Print this help message and exit

ARGS:
    FILES    Markdown files to open (.md, .markdown, .txt)

EXAMPLES:
    mdviewer document.md
    mdviewer doc1.md doc2.md notes.txt
    mdviewer --help

FEATURES:
    GitHub Flavored Markdown: Tables, task lists, strikethrough, autolinks
    Obsidian-style: Wikilinks [[Page]], emoji :rocket:, callouts [!NOTE]
    Math: Inline $E=mc^2$ and display $$\int_0^\infty$$
    Diagrams: Mermaid code blocks
    Security: All HTML sanitized via ammonia — XSS-safe
HELP
    exit 0
fi
open -a "PLACEHOLDER_APP" -- "$@"
WRAPPER
    # Replace placeholder with actual app path
    sed -i '' "s|PLACEHOLDER_APP|${APPS_DIR}/${APP_BUNDLE}|" "$wrapper"
    chmod +x "$wrapper"

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
