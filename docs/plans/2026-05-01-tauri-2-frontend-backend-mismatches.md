# Tauri 2.x Frontend-Backend Mismatches — Known Pitfalls

> **Date:** 2026-05-01
> **Status:** Fixed (both issues resolved)

This doc captures two critical Tauri 2.x pitfalls that caused runtime errors.
Both stem from incorrect assumptions about how Tauri 2.x handles naming conventions.

---

## Issue 1: "Command readFile not found" on every window launch

### Symptom

Running `./target/release/mdviewer docs/plans/*.md` shows an error dialog on each window:
> "Error: Command readFile not found"

### Root Cause

**Tauri 2.x `#[command]` registers commands with their exact Rust function name (snake_case).**
There is **no auto-conversion of command names** from camelCase to snake_case.

Only **argument names** are auto-converted:
- Frontend passes `{ outputPath: "file.html" }` → Backend receives `output_path: &str`
- Frontend passes `{ markdown: "hello" }` → Backend receives `markdown: &str`

But the **command name itself** must match exactly:
- Backend: `#[command] pub fn render_md(...) → Frontend: `invoke('render_md', ...)`
- Backend: `#[command] pub fn read_file(...) → Frontend: `invoke('read_file', ...)`

### What Was Wrong

The frontend was using camelCase command names:
```javascript
invoke('readFile', { path: filePath })      // ❌ wrong
invoke('renderMd', { markdown: content })    // ❌ wrong
invoke('exportHtml', { ... })                // ❌ wrong
invoke('getCliPaths')                        // ❌ wrong
invoke('setWindowTitle', { ... })            // ❌ wrong
```

### Fix

Changed all frontend `invoke()` calls to use snake_case command names matching the Rust function names:
```javascript
invoke('read_file', { path: filePath })      // ✅ correct
invoke('render_md', { markdown: content })    // ✅ correct
invoke('export_html', { ... })                // ✅ correct
invoke('get_cli_paths')                       // ✅ correct
invoke('set_window_title', { ... })           // ✅ correct
```

### Prevention

The test `test_frontend_invokes_match_backend_commands` in `src-tauri/src/lib.rs` verifies
that every `invoke()` call in `dist/index.html` matches a registered backend command.
It uses snake_case names. If you add a new command, update both the backend registration
and this test's allowed list.

---

## Issue 2: "The application 'Markdown Viewer' can't be opened" on double-click

### Symptom

Double-clicking a `.md` file in Finder (or running `open -a ~/Applications/Markdown\ Viewer.app`)
shows:
> "The application 'Markdown Viewer' can't be opened."

### Root Cause

**Tauri 2.x creates an adhoc code signature that becomes invalid after the app is copied.**

The Tauri build process creates an adhoc signature with:
- `Info.plist=not bound` — Info.plist is not included in the signature
- `Sealed Resources=none` — no sealed resources in the signature

When the app is copied to `~/Applications/`, file hashes change (modification times, etc.),
and macOS detects the signature mismatch:

```
OS_REASON_CODESIGNING | embedded signature doesn't match attached signature
```

From `spctl --assess`:
```
code has no resources but signature indicates they must be present
```

From system logs (`log show`):
```
launchd: xpcproxy exited due to OS_REASON_CODESIGNING | embedded signature doesn't match attached signature
```

The app gets SIGKILL (exit 137) because macOS security rejects the invalid signature.

### Fix

Added a re-signing step to `bin/install.sh` after copying the app to `~/Applications/`:

```bash
# 5. Re-sign the app bundle.
# Tauri 2.x creates an adhoc signature that becomes invalid after copy
# (file hashes change, causing "embedded signature doesn't match attached signature").
# Re-signing fixes the signature so the app launches from ~/Applications.
info "Re-signing app bundle for ~/Applications..."
codesign --sign - --force --deep "${APPS_DIR}/${APP_BUNDLE}" 2>/dev/null || true
```

After re-signing, `codesign -dv` shows:
- `Identifier=app.mdviewer` (correct bundle ID)
- `Info.plist entries=15` (Info.plist is now bound)
- `Sealed Resources version=2 rules=13 files=1` (resources are sealed)

### Prevention

Any script that copies a Tauri-built `.app` bundle must re-sign it afterward.
The adhoc signature from Tauri's build is only valid in its original location.

---

## Summary of Tauri 2.x Naming Rules

| What | Naming Convention | Example |
|---|---|---|
| Backend command name | **Exact Rust function name** (snake_case) | `render_md` |
| Frontend invoke name | **Must match backend exactly** | `invoke('render_md', ...)` |
| Backend argument name | snake_case | `output_path` |
| Frontend argument key | camelCase (auto-converted) | `{ outputPath: "..." }` |

**Key takeaway:** Command names must match exactly. Argument names are auto-converted.
