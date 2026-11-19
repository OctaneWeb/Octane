//! Percent-decoding, shared by request paths and query strings

use std::borrow::Cow;

/// Decode the `%XX` escapes in `string`, and `+` as a space when
/// `plus_is_space` (query strings and form bodies, not paths). A `%` that
/// isn't followed by two hex digits is kept as it is, and invalid UTF-8
/// becomes U+FFFD. It is borrowed when there is nothing to decode.
pub(crate) fn unescape(string: &str, plus_is_space: bool) -> Cow<'_, str> {
    let bytes = string.as_bytes();
    if !bytes
        .iter()
        .any(|&b| b == b'%' || (plus_is_space && b == b'+'))
    {
        return Cow::Borrowed(string);
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' if plus_is_space => out.push(b' '),
            b'%' => match (hex(bytes.get(i + 1)), hex(bytes.get(i + 2))) {
                (Some(high), Some(low)) => {
                    out.push(high << 4 | low);
                    i += 2;
                }
                _ => out.push(b'%'),
            },
            b => out.push(b),
        }
        i += 1;
    }
    // valid UTF-8 keeps the buffer it was decoded into
    Cow::Owned(
        String::from_utf8(out)
            .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()),
    )
}

fn hex(byte: Option<&u8>) -> Option<u8> {
    match byte? {
        b @ b'0'..=b'9' => Some(b - b'0'),
        b @ b'a'..=b'f' => Some(b - b'a' + 10),
        b @ b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::unescape;
    use std::borrow::Cow;

    #[test]
    fn borrows_when_nothing_to_decode() {
        assert!(matches!(unescape("plain", true), Cow::Borrowed("plain")));
        // `+` is only special in queries
        assert!(matches!(unescape("a+b", false), Cow::Borrowed("a+b")));
    }

    #[test]
    fn decodes_escapes() {
        assert_eq!(unescape("Ada%20Lovelace", false), "Ada Lovelace");
        assert_eq!(unescape("caf%C3%A9", false), "café");
        assert_eq!(unescape("caf%c3%a9", false), "café");
        assert_eq!(unescape("a%2Fb", false), "a/b");
        assert_eq!(unescape("1+1%3D2", true), "1 1=2");
        assert_eq!(unescape("1+1%3D2", false), "1+1=2");
    }

    #[test]
    fn keeps_what_it_cant_decode() {
        assert_eq!(unescape("100%", false), "100%");
        assert_eq!(unescape("%zz%4", false), "%zz%4");
        assert_eq!(unescape("%FF", false), "\u{FFFD}");
    }
}
