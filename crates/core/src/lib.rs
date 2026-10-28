//! Platform-independent core of skon's Markdown Viewer.
//!
//! Everything that does not need AppKit or WebKit lives here so that it can be
//! unit tested on its own: Markdown rendering, settings, stylesheet generation
//! and the web assets that run inside the viewer page.

pub mod ai;
pub mod highlight;
pub mod markdown;
pub mod page;
pub mod search;
pub mod settings;
pub mod settings_search;
pub mod style;
pub mod text;
pub mod url;

/// File name extensions of Markdown documents (STANDARD §4.1). They must
/// match the `UTImportedTypeDeclarations` entry of `packaging/Info.plist`.
pub const MARKDOWN_EXTENSIONS: [&str; 8] = [
    "md", "markdown", "mdown", "mkd", "mkdn", "mdwn", "mdtxt", "mdtext",
];

#[cfg(test)]
mod tests {
    #[test]
    fn info_plist_declares_every_extension() {
        let plist = include_str!("../../../packaging/Info.plist");
        for extension in super::MARKDOWN_EXTENSIONS {
            assert!(
                plist.contains(&format!("<string>{extension}</string>")),
                "Info.plist misses {extension}"
            );
        }
    }
}
