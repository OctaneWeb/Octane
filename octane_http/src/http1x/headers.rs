use crate::{StatusCode, CRLF};
use core::str;

/// One `name: value` field of a request head, borrowed from the buffer it
/// was parsed out of. The value has its surrounding whitespace removed.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Header<'a> {
    pub name: &'a str,
    pub value: &'a [u8],
}

impl<'a> Header<'a> {
    /// Split and validate a single header line (without its CRLF).
    ///
    /// The name must be a token (RFC 9110 section 5.1), which also rejects
    /// obsolete line folding, and the value may not contain control
    /// characters other than HTAB.
    pub fn parse(line: &'a [u8]) -> Result<Self, StatusCode> {
        let colon = line
            .iter()
            .position(|&b| b == b':')
            .ok_or(StatusCode::BadRequest)?;
        let (name, value) = (&line[..colon], &line[colon + 1..]);
        if !is_token(name) {
            return Err(StatusCode::BadRequest);
        }
        let value = trim_ows(value);
        if value.iter().any(|&b| is_ctl(b) && b != b'\t') {
            return Err(StatusCode::BadRequest);
        }
        Ok(Self {
            // tokens are ASCII, so this never fails
            name: str::from_utf8(name).map_err(|_| StatusCode::BadRequest)?,
            value,
        })
    }

    /// The value as a `str`, `None` if it isn't valid UTF-8
    pub fn value_str(&self) -> Option<&'a str> {
        str::from_utf8(self.value).ok()
    }
}

/// The validated header block of a request head.
///
/// Nothing is copied or collected: lookups and iteration walk the original
/// bytes. Every line was checked once by [`Headers::parse`], so iterating
/// can't fail.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Headers<'a> {
    block: &'a [u8],
}

impl<'a> Headers<'a> {
    /// Validate a header block, the lines between the request-line and the
    /// empty line, joined by CRLF. An empty block has no headers.
    pub fn parse(block: &'a [u8]) -> Result<Self, StatusCode> {
        for line in Lines(Some(block).filter(|block| !block.is_empty())) {
            Header::parse(line)?;
        }
        Ok(Self { block })
    }

    /// Iterate over every header in the order it was sent
    pub fn iter(&self) -> HeaderIter<'a> {
        HeaderIter {
            lines: Lines(Some(self.block).filter(|block| !block.is_empty())),
        }
    }

    /// The first header named `name`, compared ASCII case-insensitively
    pub fn get(&self, name: &str) -> Option<&'a [u8]> {
        self.get_all(name).next()
    }

    /// The first header named `name` as a `str`
    pub fn get_str(&self, name: &str) -> Option<&'a str> {
        self.get(name).and_then(|value| str::from_utf8(value).ok())
    }

    /// Every header named `name`, compared ASCII case-insensitively
    pub fn get_all<'n>(&self, name: &'n str) -> impl Iterator<Item = &'a [u8]> + 'n
    where
        'a: 'n,
    {
        self.iter()
            .filter(move |header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value)
    }

    /// Whether any header is named `name`
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.block.is_empty()
    }

    /// The raw, validated header block
    pub fn as_bytes(&self) -> &'a [u8] {
        self.block
    }

    /// The body length declared by `Content-Length`, 0 if it isn't present
    /// (RFC 9112 section 6.3). Repeated identical values, in one field like
    /// `5, 5` or across fields, are accepted. A value that isn't a decimal
    /// number or values that disagree give a `400`, a length over `max`
    /// gives a `413`.
    pub fn content_length(&self, max: usize) -> Result<usize, StatusCode> {
        let mut digits: Option<&[u8]> = None;
        for value in self.get_all("content-length") {
            for member in value.split(|&b| b == b',') {
                let member = trim_ows(member);
                if member.is_empty() || !member.iter().all(u8::is_ascii_digit) {
                    return Err(StatusCode::BadRequest);
                }
                // compare numbers rather than spellings, "05" and "5" agree
                let zeros = member.iter().take_while(|&&b| b == b'0').count();
                let member = &member[zeros..];
                if digits.is_some_and(|digits| digits != member) {
                    return Err(StatusCode::BadRequest);
                }
                digits = Some(member);
            }
        }
        // anything past usize::MAX is too large anyway
        let length = digits.unwrap_or(b"").iter().fold(0usize, |n, b| {
            n.saturating_mul(10).saturating_add(usize::from(b - b'0'))
        });
        if length > max {
            return Err(StatusCode::PayloadTooLarge);
        }
        Ok(length)
    }

    /// Whether the `Connection` header lists `option`, like `close`
    pub fn connection_has(&self, option: &str) -> bool {
        self.get_all("connection").any(|value| {
            value
                .split(|&b| b == b',')
                .any(|member| trim_ows(member).eq_ignore_ascii_case(option.as_bytes()))
        })
    }
}

impl<'a> IntoIterator for Headers<'a> {
    type Item = Header<'a>;
    type IntoIter = HeaderIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over the fields of a [`Headers`] block
#[derive(Debug, Clone)]
pub struct HeaderIter<'a> {
    lines: Lines<'a>,
}

impl<'a> Iterator for HeaderIter<'a> {
    type Item = Header<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // the block was validated by `Headers::parse`
        self.lines.next().and_then(|line| Header::parse(line).ok())
    }
}

/// CRLF separated lines, `None` once exhausted
#[derive(Debug, Clone)]
struct Lines<'a>(Option<&'a [u8]>);

impl<'a> Iterator for Lines<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let rest = self.0?;
        match crate::http1x::find_in_slice(rest, CRLF) {
            Some(end) => {
                self.0 = Some(&rest[end + CRLF.len()..]);
                Some(&rest[..end])
            }
            None => {
                self.0 = None;
                Some(rest)
            }
        }
    }
}

/// tchar from RFC 9110 section 5.6.2
#[inline]
pub(crate) fn is_tchar(byte: u8) -> bool {
    matches!(
        byte,
        b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z'
            | b'!'
            | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
    )
}

#[inline]
pub(crate) fn is_token(value: &[u8]) -> bool {
    !value.is_empty() && value.iter().all(|&b| is_tchar(b))
}

#[inline]
pub(crate) fn is_ctl(byte: u8) -> bool {
    byte <= 0x1f || byte == 0x7f
}

/// Strip optional whitespace (SP and HTAB) from both ends
pub(crate) fn trim_ows(mut value: &[u8]) -> &[u8] {
    while let [b' ' | b'\t', rest @ ..] = value {
        value = rest;
    }
    while let [rest @ .., b' ' | b'\t'] = value {
        value = rest;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterates_and_looks_up_case_insensitively() {
        let headers = Headers::parse(b"Host: a\r\nX-Test:  b\tc \t\r\nhost: d").unwrap();
        let mut iter = headers.iter();
        assert_eq!(
            iter.next(),
            Some(Header {
                name: "Host",
                value: b"a"
            })
        );
        assert_eq!(iter.next().unwrap().value, b"b\tc");
        assert_eq!(iter.next().unwrap().name, "host");
        assert_eq!(iter.next(), None);

        assert_eq!(headers.get("HOST"), Some(&b"a"[..]));
        assert_eq!(headers.get_str("x-test"), Some("b\tc"));
        assert_eq!(headers.get_all("host").count(), 2);
        assert!(!headers.contains("missing"));
    }

    #[test]
    fn empty_block_has_no_headers() {
        let headers = Headers::parse(b"").unwrap();
        assert!(headers.is_empty());
        assert_eq!(headers.iter().count(), 0);
    }

    #[test]
    fn rejects_malformed_lines() {
        for block in [
            &b"NoColon"[..],
            b": empty-name",
            b"Bad Name: x",
            b" Folded: x",
            b"Host: a\r\n\tcontinued",
            b"X: bell\x07",
            b"Host: a\r\n",
        ] {
            assert_eq!(Headers::parse(block), Err(StatusCode::BadRequest));
        }
        // HTAB is the only control character a value may hold
        assert!(Headers::parse(b"X: a\tb").is_ok());
    }

    #[test]
    fn content_length() {
        let parse = |block: &[u8]| Headers::parse(block).unwrap().content_length(100);
        assert_eq!(parse(b"Host: a"), Ok(0));
        assert_eq!(parse(b"Content-Length: 42"), Ok(42));
        assert_eq!(parse(b"Content-Length: 007"), Ok(7));
        assert_eq!(parse(b"Content-Length: 0"), Ok(0));
        assert_eq!(parse(b"Content-Length: 5\r\ncontent-length: 5"), Ok(5));
        assert_eq!(parse(b"Content-Length: 5, 05"), Ok(5));

        let bad = Err(StatusCode::BadRequest);
        assert_eq!(parse(b"Content-Length: 5\r\nContent-Length: 6"), bad);
        assert_eq!(parse(b"Content-Length: +5"), bad);
        assert_eq!(parse(b"Content-Length: 0x10"), bad);
        assert_eq!(parse(b"Content-Length: "), bad);
        assert_eq!(parse(b"Content-Length: 5,"), bad);

        let too_large = Err(StatusCode::PayloadTooLarge);
        assert_eq!(parse(b"Content-Length: 101"), too_large);
        assert_eq!(parse(b"Content-Length: 99999999999999999999999"), too_large);
    }

    #[test]
    fn connection_options() {
        let headers = Headers::parse(b"Connection: Upgrade, Close").unwrap();
        assert!(headers.connection_has("close"));
        assert!(headers.connection_has("upgrade"));
        assert!(!headers.connection_has("keep-alive"));
    }
}
