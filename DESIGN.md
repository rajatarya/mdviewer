# Markdown Viewer Design

## Security
- **HTML Sanitization**: All rendered HTML passes through [ammonia](https://crates.io/crates/ammonia) with default configuration
- **XSS Protection**: Script tags and other dangerous elements are automatically removed
- **Safe Defaults**: No custom sanitization rules needed (ammonia's defaults match GitHub's security model)

## Mermaid Diagram Support

Mermaid code fences (```
mermaid
```) are rendered as standard GitHub-flavored Markdown code blocks with `class="language-mermaid"`. This matches GitHub's behavior exactly and requires no custom processing in the Rust core.

The pulldown-cmark parser automatically:
- Preserves the Mermaid syntax in code blocks
- Applies HTML escaping to special characters (e.g., `>` becomes `&gt;`)
- Generates valid HTML output compatible with Mermaid.js initialization

This approach keeps the Rust core simple and testable while delegating rendering to the webview layer.

## Test Coverage
- ✅ Header rendering
- ✅ Mermaid fence rendering
- ✅ XSS sanitization
- ❌ [TODO] Wikilink resolution
- ❌ [TODO] Callout blocks
- ❌ [TODO] Frontmatter extraction