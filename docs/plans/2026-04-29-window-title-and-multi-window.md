# Window Title Fix & Multi-Window Support

> **Bug fix:** Window title never changed from "Markdown Viewer" to "filename : Markdown Viewer" when launching with a file via CLI or double-click.
> **Feature:** Multiple files now open in separate windows instead of all routed through the main window.

---

## The Bug

When launching the app with a file (`mdviewer doc.md` or double-clicking in Finder), the window title remained "Markdown Viewer" instead of showing "doc.md : Markdown Viewer".

**Root cause:** The setup handler never set the main window's title. The frontend's `setWindowTitle` relied on the Tauri JS API (`win.setTitle()`) which failed silently.

**Secondary issue:** The macOS `Opened` event handler stored file paths in `CliPaths` state, which the frontend would then load into the **same** main window. This meant multiple files opened sequentially in one window rather than in separate windows.

---

## Changes

### 1. New Rust commands (`src-tauri/src/lib.rs`)

| Command | Purpose |
|---|---|
| `set_window_title(app, label, title)` | Set a window's title by its label |
| `window_title(filename)` | Format a title as "filename : Markdown Viewer" |
| `create_window(app, file_path, title)` | Create a new window for a file |
| `create_window_for_file(app, file_path, display)` | Internal: build window + emit `mdviewer:open-file` event |

### 2. Setup handler now sets the main window title

```rust
.setup(|app| {
    commands::init_cli_paths(app)?;
    // Set main window title for the first CLI file
    if let Some(first_path) = file_paths.first() {
        let display = first_path.split('/').next_back().unwrap_or(first_path);
        let title = commands::window_title(display);
        if let Some(main_window) = app.get_webview_window("main") {
            main_window.set_title(&title).ok();
        }
    }
    // Create additional windows for remaining files
    for path_str in file_paths.iter().skip(1) {
        let _ = commands::create_window_for_file(app.app_handle(), path_str, display);
    }
    Ok(())
})
```

**Key detail:** The main window label is `"main"` (Tauri's default), not `"window-0"`.

### 3. macOS `Opened` event now creates new windows

Before: stored paths in `CliPaths` → frontend loaded them into the same window.
After: creates a new window for each file via `create_window_for_file()`, which emits `mdviewer:open-file` for the new window's frontend to load the file.

### 4. Frontend changes (`dist/index.html`)

- **`setWindowTitle(displayPath)`** — New function that calls `invoke('set_window_title', { label, title })` via the Rust backend command
- **`loadFile()`** — Now calls `setWindowTitle()` after reading the file
- **`renderMarkdown()`** — Also calls `setWindowTitle()` for drag-drop/picker files
- **`mdviewer:open-file` listener** — Loads files when new windows are created by the Rust backend
- **`getCurrent`** — Imported from `window.__TAURI__.window` to get the current window's label

---

## Data Flow (Before vs After)

### Before: Single window, title never updated

```
CLI: mdviewer doc.md
  → init_cli_paths() → CliPaths = ["doc.md"]
  → setup() → no title change
  → frontend init() → get_cli_paths() → loadFile()
    → win.setTitle() → FAILS SILENTLY
    → title stays "Markdown Viewer"
```

### After: Title set in Rust + new windows for multiple files

```
CLI: mdviewer doc1.md doc2.md
  → init_cli_paths() → CliPaths = ["doc1.md", "doc2.md"]
  → setup():
    → main window (label="main") title set to "doc1.md : Markdown Viewer"
    → new window created for doc2.md (label="window-1")
    → window-1 emits "mdviewer:open-file" → its frontend loads doc2.md
```

```
macOS Opened (double-click):
  → RunEvent::Opened → open_file_plugin():
    → creates new window with title "filename : Markdown Viewer"
    → emits "mdviewer:open-file" → frontend loads file
```

---

## Testing

```bash
# Build & run
cargo build
cargo tauri dev -- -- /tmp/test.md

# Verify: window title shows "test.md : Markdown Viewer"
# Verify: cargo tauri dev -- -- /tmp/test.md /tmp/other.md
# Verify: two windows open, each with correct title

# Run tests
cargo test --lib          # 35 tests pass
cargo clippy -- -D warnings  # zero warnings
cargo fmt --check          # clean
```

---

## Related Files

- `src-tauri/src/lib.rs` — `set_window_title`, `create_window`, `create_window_for_file`, `window_title`, setup handler, `open_file_plugin`
- `dist/index.html` — `setWindowTitle()`, `mdviewer:open-file` listener, `getCurrent` import
- `src-tauri/Cargo.toml` — no dependency changes
