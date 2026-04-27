# Markdown Viewer Design

## Mermaid Diagram Support

Mermaid code fences (```
mermaid
```) are rendered as standard GitHub-flavored Markdown code blocks with `class="language-mermaid"`. This matches GitHub's behavior exactly and requires no custom processing in the Rust core.

The pulldown-cmark parser automatically:
- Preserves the Mermaid syntax in code blocks
- Applies HTML escaping to special characters (e.g., `>` becomes `&gt;`)
- Generates valid HTML output compatible with Mermaid.js initialization

This approach keeps the Rust core simple and testable while delegating rendering to the webview layer.