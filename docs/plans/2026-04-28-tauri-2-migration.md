# Tauri 2.x Migration — Key Differences & Implementation Patterns

> **IMPORTANT:** This project uses **Tauri 2.x** (currently 2.10.3). Many Tauri 1.x patterns
> no longer work. This doc captures the critical differences so future work doesn't regress.

---

## Version Summary

| Component | Tauri 1.x (old) | Tauri 2.x (current) |
|---|---|---|
| `tauri` crate | `1.6` | `2.10.3` |
| `tauri-build` | `1.6` | `2.5.6` |
| Plugins | `tauri-plugin-*` v1 | `tauri-plugin-*` v2 |
| Plugin init | `.plugin(tauri_plugin_foo::init())` | Same API, different internals |
| JS API | `window.__TAURI__.*` direct access | Only `invoke()` + `listen()` exposed |
| Features | `features = ["api-all"]` | Explicit features: `["wry", "custom-protocol"]` |

---

## Critical: JS API Changes

### What works in the webview

```javascript
// ✅ Tauri 2.x — these are available
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
```

### What does NOT work

```javascript
// ❌ Tauri 1.x pattern — NOT available in Tauri 2.x
const { dialog } = window.__TAURI__.dialog;       // doesn't exist
const { fs } = window.__TAURI__.fs;               // doesn't exist
const { open } = window.__TAURI__.dialog;         // doesn't exist
```

**Rule:** In Tauri 2.x, **only** `invoke()` and `listen()` are exposed to the webview.
Every plugin feature must be called via `invoke()` with the plugin's command name.

### How to call plugin commands

Tauri 2.x plugin commands are called via `invoke()`:

```javascript
// Dialog save — command name from plugin source (snake_case)
const outputPath = await invoke('dialog_save', {
  options: {
    title: 'Export HTML',
    filters: [{ name: 'HTML', extensions: ['html'] }],
    defaultPath: 'document.html',
  },
});
// Returns: string path or null if cancelled

// Dialog open
const result = await invoke('dialog_open', {
  options: {
    title: 'Open File',
    multiple: false,
  },
});
// Returns: { file: string | null } or similar

// Dialog message
const result = await invoke('dialog_message', {
  title: 'Confirm',
  message: 'Are you sure?',
  kind: 'info',
  buttons: 'okCancel',
});
```

**Finding command names:** Check the plugin's `commands.rs` source for `#[command]` functions.
The function name in snake_case is the invoke name.

---

## Plugin Registration

Plugins must be registered in `src-tauri/src/lib.rs` via the builder pattern:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_cli::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_log::init(|api| {
        api.level(log::LevelFilter::Info)
    }))
    .invoke_handler(tauri::generate_handler![
        commands::render_md,
        commands::export_html,
        // ... custom commands
    ])
    .run(tauri::generate_context!())
```

### Adding a new plugin

1. Add to `Cargo.toml`: `tauri-plugin-<name> = "2"`
2. Register in `lib.rs`: `.plugin(tauri_plugin_<name>::init())`
3. Use via `invoke()` in JS with the plugin's command name

### Available plugins (used)

| Plugin | Cargo.toml | JS invoke name(s) | Purpose |
|---|---|---|---|
| `tauri-plugin-cli` | `tauri-plugin-cli = "2"` | (handled by Tauri CLI) | CLI argument parsing |
| `tauri-plugin-dialog` | `tauri-plugin-dialog = "2"` | `dialog_save`, `dialog_open`, `dialog_message` | File save/open dialogs, message boxes |
| `tauri-plugin-log` | `tauri-plugin-log = "2"` | (handled by Tauri logger) | Logging |

### Plugins NOT yet added

| Plugin | When to add | JS invoke name(s) |
|---|---|---|
| `tauri-plugin-fs` | If you need file metadata, permissions, etc. | `fs_read`, `fs_write`, `fs_stat`, etc. |
| `tauri-plugin-http` | If you need to fetch URLs | `http_request`, etc. |

---

## Backend Command Naming

Custom commands use `#[command]` attribute and are registered via `generate_handler!`:

```rust
#[command]
pub fn render_md(markdown: &str) -> Result<String, String> { ... }

#[command]
pub fn export_html(content: &str, output_path: &str, title: &str) -> Result<(), String> { ... }
```

**Naming convention:** snake_case function names become the invoke names:
- `render_md` → `invoke('render_md', ...)`
- `export_html` → `invoke('export_html', ...)`

**Important:** The test `test_frontend_invokes_match_backend_commands` verifies that every
`invoke()` call in `dist/index.html` matches a registered backend command. If you add a
new invoke, you must also add the command name to this test's allowed list.

---

## Frontend State Management

### `currentContent` must be set on every file open

```javascript
// renderMarkdown() — called by file picker and drag-and-drop
async function renderMarkdown(content, path) {
  currentContent = content;  // ← MUST set this for export to work
  // ...
}

// loadFile() — called by CLI args
async function loadFile(filePath) {
  // ...
  currentContent = content;  // ← set by startWatching() below
  await doRender(content);
}

// startWatching() — sets currentContent for CLI-loaded files
function startWatching(filePath, initialContent) {
  currentContent = initialContent;  // ← this is the only place for CLI files
}
```

**Bug pattern:** If `currentContent` is not set, the export handler silently does nothing
(`if (!currentContent) return;`).

---

## File Open Patterns

There are **three** ways a file gets loaded, and each sets state differently:

| Method | Function called | Sets `currentContent`? | Sets `currentFilePath`? | Watches file? |
|---|---|---|---|---|
| File picker button | `renderMarkdown()` via file input | ✅ Yes | ❌ No (null) | ❌ No |
| Drag & drop | `renderMarkdown()` | ✅ Yes | ❌ No (null) | ❌ No |
| CLI arg | `loadFile()` → `startWatching()` | ✅ Yes (via startWatching) | ✅ Yes | ✅ Yes |

**Key difference:** CLI-loaded files get live file watching; picker/drag files do not.
This is intentional — CLI files are "monitored", picker files are "one-shot".

---

## Checklist Before Adding New Features

1. **Is it a plugin command?** → Use `invoke()`, not `window.__TAURI__.*`
2. **Does the plugin exist in Cargo.toml?** → Add `tauri-plugin-* = "2"` if needed
3. **Is the plugin registered in lib.rs?** → `.plugin(tauri_plugin_*::init())`
4. **Does the frontend invoke match a registered command?** → Check the test
5. **Does `currentContent` get set for this code path?** → Export depends on it
6. **Does the command name use snake_case?** → `render_md` not `renderMd`

---

## Debugging Tips

### "Button does nothing" → check `currentContent`

The most common silent failure: the export handler bails early if `currentContent` is null.
Always verify the code path that opens files sets this variable.

### "invoke() call does nothing" → check if it's a v1 API

`window.__TAURI__.dialog`, `window.__TAURI__.fs`, etc. don't exist in Tauri 2.x.
Use `invoke()` with the plugin's command name instead.

### "Command not found" → check snake_case

Backend commands use snake_case (`render_md`). The frontend invoke name must match exactly.
The test `test_frontend_invokes_match_backend_commands` catches mismatches.

### "Plugin command not available" → check registration

Every plugin (including built-in ones like dialog) must be registered via `.plugin()` in the
Tauri builder. Unregistered plugins silently fail — no error, no command.
