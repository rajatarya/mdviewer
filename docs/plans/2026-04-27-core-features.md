# Core Markdown Features Implementation Plan

> **REQUIRED SUB-SKILL:** Use the executing-plans skill to implement this plan task-by-task.

**Goal:** Implement and test ALL core Markdown features before UI development

**Architecture:** Extend Rust core with preprocessing pipeline before pulldown-cmark

**Tech Stack:** pulldown-cmark, pulldown-cmark-to-cmark, serde_yaml, regex

---


### Task 0: Verify Basic GFM Features

**TDD scenario:** Verify existing behavior — add tests for features already supported by pulldown-cmark

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write tests for basic GFM**
```rust
#[test]
fn it_renders_tables() {
    let input = "| Header |\n|--------|\n| Cell   |";
    let expected = "<table>\n<thead>\n<tr>\n<th>Header</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td>Cell</td>\n</tr>\n</tbody>\n</table>\n";
    assert_eq!(render_markdown(input), expected);
}

#[test]
fn it_renders_task_lists() {
    let input = "- [x] Done\n- [ ] Pending";
    let expected = "<ul>\n<li><input type=\"checkbox\" checked> Done</li>\n<li><input type=\"checkbox\"> Pending</li>\n</ul>\n";
    assert_eq!(render_markdown(input), expected);
}
```

**Step 2: Run tests to verify**

Run: `cargo test it_renders_tables it_renders_task_lists`
Expected: PASS (pulldown-cmark with GFM handles these)

**Step 3: Document findings in DESIGN.md**

```markdown
## Basic GitHub Flavored Markdown

pulldown-cmark with `Options::ENABLE_GFM` automatically supports:
- Tables
- Task lists
- Strikethrough (`~~text~~`)
- Autolinks (`http://example.com`)
- Fenced code blocks
```

**Step 4: Commit**

```bash
git add src-tauri/src/lib.rs DESIGN.md
git commit -m "test(gfm): verify basic GFM features"
```

---


### Task 1: Implement Emoji Shortcodes

**TDD scenario:** New feature — full TDD cycle

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write failing emoji test**

```rust
#[test]
fn it_renders_emoji_shortcodes() {
    let input = ":rocket: :heart:";
    let expected = "🚀 ❤️";
    assert_eq!(render_markdown(input), expected);
}
```

**Step 2: Run test to verify failure**

Run: `cargo test it_renders_emoji_shortcodes`
Expected: FAIL (shortcodes not processed)

**Step 3: Implement emoji processor**

```rust
use regex::Regex;

fn preprocess_emojis(markdown: &str) -> String {
    let re = Regex::new(r":([a-z0-9_+-]+):").unwrap();
    re.replace_all(markdown, |caps: &regex::Captures| {
        match &caps[1] {
            "rocket" => "🚀",
            "heart" => "❤️",
            _ => &caps[0],
        }.to_string()
    }).into_owned()
}
```

**Step 4: Integrate with rendering pipeline**

```rust
pub fn render_markdown(markdown: &str) -> String {
    let with_emojis = preprocess_emojis(markdown);
    // ... rest of pipeline ...
}
```

**Step 5: Run test to verify pass**

Run: `cargo test it_renders_emoji_shortcodes`
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(core): add emoji shortcode support"
```

---


### Task 2: Implement Footnotes

**TDD scenario:** New feature — enable pulldown-cmark option

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write failing footnote test**

```rust
#[test]
fn it_renders_footnotes() {
    let input = "Text[^1]\n\n[^1]: Footnote";
    let expected = "<p>Text<a href=\"#fn1\" class=\"footnote-ref\">1</a></p>\n<div class=\"footnote-definition\" id=\"fn1\"><sup>1</sup> Footnote</div>\n";
    assert_eq!(render_markdown(input), expected);
}
```

**Step 2: Run test to verify failure**

Run: `cargo test it_renders_footnotes`
Expected: FAIL (footnotes not rendered)

**Step 3: Enable footnotes in pulldown-cmark**

```rust
let mut options = Options::empty();
options.insert(Options::ENABLE_GFM);
options.insert(Options::ENABLE_FOOTNOTES);
```

**Step 4: Run test to verify pass**

Run: `cargo test it_renders_footnotes`
Expected: PASS

**Step 5: Document in DESIGN.md**

```markdown
## Footnotes

Enabled via `Options::ENABLE_FOOTNOTES` in pulldown-cmark.

Renders as:
```html
<p>Text<a href="#fn1" class="footnote-ref">1</a></p>
<div class="footnote-definition" id="fn1"><sup>1</sup> Footnote</div>
```
```

**Step 6: Commit**

```bash
git add src-tauri/src/lib.rs DESIGN.md
git commit -m "feat(gfm): enable footnote support"
```

---


### Task 3: Implement Wikilink Resolution

**TDD scenario:** New feature — full TDD cycle

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write failing wikilink tests**

```rust
#[test]
fn it_resolves_wikilinks() {
    let input = "[[My Document]]";
    let expected = r#"<a href=\"my-document.html\">My Document</a>"#;
    assert_eq!(render_markdown(input), expected);
}

#[test]
fn it_resolves_heading_links() {
    let input = "[[#Heading]]";
    let expected = r#"<a href=\"#heading\">Heading</a>"#;
    assert_eq!(render_markdown(input), expected);
}
```

**Step 2: Run tests to verify failure**

Run: `cargo test it_resolves_wikilinks it_resolves_heading_links`
Expected: FAIL (links not processed)

**Step 3: Implement wikilink processor**

```rust
fn preprocess_wikilinks(markdown: &str, vault_root: &Path) -> String {
    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    re.replace_all(markdown, |caps: &regex::Captures| {
        let target = &caps[1];
        if target.starts_with('#') {
            format!(r#"<a href=\"{}\">{}</a>"#, target, &target[1..])
        } else {
            let path = vault_root.join(target.replace(' ', "_").to_lowercase() + ".md");
            let href = path.strip_prefix(vault_root).unwrap().with_extension("html").to_string_lossy();
            format!(r#"<a href=\"{}\">{}</a>"#, href, target)
        }
    }).into_owned()
}
```

**Step 4: Integrate with rendering pipeline**

```rust
pub fn render_markdown(markdown: &str, vault_root: &Path) -> String {
    let with_wikilinks = preprocess_wikilinks(markdown, vault_root);
    // ... rest of pipeline ...
}
```

**Step 5: Run tests to verify pass**

Run: `cargo test it_resolves_wikilinks it_resolves_heading_links`
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(obsidian): add wikilink resolution"
```

---


### Task 4: Implement Callout Blocks

**TDD scenario:** New feature — full TDD cycle

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write failing callout tests**

```rust
#[test]
fn it_renders_github_alerts() {
    let input = "> [!NOTE]\n> Note content";
    let expected = "<div class=\"callout note\"><p>Note content</p></div>\n";
    assert_eq!(render_markdown(input), expected);
}

#[test]
fn it_renders_foldable_callouts() {
    let input = "> [!note]+\n> Foldable content";
    let expected = "<details class=\"callout note\"><summary>Note</summary><p>Foldable content</p></details>\n";
    assert_eq!(render_markdown(input), expected);
}
```

**Step 2: Run tests to verify failure**

Run: `cargo test it_renders_github_alerts it_renders_foldable_callouts`
Expected: FAIL (callouts not processed)

**Step 3: Implement callout processor**

```rust
fn preprocess_callouts(markdown: &str) -> String {
    let re = Regex::new(r"> \[!(\w+)](\+)?\n(> .+\n)+").unwrap();
    re.replace_all(markdown, |caps: &regex::Captures| {
        let kind = caps[1].to_lowercase();
        let foldable = caps.get(2).is_some();
        let content = &caps[0][caps[0].find('>').unwrap()..]
            .lines()
            .map(|l| &l[2..])
            .collect::<Vec<_>>()
            .join("\n");
        
        if foldable {
            format!("<details class=\"callout {}\"><summary>{}</summary><p>{}</p></details>\n", 
                kind, kind, content)
        } else {
            format!("<div class=\"callout {}\"><p>{}</p></div>\n", kind, content)
        }
    }).into_owned()
}
```

**Step 4: Integrate with rendering pipeline**

```rust
pub fn render_markdown(markdown: &str) -> String {
    let with_callouts = preprocess_callouts(markdown);
    // ... rest of pipeline ...
}
```

**Step 5: Run tests to verify pass**

Run: `cargo test it_renders_github_alerts it_renders_foldable_callouts`
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(obsidian): add callout block support"
```

---


### Task 5: Implement Frontmatter Extraction

**TDD scenario:** New feature — full TDD cycle

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write failing frontmatter test**

```rust
#[test]
fn it_extracts_frontmatter() {
    let input = "---\ntitle: Test\n---\n# Content";
    let (frontmatter, html) = extract_frontmatter(input);
    assert_eq!(frontmatter, "{\"title\":\"Test\"}");
    assert_eq!(html, "<h1>Content</h1>\n");
}
```

**Step 2: Run test to verify failure**

Run: `cargo test it_extracts_frontmatter`
Expected: FAIL (function not implemented)

**Step 3: Implement extraction logic**

```rust
use serde_yaml;

pub fn extract_frontmatter(content: &str) -> (String, String) {
    if let Some(pos) = content.strip_prefix("---\n") {
        if let Some(end) = pos.find("\n---\n") {
            let yaml = &pos[..end];
            let html = &pos[end + 5..];
            (
                serde_yaml::to_string(&serde_yaml::from_str::<serde_json::Value>(yaml).unwrap()).unwrap(),
                html.to_string()
            )
        } else {
            (String::new(), content.to_string())
        }
    } else {
        (String::new(), content.to_string())
    }
}
```

**Step 4: Update main rendering function**

```rust
pub fn render_markdown_with_frontmatter(markdown: &str) -> (String, String) {
    let (frontmatter, content) = extract_frontmatter(markdown);
    (frontmatter, render_markdown(&content))
}
```

**Step 5: Run test to verify pass**

Run: `cargo test it_extracts_frontmatter`
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(core): add frontmatter extraction"
```

---


### Task 6: Implement Math Block Processing

**TDD scenario:** New feature — full TDD cycle

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write failing math test**

```rust
#[test]
fn it_processes_math_blocks() {
    let input = "$E = mc^2$\n\n$$\n\int_0^\infty x^2 dx\n$$";
    let expected = "<span class=\"math\">E = mc^2</span>\n\n<div class=\"math\">\n\int_0^\infty x^2 dx\n</div>\n";
    assert_eq!(render_markdown(input), expected);
}
```

**Step 2: Run test to verify failure**

Run: `cargo test it_processes_math_blocks`
Expected: FAIL (math not processed)

**Step 3: Implement math processor**

```rust
fn preprocess_math(markdown: &str) -> String {
    let inline_re = Regex::new(r"\$([^$]+)\$").unwrap();
    let block_re = Regex::new(r"\$\$\n([^$]+)\n\$\$").unwrap();

    let with_inline = inline_re.replace_all(markdown, "<span class=\"math\">$1</span>");
    block_re.replace_all(&with_inline, "<div class=\"math\">\n$1\n</div>").into_owned()
}
```

**Step 4: Integrate with rendering pipeline**

```rust
pub fn render_markdown(markdown: &str) -> String {
    let with_math = preprocess_math(markdown);
    // ... rest of pipeline ...
}
```

**Step 5: Run test to verify pass**

Run: `cargo test it_processes_math_blocks`
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(math): add math block processing"
```