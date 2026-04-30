# Project Context — Markdown Viewer

## Project Summary
Lightweight, fast native Markdown viewer for macOS using Tauri 2.x. Renders GFM + Obsidian-style Markdown (math, diagrams, callouts, emojis) as sanitized HTML in a native window. Single HTML file, no build step.

## Architecture
- **Tauri 2.x** app: Rust backend (`src-tauri/src/lib.rs`) + vanilla JS frontend (`dist/index.html`)
- **`lib.rs`** — everything in one file: CLI handling, window management, rendering pipeline, tests
- **`dist/index.html`** — single file frontend, inline JS/CSS, no bundler
- **`main.rs`** — entry point (CLI --help check → `mdviewer_core::run()`)
- **`tauri.conf.json`** — app config, file associations, CLI plugin config

## Rendering Pipeline (`render_markdown()` in lib.rs)
1. Extract fenced code blocks (placeholders protect from regex)
2. Preprocess: math → emojis → wikilinks → callouts → restore fences
3. Parse with `pulldown-cmark` (GFM + tables + tasklists + footnotes)
4. Sanitize with `ammonia` (XSS-safe)

## Window Management
- Main window label: `"main"` (Tauri default)
- Additional windows: `"window-N"` (auto-incremented)
- **CLI files:** setup handler sets main window title for first file, creates new windows for remaining files
- **macOS Opened event:** creates a new window per file, emits `mdviewer:open-file` event
- Frontend: polls `get_cli_paths()` on startup, listens for `mdviewer:open-file` events
- Title format: `"filename : Markdown Viewer"` (set via Rust `set_window_title` command)

## File Associations
- `.md`, `.markdown`, `.txt` → role = "Viewer", contentTypes: `public.plain-text`, `public.text`

## Current Bugs / Open Issues
- *(none known — window title bug was fixed in ca3e443)*

## Recent Changes
- **2026-04-29** — feat(core): fix window title + multi-window support
  - Added `set_window_title` Rust command, setup handler sets main window title
  - macOS Opened event creates new windows per file (was: stored in CliPaths)
  - Frontend `setWindowTitle()` uses Rust backend via `invoke()`
  - See: `docs/plans/2026-04-29-window-title-and-multi-window.md`
- **2026-04-28** — fix(cli): remove invalid help arg from Tauri CLI config
- **2026-04-28** — fix(install): use wrapper script instead of symlink, fix bundle path
- **2026-04-27** — feat(install): add install.sh, feat(core): add --help CLI flag

## Development Workflow
```bash
cargo test        # All tests must pass (35 tests)
cargo clippy -- -D warnings  # Zero warnings
cargo fmt --check # Clean formatting
make all          # Run everything above
make run          # Requires Tauri CLI
```

## Key Files
- `src-tauri/src/lib.rs` — Core logic, rendering pipeline, CLI, window mgmt, tests
- `src-tauri/src/main.rs` — Entry point
- `src-tauri/tauri.conf.json` — App config, file associations, CLI plugin
- `dist/index.html` — Frontend (single file, vanilla JS)
- `AGENTS.md` — Project guidelines, code standards, commit format
- `docs/plans/` — Detailed design docs for features
