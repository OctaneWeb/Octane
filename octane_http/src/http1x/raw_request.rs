use crate::eater::{eat_till, eat_till_four_bytes, eat_till_two_bytes};
use crate::{StatusCode, CRLF, DOUBLE_CRLF, SP};

/// A zero-copy, borrowed view into a raw HTTP/1.x request head.
///
/// Every field points back into the buffer the request was parsed from, so
/// building a `RawRequest1x` performs no allocations and copies no bytes — the
/// method, url, version, header block and any already-buffered body bytes are
/// all just sub-slices of the original buffer.
///
/// ```
/// use octane_http::http1x::raw_request::RawRequest1x;
///
/// let buf = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
/// let req = RawRequest1x::parse(buf).unwrap();
///
/// assert_eq!(req.request_method, b"GET");
/// assert_eq!(req.request_url, b"/index.html");
/// assert_eq!(req.request_version, b"HTTP/1.1");
/// assert_eq!(req.headers, b"Host: example.com");
/// assert!(req.body_remainder.is_empty());
/// ```
#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct RawRequest1x<'a> {
    // request-line: METHOD SP request-target SP HTTP-version CRLF
    pub request_method: &'a [u8],
    pub request_url: &'a [u8],
    pub request_version: &'a [u8],

    // every header line, joined by CRLF, without the trailing CRLFCRLF
    pub headers: &'a [u8],

    // any bytes of the body that were already buffered alongside the head
    pub body_remainder: &'a [u8],
}

impl<'a> RawRequest1x<'a> {
    /// An empty request view, with every field pointing at an empty slice.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse the head of an HTTP/1.x request out of `buf`, borrowing from it.
    ///
    /// `buf` is expected to contain at least the request-line and the headers
    /// terminated by `CRLFCRLF`; any bytes after that terminator are exposed
    /// through [`RawRequest1x::body_remainder`]. No bytes are copied.
    pub fn parse(buf: &'a [u8]) -> Result<Self, StatusCode> {
        // METHOD SP ...
        let method_end = eat_till(buf, SP)?;
        let request_method = &buf[..method_end];
        if request_method.is_empty() {
            return Err(StatusCode::BadRequest);
        }

        // ... request-target SP ...  (skip the SP we just stopped on)
        let after_method = &buf[method_end + 1..];
        let url_end = eat_till(after_method, SP)?;
        let request_url = &after_method[..url_end];
        if request_url.is_empty() {
            return Err(StatusCode::BadRequest);
        }

        // ... HTTP-version CRLF  (skip the SP we just stopped on)
        let after_url = &after_method[url_end + 1..];
        let version_end = eat_till_two_bytes(after_url, CRLF)?;
        let request_version = &after_url[..version_end];
        if request_version.is_empty() {
            return Err(StatusCode::BadRequest);
        }

        // `after_version` begins at the request-line's terminating CRLF. The
        // head ends at the first CRLFCRLF, so searching from here lets a
        // request with zero headers (where the request-line CRLF is itself the
        // first half of the terminator) parse without a special case.
        let after_version = &after_url[version_end..];
        let head_end = eat_till_four_bytes(after_version, DOUBLE_CRLF)?;

        // Skip the request-line CRLF (2 bytes) to reach the header block, which
        // runs up to the start of the terminating CRLFCRLF. When there are no
        // headers `head_end` is 0, leaving an empty `[2..2]` slice.
        const HEADERS_START: usize = CRLF.len();
        let headers_end = if head_end < HEADERS_START {
            HEADERS_START
        } else {
            head_end
        };
        let headers = &after_version[HEADERS_START..headers_end];

        // Everything past the CRLFCRLF terminator is the start of the body.
        let body_remainder = &after_version[head_end + DOUBLE_CRLF.len()..];

        Ok(Self {
            request_method,
            request_url,
            request_version,
            headers,
            body_remainder,
        })
    }

    pub fn has_headers(&self) -> bool {
        !self.headers.is_empty()
    }

    pub fn has_request_version(&self) -> bool {
        !self.request_version.is_empty()
    }

    pub fn has_request_url(&self) -> bool {
        !self.request_url.is_empty()
    }

    pub fn has_request_method(&self) -> bool {
        !self.request_method.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_minimal_request_with_no_headers() {
        let buf = b"GET / HTTP/1.1\r\n\r\n";
        let req = RawRequest1x::parse(buf).unwrap();

        assert_eq!(req.request_method, b"GET");
        assert_eq!(req.request_url, b"/");
        assert_eq!(req.request_version, b"HTTP/1.1");
        assert_eq!(req.headers, b"");
        assert_eq!(req.body_remainder, b"");
        assert!(!req.has_headers());
    }

    #[test]
    fn parses_headers_and_body() {
        let buf = b"POST /submit HTTP/1.1\r\nHost: a\r\nContent-Length: 5\r\n\r\nhello";
        let req = RawRequest1x::parse(buf).unwrap();

        assert_eq!(req.request_method, b"POST");
        assert_eq!(req.request_url, b"/submit");
        assert_eq!(req.request_version, b"HTTP/1.1");
        assert_eq!(req.headers, b"Host: a\r\nContent-Length: 5");
        assert_eq!(req.body_remainder, b"hello");
        assert!(req.has_headers());
    }

    #[test]
    fn body_remainder_keeps_trailing_crlfs() {
        // A body may legitimately contain CRLFs; only the first CRLFCRLF after
        // the request-line terminates the head.
        let buf = b"PUT /x HTTP/1.1\r\nHost: a\r\n\r\nline1\r\nline2";
        let req = RawRequest1x::parse(buf).unwrap();

        assert_eq!(req.headers, b"Host: a");
        assert_eq!(req.body_remainder, b"line1\r\nline2");
    }

    #[test]
    fn parsing_is_zero_copy() {
        // Prove the returned slices borrow from `buf` rather than being copies:
        // each field's pointer must fall inside the original buffer's memory.
        let buf = b"GET /a HTTP/1.1\r\nHost: a\r\n\r\nbody".to_vec();
        let req = RawRequest1x::parse(&buf).unwrap();

        let start = buf.as_ptr() as usize;
        let end = start + buf.len();
        for field in [
            req.request_method,
            req.request_url,
            req.request_version,
            req.headers,
            req.body_remainder,
        ] {
            let p = field.as_ptr() as usize;
            assert!(p >= start && p <= end, "field escaped the source buffer");
        }
    }

    #[test]
    fn rejects_request_line_without_spaces() {
        assert_eq!(
            RawRequest1x::parse(b"GET\r\n\r\n"),
            Err(StatusCode::BadRequest)
        );
    }

    #[test]
    fn rejects_empty_version() {
        // The second SP is immediately followed by CRLF, leaving no version.
        assert_eq!(
            RawRequest1x::parse(b"GET / \r\n\r\n"),
            Err(StatusCode::BadRequest)
        );
    }

    #[test]
    fn does_not_semantically_validate_the_version_token() {
        // The raw parser only splits structurally; validating that the version
        // is a known token (e.g. HTTP/1.1) is the downstream parser's job.
        let req = RawRequest1x::parse(b"GET /  \r\n\r\n").unwrap();
        assert_eq!(req.request_version, b" ");
    }

    #[test]
    fn rejects_request_without_head_terminator() {
        assert_eq!(
            RawRequest1x::parse(b"GET / HTTP/1.1\r\nHost: a\r\n"),
            Err(StatusCode::BadRequest)
        );
    }

    #[test]
    fn rejects_empty_method() {
        assert_eq!(
            RawRequest1x::parse(b" / HTTP/1.1\r\n\r\n"),
            Err(StatusCode::BadRequest)
        );
    }
}
