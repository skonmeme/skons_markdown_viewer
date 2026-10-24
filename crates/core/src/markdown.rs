//! Markdown to HTML conversion (STANDARD §4.2).

use std::sync::Arc;

use comrak::nodes::{AstNode, NodeValue};
use comrak::options::Plugins;
use comrak::{Arena, Options, format_html_with_plugins, parse_document};

use crate::highlight::{self, Highlighter};
use crate::url::rewrite_file_url;

/// Front matter delimiter recognized at the very start of a document.
const FRONT_MATTER_DELIMITER: &str = "---";

fn options() -> Options<'static> {
    let mut options = Options::default();

    let extension = &mut options.extension;
    extension.strikethrough = true;
    extension.table = true;
    extension.autolink = true;
    extension.tasklist = true;
    extension.footnotes = true;
    extension.alerts = true;
    extension.header_id_prefix = Some(String::new());
    extension.front_matter_delimiter = Some(FRONT_MATTER_DELIMITER.to_owned());
    extension.image_url_rewriter = Some(Arc::new(|url: &str| {
        rewrite_file_url(url).unwrap_or_else(|| url.to_owned())
    }));

    let render = &mut options.render;
    render.r#unsafe = true;
    render.github_pre_lang = true;
    render.tasklist_classes = true;

    options
}

/// Renders a Markdown document into an HTML fragment.
pub fn render(source: &str) -> String {
    let options = options();
    let highlighter = Highlighter;
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&highlighter);

    let arena = Arena::new();
    let root = parse_document(&arena, source, &options);

    let mut html = String::with_capacity(source.len() * 2);
    if let Some(front_matter) = front_matter(root) {
        render_front_matter(&mut html, &front_matter);
    }
    format_html_with_plugins(root, &options, &mut html, &plugins)
        .expect("writing to a String cannot fail");
    html
}

/// Returns the YAML text of the document's front matter, without delimiters.
fn front_matter<'a>(root: &'a AstNode<'a>) -> Option<String> {
    let first = root.first_child()?;
    let data = first.data();
    let NodeValue::FrontMatter(block) = &data.value else {
        return None;
    };
    let mut lines: Vec<&str> = block.lines().collect();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines
        .last()
        .is_some_and(|line| line.trim() == FRONT_MATTER_DELIMITER)
    {
        lines.pop();
    }
    let body = lines.get(1..).unwrap_or_default();
    Some(body.iter().map(|line| format!("{line}\n")).collect())
}

fn render_front_matter(html: &mut String, yaml: &str) {
    html.push_str("<details class=\"smv-front-matter\"><summary>Front matter</summary>\n");
    highlight::write_code_block(html, "yaml", yaml).expect("writing to a String cannot fail");
    html.push_str("</details>\n");
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn renders_gfm_extensions() {
        let html = render(
            "| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done\n- [ ] todo\n\n~~old~~ https://example.com\n",
        );
        assert!(html.contains("<table>"), "{html}");
        assert!(html.contains("task-list-item"), "{html}");
        assert!(html.contains("checked=\"\""), "{html}");
        assert!(html.contains("<del>old</del>"), "{html}");
        assert!(html.contains("<a href=\"https://example.com\">"), "{html}");
    }

    #[test]
    fn headings_get_ids_without_prefix() {
        let html = render("# Hello World\n## 한글 제목\n");
        assert!(html.contains("id=\"hello-world\""), "{html}");
        assert!(html.contains("id=\"한글-제목\""), "{html}");
    }

    #[test]
    fn footnotes_and_alerts() {
        let html = render("Text[^1].\n\n[^1]: Note.\n\n> [!WARNING]\n> Careful.\n");
        assert!(html.contains("footnotes"), "{html}");
        assert!(html.contains("markdown-alert-warning"), "{html}");
    }

    #[test]
    fn raw_html_is_kept() {
        let html = render("<p align=\"center\"><b>hi</b></p>\n");
        assert!(html.contains("<p align=\"center\"><b>hi</b></p>"), "{html}");
    }

    #[test]
    fn code_blocks_are_highlighted() {
        let html = render("```rust\nfn main() {}\n```\n\n```\nplain <text>\n```\n");
        assert!(
            html.contains("<pre class=\"smv-code\" data-lang=\"rust\"><code>"),
            "{html}"
        );
        assert!(html.contains("sy-"), "{html}");
        assert!(
            html.contains("<pre class=\"smv-code\"><code>plain &lt;text&gt;\n</code></pre>"),
            "{html}"
        );
    }

    #[test]
    fn front_matter_is_shown_collapsed() {
        let html = render("---\ntitle: Test\n---\n\n# Body\n");
        assert!(
            html.starts_with("<details class=\"smv-front-matter\">"),
            "{html}"
        );
        assert!(html.contains("title"), "{html}");
        assert!(!html.contains("---"), "{html}");
        assert!(html.contains("id=\"body\""), "{html}");
    }

    #[test]
    fn file_image_urls_are_rewritten() {
        let html = render("![x](file:///tmp/a.png) ![y](rel/b.png)\n");
        assert!(html.contains("src=\"smv://local/tmp/a.png\""), "{html}");
        assert!(html.contains("src=\"rel/b.png\""), "{html}");
    }
}
