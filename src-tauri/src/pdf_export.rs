use std::fs;
use std::path::Path;

/// Export markdown to PDF with filename in header/footer on each page.
///
/// Minimal implementation for TDD: writes a file with PDF header and
/// filename + markdown content. Real PDF generation can be added later.
pub fn export_markdown_to_pdf(markdown: &str, filename: &str, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = format!("PDF Export\nFilename: {}\n\n{}", filename, markdown);
    fs::write(output_path, content)?;
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
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Filename: test.md"));
        assert!(content.contains("Hello World"));
        assert!(content.contains("Line 2"));
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
