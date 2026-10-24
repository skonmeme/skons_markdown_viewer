//! Small conversions shared by the AppKit modules.

use objc2::rc::Retained;
use objc2_app_kit::{NSColor, NSColorSpace};
use objc2_foundation::{NSString, NSURL};
use smv_core::settings::Rgb;

pub fn ns(text: &str) -> Retained<NSString> {
    NSString::from_str(text)
}

pub fn file_url(path: &str) -> Retained<NSURL> {
    NSURL::fileURLWithPath(&ns(path))
}

/// Path of a `file:` URL, decoded.
pub fn url_path(url: &NSURL) -> Option<String> {
    url.path().map(|p| p.to_string())
}

pub fn color(rgb: Rgb) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(
        f64::from(rgb.r) / 255.0,
        f64::from(rgb.g) / 255.0,
        f64::from(rgb.b) / 255.0,
        1.0,
    )
}

pub fn rgb(color: &NSColor) -> Option<Rgb> {
    let srgb = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
    let channel = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(Rgb::new(
        channel(srgb.redComponent()),
        channel(srgb.greenComponent()),
        channel(srgb.blueComponent()),
    ))
}

/// Whether `path` has one of the Markdown file extensions (STANDARD §4.1).
pub fn is_markdown_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| {
            smv_core::MARKDOWN_EXTENSIONS
                .iter()
                .any(|m| m.eq_ignore_ascii_case(e))
        })
}
