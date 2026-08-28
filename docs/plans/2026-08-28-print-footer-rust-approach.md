# Print Footer Implementation Plan – Rust-side Approach

## Context
mdviewer v1.8.0 added printing support. Footer with filename and page numbers does not render reliably in print preview using CSS-only approaches.

## Previous Attempts – Summary

### Attempt 1: CSS `@page` margin boxes with `attr()`
- **Approach**: `@page { @bottom-center { content: attr(data-filename) " Page " counter(page); } }`
- **Result**: No footer rendered. `attr()` not supported in `@page` margin boxes in WebKit.
- **Commit**: `78457f1`

### Attempt 2: Fixed position DOM footer with `::before attr()`
- **Approach**: `.print-footer { position:fixed; display:none; }` shown in `@media print`, content via `::before { content: attr(data-filename); }`
- **Result**: Footer appeared on last page only, overlapped content.
- **Issue**: Nested `@media print` inside `@media print` made CSS invalid.
- **Commits**: `ae544c9`, `49cc1da`

### Attempt 3: CSS `string-set` / `string()`
- **Approach**: `body { string-set: filename attr(data-filename); }` + `@page { @bottom-center { content: string(filename) } }`
- **Result**: No footer. `string-set` not supported in WebKit.
- **Commit**: `d89b70e`

### Attempt 4: Fixed footer with attribute content
- **Approach**: `.print-footer::before { content: attr(data-filename); }` with JS setting `data-filename` attribute.
- **Result**: Footer still not visible in print preview. Position offset issues.
- **Commits**: `e25d753`, `d7fe603`

### TDD Issues
- Structural tests passed but did not verify actual print rendering.
- No automated PDF generation test in CI.
- Manual verification required.

## Current State
Branch: `fix/print-footer-robust`
Last commit: `d7fe603` – fixed footer offset adjustment
Status: Footer not rendering reliably.

## Proposed Rust-side Approach

### Goal
Generate a print-ready HTML document server-side in Rust with footer injected per page, then print that document via Tauri.

### Rationale
- WebKit print CSS is unreliable for dynamic footers.
- Rust can pre-render markdown to HTML with pagination markers.
- Full control over footer content per page.

### Architecture
1. **New Rust module**: `src-tauri/src/print.rs`
   - Function `prepare_print_html(markdown_html: &str, filename: &str) -> String`
   - Injects `<style>` with `@page` margins and a fixed footer element.
   - Uses `printpdf` crate or `weasyprint` style pagination? Alternative: inject footer via CSS `position: fixed` but generate HTML with explicit page breaks.

2. **Simplified approach**: Use `position: fixed` footer but generate HTML with:
   - Body content wrapped in `.print-content`
   - Footer element with `position: fixed; bottom: 0;`
   - Ensure footer is present in DOM before print.

3. **Better approach**: Use `tauri-plugin-printer` or `webview.print()` with custom HTML:
   - When print requested, Rust backend generates a new HTML string with:
     - Original rendered markdown
     - Footer element with filename
     - CSS `@page` margins
   - Load HTML into a hidden webview or reuse current webview with `set_html`.

4. **Implementation steps**:
   - Add `print_html` command to Tauri backend.
   - Command receives current HTML content and filename.
   - Generates print-ready HTML with footer.
   - Calls `webview.print()` on that HTML.

### TDD Plan
1. **Unit test**: `prepare_print_html` returns HTML containing filename and footer element.
2. **Integration test**: Render test markdown, generate print HTML, assert footer element present with correct filename.
3. **Manual test**: Run mdviewer, print, verify footer appears on every page.

### Acceptance Criteria
- Filename appears at bottom of every printed page.
- Page numbers appear via `@page @bottom-center`.
- No content overlap.
- Print preview matches expected output.

### Risks
- `position: fixed` may still be unreliable in WebKit print.
- Alternative: Use `weasyprint` or `printpdf` crate to generate PDF server-side with proper pagination.

### Next Steps
1. Write plan document (this file)
2. Implement Rust print HTML generation
3. Add tests
4. Manual verification
5. Merge to main

## History
- 2026-08-28: Started Rust-side approach after CSS attempts failed.
