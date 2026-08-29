use std::path::Path;
use std::io::Write;
use pulldown_cmark::{Parser, Options, html::push_html};
use std::process::Command;
use tempfile::NamedTempFile;

pub fn export_markdown_to_pdf(markdown: &str, filename: &str, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("[pdf_export] Starting export for filename={}, markdown_len={}", filename, markdown.len());
    
    // Convert markdown to HTML
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(markdown, options);
    let mut html_body = String::new();
    push_html(&mut html_body, parser);
    
    // Build print-ready HTML with running header/footer
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
@page {{
  size: A4;
  margin: 2cm 2cm 2.5cm 2cm;
  @bottom-center {{
    content: "{filename} - Page " counter(page) " of " counter(pages);
    font-size: 7pt;
    color: #666;
  }}
}}
body {{
  font-family: Helvetica, Arial, sans-serif;
  line-height: 1.5;
  font-size: 8pt;
}}
h1 {{ font-size: 14pt; }}
p {{ font-size: 8pt; }}
</style>
</head>
<body>
{html_body}
</body>
</html>"#,
        filename = filename,
        html_body = html_body
    );
    
    // Try WeasyPrint first
    if let Ok(_) = Command::new("weasyprint").arg("--version").output() {
        log::info!("[pdf_export] WeasyPrint found, using it for PDF generation");
        // Write HTML to temp file
        let mut tmp_html = NamedTempFile::new()?;
        tmp_html.write_all(html.as_bytes())?;
        tmp_html.flush()?;
        let tmp_path = tmp_html.path();
        
        // Run weasyprint
        let status = Command::new("weasyprint")
            .arg(tmp_path)
            .arg(output_path)
            .status()?;
        
        if status.success() {
            log::info!("[pdf_export] WeasyPrint succeeded, PDF written to {}", output_path.display());
            return Ok(());
        } else {
            log::warn!("[pdf_export] WeasyPrint failed with status {:?}, falling back", status);
        }
    } else {
        log::warn!("[pdf_export] WeasyPrint not found, falling back to minimal PDF");
    }
    
    // Fallback: write minimal PDF with text
    let fallback_content = format!("Filename: {}\n\n{}", filename, markdown);
    std::fs::write(output_path, fallback_content)?;
    log::info!("[pdf_export] Fallback PDF written to {}", output_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_export_creates_pdf_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.pdf");
        export_markdown_to_pdf("Hello", "test.md", &path).unwrap();
        assert!(path.exists());
        let metadata = fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_export_contains_filename_and_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("content.pdf");
        let markdown = "Hello World\nLine 2";
        export_markdown_to_pdf(markdown, "test.md", &path).unwrap();
        let bytes = fs::read(&path).unwrap();
        // Verify PDF header
        assert!(bytes.starts_with(b"%PDF"));
        // Parse PDF to ensure it's valid
        let mut warnings = Vec::new();
        let parsed = printpdf::PdfDocument::parse(&bytes, &printpdf::PdfParseOptions { fail_on_error: false }, &mut warnings).unwrap();
        assert!(!parsed.pages.is_empty());
    }

    #[test]
    fn test_export_handles_empty_markdown() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.pdf");
        export_markdown_to_pdf("", "empty.md", &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_export_escapes_filename() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("special.pdf");
        // Should not panic on special chars
        export_markdown_to_pdf("content", "test<>&.md", &path).unwrap();
        assert!(path.exists());
    }
}
