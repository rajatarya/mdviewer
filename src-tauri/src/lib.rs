// Markdown rendering core

use pulldown_cmark::{html::push_html, Options, Parser};
use tauri::command;

mod commands {
    use super::*;

    #[command]
    pub fn render_md(markdown: &str) -> String {
        render_markdown(markdown)
    }

    #[command]
    pub fn extract_fm(markdown: &str) -> (String, String) {
        extract_frontmatter(markdown)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::render_md,
            commands::extract_fm
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
}
