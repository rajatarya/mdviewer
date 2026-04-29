# Markdown Viewer

<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="Markdown Viewer Icon" width="128" height="128" />
</p>

<p align="center">
  <strong>A fast, lightweight native Markdown viewer for macOS</strong>
</p>

<p align="center">
  Render GitHub-flavored and Obsidian-style Markdown as beautiful HTML in a native window — with instant load time and a tiny binary footprint.
</p>

<p align="center">
  <a href="#features"><strong>Features</strong></a> ·
  <a href="#installation"><strong>Installation</strong></a> ·
  <a href="#usage"><strong>Usage</strong></a> ·
  <a href="#development"><strong>Development</strong></a> ·
  <a href="#contributing"><strong>Contributing</strong></a> ·
  <a href="#colophon"><strong>Colophon</strong></a>
</p>

---

## Features

| Category | Supported |
|---|---|
| **GitHub Flavored Markdown** | Tables, task lists, strikethrough, autolinks, fenced code blocks |
| **Obsidian-style** | Wikilinks `[[Page]]`, emoji shortcodes `:rocket:`, callouts `> [!NOTE]` |
| **Math** | Inline `$E = mc^2$` and display `$$\int_0^\infty$$` via KaTeX |
| **Diagrams** | Mermaid code blocks rendered with Mermaid.js |
| **Security** | All HTML sanitized via `ammonia` — XSS-safe by default |
| **Performance** | Zero webview overhead, instant load, ~few MB binary |

### Callout Types

`NOTE` · `TIP` · `WARNING` · `CAUTION` · `IMPORTANT` — with foldable support (`> [!TIP]+`)

### Wikilinks

- `[[My Document]]` → links to `my-document.html`
- `[[#Heading]]` → anchors within the page

---

## Installation

### Option 1: Download (macOS)

1. Go to the [Releases](https://github.com/rajatarya/mdviewer/releases) page
2. Download the latest `.dmg` (Apple Silicon)
3. Open the `.dmg` and drag **Markdown Viewer** to Applications
4. **First launch**: macOS may show "app is damaged" or won't open due to quarantine. Run:

   ```bash
   xattr -cr /Applications/Markdown\ Viewer.app
   ```

   Then launch normally. This removes the quarantine attribute — a standard macOS security step for unsigned apps.

> **Tested on**: macOS 26.4 (25E246) on Apple Silicon (M-series).

### Option 2: Build from Source

**Prerequisites:**
- Rust 1.94+ stable (`rustup`)
- Node.js 18+ (for Tauri CLI)
- Xcode Command Line Tools

```bash
# Clone the repo
git clone https://github.com/rajatarya/mdviewer.git
cd mdviewer

# Install Tauri CLI
npm install -g @tauri-apps/cli

# Build and install (copies .app to ~/Applications, creates ~/.local/bin/mdviewer wrapper)
make install
```

Or install without rebuilding (after a fresh bundle):

```bash
make install-fast
```

The wrapper script at `~/.local/bin/mdviewer` launches the `.app` with `open -a` and
passes through any arguments. It also handles `--help` inline (no app launch needed).

---

## Usage

### CLI

```bash
# Open a single file
mdviewer document.md

# Open multiple files
mdviewer doc1.md doc2.md notes.txt

# Show help
mdviewer --help

# Pipe from stdin
cat document.md | mdviewer -
```

### GUI

1. Launch **Markdown Viewer** from Applications
2. Open a file via **File → Open** or drag-and-drop onto the app icon
3. Navigate between files using the sidebar

Double-clicking a `.md`, `.markdown`, or `.txt` file in Finder will also open it in Markdown Viewer (file association registered during install).

---

## Development

```bash
# Run in development mode (hot-reload enabled)
cargo tauri dev

# Format and lint
cargo fmt
cargo clippy -- -D warnings

# Run tests
cargo test --lib

# Full check before committing
make all
```

### Architecture

```
Tauri 2.x (Rust backend)
  ├─ pulldown-cmark  → Markdown parsing
  ├─ ammonia         → HTML sanitization
  ├─ regex           → Text preprocessing
  └─ Webview (HTML/CSS/JS)
      ├─ KaTeX       → Math rendering
      └─ Mermaid.js  → Diagram rendering
```

See [DESIGN.md](DESIGN.md) for the full architecture and rendering pipeline.

---

## Contributing

Forks and issues are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) for details. We encourage personal forks and bug reports, and discourage blind AI-generated PRs.

---

## Colophon

This app was designed and implemented entirely by **[pi.dev](https://pi.dev)** — an AI coding agent — using the **Qwen3.6-35B-A3B-GGUF:BF16** model by [Unsloth](http://unsloth.ai/) ([🤗 Hugging Face](https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF)).

The model was hosted locally via [llama.cpp](https://github.com/ggml-org/llama.cpp) on an M5 MacBook Pro (128 GB RAM), with the GGUF weights downloaded directly from Hugging Face.

Every line of Rust, every CSS rule, every JavaScript function was written through conversation with the agent. The result: a complete, tested, production-ready native macOS app — from zero to shipped — without a single human typing code.

**Stack:** Tauri 2 · Rust · pulldown-cmark · ammonia · KaTeX · Mermaid.js

---

## License

[MIT](LICENSE) · © 2026 Rajat Arya
