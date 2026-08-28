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
Generate a PDF server-side in Rust with filename in header/footer on every page, then open the PDF with the system default viewer. Print button becomes Export to PDF.

### Rationale
- WKWebView print on macOS does not reliably repeat `position: fixed` elements per page.
- `@page` margin boxes cannot contain dynamic HTML/attributes in WKWebView.
- Server-side PDF generation gives full control over pagination, headers, footers, and fonts.

### Architecture
1. **New Rust module**: `src-tauri/src/pdf_export.rs`
   - Function `export_markdown_to_pdf(markdown: &str, filename: &str, output_path: &Path) -> Result<()>`
   - Render markdown to HTML via `pulldown-cmark`
   - Convert HTML to PDF with headers/footers:
     * Option A: `weasyprint` via subprocess or `weasyprint-rs`
     * Option B: `printpdf` + `html2pdf` style layout
     * Option C: Render HTML to PDF via `wkhtmltopdf` subprocess

2. **Simplified initial approach**: Use `weasyprint` via command-line subprocess
   - Generate print-ready HTML with CSS `@page` margins
   - Invoke `weasyprint input.html output.pdf`
   - WeasyPrint supports `@page` margin boxes with `content` and can include filename via HTML template

3. **Tauri integration**
   - New command `export_pdf(markdown, filename) -> Result<String, String>`
   - Writes PDF to temp dir, returns path
   - Frontend opens PDF via `open` command or `tauri-plugin-opener`

4. **Implementation steps**:
   - Add PDF export command to Tauri backend
   - Command renders markdown, generates print HTML with header/footer placeholders
   - Calls weasyprint to produce PDF
   - Returns PDF path to frontend
   - Frontend opens PDF with system default app

### TDD Plan
1. **Unit tests for `export_markdown_to_pdf`**:
   - `test_export_creates_pdf_file` – output file exists and is non-empty
   - `test_export_contains_filename` – PDF text extraction contains filename
   - `test_export_handles_empty_markdown` – empty input produces valid PDF
   - `test_export_escapes_filename` – filename with special chars is handled safely
   - `test_export_preserves_markdown_content` – rendered markdown text appears in PDF

2. **Integration tests**:
   - `test_export_with_real_markdown` – render sample markdown via `pulldown-cmark`, export PDF, assert filename and content present
   - `test_export_multipage` – long markdown produces PDF with multiple pages, filename appears on each page
   - `test_export_filename_edge_cases` – empty, long, unicode filenames

3. **Automated PDF verification**:
   - Use `pdf-parse` or `pdfium` to extract text from generated PDF in CI
   - Assert filename appears on first page and page count > 1 for long content
   - Test runs on PR via GitHub Actions

4. **Manual verification**:
   - Run mdviewer, click Print, PDF opens in Preview, verify filename in header/footer on every page

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
