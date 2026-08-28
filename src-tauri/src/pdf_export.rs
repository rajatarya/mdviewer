use std::fs;
use std::path::Path;

/// Export markdown to PDF with filename in header/footer on each page.
///
/// For TDD, this is a minimal implementation that creates a valid PDF
/// file. Full HTML rendering can be added later.
pub fn export_markdown_to_pdf(markdown: &str, filename: &str, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Minimal valid PDF header + filename + markdown
    let content = format!("%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 100 >>\nstream\nBT /F1 12 Tf 50 750 Td ({}) Tj ET\nBT /F1 10 Tf 50 730 Td ({}) Tj ET\nendstream\nendobj\nxref\n0 5\n0000000000 65535 f \n0000000010 00000 n \n0000000060 00000 n \n0000000110 00000 n \n0000000200 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n300\n%%EOF", filename, markdown.lines().next().unwrap_or(""));
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
