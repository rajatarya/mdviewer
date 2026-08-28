use html_escape::encode_safe;

/// Prepare print-ready HTML with footer containing filename.
///
/// The returned HTML wraps the provided `markdown_html` in a print-optimized
/// container with a fixed footer element. The footer is hidden on screen and
/// shown in print media. Page numbers are rendered via `@page` margin boxes.
///
/// # Arguments
/// * `markdown_html` - Rendered markdown HTML content.
/// * `filename` - Filename to display in footer.
///
/// # Returns
/// A complete HTML document string ready for printing.
pub fn prepare_print_html(markdown_html: &str, filename: &str) -> String {
    let escaped_filename = encode_safe(filename);
    
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Print</title>
<style>
  @media screen {{
    .print-footer, .print-header {{ display: none; }}
  }}
  @media print {{
    @page {{
      margin: 0.5in 0.5in 1.25in 0.5in;
      @bottom-center {{
        content: "Page " counter(page);
        font-size: 9pt;
        color: #333;
      }}
    }}
    .print-header {{
      position: fixed;
      top: 0.25in;
      left: 0;
      right: 0;
      text-align: center;
      font-size: 9pt;
      color: #333;
      border-bottom: 1px solid #ccc;
      padding-bottom: 4pt;
      display: block;
    }}
    .print-footer {{
      position: fixed;
      bottom: 0.25in;
      left: 0;
      right: 0;
      text-align: center;
      font-size: 9pt;
      color: #333;
      border-top: 1px solid #ccc;
      padding-top: 4pt;
      display: block;
    }}
  }}
  body {{ margin: 0; padding: 0; }}
  .print-content {{ padding: 0.5in 0.5in 1.25in 0.5in; }}
</style>
</head>
<body>
<div class="print-header">Filename: {escaped_filename}</div>
<div class="print-content">
{markdown_html}
</div>
<div class="print-footer" data-filename="{escaped_filename}">{escaped_filename}</div>
</body>
</html>"#,
        markdown_html = markdown_html,
        escaped_filename = escaped_filename
    )
}

#[cfg(test)]
mod tests {
    use super::prepare_print_html;

    #[test]
    fn test_prepare_print_html_contains_filename() {
        let html = prepare_print_html("<p>content</p>", "test.md");
        assert!(html.contains("test.md"));
    }

    #[test]
    fn test_prepare_print_html_contains_footer_element() {
        let html = prepare_print_html("<p>content</p>", "test.md");
        assert!(html.contains(r#"class="print-footer""#));
        assert!(html.contains(r#"data-filename="test.md""#));
    }

    #[test]
    fn test_prepare_print_html_contains_header_element() {
        let html = prepare_print_html("<p>content</p>", "test.md");
        assert!(html.contains(r#"class="print-header""#));
        assert!(html.contains("Filename: test.md"));
    }

    #[test]
    fn test_prepare_print_html_preserves_content() {
        let content = "<h1>Title</h1><p>Body</p>";
        let html = prepare_print_html(content, "test.md");
        assert!(html.contains(content));
        assert!(html.contains(r#"class="print-content""#));
    }

    #[test]
    fn test_prepare_print_html_escapes_filename() {
        let html = prepare_print_html("<p>content</p>", "test<>&.md");
        // HTML special chars should be escaped
        assert!(!html.contains("test<>&.md"));
        assert!(html.contains("test&lt;&gt;&amp;.md"));
    }

    #[test]
    fn test_prepare_print_html_has_print_css() {
        let html = prepare_print_html("<p>content</p>", "test.md");
        assert!(html.contains("@media print"));
        assert!(html.contains("@page"));
        assert!(html.contains("counter(page)"));
    }
}
