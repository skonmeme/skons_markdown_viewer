//! Generation of the user stylesheet from settings
//! (STANDARD §5, §7.1 and §8).

use std::fmt::Write;

use crate::highlight::{Theme, theme_css};
use crate::settings::{ColorMode, FontSet, MarginMode, Settings};

/// Smallest automatic margin, in CSS pixels.
pub const AUTO_MARGIN_MIN: f64 = 8.0;
/// Viewport width up to which the automatic margin stays at its minimum.
pub const AUTO_MARGIN_THRESHOLD: f64 = 480.0;
/// Viewport width from which both automatic margins together take
/// [`AUTO_MARGIN_SHARE`] of the width.
pub const AUTO_MARGIN_SHARE_WIDTH: f64 = 1920.0;
/// Share of the width taken by both automatic margins together from
/// [`AUTO_MARGIN_SHARE_WIDTH`] on.
pub const AUTO_MARGIN_SHARE: f64 = 0.15;
/// Margin gained between the threshold and [`AUTO_MARGIN_SHARE_WIDTH`].
const AUTO_MARGIN_RISE: f64 = AUTO_MARGIN_SHARE_WIDTH * AUTO_MARGIN_SHARE / 2.0 - AUTO_MARGIN_MIN;
/// Width over which [`AUTO_MARGIN_RISE`] is gained.
const AUTO_MARGIN_RUN: f64 = AUTO_MARGIN_SHARE_WIDTH - AUTO_MARGIN_THRESHOLD;
/// A fixed margin never exceeds this share of the viewport width.
pub const FIXED_MARGIN_MAX_RATIO: f64 = 0.25;

/// Font family alias that maps Hangul code points to the Korean font.
pub const KOREAN_FAMILY_ALIAS: &str = "smv-korean";

/// Unicode blocks routed to the Korean font: Hangul Jamo, Hangul
/// Compatibility Jamo, Hangul Jamo Extended-A, Hangul Syllables, Hangul Jamo
/// Extended-B and the halfwidth Hangul forms.
pub const HANGUL_UNICODE_RANGE: &str =
    "U+1100-11FF, U+3130-318F, U+A960-A97F, U+AC00-D7AF, U+D7B0-D7FF, U+FFA0-FFDC";

/// Built-in palette used in [`ColorMode::Automatic`]: (light, dark).
pub const AUTOMATIC_TEXT: (&str, &str) = ("#1f2328", "#e6e6e6");
pub const AUTOMATIC_BACKGROUND: (&str, &str) = ("#ffffff", "#1e1e1e");

/// One face of an installed font family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFace {
    pub postscript_name: String,
    /// CSS weight (100–900).
    pub weight: u16,
    pub italic: bool,
}

/// Automatic margin for a viewport of `width` CSS pixels:
/// `max(8, min(8 + 136 × (width − 480) / 1440, 0.075 × width))`.
pub fn automatic_margin(width: f64) -> f64 {
    let linear =
        AUTO_MARGIN_MIN + (width - AUTO_MARGIN_THRESHOLD) * AUTO_MARGIN_RISE / AUTO_MARGIN_RUN;
    linear
        .min(width * AUTO_MARGIN_SHARE / 2.0)
        .max(AUTO_MARGIN_MIN)
}

/// Builds the stylesheet that applies `settings` on top of the base
/// stylesheet. `korean_faces` lists the faces of the active font set's
/// Korean family; it is empty when that family is the system font or is
/// not installed.
pub fn user_stylesheet(settings: &Settings, korean_faces: &[FontFace]) -> String {
    let font_set = settings.active_font_set();
    let korean_faces: &[FontFace] = if font_set.korean.is_some() {
        korean_faces
    } else {
        &[]
    };
    let mut css = String::new();

    write_font_faces(&mut css, korean_faces);

    let has_alias = !korean_faces.is_empty();
    css.push_str(":root {\n");
    let _ = writeln!(
        css,
        "  --smv-font-body: {};",
        body_font_stack(&font_set, has_alias)
    );
    let _ = writeln!(
        css,
        "  --smv-font-mono: {};",
        mono_font_stack(&font_set, has_alias)
    );
    let _ = writeln!(css, "  --smv-font-size: {}px;", font_set.size);
    let _ = writeln!(css, "  --smv-line-height: {};", font_set.line_height);
    let _ = writeln!(css, "  --smv-margin: {};", margin_value(settings));
    match settings.color_mode {
        ColorMode::Automatic => {
            css.push_str("  color-scheme: light dark;\n");
            let _ = writeln!(
                css,
                "  --smv-fg: light-dark({}, {});",
                AUTOMATIC_TEXT.0, AUTOMATIC_TEXT.1
            );
            let _ = writeln!(
                css,
                "  --smv-bg: light-dark({}, {});",
                AUTOMATIC_BACKGROUND.0, AUTOMATIC_BACKGROUND.1
            );
        }
        ColorMode::Custom => {
            let scheme = if settings.background_color.is_dark() {
                "dark"
            } else {
                "light"
            };
            let _ = writeln!(css, "  color-scheme: {scheme};");
            let _ = writeln!(css, "  --smv-fg: {};", settings.text_color.to_hex());
            let _ = writeln!(css, "  --smv-bg: {};", settings.background_color.to_hex());
        }
    }
    css.push_str("}\n");

    // Token colors: exactly one theme applies at a time, so rules of one
    // theme never leak into the other. Printing always uses the light theme.
    let (light_media, dark_media) = match settings.color_mode {
        ColorMode::Automatic => (
            Some("print, (prefers-color-scheme: light)"),
            Some("screen and (prefers-color-scheme: dark)"),
        ),
        ColorMode::Custom if settings.background_color.is_dark() => (Some("print"), Some("screen")),
        ColorMode::Custom => (None, None),
    };
    match light_media {
        Some(media) => {
            let _ = writeln!(css, "@media {media} {{\n{}}}", theme_css(Theme::Light));
        }
        None => css.push_str(theme_css(Theme::Light)),
    }
    if let Some(media) = dark_media {
        let _ = writeln!(css, "@media {media} {{\n{}}}", theme_css(Theme::Dark));
    }

    let _ = writeln!(
        css,
        "@media print {{\n  :root {{ color-scheme: light; --smv-fg: #000000; --smv-bg: #ffffff; --smv-margin: 0px; }}\n}}"
    );
    css
}

fn margin_value(settings: &Settings) -> String {
    match settings.margin_mode {
        MarginMode::Automatic => format!(
            "max({AUTO_MARGIN_MIN}px, min({AUTO_MARGIN_MIN}px + (100vw - {AUTO_MARGIN_THRESHOLD}px) * {AUTO_MARGIN_RISE} / {AUTO_MARGIN_RUN}, {}vw))",
            AUTO_MARGIN_SHARE / 2.0 * 100.0
        ),
        MarginMode::Fixed => format!(
            "min({}px, {}vw)",
            settings.fixed_margin,
            FIXED_MARGIN_MAX_RATIO * 100.0
        ),
    }
}

fn write_font_faces(css: &mut String, faces: &[FontFace]) {
    let mut seen: Vec<(u16, bool)> = Vec::new();
    for face in faces {
        if seen.contains(&(face.weight, face.italic)) {
            continue;
        }
        seen.push((face.weight, face.italic));
        let _ = writeln!(
            css,
            "@font-face {{ font-family: {}; src: local({}); font-weight: {}; font-style: {}; unicode-range: {}; }}",
            css_string(KOREAN_FAMILY_ALIAS),
            css_string(&face.postscript_name),
            face.weight,
            if face.italic { "italic" } else { "normal" },
            HANGUL_UNICODE_RANGE,
        );
    }
}

/// Font stack for body text (STANDARD §7.1).
fn body_font_stack(set: &FontSet, has_korean_alias: bool) -> String {
    let mut stack = Vec::new();
    if has_korean_alias {
        stack.push(css_string(KOREAN_FAMILY_ALIAS));
    }
    match &set.latin {
        Some(family) => stack.push(css_string(family)),
        None => stack.push("-apple-system".to_owned()),
    }
    stack.push(css_string(
        set.korean.as_deref().unwrap_or("Apple SD Gothic Neo"),
    ));
    stack.push("sans-serif".to_owned());
    stack.join(", ")
}

/// Font stack for code (STANDARD §7.1).
fn mono_font_stack(set: &FontSet, has_korean_alias: bool) -> String {
    let mut stack = Vec::new();
    if let Some(family) = &set.monospace {
        stack.push(css_string(family));
    }
    if has_korean_alias {
        stack.push(css_string(KOREAN_FAMILY_ALIAS));
    }
    stack.extend(["ui-monospace", "Menlo"].map(str::to_owned));
    if let Some(family) = &set.korean {
        stack.push(css_string(family));
    }
    stack.push("monospace".to_owned());
    stack.join(", ")
}

/// Quotes a value as a CSS string. `<` is escaped as well so the result can
/// never close the surrounding `<style>` element.
fn css_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '<' => out.push_str("\\3C "),
            '\n' | '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Rgb;

    fn faces() -> Vec<FontFace> {
        vec![
            FontFace {
                postscript_name: "NanumGothic".into(),
                weight: 400,
                italic: false,
            },
            FontFace {
                postscript_name: "NanumGothicBold".into(),
                weight: 700,
                italic: false,
            },
            FontFace {
                postscript_name: "NanumGothicDup".into(),
                weight: 700,
                italic: false,
            },
        ]
    }

    fn custom_settings() -> Settings {
        let mut settings = Settings::default();
        settings.duplicate_active_font_set();
        settings.edit_active_font_set(|set| {
            set.latin = Some("Georgia".into());
            set.korean = Some("Nanum Gothic".into());
            set.monospace = Some("D2Coding".into());
            set.size = 18.0;
            set.line_height = 1.75;
        });
        settings
    }

    #[test]
    fn automatic_margin_formula() {
        assert_eq!(automatic_margin(400.0), 8.0);
        assert_eq!(automatic_margin(480.0), 8.0);
        assert_eq!(automatic_margin(1200.0), 76.0);
        assert_eq!(automatic_margin(1920.0), 144.0);
        assert_eq!(automatic_margin(2560.0), 192.0);
    }

    #[test]
    fn default_settings_use_the_default_font_set() {
        let asgn = [FontFace {
            postscript_name: "AppleSDGothicNeo-Regular".into(),
            weight: 400,
            italic: false,
        }];
        let css = user_stylesheet(&Settings::default(), &asgn);
        assert!(
            css.contains("src: local(\"AppleSDGothicNeo-Regular\")"),
            "{css}"
        );
        assert!(css.contains(
            "--smv-font-body: \"smv-korean\", -apple-system, \"Apple SD Gothic Neo\", sans-serif;"
        ));
        assert!(css.contains(
            "--smv-font-mono: \"smv-korean\", ui-monospace, Menlo, \"Apple SD Gothic Neo\", monospace;"
        ));
        assert!(css.contains("--smv-font-size: 16px;"));
        assert!(css.contains("--smv-line-height: 1.6;"));
        assert!(
            css.contains("--smv-margin: max(8px, min(8px + (100vw - 480px) * 136 / 1440, 7.5vw));")
        );
        assert!(css.contains("light-dark(#1f2328, #e6e6e6)"));
        assert!(css.contains("@media print, (prefers-color-scheme: light) {"));
        assert!(css.contains("@media screen and (prefers-color-scheme: dark) {"));
    }

    #[test]
    fn missing_korean_faces_fall_back_to_the_family_name() {
        let css = user_stylesheet(&Settings::default(), &[]);
        assert!(!css.contains("@font-face"));
        assert!(
            css.contains("--smv-font-body: -apple-system, \"Apple SD Gothic Neo\", sans-serif;")
        );
    }

    #[test]
    fn korean_font_faces_cover_hangul_only() {
        let css = user_stylesheet(&custom_settings(), &faces());
        assert_eq!(css.matches("@font-face").count(), 2, "{css}");
        assert!(
            css.contains("src: local(\"NanumGothicBold\"); font-weight: 700; font-style: normal;")
        );
        assert!(css.contains(&format!("unicode-range: {HANGUL_UNICODE_RANGE};")));
        assert!(css.contains(
            "--smv-font-body: \"smv-korean\", \"Georgia\", \"Nanum Gothic\", sans-serif;"
        ));
        assert!(css.contains(
            "--smv-font-mono: \"D2Coding\", \"smv-korean\", ui-monospace, Menlo, \"Nanum Gothic\", monospace;"
        ));
        assert!(css.contains("--smv-font-size: 18px;"));
        assert!(css.contains("--smv-line-height: 1.75;"));
    }

    #[test]
    fn custom_colors_pick_scheme_by_background() {
        let mut settings = Settings {
            color_mode: ColorMode::Custom,
            text_color: Rgb::new(0xdd, 0xdd, 0xdd),
            background_color: Rgb::new(0x10, 0x10, 0x10),
            ..Settings::default()
        };
        let dark = user_stylesheet(&settings, &[]);
        assert!(dark.contains("color-scheme: dark;"));
        assert!(dark.contains("--smv-bg: #101010;"));
        // Light tokens for print, dark tokens for screen, plus the print overrides.
        assert_eq!(dark.matches("@media print {").count(), 2);
        assert!(dark.contains("@media screen {"));

        settings.background_color = Rgb::new(0xf4, 0xec, 0xd8);
        let light = user_stylesheet(&settings, &[]);
        assert!(light.contains("color-scheme: light;"));
        assert!(!light.contains("@media screen"));
        assert_eq!(light.matches("@media print {").count(), 1);
    }

    #[test]
    fn fixed_margin_is_capped() {
        let settings = Settings {
            margin_mode: MarginMode::Fixed,
            fixed_margin: 96.0,
            ..Settings::default()
        };
        assert!(user_stylesheet(&settings, &[]).contains("--smv-margin: min(96px, 25vw);"));
    }

    #[test]
    fn print_uses_black_on_white() {
        let css = user_stylesheet(&Settings::default(), &[]);
        assert!(css.contains("@media print"));
        assert!(css.contains("--smv-fg: #000000; --smv-bg: #ffffff; --smv-margin: 0px;"));
    }

    #[test]
    fn css_strings_are_escaped() {
        assert_eq!(css_string("a\"b\\c</style>"), "\"a\\\"b\\\\c\\3C /style>\"");
    }
}
