//! Syntax highlighting of fenced code blocks (STANDARD §4.2).
//!
//! Code is tokenized with syntect using the syntax collection bundled by
//! `two-face`. The output uses CSS classes (prefixed with `sy-`) instead of
//! inline colors, so the light and dark themes can be switched with a
//! stylesheet alone.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Write};
use std::sync::OnceLock;

use comrak::adapters::SyntaxHighlighterAdapter;
use syntect::html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;
use two_face::theme::EmbeddedThemeName;

/// Class style shared by the HTML generator and the theme stylesheets.
const CLASS_STYLE: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "sy-" };

/// Code blocks larger than this are shown without highlighting.
pub const MAX_HIGHLIGHT_BYTES: usize = 256 * 1024;

/// Language names that are shown as plain text.
const PLAIN_TEXT: &[&str] = &["text", "txt", "plain", "plaintext", "none", "nohighlight"];

/// Info-string aliases that syntect does not resolve by itself.
const ALIASES: &[(&str, &str)] = &[
    ("shell", "bash"),
    ("console", "bash"),
    ("zsh", "bash"),
    ("sh", "bash"),
    ("c++", "cpp"),
    ("objective-c", "objc"),
    ("objectivec", "objc"),
    ("golang", "go"),
    ("jsonc", "json"),
    ("json5", "json"),
    ("kt", "kotlin"),
    ("ps1", "powershell"),
    ("docker", "dockerfile"),
];

/// Light or dark token colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
}

/// Resolves a fenced code block language (the first word of the info string).
pub fn find_syntax(language: &str) -> Option<&'static SyntaxReference> {
    let token = language.trim().to_ascii_lowercase();
    if token.is_empty() || PLAIN_TEXT.contains(&token.as_str()) {
        return None;
    }
    let token = ALIASES
        .iter()
        .find(|(alias, _)| *alias == token)
        .map_or(token.as_str(), |(_, target)| target);
    syntax_set().find_syntax_by_token(token)
}

/// Returns the highlighted HTML for `code`, or `None` when the language is
/// unknown or the code is too large.
pub fn highlight(language: &str, code: &str) -> Option<String> {
    if code.len() > MAX_HIGHLIGHT_BYTES {
        return None;
    }
    let syntax = find_syntax(language)?;
    let mut generator =
        ClassedHTMLGenerator::new_with_class_style(syntax, syntax_set(), CLASS_STYLE);
    for line in LinesWithEndings::from(code) {
        generator
            .parse_html_for_line_which_includes_newline(line)
            .ok()?;
    }
    Some(generator.finalize())
}

/// Returns the token color rules for a theme.
pub fn theme_css(theme: Theme) -> &'static str {
    static LIGHT: OnceLock<String> = OnceLock::new();
    static DARK: OnceLock<String> = OnceLock::new();
    let (cell, name) = match theme {
        Theme::Light => (&LIGHT, EmbeddedThemeName::Github),
        Theme::Dark => (&DARK, EmbeddedThemeName::OneHalfDark),
    };
    cell.get_or_init(|| {
        let themes = two_face::theme::extra();
        css_for_theme_with_class_style(themes.get(name), CLASS_STYLE).unwrap_or_default()
    })
}

/// Escapes text for use inside HTML element content or attribute values.
pub fn escape_html(output: &mut dyn Write, text: &str) -> fmt::Result {
    let mut last = 0;
    for (index, byte) in text.bytes().enumerate() {
        let entity = match byte {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' => "&quot;",
            _ => continue,
        };
        output.write_str(&text[last..index])?;
        output.write_str(entity)?;
        last = index + 1;
    }
    output.write_str(&text[last..])
}

/// Writes a complete highlighted code block.
pub fn write_code_block(output: &mut dyn Write, language: &str, code: &str) -> fmt::Result {
    write_pre_open(output, Some(language))?;
    output.write_str("<code>")?;
    write_code(output, language, code)?;
    output.write_str("</code></pre>\n")
}

fn write_pre_open(output: &mut dyn Write, language: Option<&str>) -> fmt::Result {
    output.write_str("<pre class=\"smv-code\"")?;
    if let Some(language) = language.filter(|l| !l.is_empty()) {
        output.write_str(" data-lang=\"")?;
        escape_html(output, language)?;
        output.write_str("\"")?;
    }
    output.write_str(">")
}

fn write_code(output: &mut dyn Write, language: &str, code: &str) -> fmt::Result {
    match highlight(language, code) {
        Some(html) => output.write_str(&html),
        None => escape_html(output, code),
    }
}

/// Comrak plugin that renders fenced code blocks through [`highlight`].
#[derive(Debug, Default)]
pub struct Highlighter;

impl SyntaxHighlighterAdapter for Highlighter {
    fn write_highlighted(
        &self,
        output: &mut dyn Write,
        lang: Option<&str>,
        code: &str,
    ) -> fmt::Result {
        write_code(output, lang.unwrap_or_default(), code)
    }

    fn write_pre_tag(
        &self,
        output: &mut dyn Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        write_pre_open(output, attributes.get("lang").map(|l| l.as_ref()))
    }

    fn write_code_tag(
        &self,
        output: &mut dyn Write,
        _attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        output.write_str("<code>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_common_languages_and_aliases() {
        for language in [
            "rust", "Rust", "rs", "python", "js", "ts", "tsx", "toml", "yaml", "sh",
        ] {
            assert!(find_syntax(language).is_some(), "{language} should resolve");
        }
        assert_eq!(
            find_syntax("zsh").map(|s| &s.name),
            find_syntax("bash").map(|s| &s.name)
        );
        assert!(find_syntax("text").is_none());
        assert!(find_syntax("").is_none());
        assert!(find_syntax("no-such-language").is_none());
    }

    #[test]
    fn highlights_with_prefixed_classes() {
        let html = highlight("rust", "fn main() {}\n").expect("rust is known");
        assert!(html.contains("sy-"), "{html}");
        assert!(html.contains("main"));
    }

    #[test]
    fn unknown_languages_are_escaped() {
        let mut out = String::new();
        write_code_block(&mut out, "nope", "<b>&</b>").unwrap();
        assert_eq!(
            out,
            "<pre class=\"smv-code\" data-lang=\"nope\"><code>&lt;b&gt;&amp;&lt;/b&gt;</code></pre>\n"
        );
    }

    #[test]
    fn huge_blocks_are_not_highlighted() {
        let code = "x".repeat(MAX_HIGHLIGHT_BYTES + 1);
        assert!(highlight("rust", &code).is_none());
    }

    #[test]
    fn theme_stylesheets_exist() {
        assert!(theme_css(Theme::Light).contains(".sy-"));
        assert!(theme_css(Theme::Dark).contains(".sy-"));
        assert_ne!(theme_css(Theme::Light), theme_css(Theme::Dark));
    }
}
