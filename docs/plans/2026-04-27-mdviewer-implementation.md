# Markdown Viewer Implementation Plan

> **REQUIRED SUB-SKILL:** Use the executing-plans skill to implement this plan task-by-task.

**Goal:** Create a lightweight macOS Markdown viewer supporting GitHub/Obsidian features with Mermaid rendering

**Architecture:** Tauri 1.6 + Rust core (pulldown-cmark) + Webview (vanilla JS)

**Tech Stack:** Rust 1.94, Tauri 1.6, pulldown-cmark 0.13.3, Mermaid 10.6.1, KaTeX 0.16.11

---


### Task 1: Fix Dependency Versions

**TDD scenario:** Trivial change — fix build dependencies before testing

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: Verify current dependency state**

Run: `cargo check`
Expected: Dependency resolution errors

**Step 2: Update to compatible versions**

```toml
[dependencies]
tauri = { version = "1.6", features = ["api-all"] }
pulldown-cmark = "0.13.3"
pulldown-cmark-to-cmark = "22.0.0"
ammonia = "4.1.2"
notify = "8.0.0"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
thiserror = "2.0.18"

[build-dependencies]
tauri-build = { version = "1.6", features = [] }

[lib]
name = "mdviewer_core"
path = "src/lib.rs"
```

**Step 3: Verify build**

Run: `cargo check`
Expected: `Finished dev [unoptimized + debuginfo] target(s) in Xs`

**Step 4: Commit**


```bash
git add src-tauri/Cargo.toml
git commit -m "fix(deps): update to compatible crate versions"
```

---


### Task 2: Write First Failing Test

**TDD scenario:** New feature — full TDD cycle

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write failing header test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_renders_header() {
        let input = "# Header";
        let expected = "<h1>Header</h1>\n";
        assert_eq!(render_markdown(input), expected);
    }
}
```

**Step 2: Run test to verify failure**

Run: `cargo test it_renders_header`
Expected: FAIL (function `render_markdown` not found)

**Step 3: Write minimal implementation**

```rust
use pulldown_cmark::{Parser, Options, html::push_html};

pub fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_GFM);
    let parser = Parser::new_ext(markdown, options);
    let mut html = String::new();
    push_html(&mut html, parser);
    html
}
```

**Step 4: Run test to verify pass**

Run: `cargo test it_renders_header`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "test(core): add header rendering test"
```

---


### Task 3: Add Mermaid Fence Test

**TDD scenario:** New feature — full TDD cycle

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write failing Mermaid test**

```rust
#[test]
fn it_renders_mermaid_fence() {
    let input = "```mermaid\ngraph TD;\n    A-->B;\n```";
    let expected = "<pre><code class=\"language-mermaid\">graph TD;\n    A-->B;\n</code></pre>\n";
    assert_eq!(render_markdown(input), expected);
}
```

**Step 2: Run test to verify failure**

Run: `cargo test it_renders_mermaid_fence`
Expected: FAIL (wrong HTML output)

**Step 3: Verify pulldown-cmark behavior**

Run: `cargo run --example test_mermaid`

**Step 4: Document findings**

Add to `DESIGN.md`:
```
Mermaid fences are rendered as standard code blocks with class="language-mermaid"
This matches GitHub behavior and requires no custom processing
```

**Step 5: Update test expectation**

```rust
let expected = "<pre><code class=\"language-mermaid\">graph TD;\n    A-->B;\n</code></pre>\n";
```

**Step 6: Run test to verify pass**

Run: `cargo test it_renders_mermaid_fence`
Expected: PASS

**Step 7: Commit**

```bash
git add src-tauri/src/lib.rs DESIGN.md
git commit -m "feat(core): support mermaid fences via GFM"
```