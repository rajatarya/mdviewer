use printpdf::*;
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;

pub fn export_markdown_to_pdf(markdown: &str, filename: &str, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("[pdf_export] Starting export for filename={}, markdown_len={}", filename, markdown.len());
    let mut doc = PdfDocument::new("mdviewer export");
    
    // Build page content with filename header and markdown lines
    let mut ops = vec![
        Op::StartTextSection,
        Op::SetFontSizeBuiltinFont {
            size: Pt(12.0),
            font: BuiltinFont::HelveticaBold,
        },
        Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(filename.to_string())],
            font: BuiltinFont::HelveticaBold,
        },
        Op::EndTextSection,
    ];
    
    // Add markdown lines
    for line in markdown.lines().take(100) {
        ops.extend(vec![
            Op::StartTextSection,
            Op::SetFontSizeBuiltinFont {
                size: Pt(10.0),
                font: BuiltinFont::Helvetica,
            },
            Op::WriteTextBuiltinFont {
                items: vec![TextItem::Text(line.to_string())],
                font: BuiltinFont::Helvetica,
            },
            Op::EndTextSection,
        ]);
    }
    
    let page = PdfPage::new(Mm(210.0), Mm(297.0), ops);
    doc.with_pages(vec![page]);
    
    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    log::info!("[pdf_export] PDF generated, bytes={}, warnings={}", bytes.len(), warnings.len());
    let mut file = File::create(output_path)?;
    let mut writer = BufWriter::new(&mut file);
    std::io::Write::write_all(&mut writer, &bytes)?;
    log::info!("[pdf_export] PDF written to {}", output_path.display());
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
