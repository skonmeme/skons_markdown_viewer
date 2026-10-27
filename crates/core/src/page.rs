//! The HTML page that hosts a rendered document (STANDARD §4.2).

/// Stylesheet shared by every document; it reads the custom properties set
/// by the user stylesheet.
pub const BASE_CSS: &str = include_str!("../assets/base.css");

/// Script injected into the isolated `smv` content world of every page.
pub const VIEWER_JS: &str = include_str!("../assets/viewer.js");

/// Name of the isolated JavaScript content world used by the app.
pub const CONTENT_WORLD: &str = "smv";

/// Builds the complete page for a rendered document.
pub fn page(content_html: &str, user_css: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n\
         <style id=\"smv-base\">\n{BASE_CSS}</style>\n\
         <style id=\"smv-user\">\n{user_css}</style>\n\
         </head>\n<body>\n<article id=\"smv-content\">\n{content_html}</article>\n</body>\n</html>\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_contains_content_and_both_stylesheets() {
        let html = page("<p>hi</p>", ":root{}");
        assert!(html.contains("<article id=\"smv-content\">\n<p>hi</p></article>"));
        assert!(html.contains("<style id=\"smv-user\">\n:root{}</style>"));
        assert!(html.contains("--smv-font-body"));
    }

    #[test]
    fn script_exposes_the_bridge() {
        for name in [
            "update",
            "setStyle",
            "find",
            "step",
            "clear",
            "finish",
            "selectionText",
            "aiBlocks",
            "showBlocks",
        ] {
            assert!(VIEWER_JS.contains(name), "{name} missing");
        }
    }
}
