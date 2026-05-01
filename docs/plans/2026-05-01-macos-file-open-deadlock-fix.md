# macOS File-Open Architecture: Deadlocks, Single-Instance, and Window Cascade

> **Date:** 2026-05-01
> **Status:** Fixed
> **Affects:** Tauri 2.10.3 (and 2.11) on macOS

This doc captures the architectural fixes that landed for double-click-from-Finder
support. Read it before touching `main.rs`, `lib.rs::run()`, or the
`open_file_plugin()` / single-instance plumbing — multiple non-obvious traps live
in this code path.

---

## TL;DR — Rules to follow

1. **Never call `WebviewWindowBuilder::build()` synchronously from inside any
   `RunEvent` handler.** It will deadlock the app's main thread.
2. **Don't roll your own single-instance with a lock file or polling thread.**
   Use `tauri-plugin-single-instance`. Crucially, never call AppKit/WebKit APIs
   from a background thread on macOS — `WKWebView` init and `NSWindow` creation
   require the main run loop.
3. **`run_on_main_thread()` from the main thread runs synchronously inline** —
   it is *not* a deferral mechanism in that case (see Tauri 2.x source,
   `tauri-runtime-wry/src/lib.rs:235`). To actually defer work past the current
   event-loop iteration, hop to a tokio task first via
   `tauri::async_runtime::spawn`.
4. **macOS file-association files arrive via Apple Events**, not as `argv`.
   The new process gets `argv = [binary]`. The file path is delivered to
   Tauri as `RunEvent::Opened { urls }`. Don't try to read the file path from
   `std::env::args()` for double-click launches.
5. **Window placement is not cascaded automatically** when windows are built
   back-to-back during startup. Call `.position(x, y)` explicitly so they
   don't stack on top of each other invisibly.

---

## Issue 1: `RunEvent::Opened` + window creation = main-thread deadlock

### Symptom

App opens the *first* file fine. Double-clicking a *second* `.md` file in
Finder (or running a second `open file2.md`) wedges the running app.
AppleScript queries against the process return error `-1712 (AppleEvent
timed out)`. Force-quit is required to recover.

### Root cause

Tauri 2 holds `manager.plugins.lock()` for the duration of any `RunEvent`
plugin dispatch. Source: `tauri-2.11.0/src/app.rs:2636`:

```rust
manager
    .plugins
    .lock()
    .expect("poisoned plugin store")
    .on_event(app_handle, &event);
```

Inside our `RunEvent::Opened` handler we called
`commands::create_window_for_file()` → `WebviewWindowBuilder::build()`.
The window-creation path internally fires `window_created` on each plugin —
which acquires the same `manager.plugins.lock()`. Self-deadlock on the main
thread, the run loop never makes progress, AppleEvents back up.

The "first file" worked because that path took the "fill empty main"
branch in `open_or_create_window()` (`set_title` + `eval`) — neither of
those touches the plugin store.

### Why `run_on_main_thread()` does not save you

Looking at `tauri-runtime-wry/src/lib.rs:235`:

```rust
pub(crate) fn send_user_message<T: UserEvent>(
    context: &Context<T>,
    message: Message<T>,
) -> Result<()> {
    if current_thread().id() == context.main_thread_id {
        handle_user_message(...);  // synchronous, inline
        Ok(())
    } else {
        context.proxy.send_event(message)  // queued for next iteration
            .map_err(|_| Error::FailedToSendMessage)
    }
}
```

When called from the main thread, `run_on_main_thread()` runs the closure
*synchronously inline*. The deferral only happens when called from a
non-main thread, via `proxy.send_event` which queues a `Message::Task`
that the run loop picks up in its next iteration.

### Fix

Hop to a tokio worker first, then back to the main thread:

```rust
.on_event(|app, event| {
    if let RunEvent::Opened { urls } = event {
        for url in urls {
            if let Ok(path) = url.to_file_path() {
                let path_str = path.to_string_lossy().into_owned();
                if commands::is_md_file(&path_str) {
                    let app = app.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = app.clone().run_on_main_thread(move || {
                            commands::open_or_create_window(&app, &path_str);
                        });
                    });
                }
            }
        }
    }
})
```

`async_runtime::spawn` puts us on a tokio worker thread, so the subsequent
`run_on_main_thread` goes through `proxy.send_event`. The closure runs as
`Message::Task` in the next event-loop iteration, *after* `on_event`
returns and the plugin-store mutex is released. `build()` then succeeds.

### How to know you've broken this again

A stress test: with the app running and at least one window open, run
`open file.md` twice in quick succession. If AppleScript queries
(`tell application "Markdown Viewer" to count windows`) return `-1712
timeout`, you re-introduced the deadlock.

---

## Issue 2: Homemade single-instance via lock file + polling thread

### Symptom (historical — fixed in this revision)

A previous attempt to handle the "macOS sometimes spawns a duplicate
process" case (commit `257820e`, since superseded) used:
- `~/tmp/mdviewer.lock` with PID-based liveness check
- `--open-url=file://...` exit path
- A background polling thread reading `/tmp/mdviewer/open_urls.txt` and
  calling `WebviewWindowBuilder::build()`

This deadlocked the running instance because **`WebviewWindowBuilder::build()`
on macOS must run on the main thread** — AppKit/WKWebView init isn't safe
from a `std::thread::spawn` worker.

### Root cause

Two layers of incorrect assumptions:

1. macOS spawned duplicate processes for double-clicks because the user
   had two copies of the `.app` bundle (`/Applications/Markdown Viewer.app`
   with the wrong adhoc-signed identifier `mdviewer-<random>`, and
   `~/Applications/Markdown Viewer.app` with the correct `app.mdviewer`).
   LaunchServices treated them as different apps.
2. The mitigation called UI APIs from a background thread.

### Fix

- Removed the entire homemade scheme. `main.rs` is now ~10 lines:
  `--help` check, then `mdviewer_core::run()`.
- Added `tauri-plugin-single-instance = "2.4.1"` (target-cfg'd to skip
  iOS/Android).
- Single-instance callback dispatches via `app.run_on_main_thread()`. The
  callback runs inside `tauri::async_runtime::spawn` already (see
  `tauri-plugin-single-instance/src/platform_impl/macos.rs::listen_for_other_instances`),
  so `run_on_main_thread()` correctly defers in this path.
- The ops-side fix: only ever install via `bin/install.sh` (which targets
  `~/Applications/`). Don't drag the `.app` from a DMG to `/Applications/`.
  Two installs with mismatched signing identifiers reproduces the same
  duplicate-process behaviour LaunchServices is supposed to prevent.

### Apple Events vs `argv`

When macOS launches an app via file association (Finder double-click,
`open file.md`), the file path is delivered as a `kAEOpenDocuments` Apple
Event, *not* via `argv`. The new process always sees `argv = [binary]`.
Tauri 2 surfaces the Apple Event as `RunEvent::Opened { urls: Vec<Url> }`.

This means the single-instance plugin's args-based callback **does not see
the file path** when macOS spawns a duplicate due to LaunchServices
confusion — the Apple Event was sent to the duplicate, which exits before
processing it. The reliable fix is to keep LaunchServices happy (single
correctly-signed bundle) so it routes the event to the running instance,
where `RunEvent::Opened` fires.

---

## Issue 3: Multi-file launch — windows hidden behind each other

### Symptom

`mdviewer a.md b.md c.md` reported "only the first file opens, the rest
are lost." Logs showed Rust did create `window-1` and `window-2`, and
their JS fired `on_page_load`. AppleScript revealed all three windows
existed — but the main window and `window-1` were positioned at the
*identical* coordinates, so the second was hidden directly behind the
first.

### Root cause

`WebviewWindowBuilder` was called without `.position(...)`, and AppKit's
auto-cascade does not consistently kick in when windows are created
back-to-back during startup before any of them has been displayed.

### Fix

Added `commands::cascade_position(window_index)` returning a distinct,
monotonically-offset `(x, y)` per index, and call `.position(x, y)` in
`create_window_for_file`.

```rust
pub fn cascade_position(window_index: usize) -> (f64, f64) {
    let base_x = 120.0;
    let base_y = 120.0;
    let step = 30.0;
    let n = window_index as f64;
    (base_x + n * step, base_y + n * step)
}
```

Three unit tests pin the behaviour: distinct positions per index,
monotonic growth, and ≥20px separation between adjacent windows (so the
user can see both window borders).

---

## Architecture map (post-fix)

```
Finder double-click .md  ─┐
                          ▼
                    [LaunchServices]
                          │
            ┌─────────────┴─────────────┐
            │                           │
   already running?                  not running?
            │                           │
            ▼                           ▼
   Apple Event to existing        New process launches
   instance                       │
            │                     ▼
            ▼              tauri-plugin-single-instance
   RunEvent::Opened       checks /tmp/app_mdviewer_si.sock
   (main thread,          (no peer → become singleton)
   plugins.lock held)              │
            │                      ▼
            ▼              Tauri Builder::run() →
   tauri::async_runtime    setup() reads CliPaths
   ::spawn                 (empty for Finder launches)
            │                      │
            ▼                      ▼
   app.run_on_main_thread   main window created from config
   (queues Message::Task)          │
            │                      ▼
            ▼              Apple Event arrives →
   Next event-loop tick     RunEvent::Opened (same path
   runs the closure         as the "already running" branch)
            │
            ▼
   open_or_create_window
   ├─ main empty? → set_title + eval loadFile in main
   ├─ main full?  → create_window_for_file with cascade_position
   └─ no main?    → push to CliPaths, setup() will pick it up


Terminal launch (wrapper)  ─┐
                            ▼
              `open -a APP -- file1 file2 file3`
                            │
                            ▼
               Same path as Finder double-click
               (Apple Events deliver each file)


Direct binary launch       ─┐
(target/release/mdviewer)   ▼
                  init_cli_paths reads std::env::args
                            │
                            ▼
                  setup() processes CliPaths:
                  - sets title on default main window
                  - calls create_window_for_file
                    for files [1..] (cascade_position works
                    because we're inside setup(), not
                    inside an event handler — no plugin
                    lock held)
```

---

## Tests covering this area

- `tests::test_cascade_position_distinct_per_index`
- `tests::test_cascade_position_offsets_grow_monotonically`
- `tests::test_cascade_position_offset_visible_apart`
- `tests::test_frontend_invokes_match_backend_commands`
  (catches drift between Rust `#[command]` names and frontend `invoke()` calls)
- `tests::test_frontend_invoke_args_are_camelcase`
  (catches drift between Rust snake_case args and Tauri 2's camelCase serialisation)

The Bug 1 deadlock is hard to encode as a unit test (needs a live event
loop and a real Apple Event). The defensive measure is the prominent
comment in `open_file_plugin()` that explains *why* the spawn-then-dispatch
pattern is required, and this doc.

---

## Manual regression test (run before any release that touches this code)

```bash
# Build + install fresh
make bundle
bin/install.sh --no-build

# 1. Cold-start Finder analog
pkill -f mdviewer; sleep 2
open README.md
# → app launches, single window with README.md

# 2. Second open while running (was the original hang)
open DESIGN.md
# → second window appears, app remains responsive

# 3. AppleScript responsiveness check
osascript -e 'tell application "System Events" to tell process "mdviewer" \
  to get {name, position} of every window'
# → returns the window list (no -1712 timeout)

# 4. Multi-file via wrapper (cold)
pkill -f mdviewer; sleep 2
mdviewer README.md DESIGN.md PROJECT.md
# → three windows at distinct positions, all rendered

# 5. Add a 4th while warm
mdviewer AGENTS.md
# → fourth window cascaded next; existing three unchanged
```

---

## Don'ts

- **Don't** add a static `windows` entry to `tauri.conf.json` and *also*
  imperatively create one in `setup()`. Pick one. (We currently rely on
  the static main window from config.)
- **Don't** call `WebviewWindowBuilder::build()` from a `std::thread::spawn`.
  Use `tauri::async_runtime::spawn` + `run_on_main_thread`.
- **Don't** call `WebviewWindowBuilder::build()` synchronously from any
  `RunEvent` handler. Defer with `async_runtime::spawn`.
- **Don't** install the app to `/Applications/` by dragging the bundle
  out of the DMG. Use `bin/install.sh`. Two installations with mismatched
  signing identifiers will reproduce the LaunchServices duplicate-process
  symptom that started this whole rabbit hole.
- **Don't** parse file paths from `std::env::args()` for Finder launches
  — they aren't there. Use `RunEvent::Opened`.
