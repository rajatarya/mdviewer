// Markdown rendering core

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use pulldown_cmark::{Parser, Options, html::push_html};
use ammonia::Builder;

pub fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_GFM);
    let parser = Parser::new_ext(markdown, options);
    let mut unsafe_html = String::new();
    push_html(&mut unsafe_html, parser);
    
    // Sanitize HTML to prevent XSS
    Builder::new()
        .rm_tags(&["script"])
        .add_tag_attributes("code", &["class"])
        .clean(&unsafe_html)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_renders_header() {
        let input = "# Header";
        let expected = "<h1>Header</h1>\n";
        assert_eq!(render_markdown(input), expected);
    }

    #[test]
    fn it_renders_mermaid_fence() {
        let input = "```mermaid\ngraph TD;\n    A-->B;\n```";
        let expected = "<pre><code class=\"language-mermaid\">graph TD;\n    A--&gt;B;\n</code></pre>\n";
        assert_eq!(render_markdown(input), expected);
    }

    #[test]
    fn it_sanitizes_xss() {
        let input = "<script>alert('xss')</script>";
        let output = render_markdown(input);
        assert!(!output.contains("<script>"));
    }
}