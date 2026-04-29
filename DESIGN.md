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
Configured in `tauri.conf.json` under `plugins.cli` with a `--help` flag and a
positional `files` argument.

**Flow:**
1. `main.rs` checks for `--help` flag via `has_help_flag()` before app init
2. If `--help` is present, prints formatted help message and exits (no window)
3. Tauri CLI plugin parses remaining args at startup
4. `init_cli_paths()` extracts positional `files` args, filters for `.md`/`.markdown`/`.txt`
5. Paths stored in `CliPaths` state
6. Frontend calls `get_cli_paths` on init and auto-loads the first file

**Usage:**
```bash
mdviewer path/to/file.md
mdviewer doc1.md doc2.md
mdviewer --help
```

### Installation Script

`bin/install.sh` handles the full install workflow:

1. **Builds** the app (`make bundle`), unless `--no-build` is passed
2. **Copies** the `.app` to `~/Applications/`
3. **Removes** the macOS quarantine attribute (`xattr -dr com.apple.quarantine`)
4. **Creates** a wrapper script `~/.local/bin/mdviewer` that calls `open -a` on the `.app`
5. **Registers** file associations via `/usr/bin/lsregister`

```bash
make install          # build + install
make install-fast     # install from existing .app
make uninstall        # remove wrapper + file associations
./bin/install.sh --help  # usage info
```

The wrapper script intercepts `--help`/`-h` to print usage without launching the app, and passes all other arguments through to `open -a`. Users may need to add `~/.local/bin` to their `PATH` (most modern shells include it by default).

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
