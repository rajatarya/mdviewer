# Markdown Viewer Design

## Architecture

**Tauri 2.x + Rust Core + Webview**

### Rendering Pipeline
1. **Math Preprocessing** — `$inline$` → `<span>`, `$$block$$` → `<div>`
2. **Emoji Shortcodes** — `:rocket:` → `🚀` (50+ common emojis)
3. **Wikilink Resolution** — `[[Page]]` → `<a href="page.html">`, `[[#Heading]]` → `<a href="#heading">`
4. **Callout Processing** — `> [!NOTE]` → `<div class="callout note">`, `> [!TIP]+` → `<details>`
5. **Markdown Parsing** — pulldown-cmark with GFM, Tables, TaskLists, Footnotes
6. **HTML Sanitization** — ammonia with allowlist for tables, math, callouts, code

## Security
- **HTML Sanitization**: All rendered HTML passes through [ammonia](https://crates.io/crates/ammonia) with allowlist
- **XSS Protection**: Script tags removed; only whitelisted tags/attributes pass through
- **Safe Defaults**: No custom sanitization rules needed (ammonia's defaults match GitHub's security model)

## Core Features

### GitHub Flavored Markdown
- ✅ Tables
- ✅ Task lists
- ✅ Strikethrough (`~~text~~`)
- ✅ Autolinks (`http://example.com`)
- ✅ Fenced code blocks

### Mermaid Diagram Support
Mermaid code fences (```mermaid) are rendered as standard GitHub-flavored Markdown code blocks with `class="language-mermaid"`. Matches GitHub's behavior exactly.

The pulldown-cmark parser automatically:
- Preserves the Mermaid syntax in code blocks
- Applies HTML escaping to special characters
- Generates valid HTML output compatible with Mermaid.js initialization

### Emoji Shortcodes
Supports 50+ common emoji shortcodes: `:rocket:`, `:heart:`, `:thumbsup:`, `:fire:`, `:star:`, etc.

### Math Support
- **Inline math**: `$E = mc^2$` → `<span class="math-inline">E = mc^2</span>`
- **Display math**: `$$\int_0^\infty x^2 dx$$` → `<div class="math-display">...</div>`
- XSS-safe: script tags within math expressions are stripped by ammonia

### Footnotes
Enabled via `Options::ENABLE_FOOTNOTES` in pulldown-cmark.
Renders as:
```html
<p>Text<a href="#fn1" class="footnote-ref">1</a></p>
<div class="footnote-definition" id="fn1"><sup>1</sup> Footnote</div>
```

### Wikilinks (Obsidian-style)
- `[[My Document]]` → `<a href="my-document.html">My Document</a>`
- `[[#Heading]]` → `<a href="#heading">Heading</a>` (intra-page links)

### Callout Blocks (GitHub/Obsidian)
- `> [!NOTE]` → `<div class="callout note">`
- `> [!TIP]+` → `<details class="callout tip" open="open"><summary>Tip>...</summary></details>` (foldable)
- Supported types: NOTE, TIP, WARNING, CAUTION, IMPORTANT

### Frontmatter Extraction
YAML frontmatter between `---` delimiters is extracted separately:
```rust
let (frontmatter_json, content) = extract_frontmatter(md);
```

### CLI Support
Uses `tauri-plugin-cli` (v2) for structured command-line argument parsing.
Configured in `tauri.conf.json` under `plugins.cli` with a positional `files` argument.

**Flow:**
1. Tauri CLI plugin parses args at startup
2. `init_cli_paths()` extracts positional `files` args, filters for `.md`/`.markdown`/`.txt`
3. Paths stored in `CliPaths` state
4. Frontend calls `get_cli_paths` on init and auto-loads the first file

**Usage:**
```bash
mdviewer path/to/file.md
mdviewer doc1.md doc2.md
```

## Test Coverage
- ✅ Header rendering
- ✅ Mermaid fence rendering
- ✅ XSS sanitization
- ✅ Table rendering
- ✅ Task list rendering
- ✅ Wikilink resolution
- ✅ Emoji shortcode rendering
- ✅ Inline math rendering
- ✅ Display math rendering
- ✅ Math XSS sanitization
- ✅ Callout (note) rendering
- ✅ Callout (warning) rendering
- ✅ Callout (foldable) rendering
- ✅ Footnote rendering
- ✅ Frontmatter extraction
- ✅ No frontmatter handling
