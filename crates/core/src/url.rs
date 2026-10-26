//! The `smv:` URL scheme that maps local files into the viewer page
//! (STANDARD §4.2).
//!
//! A file at the absolute path `/a/b c.md` is addressed as
//! `smv://local/a/b%20c.md`, so relative references inside a document resolve
//! exactly like they would against the file system.

use std::fmt::Write;

/// URL scheme served by the viewer's resource handler.
pub const SCHEME: &str = "smv";

/// Fixed authority of every viewer URL.
pub const HOST: &str = "local";

/// Converts an absolute file system path into a viewer URL.
pub fn path_to_url(path: &str) -> String {
    let mut url = format!("{SCHEME}://{HOST}");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            url.push(char::from(byte));
        } else {
            write!(url, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    url
}

/// Rewrites a `file:` URL into the equivalent viewer URL.
///
/// Returns `None` for every other kind of URL. The remainder of the URL is
/// kept verbatim because it is already in URL syntax.
pub fn rewrite_file_url(url: &str) -> Option<String> {
    let rest = strip_prefix_ignore_case(url, "file://")?;
    let rest = strip_prefix_ignore_case(rest, "localhost").unwrap_or(rest);
    rest.starts_with('/')
        .then(|| format!("{SCHEME}://{HOST}{rest}"))
}

/// Parses the first range of an HTTP `Range` header (`bytes=...`) for a
/// resource of `length` bytes. Returns the inclusive byte range, or `None`
/// when the header is malformed or the range cannot be satisfied.
pub fn parse_byte_range(header: &str, length: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    let (start, end) = spec.split(',').next()?.trim().split_once('-')?;
    let last = length.checked_sub(1)?;
    if start.is_empty() {
        let suffix: u64 = end.parse().ok()?;
        return (suffix > 0).then(|| (length.saturating_sub(suffix), last));
    }
    let start: u64 = start.parse().ok()?;
    let end = match end {
        "" => last,
        end => end.parse::<u64>().ok()?.min(last),
    };
    (start <= end).then_some((start, end))
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &text[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_reserved_and_non_ascii_bytes() {
        assert_eq!(
            path_to_url("/Users/me/My Notes/#1 안.md"),
            "smv://local/Users/me/My%20Notes/%231%20%EC%95%88.md"
        );
    }

    #[test]
    fn keeps_unreserved_characters() {
        assert_eq!(path_to_url("/a-b_c.d~e/f"), "smv://local/a-b_c.d~e/f");
    }

    #[test]
    fn rewrites_file_urls_only() {
        assert_eq!(
            rewrite_file_url("file:///tmp/a%20b.png").as_deref(),
            Some("smv://local/tmp/a%20b.png")
        );
        assert_eq!(
            rewrite_file_url("FILE://localhost/tmp/x.png").as_deref(),
            Some("smv://local/tmp/x.png")
        );
        assert_eq!(rewrite_file_url("https://example.com/x.png"), None);
        assert_eq!(rewrite_file_url("images/x.png"), None);
        assert_eq!(rewrite_file_url("file://server/share/x.png"), None);
    }

    #[test]
    fn byte_ranges() {
        assert_eq!(parse_byte_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_byte_range("bytes=500-", 1000), Some((500, 999)));
        assert_eq!(parse_byte_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_byte_range("bytes=900-5000", 1000), Some((900, 999)));
        assert_eq!(parse_byte_range("bytes=0-1, 5-6", 1000), Some((0, 1)));
        assert_eq!(parse_byte_range("bytes=1000-", 1000), None);
        assert_eq!(parse_byte_range("bytes=5-1", 1000), None);
        assert_eq!(parse_byte_range("bytes=-0", 1000), None);
        assert_eq!(parse_byte_range("items=0-1", 1000), None);
        assert_eq!(parse_byte_range("bytes=0-", 0), None);
    }
}
