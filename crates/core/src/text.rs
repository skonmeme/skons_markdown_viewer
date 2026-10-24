//! Decoding of document bytes into text (STANDARD §4.1).

/// Decodes the raw bytes of a Markdown file.
///
/// UTF-8 (with or without a byte order mark) and UTF-16 with a byte order
/// mark are recognized. Anything else is decoded as UTF-8 with invalid
/// sequences replaced by U+FFFD.
pub fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16(rest, u16::from_le_bytes);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16(rest, u16::from_be_bytes);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn decode_utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let units = bytes.chunks_exact(2).map(|pair| unit([pair[0], pair[1]]));
    char::decode_utf16(units)
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn plain_utf8() {
        assert_eq!(decode("# 안녕".as_bytes()), "# 안녕");
    }

    #[test]
    fn utf8_with_bom() {
        assert_eq!(decode(b"\xEF\xBB\xBF# hi"), "# hi");
    }

    #[test]
    fn utf16_both_endians() {
        let le: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("한a".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        let be: Vec<u8> = [0xFE, 0xFF]
            .into_iter()
            .chain("한a".encode_utf16().flat_map(u16::to_be_bytes))
            .collect();
        assert_eq!(decode(&le), "한a");
        assert_eq!(decode(&be), "한a");
    }

    #[test]
    fn invalid_utf8_is_replaced() {
        assert_eq!(decode(b"a\xFFb"), "a\u{FFFD}b");
    }
}
