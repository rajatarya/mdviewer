# macOS File Open Feature — How It Works

> **Feature:** Double-clicking an .md/.markdown/.txt file in Finder launches Markdown Viewer and renders the file.
> Also works with `open file.md` from terminal and dock badge clicks.

---

## The Three Code Paths

A file can arrive at the app via **two** mechanisms, each setting `CliPaths` state differently:

| Source | How it arrives | Handler | State set |
|---|---|---|---|
| **macOS Opened event** (double-click in Finder, `open` cmd, dock badge) | `RunEvent::Opened { urls }` | `open_file_plugin()` (plugin builder) | ✅ Path stored in `CliPaths` |
| **CLI args** (`cargo run file.md`) | `app.cli().matches()` | `init_cli_paths()` (setup closure) | ✅ Path stored in `CliPaths` |

### Critical: `init_cli_paths()` must NOT overwrite paths set by the `Opened` event

The `Opened` event fires **before** `.setup()` runs. If `init_cli_paths()` unconditionally writes to `CliPaths`, it clobbers the paths from the `Opened` event with an empty vec (since CLI args are empty).

**Fix:** `init_cli_paths()` only writes to `CliPaths` when CLI args actually contain paths:
```rust
if !paths.is_empty() {
    let state = app.state::<CliPaths>();
    *state.0.lock().unwrap() = paths;
}
```

---

## Data Flow Diagram

```
User double-clicks file.md in Finder
    │
    ▼
macOS launches app with file.md
    │
    ├─→ RunEvent::Opened { urls } ──→ open_file_plugin() ──→ CliPaths = ["/path/to/file.md"]
    │
    └─→ .setup() ──→ init_cli_paths() ──→ CliPaths unchanged (CLI args empty)
                                       │
                                       ▼
                              Frontend polls get_cli_paths()
                                       │
                                       ▼
                              CliPaths has ["/path/to/file.md"]
                                       │
                                       ▼
                              loadFile("/path/to/file.md")
                                       │
                                       ├─→ invoke('read_file', { path })
                                       ├─→ startWatching() (live file updates)
                                       └─→ doRender() → render_md → HTML → preview
```

---

## Key Components

### 1. `open_file_plugin()` (lib.rs)

A Tauri plugin builder that listens for `RunEvent::Opened`. Fires on:
- Double-click in Finder
- `open file.md` from terminal
- Dock badge click

```rust
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn open_file_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("mdviewer-open-file")
        .on_event(|app, event| {
            if let RunEvent::Opened { urls } = event {
                let paths = app.state::<commands::CliPaths>();
                let mut paths = paths.0.lock().unwrap();
                paths.clear();
                for url in urls {
                    if let Ok(path) = url.to_file_path() {
                        let path_str = path.to_string_lossy().into_owned();
                        if commands::is_md_file(&path_str) {
                            paths.push(path_str);
                        }
                    }
                }
            }
        })
        .build()
}
```

### 2. `CliPaths` state (lib.rs)

Shared mutable state (`Mutex<Vec<String>>`) that bridges the Rust event handlers and the frontend:

```rust
pub struct CliPaths(pub Mutex<Vec<String>>);
```

Registered in the builder: `.manage(paths)`

### 3. `init_cli_paths()` (lib.rs)

Called during `.setup()`. Reads CLI args and writes to `CliPaths` **only if paths exist**.

### 4. `get_cli_paths()` (lib.rs)

Frontend command to read `CliPaths`. Called by the frontend's polling loop.

### 5. Frontend polling (index.html)

```javascript
async function init() {
    const cliPaths = await invoke('get_cli_paths');
    if (cliPaths.length > 0) {
        await loadFile(cliPaths[0]);
        _pollingStopped = true;
    }
    // Keep polling for macOS Opened events that arrive after webview loads
    const poll = async () => {
        if (_pollingStopped) return;
        const cliPaths = await invoke('get_cli_paths');
        if (cliPaths.length > 0) {
            await loadFile(cliPaths[0]);
            _pollingStopped = true;
        }
        setTimeout(poll, 500);
    };
    poll();
}
```

Polling is needed because the `Opened` event can fire **after** the webview finishes loading (race condition on slow macOS app launch).

### 6. File associations (tauri.conf.json)

```json
"fileAssociations": [
  {
    "ext": ["md", "markdown", "txt"],
    "name": "Markdown",
    "role": "Viewer",
    "contentTypes": ["public.plain-text", "public.text"]
  }
]
```

This registers the app as a handler for .md/.markdown/.txt files in macOS Finder.

---

## Common Pitfalls

### App launches but file doesn't render
- **Cause:** `init_cli_paths()` overwrites `CliPaths` with empty vec
- **Fix:** Only write when CLI args are non-empty (see above)

### File opens correctly but live updates don't work
- **Cause:** `currentFilePath` is null (file came from picker/drag, not CLI/Opened)
- **Note:** This is intentional — live watching only for CLI/Opened files

### `Opened` event fires but nothing happens
- **Cause:** File extension not in `is_md_file()` check
- **Check:** `.md`, `.markdown`, `.txt` are the only accepted extensions

### Polling never stops
- **Cause:** `_pollingStopped` flag not set after loading
- **Fix:** Set `_pollingStopped = true` after successful `loadFile()`

---

## Testing

```bash
# Build release and install
make install

# Test: double-click a .md file in Finder
# Expected: app launches, file renders

# Test: open from terminal
open /path/to/file.md
# Expected: app launches (or focuses), file renders

# Test: CLI args directly
cargo run -- /path/to/file.md
# Expected: file renders immediately
```

---

## Related Files

- `src-tauri/src/lib.rs` — `open_file_plugin()`, `CliPaths`, `init_cli_paths()`, `get_cli_paths()`
- `dist/index.html` — `init()`, `loadFile()`, polling logic
- `src-tauri/tauri.conf.json` — `fileAssociations`
