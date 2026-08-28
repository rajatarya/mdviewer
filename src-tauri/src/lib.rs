// Markdown rendering core

use pulldown_cmark::{html::push_html, Options, Parser};
use tauri::{command, AppHandle, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder};

mod commands {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tauri::Emitter;

    /// Per-window file path mapping: window label → file path.
    /// Used so each window knows which file to load on init,
    /// avoiding the race condition where the open-file event
    /// fires before the frontend is ready to listen.
    ///
    /// Check whether a path looks like a markdown/text file.
    pub fn is_md_file(path: &str) -> bool {
        let lower = path.to_lowercase();
        lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".txt")
    }

    /// Managed state: CLI file paths to open on startup.
    #[derive(Default)]
    pub struct CliPaths(pub Mutex<Vec<String>>);

    /// Per-window file path mapping: window label → file path.
    #[derive(Default)]
    pub struct WindowFiles(pub Mutex<HashMap<String, String>>);

    /// Read CLI args and store markdown file paths in state.
    /// Reads directly from std::env::args() for reliability —
    /// avoids Tauri CLI plugin configuration issues.
    pub fn init_cli_paths(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
        let paths: Vec<String> = std::env::args()
            .skip(1) // skip program name
            .filter(|arg| !arg.starts_with('-')) // skip flags
            .filter(|p| is_md_file(p))
            .collect();

        log::info!("init_cli_paths: args parsed, md files = {:?}", paths);
        if !paths.is_empty() {
            let state = app.state::<CliPaths>();
            *state.0.lock().unwrap() = paths;
        }

        Ok(())
    }

    #[command]
    pub fn get_cli_paths(app: tauri::AppHandle) -> Vec<String> {
        app.state::<CliPaths>().0.lock().unwrap().clone()
    }

    /// Get the file path assigned to a specific window by label.
    /// Returns None if no file is assigned to this window.
    #[command]
    pub fn get_window_file(app: tauri::AppHandle, label: String) -> Option<String> {
        app.state::<WindowFiles>()
            .0
            .lock()
            .unwrap()
            .get(&label)
            .cloned()
    }

    /// Set the window title for a specific window by label.
    #[command]
    pub fn set_window_title(app: tauri::AppHandle, label: String, title: String) {
        if let Some(window) = app.get_webview_window(&label) {
            window.set_title(&title).ok();
        }
    }

    /// Format a window title as "filename : Markdown Viewer".
    pub fn window_title(filename: &str) -> String {
        format!("{} : Markdown Viewer", filename)
    }

    pub(super) fn build_menus_from_app<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
    ) -> Result<tauri::menu::Menu<R>, Box<dyn std::error::Error>> {
        // App menu
        let about_item = tauri::menu::MenuItemBuilder::new("About Markdown Viewer")
            .id("app_about")
            .build(app)?;
        let quit_item = tauri::menu::MenuItemBuilder::new("Quit")
            .id("app_quit")
            .accelerator("CmdOrCtrl+Q")
            .build(app)?;
        let app_menu = tauri::menu::SubmenuBuilder::new(app, "Markdown Viewer")
            .item(&about_item)
            .separator()
            .item(&quit_item)
            .build()?;

        // File menu
        let open_item = tauri::menu::MenuItemBuilder::new("Open…")
            .id("file_open")
            .accelerator("CmdOrCtrl+O")
            .build(app)?;
        let export_item = tauri::menu::MenuItemBuilder::new("Export…")
            .id("file_export")
            .build(app)?;
        let print_item = tauri::menu::MenuItemBuilder::new("Print…")
            .id("print")
            .accelerator("CmdOrCtrl+P")
            .build(app)?;
        let close_item = tauri::menu::MenuItemBuilder::new("Close")
            .id("file_close")
            .accelerator("CmdOrCtrl+W")
            .build(app)?;
        let exit_item = tauri::menu::MenuItemBuilder::new("Exit")
            .id("file_exit")
            .accelerator("CmdOrCtrl+Q")
            .build(app)?;

        let file_menu = tauri::menu::SubmenuBuilder::new(app, "File")
            .item(&open_item)
            .item(&export_item)
            .separator()
            .item(&print_item)
            .separator()
            .item(&close_item)
            .item(&exit_item)
            .build()?;

        // View menu
        let zoom_in_item = tauri::menu::MenuItemBuilder::new("Zoom In")
            .id("view_zoom_in")
            .accelerator("CmdOrCtrl+=")
            .build(app)?;
        let zoom_out_item = tauri::menu::MenuItemBuilder::new("Zoom Out")
            .id("view_zoom_out")
            .accelerator("CmdOrCtrl+-")
            .build(app)?;
        let zoom_reset_item = tauri::menu::MenuItemBuilder::new("Actual Size")
            .id("view_zoom_reset")
            .accelerator("CmdOrCtrl+0")
            .build(app)?;
        let theme_item = tauri::menu::MenuItemBuilder::new("Toggle Theme")
            .id("view_toggle_theme")
            .build(app)?;

        let view_menu = tauri::menu::SubmenuBuilder::new(app, "View")
            .item(&zoom_in_item)
            .item(&zoom_out_item)
            .item(&zoom_reset_item)
            .separator()
            .item(&theme_item)
            .build()?;

        // Window menu
        let mut window_submenu_builder = tauri::menu::SubmenuBuilder::new(app, "Window");
        let bring_front_item = tauri::menu::MenuItemBuilder::new("Bring All to Front")
            .id("window_bring_front")
            .build(app)?;
        window_submenu_builder = window_submenu_builder.item(&bring_front_item);
        window_submenu_builder = window_submenu_builder.separator();
        for (label, window) in app.webview_windows().iter() {
            let title = window.title().unwrap_or_else(|_| label.clone());
            let item_id = format!("window_{}", label);
            let item = tauri::menu::MenuItemBuilder::new(title)
                .id(&item_id)
                .build(app)?;
            window_submenu_builder = window_submenu_builder.item(&item);
        }
        let window_menu = window_submenu_builder.build()?;

        let menu = tauri::menu::MenuBuilder::new(app)
            .item(&app_menu)
            .item(&file_menu)
            .item(&view_menu)
            .item(&window_menu)
            .build()?;
        Ok(menu)
    }

    /// Cascade offset for the Nth secondary window so each new window lands
    /// at a distinct, visible position rather than stacking on top of the
    /// previous one. Tauri / AppKit do not consistently auto-cascade when
    /// windows are created back-to-back during startup.
    pub fn cascade_position(window_index: usize) -> (f64, f64) {
        let base_x = 120.0;
        let base_y = 120.0;
        let step = 30.0;
        let n = window_index as f64;
        (base_x + n * step, base_y + n * step)
    }

    /// Create a window for a file (generic version for use in plugin closures).
    ///
    /// MUST be called on the main thread on macOS — `WebviewWindowBuilder::build()`
    /// initializes AppKit/WKWebView objects, which require the main run loop.
    pub(super) fn create_window_for_file<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        file_path: &str,
        display: &str,
    ) -> Result<(), String> {
        let window_count = app.webview_windows().len();
        // Use sanitized filename as label for easier identification
        let sanitized = display
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();
        let label = format!("window-{}-{}", window_count, sanitized);
        let title = commands::window_title(display);
        let (x, y) = cascade_position(window_count);
        log::info!(
            "create_window_for_file: label={}, file={}, title={}, position=({},{})",
            label,
            file_path,
            title,
            x,
            y
        );
        // Pass file path as URL query parameter (URL-encoded) — available immediately on page load.
        let url = format!("index.html?file={}", urlencoding::encode(file_path));
        let _window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
            .title(&title)
            .inner_size(1280.0, 1024.0)
            .position(x, y)
            .build()
            .map_err(|e| format!("Failed to create window: {}", e))?;
        log::info!(
            "create_window_for_file: window created successfully for {}",
            file_path
        );
        // Rebuild menu to include new window in Window menu
        if let Ok(menu) = build_menus_from_app(app) {
            let _ = app.set_menu(menu);
        }
        Ok(())
    }

    /// Route a file path to the right window:
    ///   - Main exists and is empty (no CLI/Opened path bound) → fill main, set title.
    ///   - Main exists and already has a file → create a new window.
    ///   - Main doesn't exist yet (Apple Event arrived before setup) → push to CliPaths
    ///     so setup() / on_page_load picks it up when the main window is built.
    ///
    /// MUST be called on the main thread on macOS.
    pub(super) fn open_or_create_window<R: tauri::Runtime>(
        app: &tauri::AppHandle<R>,
        file_path: &str,
    ) {
        let display = file_path.split('/').next_back().unwrap_or(file_path);
        log::info!("open_or_create_window called for file: {}", file_path);

        let main = app.get_webview_window("main");
        let cli_paths = app.state::<CliPaths>();

        let should_fill_main = {
            let mut paths = cli_paths.0.lock().unwrap();
            if paths.is_empty() {
                paths.push(file_path.to_string());
                main.is_some()
            } else {
                false
            }
        };

        if should_fill_main {
            log::info!(
                "open_or_create_window: filling main window with {}",
                file_path
            );
            if let Some(main) = main {
                let _ = main.set_title(&window_title(display));
                let js = format!(
                    "(function() {{ if (typeof loadFile === 'function') {{ loadFile({}); }} }})();",
                    serde_json::to_string(file_path).unwrap_or_default()
                );
                let _ = main.eval(&js);
            }
            return;
        }

        // Main doesn't exist yet → CliPaths now holds this path; setup() will use it.
        if main.is_none() {
            log::info!(
                "open_or_create_window: main window not exists yet, queued file {}",
                file_path
            );
            return;
        }

        // Main exists and already has a file → new window.
        log::info!(
            "open_or_create_window: creating new window for {}",
            file_path
        );
        let _ = create_window_for_file(app, file_path, display);
    }

    /// Create a new window for the given file path (command version).
    /// The window title is set to the display name (basename).
    /// Emits a "mdviewer:open-file" event with the file path for the frontend.
    #[command]
    pub fn create_window(app: AppHandle, file_path: &str, title: &str) -> Result<(), String> {
        create_window_for_file(&app, file_path, title)
    }

    #[command]
    pub fn render_md(markdown: &str) -> String {
        render_markdown(markdown)
    }

    #[command]
    pub fn render_md_for_file(markdown: &str, base_dir: &str) -> String {
        render_markdown_with_base(markdown, Some(base_dir))
    }

    #[command]
    pub fn extract_fm(markdown: &str) -> (String, String) {
        extract_frontmatter(markdown)
    }

    /// Read a file and return its content.
    #[command]
    pub fn read_file(path: &str) -> Result<String, String> {
        std::fs::read_to_string(path).map_err(|e| e.to_string())
    }

    /// Watch a file for changes. Spawns a background thread that emits
    /// "mdviewer:file-changed" Tauri events with updated content when the file is modified.
    /// Returns the initial file content.
    #[command]
    pub fn print_window(app_handle: tauri::AppHandle, label: String) {
        if let Some(window) = app_handle.get_webview_window(&label) {
            let _ = window.print();
        }
    }

    #[command]
    pub fn get_about_info(_app_handle: tauri::AppHandle) -> Result<(String, String), String> {
        let version = env!("CARGO_PKG_VERSION").to_string();
        // Try to get git sha from environment or git command
        let git_sha = std::process::Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|out| if out.status.success() {
                String::from_utf8(out.stdout).ok()
            } else { None })
            .unwrap_or_else(|| "unknown".to_string())
            .trim()
            .to_string();
        Ok((version, git_sha))
    }

    #[command]
    pub fn open_file_new_window(app_handle: tauri::AppHandle) -> Result<(), String> {
        use tauri_plugin_dialog::DialogExt;
        // Open file dialog blocking but runs in spawn thread
        let file_path = app_handle
            .dialog()
            .file()
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .blocking_pick_file();
        if let Some(path) = file_path {
            if let Some(path_str) = path.as_path() {
                let path_str = path_str.to_string_lossy().to_string();
                if is_md_file(&path_str) {
                    let display = path_str.split('/').next_back().unwrap_or(&path_str);
                    // Create new window for file
                    let _ = commands::create_window_for_file(&app_handle, &path_str, display);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    #[command]
    pub fn watch_file(path: &str, app_handle: tauri::AppHandle) -> Result<String, String> {
        use std::path::PathBuf;

        let path = PathBuf::from(path);
        if !path.exists() {
            return Err(format!("File not found: {}", path.display()));
        }
        let current = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        // Compute initial content hash for dedup.
        let content_hash = {
            let mut s = DefaultHasher::new();
            current.hash(&mut s);
            s.finish()
        };

        let path_clone = path.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let app_handle_clone = app_handle.clone();

        // Spawn a background watcher thread.
        std::thread::spawn(move || {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            use std::sync::atomic::Ordering;
            let mut last_hash = content_hash;

            loop {
                if stopped.load(Ordering::Relaxed) {
                    break;
                }
                // Poll every 1s. On macOS, fsevents-based watchers work better but
                // polling is simpler and cross-platform.
                std::thread::sleep(std::time::Duration::from_millis(1000));

                match std::fs::read_to_string(&path_clone) {
                    Ok(new_content) => {
                        let mut s = DefaultHasher::new();
                        new_content.hash(&mut s);
                        let h = s.finish();
                        if h != last_hash {
                            last_hash = h;
                            // Emit event to frontend with the updated content.
                            let _ = app_handle_clone.emit("mdviewer:file-changed", &new_content);
                        }
                    }
                    Err(_) => {
                        // File was deleted or renamed — stop watching.
                        break;
                    }
                }
            }
        });

        Ok(current)
    }

    /// Export rendered markdown as a standalone HTML file.
    #[command]
    pub fn export_html(content: &str, output_path: &str, title: &str) -> Result<(), String> {
        let html = render_markdown(content);
        let full_html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
    max-width: 800px; margin: 0 auto; padding: 32px 24px; line-height: 1.6;
    color: #1a1a1a; background: #fff; }}
  h1, h2 {{ border-bottom: 1px solid #e0e0e0; padding-bottom: 0.3em; }}
  code {{ background: #f6f8fa; padding: 0.2em 0.4em; border-radius: 3px; font-size: 0.875em;
    font-family: 'SFMono-Regular', Consolas, monospace; }}
  pre {{ background: #f6f8fa; padding: 16px; border-radius: 6px; overflow-x: auto; }}
  pre code {{ background: none; padding: 0; }}
  table {{ border-collapse: collapse; width: 100%; }}
  th, td {{ border: 1px solid #e0e0e0; padding: 6px 13px; }}
  th {{ background: #f6f8fa; }}
  blockquote {{ border-left: 0.25em solid #e0e0e0; padding-left: 1em; color: #666; }}
  a {{ color: #0366d6; text-decoration: none; }}
  a:hover {{ text-decoration: underline; }}
  .callout {{ margin: 1em 0; padding: 1em; border-radius: 6px; border-left: 4px solid; }}
  .callout.note {{ background: #dbeafe; border-color: #3b82f6; }}
  .callout.tip {{ background: #d1fae5; border-color: #10b981; }}
  .callout.warning {{ background: #fef3c7; border-color: #f59e0b; }}
  .callout.caution {{ background: #fee2e2; border-color: #ef4444; }}
  .callout.important {{ background: #ede9fe; border-color: #8b5cf6; }}
  .math-inline {{ color: #0366d6; }}
  .mermaid {{ text-align: center; }}
  img {{ max-width: 100%; }}
</style>
</head>
<body>
{html}
</body>
</html>"#,
            title = title,
            html = html
        );
        std::fs::write(output_path, full_html)
            .map_err(|e| format!("Failed to write {}: {}", output_path, e))?;
        Ok(())
    }
}

// ─── macOS / iOS Document Open Plugin ────────────────────────────────────────

/// Plugin that handles macOS/iOS "open file" events (double-click in Finder,
/// `open file.md` from terminal, dock badge, etc.). Routes each file through
/// `open_or_create_window` so the empty main window is filled on first launch
/// instead of leaving an extra blank window behind.
///
/// Deadlock avoidance: `WebviewWindowBuilder::build()` cannot be called
/// (directly or via `run_on_main_thread`) from inside this handler. Tauri
/// holds `manager.plugins.lock()` for the duration of `on_event`, and the
/// window-creation path acquires the same lock to fire `window_created` on
/// plugins. `run_on_main_thread` from the main thread is synchronous in
/// Tauri 2, so deferring through it doesn't help. The fix: hop to a tokio
/// task with `async_runtime::spawn`, then back to the main thread with
/// `run_on_main_thread` from a non-main thread — that path uses the event
/// loop proxy and runs the closure in the next iteration, after `on_event`
/// has returned and released the plugin lock.
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn open_file_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("mdviewer-open-file")
        .on_event(|app, event| {
            if let RunEvent::Opened { urls } = event {
                log::info!(
                    "open_file_plugin: RunEvent::Opened with {} urls",
                    urls.len()
                );
                for url in urls {
                    if let Ok(path) = url.to_file_path() {
                        let path_str = path.to_string_lossy().into_owned();
                        if commands::is_md_file(&path_str) {
                            log::info!(
                                "open_file_plugin: opening md file via Finder event: {}",
                                path_str
                            );
                            let app = app.app_handle().clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = app.clone().run_on_main_thread(move || {
                                    commands::open_or_create_window(&app, &path_str);
                                });
                            });
                        } else {
                            log::info!("open_file_plugin: ignored non-md file: {}", path_str);
                        }
                    } else {
                        log::warn!(
                            "open_file_plugin: failed to convert url to file path: {:?}",
                            url
                        );
                    }
                }
            }
        })
        .build()
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn open_file_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    // No-op on non-macOS/iOS platforms
    tauri::plugin::Builder::new("mdviewer-open-file").build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = commands::CliPaths(std::sync::Mutex::new(Vec::new()));
    let window_files =
        commands::WindowFiles(std::sync::Mutex::new(std::collections::HashMap::new()));

    let mut builder = tauri::Builder::default().manage(paths).manage(window_files);

    // Single-instance plugin: when macOS (or another OS) spawns a duplicate process,
    // the new process forwards its argv to this running instance instead of starting
    // a second app. The plugin's callback runs on a tokio task — NOT the main thread —
    // so window creation must be dispatched via `run_on_main_thread`.
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            log::info!("single_instance: new invocation with args {:?}", args);
            let app = app.clone();
            let _ = app.clone().run_on_main_thread(move || {
                for arg in args.iter().skip(1) {
                    if !arg.starts_with('-') && commands::is_md_file(arg) {
                        log::info!("single_instance: opening file from args: {}", arg);
                        commands::open_or_create_window(&app, arg);
                    } else {
                        log::debug!("single_instance: skipping arg: {}", arg);
                    }
                }
            });
        }));
    }

    builder
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(open_file_plugin())
        .setup(|app| {
            log::info!("setup: initializing app");

            let menu = commands::build_menus_from_app(app.handle())?;
            app.set_menu(menu)?;

            // Handle menu events
            let app_handle = app.handle().clone();
            app.on_menu_event(move |_, event| {
                let id = event.id().as_ref();
                match id {
                    "app_about" => {
                        // Show about info with version and git sha, open project homepage
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.eval(r#"
                                window.__TAURI__.core.invoke('get_about_info').then(([version, gitSha]) => {
                                    const html = `<div style="font-family: system-ui; padding: 20px; max-width: 400px;"><h2>Markdown Viewer</h2><p><strong>Version:</strong> ${version}</p><p><strong>Git SHA:</strong> ${gitSha.substring(0, 12)}</p><p><strong>Project Homepage:</strong> <a href="https://github.com/rajatarya/mdviewer" target="_blank">https://github.com/rajatarya/mdviewer</a></p></div>`;
                                    const modal = document.createElement('div');
                                    modal.style.cssText = 'position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,0,0,0.5);display:flex;align-items:center;justify-content:center;z-index:10000';
                                    const box = document.createElement('div');
                                    box.style.cssText = 'background:white;color:black;padding:20px;border-radius:8px;max-width:500px';
                                    box.innerHTML = html + '<button onclick="this.closest(\'div\').remove()" style="margin-top:10px">Close</button>';
                                    modal.appendChild(box);
                                    document.body.appendChild(modal);
                                }).catch(e => {
                                    alert('Markdown Viewer\\nVersion unknown\\n\\nProject Homepage: https://github.com/rajatarya/mdviewer');
                                    window.open('https://github.com/rajatarya/mdviewer', '_blank');
                                });
                            "#);
                        }
                    }
                    "app_quit" => {
                        app_handle.exit(0);
                    }
                    _ => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            match id {
                                "print" => {
                                    let _ = window.eval(r#"
                                        const filenameEl = document.getElementById('filename');
                                        if (filenameEl && filenameEl.textContent) {
                                            const baseName = filenameEl.textContent.replace(/\.[^.]+$/, '');
                                            document.title = `${baseName}.pdf`;
                                            document.body.dataset.filename = filenameEl.textContent;
                                        }
                                        window.print();
                                    "#);
                                }
                                "file_open" => {
                                    // Open file dialog and create new window in a thread
                                    let handle = app_handle.clone();
                                    std::thread::spawn(move || {
                                        let _ = commands::open_file_new_window(handle);
                                    });
                                }
                                "file_export" => {
                                    let _ = window.eval("document.getElementById('export-btn')?.click()");
                                }
                                "file_close" => {
                                    let _ = window.close();
                                }
                                "file_exit" => {
                                    app_handle.exit(0);
                                }
                                "view_zoom_in" => {
                                    let _ = window.eval("document.getElementById('zoom-in-btn')?.click()");
                                }
                                "view_zoom_out" => {
                                    let _ = window.eval("document.getElementById('zoom-out-btn')?.click()");
                                }
                                "view_zoom_reset" => {
                                    let _ = window.eval("document.getElementById('zoom-reset-btn')?.click()");
                                }
                                "view_toggle_theme" => {
                                    let _ = window.eval("document.getElementById('theme-btn')?.click()");
                                }
                                "window_bring_front" => {
                                    // Bring all windows to front
                                    for win in app_handle.webview_windows().values() {
                                        let _ = win.set_focus();
                                    }
                                }
                                _ => {
                                    // Handle per-window bring to front: id format window_<label>
                                    if let Some(stripped) = id.strip_prefix("window_") {
                                        if let Some(win) = app_handle.get_webview_window(stripped) {
                                            let _ = win.set_focus();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });

            commands::init_cli_paths(app)?;

            let paths = app.state::<commands::CliPaths>();
            let file_paths = paths.0.lock().unwrap().clone();
            log::info!("setup: CLI file paths = {:?}", file_paths);
            if let Some(first_path) = file_paths.first() {
                let display = first_path.split('/').next_back().unwrap_or(first_path);
                let title = commands::window_title(display);
                if let Some(main_window) = app.get_webview_window("main") {
                    main_window.set_title(&title).ok();
                    log::info!("setup: set main window title to {}", title);
                }
            }
            for path_str in file_paths.iter().skip(1) {
                let display = path_str.split('/').next_back().unwrap_or(path_str);
                log::info!("setup: creating window for CLI arg {}", path_str);
                let _ = commands::create_window_for_file(app.app_handle(), path_str, display);
            }
            Ok(())
        })
        .on_page_load(|webview, _payload| {
            log::info!("on_page_load: window label = {}", webview.window().label());
            // When the main window loads, check if it has a CLI file to open.
            if webview.window().label() == "main" {
                let app_handle = webview.app_handle().clone();
                let paths = app_handle.state::<commands::CliPaths>();
                let file_paths = paths.0.lock().unwrap().clone();
                log::info!("on_page_load: main window loading, CLI paths = {:?}", file_paths);
                if let Some(first_path) = file_paths.first() {
                    log::info!("on_page_load: loading first CLI file into main window: {}", first_path);
                    let js = format!(
                        "(function() {{ if (typeof loadFile === 'function') {{ loadFile({}); }} }})();",
                        serde_json::to_string(first_path).unwrap_or_default()
                    );
                    let _ = webview.eval(&js);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::render_md,
            commands::render_md_for_file,
            commands::extract_fm,
            commands::read_file,
            commands::watch_file,
            commands::export_html,
            commands::get_cli_paths,
            commands::get_window_file,
            commands::create_window,
            commands::set_window_title,
            commands::print_window,
            commands::open_file_new_window,
            commands::get_about_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use ammonia::Builder;
use regex::Regex;

// ─── Emoji Map ───────────────────────────────────────────────────────────────

fn emoji_map() -> &'static [(&'static str, char)] {
    &[
        ("rocket", '🚀'),
        ("heart", '❤'),
        ("thumbsup", '👍'),
        ("+1", '👍'),
        ("smile", '😊'),
        ("fire", '🔥'),
        ("star", '⭐'),
        ("eye", '👁'),
        ("memo", '📝'),
        ("warning", '⚠'),
        ("sparkles", '✨'),
        ("bulb", '💡'),
        ("lock", '🔒'),
        ("unlock", '🔓'),
        ("check", '✅'),
        ("x", '❌'),
        ("question", '❓'),
        ("lightning", '⚡'),
        ("bell", '🔔'),
        ("gear", '⚙'),
        ("book", '📖'),
        ("link", '🔗'),
        ("clipboard", '📋'),
        ("pencil", '✏'),
        ("zap", '⚡'),
        ("globe", '🌍'),
        ("camera", '📷'),
        ("music", '🎵'),
        ("sun", '☀'),
        ("moon", '🌙'),
        ("cloud", '☁'),
        ("rain", '🌧'),
        ("snow", '❄'),
        ("umbrella", '☂'),
        ("anchor", '⚓'),
        ("hammer", '🔨'),
        ("wrench", '🔧'),
        ("shield", '🛡'),
        ("key", '🔑'),
        ("gift", '🎁'),
        ("trophy", '🏆'),
        ("medal", '🎖'),
        ("flag", '🚩'),
        ("target", '🎯'),
        ("chart", '📊'),
        ("bar", '📈'),
        ("email", '📧'),
        ("phone", '📱'),
        ("computer", '💻'),
        ("mobile", '📲'),
        ("desktop", '🖥'),
        ("printer", '🖨'),
        ("battery", '🔋'),
        ("movie", '🎬'),
        ("game", '🎮'),
        ("sports", '⚽'),
        ("music_note", '🎶'),
        ("art", '🎨'),
        ("microphone", '🎤'),
        ("headphone", '🎧'),
        ("tv", '📺'),
        ("frame", '🖼'),
        ("palette", '🎨'),
    ]
}

/// Preprocess emoji shortcodes (:emoji:) into unicode characters
fn preprocess_emojis(markdown: &str) -> String {
    let re = Regex::new(r":([a-z0-9_+-]+):").unwrap();
    re.replace_all(markdown, |caps: &regex::Captures| {
        let key = &caps[1];
        for (k, v) in emoji_map() {
            if **k == *key {
                return v.to_string();
            }
        }
        caps[0].to_string() // no match, keep original
    })
    .into_owned()
}

// ─── Math Preprocessing ──────────────────────────────────────────────────────

/// Preprocess LaTeX math expressions ($inline$$display$) into styled spans
fn preprocess_math(markdown: &str) -> String {
    // Use placeholders to avoid interfering with each other
    let mut result = markdown.to_string();

    // Process display math first ($$...$$), replace with placeholders
    let block_re = Regex::new(r"\$\$(.+?)\$\$").unwrap();
    let mut block_placeholders: Vec<(usize, String)> = Vec::new();
    let mut idx = 0usize;
    let temp = block_re.replace_all(&result, |caps: &regex::Captures| {
        let placeholder = format!("\x00BLOCK_MATH_{}\x00", idx);
        block_placeholders.push((idx, caps[0].to_string()));
        idx += 1;
        placeholder
    });
    result = temp.into_owned();

    // Process inline math ($...$) on the result with placeholders
    let inline_re = Regex::new(r"\$([^\$]+)\$").unwrap();
    result = inline_re
        .replace_all(&result, "<span class=\"math-inline\">$1</span>")
        .into_owned();

    // Restore display math blocks
    for (i, original) in block_placeholders.iter() {
        let placeholder = format!("\x00BLOCK_MATH_{}\x00", i);
        result = result.replace(
            &placeholder,
            &format!(
                "<div class=\"math-display\">{}</div>",
                &original[2..original.len() - 2]
            ),
        );
    }

    result
}

// ─── Callout Preprocessing ───────────────────────────────────────────────────

/// Preprocess GitHub-style callouts ([!TYPE]) into styled HTML divs
fn preprocess_callouts(markdown: &str) -> String {
    let header_re = Regex::new(r"^> \[!(\w+)](\+)?$").unwrap();

    let lines: Vec<&str> = markdown.lines().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        if let Some(cap) = header_re.captures(line) {
            let kind = cap[1].to_lowercase();
            let foldable = cap.get(2).is_some();
            let summary = kind
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or(kind.clone())
                + &kind[1..];

            // Collect content lines
            i += 1;
            let mut content_lines: Vec<String> = Vec::new();
            while i < lines.len() {
                if let Some(content) = lines[i].strip_prefix("> ") {
                    content_lines.push(content.to_string());
                    i += 1;
                } else {
                    break;
                }
            }
            let content = content_lines.join("\n");

            if foldable {
                result.push_str(&format!(
                    "<details class=\"callout {}\" open=\"open\"><summary>{}</summary><p>{}</p></details>",
                    kind, summary, content
                ));
            } else {
                result.push_str(&format!(
                    "<div class=\"callout {}\"><p>{}</p></div>",
                    kind, content
                ));
            }
            continue;
        }
        result.push_str(line);
        result.push('\n');
        i += 1;
    }

    result
}

// ─── Wikilink Preprocessing ──────────────────────────────────────────────────

/// Preprocess wikilinks ([[link]]) into standard Markdown links.
/// Supports: [[Page]], [[#Heading]], [[Page#Heading]], [[Page|Display]]
fn preprocess_wikilinks(markdown: &str) -> String {
    let re = Regex::new(r"\[\[(.*?)\]\]").unwrap();
    re.replace_all(markdown, |caps: &regex::Captures| {
        let target = &caps[1];
        // Split on | for display text: [[Page|Display]]
        let (link_target, display_text) = if let Some(pipe_pos) = target.find('|') {
            (&target[..pipe_pos], target[pipe_pos + 1..].to_string())
        } else {
            (target, target.to_string())
        };
        if let Some(rest) = link_target.strip_prefix('#') {
            // In-page anchor: [[#Heading]] or [[#Heading|Display]]
            let heading = rest.to_lowercase().replace(' ', "-");
            // Strip leading # from display text for clean rendering
            let display = display_text.strip_prefix('#').unwrap_or(&display_text);
            let anchor = format!("#{}", heading);
            format!("[{}]({})", display, anchor)
        } else if link_target.contains('#') {
            // Page with anchor: [[Page#Heading]] or [[Page#Heading|Display]]
            let parts: Vec<&str> = link_target.splitn(2, '#').collect();
            let page = parts[0].to_lowercase().replace(' ', "-") + ".html";
            let anchor = parts[1].to_lowercase().replace(' ', "-");
            format!("[{}]({}#{})", display_text, page, anchor)
        } else {
            // Simple page link: [[Page]] or [[Page|Display]]
            let link = link_target.to_lowercase().replace(' ', "-") + ".html";
            format!("[{}]({})", display_text, link)
        }
    })
    .into_owned()
}

// ─── Image Path Resolution ───────────────────────────────────────────────────

/// Simple base64 encoder for inline image data URIs.
/// Avoids adding a base64 crate dependency.
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;
    let len = data.len();
    while i < len {
        let remaining = len - i;
        let b0 = data[i] as u32;
        if remaining > 1 {
            let b1 = data[i + 1] as u32;
            if remaining > 2 {
                let b2 = data[i + 2] as u32;
                let buf = (b0 << 16) | (b1 << 8) | b2;
                result.push(CHARS[(buf >> 18) as usize] as char);
                result.push(CHARS[((buf >> 12) & 0x3F) as usize] as char);
                result.push(CHARS[((buf >> 6) & 0x3F) as usize] as char);
                result.push(CHARS[(buf & 0x3F) as usize] as char);
                i += 3;
            } else {
                let buf = (b0 << 16) | (b1 << 8);
                result.push(CHARS[(buf >> 18) as usize] as char);
                result.push(CHARS[((buf >> 12) & 0x3F) as usize] as char);
                result.push(CHARS[((buf >> 6) & 0x3F) as usize] as char);
                result.push('=');
                i += 2;
            }
        } else {
            let buf = b0 << 16;
            result.push(CHARS[(buf >> 18) as usize] as char);
            result.push(CHARS[((buf >> 12) & 0x3F) as usize] as char);
            result.push('=');
            result.push('=');
            i += 1;
        }
    }
    result
}

/// Read an image file and return a base64 data URI.
/// Returns None if the file can't be read or the format is unsupported.
fn read_image_as_data_uri(path: &std::path::Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    let mime = match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("tiff" | "tif") => "image/tiff",
        Some("avif") => "image/avif",
        _ => return None,
    };
    let b64 = base64_encode(&data);
    Some(format!("data:{};base64,{}", mime, b64))
}

/// Resolve relative image paths in markdown to base64 data URIs.
/// Only processes paths that are NOT absolute URLs (http://, https://, data:, /).
/// Runs before markdown parsing so the parser sees data URIs natively.
fn resolve_image_paths(text: &str, base_dir: &str) -> String {
    let img_re = Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap();
    img_re
        .replace_all(text, |caps: &regex::Captures| {
            let url = &caps[2];
            if url.starts_with("http://")
                || url.starts_with("https://")
                || url.starts_with("data:")
                || url.starts_with('/')
                || url.starts_with('#')
            {
                return caps[0].to_string();
            }
            let full_path = std::path::Path::new(base_dir).join(url);
            match read_image_as_data_uri(&full_path) {
                Some(data_uri) => format!("![{}]({})", &caps[1], data_uri),
                None => caps[0].to_string(),
            }
        })
        .into_owned()
}

// ─── Main Rendering Pipeline ─────────────────────────────────────────────────

/// Render markdown string to sanitized HTML.
/// Fenced code blocks are extracted first (so `---` inside code doesn't
/// interfere with frontmatter detection), then frontmatter is stripped,
/// then inline code is extracted. All placeholders are restored before
/// markdown parsing. Downstream preprocessors (emoji, math, wikilinks,
/// callouts) operate on the cleaned content.
pub fn render_markdown(markdown: &str) -> String {
    render_markdown_with_base(markdown, None)
}

fn render_markdown_with_base(markdown: &str, base_dir: Option<&str>) -> String {
    // Phase 0 — Extract fenced blocks so regex preprocessors don't alter code.
    let fence_re = Regex::new(r"(?s)```[^\n`]*\n.*?```").unwrap();
    let mut fence_blocks: Vec<String> = Vec::new();
    let without_fences = fence_re
        .replace_all(markdown, |caps: &regex::Captures| {
            let idx = fence_blocks.len();
            fence_blocks.push(caps[0].to_string());
            format!("\x00FENCED_BLOCK_{}\x00", idx)
        })
        .into_owned();

    // Phase 0a — Strip YAML frontmatter so delimiters don't render as horizontal rules.
    // Must happen after fenced block extraction (so `---` inside code is safe) but
    // before inline code extraction (so `$...$` in inline code is protected).
    let without_frontmatter = if let Some(rest) = without_fences.strip_prefix("---\n") {
        if let Some(end_pos) = rest.find("\n---\n") {
            &rest[end_pos + 5..]
        } else {
            &without_fences
        }
    } else {
        &without_fences
    };
    let without_frontmatter = without_frontmatter.to_string();

    // Phase 0b — Extract inline code (single backticks) for the same reason.
    // Prevents emoji/math/wikilink preprocessors from matching inside code.
    // Store the FULL match (including backticks) so restoration is exact.
    let inline_re = Regex::new(r"`[^`]+`").unwrap();
    let mut inline_blocks: Vec<String> = Vec::new();
    let without_inline = inline_re
        .replace_all(&without_frontmatter, |caps: &regex::Captures| {
            let idx = inline_blocks.len();
            inline_blocks.push(caps[0].to_string());
            format!("\x00INLINE_CODE_{}\x00", idx)
        })
        .into_owned();

    // 1. Preprocess math
    let with_math = preprocess_math(&without_inline);
    // 2. Preprocess emojis
    let with_emojis = preprocess_emojis(&with_math);
    // 3. Preprocess wikilinks
    let with_wikilinks = preprocess_wikilinks(&with_emojis);
    // 4. Preprocess callouts
    let mut with_callouts = preprocess_callouts(&with_wikilinks);

    // Restore fenced blocks before markdown parsing.
    for (idx, block) in fence_blocks.iter().enumerate() {
        let placeholder = format!("\x00FENCED_BLOCK_{}\x00", idx);
        with_callouts = with_callouts.replace(&placeholder, block);
    }

    // Restore inline code blocks.
    for (idx, block) in inline_blocks.iter().enumerate() {
        let placeholder = format!("\x00INLINE_CODE_{}\x00", idx);
        with_callouts = with_callouts.replace(&placeholder, block);
    }

    // Phase 0c — Resolve relative image paths to data URIs (if base_dir provided).
    // Runs after code restoration so code blocks are protected.
    if let Some(base) = base_dir {
        with_callouts = resolve_image_paths(&with_callouts, base);
    }

    // 5. Parse markdown
    let mut options = Options::empty();
    options.insert(Options::ENABLE_GFM);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(&with_callouts, options);
    let mut unsafe_html = String::new();
    push_html(&mut unsafe_html, parser);

    // 6. Sanitize HTML
    Builder::new()
        .rm_tags(&["script"])
        .add_tags(&[
            "table", "thead", "tbody", "tr", "th", "td", "input", "details", "summary", "img",
        ])
        .add_tag_attributes("input", &["type", "checked"])
        .add_tag_attributes("code", &["class"])
        .add_tag_attributes("span", &["class"])
        .add_tag_attributes("div", &["class"])
        .add_tag_attributes("details", &["class", "open"])
        .add_tag_attributes("summary", &["class"])
        .add_tag_attributes("img", &["src", "alt", "width", "height", "class"])
        .add_url_schemes(&["data"])
        .clean(&unsafe_html)
        .to_string()
}

/// Extract YAML frontmatter and return (frontmatter_json, content_without_frontmatter)
pub fn extract_frontmatter(markdown: &str) -> (String, String) {
    if let Some(rest) = markdown.strip_prefix("---\n") {
        if let Some(end_pos) = rest.find("\n---\n") {
            let yaml_str = &rest[..end_pos];
            let content = &rest[end_pos + 5..];
            match serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
                Ok(value) => {
                    let json = serde_json::to_string(&value).unwrap_or_default();
                    (json, content.to_string())
                }
                Err(_) => (String::new(), markdown.to_string()),
            }
        } else {
            (String::new(), markdown.to_string())
        }
    } else {
        (String::new(), markdown.to_string())
    }
}

// ─── Help Message ─────────────────────────────────────────────────────────────

/// Return the formatted help message for `mdviewer --help`.
pub fn help_message() -> String {
    r#"Markdown Viewer — A lightweight Markdown viewer for macOS

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
"#
    .to_string()
}

/// Check if `--help` flag is present in command-line arguments.
pub fn has_help_flag() -> bool {
    std::env::args().any(|a| a == "--help" || a == "-h")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_renders_header() {
        let input = "# Header";
        let expected = "<h1>Header</h1>\n";
        assert_eq!(render_markdown(input), expected);
    }

    #[test]
    fn it_renders_mermaid_fence() {
        let input = "```mermaid\ngraph TD;\n    A-->B;\n```";
        let expected =
            "<pre><code class=\"language-mermaid\">graph TD;\n    A--&gt;B;\n</code></pre>\n";
        assert_eq!(render_markdown(input), expected);
    }

    #[test]
    fn it_sanitizes_xss() {
        let input = "<script>alert('xss')</script>";
        let output = render_markdown(input);
        assert!(!output.contains("<script>"));
    }

    #[test]
    fn it_renders_tables() {
        let input = " | Header |
 | --- |
 | Cell |";
        let expected = "<table><thead><tr><th>Header</th></tr></thead><tbody>\n<tr><td>Cell</td></tr>\n</tbody></table>\n";
        assert_eq!(render_markdown(input), expected);
    }

    #[test]
    fn it_renders_task_lists() {
        let input = "- [x] Done\n- [ ] Pending";
        let expected = "<ul>\n<li><input type=\"checkbox\" checked=\"\">\nDone</li>\n<li><input type=\"checkbox\">\nPending</li>\n</ul>\n";
        assert_eq!(render_markdown(input), expected);
    }

    #[test]
    fn it_resolves_wikilinks() {
        let input = "[[My Document]]";
        let expected =
            "<p><a href=\"my-document.html\" rel=\"noopener noreferrer\">My Document</a></p>\n";
        assert_eq!(render_markdown(input), expected);
    }

    #[test]
    fn it_renders_emoji_shortcodes() {
        let input = ":rocket: :heart: :thumbsup:";
        let output = render_markdown(input);
        assert!(output.contains("🚀"));
        assert!(output.contains("❤"));
        assert!(output.contains("👍"));
    }

    #[test]
    fn it_renders_emoji_shortcode_with_plus() {
        let input = ":+1: :sparkles:";
        let output = render_markdown(input);
        assert!(output.contains("👍"));
        assert!(output.contains("✨"));
    }

    #[test]
    fn it_preserves_emoji_in_inline_code() {
        let input = "Use `:rocket:` for launch";
        let output = render_markdown(input);
        assert!(
            !output.contains("🚀"),
            "emoji inside inline code must not be replaced"
        );
        assert!(output.contains("<code>"), "must still produce code element");
    }

    #[test]
    fn it_preserves_emoji_in_fenced_code() {
        let input = "```\n:rocket: :heart:\n```";
        let output = render_markdown(input);
        assert!(
            !output.contains("🚀"),
            "emoji inside fenced code must not be replaced"
        );
        assert!(!output.contains("❤"));
    }

    #[test]
    fn it_preserves_unknown_emoji_shortcodes() {
        let input = "This has :unknown_emoji: shortcode";
        let output = render_markdown(input);
        assert!(
            output.contains(":unknown_emoji:"),
            "unknown shortcodes must be preserved"
        );
    }

    #[test]
    fn it_renders_inline_math() {
        let input = "$E = mc^2$";
        let output = render_markdown(input);
        assert!(output.contains("E = mc^2"));
        assert!(output.contains(r#"class="math-inline""#));
    }

    #[test]
    fn it_renders_display_math() {
        let input = "$$\\n\\int_0^\\infty x^2 dx\\n$$";
        let output = render_markdown(input);
        assert!(output.contains(r#"<div class="math-display">"#));
        assert!(output.contains(r#"</div>"#));
    }

    #[test]
    fn it_sanitizes_math_xss() {
        let input = "$<script>alert('xss')</script>$";
        let output = render_markdown(input);
        assert!(!output.contains("<script>"));
    }

    #[test]
    fn it_renders_callout_note() {
        let input = "> [!NOTE]\n> This is a note";
        let output = render_markdown(input);
        assert!(
            output.contains(r#"class="callout note""#),
            "output: {:?}",
            output
        );
        assert!(output.contains("This is a note"), "output: {:?}", output);
    }

    #[test]
    fn it_renders_callout_warning() {
        let input = "> [!WARNING]\n> Be careful";
        let output = render_markdown(input);
        assert!(output.contains(r#"class="callout warning""#));
    }

    #[test]
    fn it_renders_callout_foldable() {
        let input = "> [!TIP]+\n> Here is a tip";
        let output = render_markdown(input);
        assert!(
            output.contains(r#"class="callout tip""#),
            "output: {:?}",
            output
        );
        assert!(output.contains("<details"), "output: {:?}", output);
        assert!(output.contains("<summary>"), "output: {:?}", output);
    }

    #[test]
    fn it_renders_footnotes() {
        let input = "Text with a footnote[^1]\n\n[^1]: This is the footnote";
        let output = render_markdown(input);
        assert!(output.contains("footnote"));
        assert!(output.contains("Text with a footnote"));
    }

    #[test]
    fn it_extracts_frontmatter() {
        let input = "---\ntitle: Test\ndate: 2024-01-01\n---\n# Content";
        let (fm, content) = extract_frontmatter(input);
        assert!(fm.contains("Test"));
        assert!(content.starts_with("# Content"));
    }

    #[test]
    fn it_handles_no_frontmatter() {
        let input = "# No frontmatter";
        let (fm, content) = extract_frontmatter(input);
        assert!(fm.is_empty());
        assert_eq!(content, "# No frontmatter");
    }

    // ─── Task 14: File watching ─────────────────────────────────────────────────

    #[test]
    fn test_read_file_success() {
        let dir = std::env::temp_dir().join("mdviewer_test_read");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.md");
        std::fs::write(&path, "# Hello World").unwrap();
        let content = commands::read_file(path.to_str().unwrap()).unwrap();
        assert_eq!(content, "# Hello World");
        let err = commands::read_file("/nonexistent/file.md");
        assert!(err.is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // ─── Task 15: Export ────────────────────────────────────────────────────────

    #[test]
    fn test_export_html_basic() {
        let dir = std::env::temp_dir().join("mdviewer_test_export");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("export.html");
        let content = "---\ntitle: Test Doc\n---\n# Hello\n\n**bold** and *italic*.";
        let result = commands::export_html(content, path.to_str().unwrap(), "Test Doc");
        assert!(result.is_ok());
        let exported = std::fs::read_to_string(&path).unwrap();
        assert!(exported.contains("<!DOCTYPE html>"));
        assert!(exported.contains("<title>Test Doc</title>"));
        assert!(exported.contains("<h1>Hello</h1>"));
        assert!(exported.contains("<strong>bold</strong>"));
        assert!(exported.contains("<em>italic</em>"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_export_html_with_callouts() {
        let dir = std::env::temp_dir().join("mdviewer_test_export2");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("export_callouts.html");
        let content = "> [!NOTE]\n> This is a note";
        let result = commands::export_html(content, path.to_str().unwrap(), "Callouts");
        assert!(result.is_ok());
        let exported = std::fs::read_to_string(&path).unwrap();
        assert!(exported.contains("callout note"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // ─── Window cascade positioning ──────────────────────────────────────────
    // Bug: when multiple files are opened on launch, additional windows
    // were placed at the same default coordinates as the main window,
    // hiding them directly behind it. Each new window must get a distinct
    // position so all windows are visible.

    #[test]
    fn test_cascade_position_distinct_per_index() {
        let p0 = commands::cascade_position(0);
        let p1 = commands::cascade_position(1);
        let p2 = commands::cascade_position(2);
        assert_ne!(p0, p1);
        assert_ne!(p1, p2);
        assert_ne!(p0, p2);
    }

    #[test]
    fn test_cascade_position_offsets_grow_monotonically() {
        let (x0, y0) = commands::cascade_position(0);
        let (x1, y1) = commands::cascade_position(1);
        let (x2, y2) = commands::cascade_position(2);
        assert!(
            x1 > x0 && y1 > y0,
            "({}, {}) not > ({}, {})",
            x1,
            y1,
            x0,
            y0
        );
        assert!(
            x2 > x1 && y2 > y1,
            "({}, {}) not > ({}, {})",
            x2,
            y2,
            x1,
            y1
        );
    }

    #[test]
    fn test_cascade_position_offset_visible_apart() {
        // Adjacent windows must be far enough apart for the user to see
        // both window borders — at least 20 pixels in each axis.
        let (x0, y0) = commands::cascade_position(0);
        let (x1, y1) = commands::cascade_position(1);
        assert!((x1 - x0) >= 20.0, "x delta {} too small", x1 - x0);
        assert!((y1 - y0) >= 20.0, "y delta {} too small", y1 - y0);
    }

    #[test]
    fn test_is_md_file_accepts_md() {
        assert!(commands::is_md_file("test.md"));
    }

    #[test]
    fn test_is_md_file_accepts_markdown() {
        assert!(commands::is_md_file("doc.markdown"));
    }

    #[test]
    fn test_is_md_file_accepts_txt() {
        assert!(commands::is_md_file("readme.txt"));
    }

    #[test]
    fn test_is_md_file_rejects_html() {
        assert!(!commands::is_md_file("page.html"));
    }

    #[test]
    fn test_is_md_file_case_insensitive() {
        assert!(commands::is_md_file("FILE.MD"));
        assert!(commands::is_md_file("file.TXT"));
    }

    #[test]
    fn test_export_html_with_math() {
        let dir = std::env::temp_dir().join("mdviewer_test_export3");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("export_math.html");
        let content = "Use $E=mc^2$ for energy.";
        let result = commands::export_html(content, path.to_str().unwrap(), "Math Doc");
        assert!(result.is_ok());
        let exported = std::fs::read_to_string(&path).unwrap();
        assert!(exported.contains("E=mc^2"));
        assert!(exported.contains("math-inline"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // ─── Task 16: Frontend-backend command name consistency ─────────────────────

    /// Verify that every invoke() call in the frontend HTML uses the correct
    /// camelCase names that Tauri 2.x auto-converts from Rust snake_case #[command]
    /// names. Catches mismatches before they reach users.
    #[test]
    fn test_frontend_invokes_match_backend_commands() {
        // The set of all commands the frontend invokes.
        // Tauri 2.x #[command] registers commands with their exact Rust function name
        // (snake_case). The frontend must use the same snake_case names.
        let registered: std::collections::HashSet<&str> = [
            // Custom commands (exact Rust function names)
            "render_md",
            "render_md_for_file",
            "extract_fm",
            "read_file",
            "watch_file",
            "export_html",
            "get_cli_paths",
            "get_window_file",
            "set_window_title",
            "print_window",
            "open_file_new_window",
            "create_window",
            // plugin-provided commands (format: "plugin:<namespace>|<command>")
            "plugin:dialog|save",
            "plugin:dialog|open",
            "plugin:dialog|message",
        ]
        .into_iter()
        .collect();

        // Read the frontend HTML and extract all invoke('...') calls.
        let html = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("dist/index.html"),
        )
        .expect("dist/index.html must exist");

        // Match both snake_case custom commands and plugin:namespace|command format
        let re = regex::Regex::new(r#"invoke\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
        let frontend_calls: std::collections::HashSet<&str> = re
            .captures_iter(&html)
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str())
            .collect();

        let mut mismatches = Vec::new();
        for cmd in &frontend_calls {
            if !registered.contains(cmd) {
                mismatches.push(*cmd);
            }
        }

        assert!(
            mismatches.is_empty(),
            "Frontend invokes unknown commands: {:?}.\n\n\
             Registered backend commands: {:?}\n\
             Frontend calls: {:?}\n\n\
             Fix: Tauri 2.x auto-converts Rust snake_case to camelCase on the frontend.
             Use camelCase invoke() names like renderMd, readFile, etc.",
            mismatches,
            registered,
            frontend_calls,
        );
    }

    // ─── Task 17: CLI --help flag ─────────────────────────────────────────────────

    #[test]
    fn test_help_message_contains_usage() {
        let msg = help_message();
        assert!(msg.contains("USAGE:"));
        assert!(msg.contains("mdviewer"));
    }

    #[test]
    fn test_help_message_contains_flags() {
        let msg = help_message();
        assert!(msg.contains("--help"));
        assert!(msg.contains("-h"));
        assert!(msg.contains("Print this help"));
    }

    #[test]
    fn test_help_message_contains_examples() {
        let msg = help_message();
        assert!(msg.contains("mdviewer document.md"));
        assert!(msg.contains("mdviewer doc1.md doc2.md"));
    }

    #[test]
    fn test_help_message_contains_features() {
        let msg = help_message();
        assert!(msg.contains("GitHub Flavored Markdown"));
        assert!(msg.contains("Wikilinks"));
        assert!(msg.contains("Math"));
        assert!(msg.contains("Mermaid"));
        assert!(msg.contains("XSS-safe"));
    }

    #[test]
    fn test_help_message_contains_file_args() {
        let msg = help_message();
        assert!(msg.contains("FILES"));
        assert!(msg.contains(".md"));
        assert!(msg.contains(".markdown"));
        assert!(msg.contains(".txt"));
    }

    #[test]
    fn test_has_help_flag_detects_double_dash() {
        let args: Vec<String> = vec!["mdviewer".into(), "--help".into()];
        assert!(args.iter().any(|a| a == "--help" || a == "-h"));
    }

    #[test]
    fn test_has_help_flag_detects_single_dash() {
        let args: Vec<String> = vec!["mdviewer".into(), "-h".into(), "file.md".into()];
        assert!(args.iter().any(|a| a == "--help" || a == "-h"));
    }

    #[test]
    fn test_has_help_flag_not_present() {
        let args: Vec<String> = vec!["mdviewer".into(), "file.md".into()];
        assert!(!args.iter().any(|a| a == "--help" || a == "-h"));
    }

    /// Verify that every custom command's invoke() call uses camelCase argument keys,
    /// matching Tauri 2.x's automatic Rust snake_case → camelCase serialization.
    /// Prevents regressions like `output_path` instead of `outputPath`.
    #[test]
    fn test_frontend_invoke_args_are_camelcase() {
        let html = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("dist/index.html"),
        )
        .expect("dist/index.html must exist");

        // Flatten multi-line invoke calls into a single line for regex matching.
        let flat = html.replace('\n', " ").replace('\r', "");

        // Match: invoke('commandName', { ... })
        let invoke_re =
            regex::Regex::new(r#"invoke\s*\(\s*['"]([^'"]+)['"]\s*,\s*\{([^}]+)\}"#).unwrap();

        let custom_commands: std::collections::HashSet<&str> = [
            "render_md",
            "render_md_for_file",
            "extract_fm",
            "read_file",
            "watch_file",
            "export_html",
            "get_cli_paths",
            "set_window_title",
        ]
        .into_iter()
        .collect();

        let mut errors = Vec::new();

        // Extract argument keys (handles "key: value" patterns).
        let key_re = regex::Regex::new(r#"([a-zA-Z_][a-zA-Z0-9_]*)\s*:"#).unwrap();

        for cap in invoke_re.captures_iter(&flat) {
            let cmd = cap.get(1).unwrap().as_str();

            // Only check custom commands (not plugin:namespace|command)
            if !custom_commands.contains(cmd) {
                continue;
            }

            let args_str = cap.get(2).unwrap().as_str();

            for key_cap in key_re.captures_iter(args_str) {
                let key = key_cap.get(1).unwrap().as_str();
                if key.contains('_') {
                    errors.push(format!(
                        "Command '{}' has snake_case arg key '{}'. Tauri 2.x expects camelCase '{}'.",
                        cmd,
                        key,
                        key.replace('_', "")
                    ));
                }
            }
        }

        assert!(
            errors.is_empty(),
            "Frontend invoke() calls use snake_case argument keys, but Tauri 2.x serializes\n\
             Rust snake_case params as camelCase. Fix the frontend invoke() calls:\n\n{}",
            errors.join("\n")
        );
    }

    // ─── Task 18: Zoom support ──────────────────────────────────────────────────

    /// Verify that the frontend HTML has zoom buttons and zoom logic.
    /// Catches regressions where zoom controls are accidentally removed.
    #[test]
    fn test_zoom_buttons_exist_in_html() {
        let html = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("dist/index.html"),
        )
        .expect("dist/index.html must exist");

        assert!(
            html.contains("id=\"zoom-in-btn\""),
            "HTML must have a zoom-in button with id='zoom-in-btn'"
        );
        assert!(
            html.contains("id=\"zoom-out-btn\""),
            "HTML must have a zoom-out button with id='zoom-out-btn'"
        );
        assert!(
            html.contains("id=\"zoom-reset-btn\""),
            "HTML must have a zoom-reset button with id='zoom-reset-btn'"
        );
    }

    /// Verify that the frontend HTML has zoom level persistence in localStorage.
    #[test]
    fn test_zoom_persists_in_localstorage() {
        let html = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("dist/index.html"),
        )
        .expect("dist/index.html must exist");

        assert!(
            html.contains("mdviewer-zoom"),
            "HTML must use 'mdviewer-zoom' key for localStorage zoom persistence"
        );
        assert!(
            html.contains("localStorage.getItem") && html.contains("localStorage.setItem"),
            "HTML must use localStorage.getItem and localStorage.setItem for zoom persistence"
        );
    }

    /// Verify that the frontend HTML uses CSS transform for zoom (top-left origin).
    #[test]
    fn test_zoom_uses_css_transform() {
        let html = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("dist/index.html"),
        )
        .expect("dist/index.html must exist");

        assert!(
            html.contains("transform") && html.contains("scale("),
            "HTML must use CSS transform: scale() for zoom"
        );
        assert!(
            html.contains("top left"),
            "HTML must use transformOrigin: top left so left edge stays fixed"
        );
    }

    // ─── Wikilink edge cases ───────────────────────────────────────────────────

    #[test]
    fn it_preserves_wikilinks_in_inline_code() {
        let input = "See `[[My Page]]` for details";
        let output = render_markdown(input);
        assert!(
            output.contains("[[My Page]]"),
            "wikilinks inside inline code must not be converted. Output: {}",
            output
        );
    }

    #[test]
    fn it_preserves_wikilinks_in_fenced_code() {
        let input = "```\n[[My Page]]\n```";
        let output = render_markdown(input);
        assert!(
            output.contains("[[My Page]]"),
            "wikilinks inside fenced code must not be converted. Output: {}",
            output
        );
    }

    #[test]
    fn it_resolves_wikilink_with_hash_target() {
        let input = "[[#Section Name]]";
        let output = render_markdown(input);
        assert!(
            output.contains(r##"href="#section-name""##),
            "intra-page wikilink should produce hash anchor. Output: {}",
            output
        );
        assert!(output.contains("Section Name"));
    }

    #[test]
    fn it_resolves_wikilink_with_page_and_hash() {
        let input = "[[Other Page#Heading]]";
        let output = render_markdown(input);
        assert!(
            output.contains(r#"href="other-page.html#heading""#),
            "page+hash wikilink should produce both. Output: {}",
            output
        );
    }

    #[test]
    fn it_resolves_wikilink_with_display_text() {
        let input = "[[My Page|Custom Text]]";
        let output = render_markdown(input);
        assert!(
            output.contains(r#"href="my-page.html""#),
            "wikilink with display text should produce correct href. Output: {}",
            output
        );
        assert!(output.contains("Custom Text"));
    }

    // ─── Callout edge cases ────────────────────────────────────────────────────

    #[test]
    fn it_preserves_callout_syntax_in_inline_code() {
        let input = "Use `> [!NOTE]` for callouts";
        let output = render_markdown(input);
        // HTML escaping turns > into &gt; inside <code>
        assert!(
            output.contains("[!NOTE]"),
            "callout syntax inside inline code must not be converted. Output: {}",
            output
        );
        assert!(output.contains("<code>"));
    }

    #[test]
    fn it_preserves_callout_syntax_in_fenced_code() {
        let input = "```\n> [!NOTE]\n> Test\n```";
        let output = render_markdown(input);
        // HTML escaping turns > into &gt; inside <pre><code>
        assert!(
            output.contains("[!NOTE]"),
            "callout syntax inside fenced code must not be converted. Output: {}",
            output
        );
    }

    #[test]
    fn it_renders_callout_caution() {
        let input = "> [!CAUTION]\n> Danger ahead";
        let output = render_markdown(input);
        assert!(output.contains(r#"class="callout caution""#));
        assert!(output.contains("Danger ahead"));
    }

    #[test]
    fn it_renders_callout_important() {
        let input = "> [!IMPORTANT]\n> Critical info";
        let output = render_markdown(input);
        assert!(output.contains(r#"class="callout important""#));
        assert!(output.contains("Critical info"));
    }

    // ─── Frontmatter integration ───────────────────────────────────────────────

    #[test]
    fn it_strips_frontmatter_from_render_markdown() {
        let input = "---\ntitle: Test\n---\n# Hello";
        let output = render_markdown(input);
        assert!(
            !output.contains("---"),
            "frontmatter delimiters should be stripped from output. Output: {}",
            output
        );
        assert!(output.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn it_strips_frontmatter_with_yaml_content() {
        let input = "---\ntitle: Test Doc\ndate: 2024-01-01\ntags:\n  - rust\n---\n# Content";
        let output = render_markdown(input);
        assert!(
            !output.contains("title:"),
            "frontmatter YAML should not appear in output. Output: {}",
            output
        );
        assert!(output.contains("<h1>Content</h1>"));
    }

    // ─── Math edge cases ───────────────────────────────────────────────────────

    #[test]
    fn it_preserves_math_syntax_in_inline_code() {
        let input = "Use `$E=mc^2$` for energy";
        let output = render_markdown(input);
        assert!(
            output.contains("$E=mc^2$"),
            "math syntax inside inline code must not be converted. Output: {}",
            output
        );
    }

    #[test]
    fn it_preserves_math_syntax_in_fenced_code() {
        let input = "```\n$$\\int_0^\\infty x^2 dx$$\n```";
        let output = render_markdown(input);
        assert!(
            output.contains("$$"),
            "math syntax inside fenced code must not be converted. Output: {}",
            output
        );
    }

    #[test]
    fn it_handles_adjacent_inline_math() {
        let input = "$a$ and $b$";
        let output = render_markdown(input);
        assert!(output.contains("<span class=\"math-inline\">a</span>"));
        assert!(output.contains("<span class=\"math-inline\">b</span>"));
    }

    #[test]
    fn it_handles_math_with_special_chars() {
        let input = "$x > y$";
        let output = render_markdown(input);
        assert!(output.contains("<span class=\"math-inline\">"));
        // > is HTML-escaped to &gt; inside the span
        assert!(output.contains("x &gt; y") || output.contains("x > y"));
    }

    // ─── Local Image Resolution ─────────────────────────────────────────────

    #[test]
    fn it_preserves_absolute_url_images() {
        // Absolute URLs must pass through unchanged
        let input = "![alt](https://example.com/img.png)";
        let output = render_markdown(input);
        assert!(
            output.contains("https://example.com/img.png"),
            "absolute URL must be preserved. Output: {}",
            output
        );
    }

    #[test]
    fn it_preserves_data_uri_images() {
        // data: URIs must pass through unchanged
        let input = "![alt](data:image/png;base64,iVBOR)";
        let output = render_markdown(input);
        assert!(
            output.contains("data:image/png;base64,iVBOR"),
            "data URIs must be preserved. Output: {}",
            output
        );
    }

    #[test]
    fn it_preserves_absolute_path_images() {
        // Absolute paths (starting with /) must pass through unchanged
        let input = "![alt](/absolute/path/img.png)";
        let output = render_markdown(input);
        assert!(
            output.contains("/absolute/path/img.png"),
            "absolute paths must be preserved. Output: {}",
            output
        );
    }

    #[test]
    fn it_resolves_relative_images_to_data_uri() {
        // Create a temp dir with a small PNG-like file and test resolution
        let dir = std::env::temp_dir().join("mdviewer_test_images");
        std::fs::create_dir_all(&dir).unwrap();
        let img_dir = dir.join("images");
        std::fs::create_dir_all(&img_dir).unwrap();
        let img_path = img_dir.join("test.png");
        // Write minimal PNG (a valid but tiny PNG file)
        let minimal_png: Vec<u8> = vec![
            0x89, 0x50, 0x4E, 0x47, // PNG signature
            0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, // IHDR chunk
            0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, // width=1
            0x00, 0x00, 0x00, 0x01, // height=1
            0x08, 0x02, 0x00, 0x00, // bit depth=8, color type=RGB
            0x00, 0x00, 0x00, 0x90, // CRC (dummy)
            0x77, 0x53, 0x48, 0x42, 0x00, 0x00, 0x00, 0x0A, // IDAT chunk
            0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0x60, 0x00, 0x00, 0x00,
            0x02, // CRC (dummy)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // IEND chunk
            0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&img_path, &minimal_png).unwrap();

        // Use a relative path from dir to images/test.png
        let markdown = "![test](images/test.png)";
        let output = render_markdown_with_base(&markdown, Some(dir.to_str().unwrap()));
        assert!(
            output.contains("data:image/png;base64,"),
            "relative image should become data URI. Output: {}",
            output
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_resolves_relative_path_images() {
        // Test with a relative path resolved against base_dir
        let dir = std::env::temp_dir().join("mdviewer_test_images_rel");
        std::fs::create_dir_all(&dir).unwrap();
        let img_dir = dir.join("images");
        std::fs::create_dir_all(&img_dir).unwrap();
        let img_path = img_dir.join("photo.jpg");
        // Write minimal JPEG-like content
        let minimal_jpg: Vec<u8> = vec![
            0xFF, 0xD8, 0xFF, 0xE0, // JPEG SOI + APP0
            0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0xFF, 0xD8,
        ];
        std::fs::write(&img_path, &minimal_jpg).unwrap();

        // Relative path from dir to images/photo.jpg
        let markdown = "![photo](images/photo.jpg)";
        let output = render_markdown_with_base(&markdown, Some(dir.to_str().unwrap()));
        assert!(
            output.contains("data:image/jpeg;base64,"),
            "relative image path should resolve to JPEG data URI. Output: {}",
            output
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_preserves_relative_images_in_fenced_code() {
        // Images inside fenced code blocks must not be resolved
        let dir = std::env::temp_dir().join("mdviewer_test_img_code");
        std::fs::create_dir_all(&dir).unwrap();
        let input = "```\n![alt](images/img.png)\n```";
        let output = render_markdown_with_base(input, Some(dir.to_str().unwrap()));
        assert!(
            output.contains("images/img.png"),
            "image path inside fenced code must be preserved as-is. Output: {}",
            output
        );
        assert!(
            !output.contains("data:image"),
            "no data URI should appear from inside fenced code. Output: {}",
            output
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_preserves_missing_image_path() {
        // Missing image file should keep the original markdown syntax
        let dir = std::env::temp_dir().join("mdviewer_test_img_missing");
        std::fs::create_dir_all(&dir).unwrap();
        let input = "![missing](images/does-not-exist.png)";
        let output = render_markdown_with_base(input, Some(dir.to_str().unwrap()));
        // The original markdown syntax should render as an img tag with the original src
        assert!(
            output.contains("images/does-not-exist.png"),
            "missing image should preserve original path. Output: {}",
            output
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
