// Markdown rendering core

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use pulldown_cmark::{Parser, Options, html::push_html};

pub fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_GFM);
    let parser = Parser::new_ext(markdown, options);
    let mut html = String::new();
    push_html(&mut html, parser);
    html
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
}