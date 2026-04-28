// Markdown rendering core

use pulldown_cmark::{html::push_html, Options, Parser};
use tauri::command;

mod commands {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tauri::Emitter;

    #[command]
    pub fn render_md(markdown: &str) -> String {
        render_markdown(markdown)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::render_md,
            commands::extract_fm,
            commands::read_file,
            commands::watch_file,
            commands::export_html,
        ])
        .setup(|_app| Ok(()))
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

/// Preprocess wikilinks ([[link]]) into standard Markdown links
fn preprocess_wikilinks(markdown: &str) -> String {
    let re = Regex::new(r"\[\[(.*?)\]\]").unwrap();
    re.replace_all(markdown, |caps: &regex::Captures| {
        let target = &caps[1];
        if let Some(rest) = target.strip_prefix('#') {
            let heading = rest.to_lowercase();
            format!("[{}]({})", rest, heading)
        } else {
            let link = target.to_lowercase().replace(' ', "-") + ".html";
            format!("[{}]({})", target, link)
        }
    })
    .into_owned()
}

// ─── Main Rendering Pipeline ─────────────────────────────────────────────────

/// Render markdown string to sanitized HTML
pub fn render_markdown(markdown: &str) -> String {
    // 1. Preprocess math (do first, before anything else)
    let with_math = preprocess_math(markdown);
    // 2. Preprocess emojis
    let with_emojis = preprocess_emojis(&with_math);
    // 3. Preprocess wikilinks
    let with_wikilinks = preprocess_wikilinks(&with_emojis);
    // 4. Preprocess callouts
    let with_callouts = preprocess_callouts(&with_wikilinks);
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
            "table", "thead", "tbody", "tr", "th", "td", "input", "details", "summary",
        ])
        .add_tag_attributes("input", &["type", "checked"])
        .add_tag_attributes("code", &["class"])
        .add_tag_attributes("span", &["class"])
        .add_tag_attributes("div", &["class"])
        .add_tag_attributes("details", &["class", "open"])
        .add_tag_attributes("summary", &["class"])
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
}
