//! Encoding responses into a caller-provided buffer, without allocating.

use crate::http1x::headers::{is_ctl, is_token};
use crate::{HttpVersion, StatusCode, CRLF};

/// Why a response couldn't be encoded
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EncodeError {
    /// The output buffer can't hold the response head
    BufferTooSmall,
    /// A header name isn't a token, a value holds CR, LF or another control
    /// character, or the header is one the server sets itself
    InvalidHeader,
    /// The status code isn't three digits, its reason phrase holds a control
    /// character, or it is an informational (1xx) status
    InvalidStatus,
}

/// Writes a response head (status-line and headers) into `out`.
///
/// Headers are written as they are added and the status-line is put in
/// front of them by [`finish`](Self::finish), so the status can be changed
/// up until then.
///
/// ```
/// use octane_http::response::ResponseHead;
/// use octane_http::{HttpVersion, StatusCode};
///
/// let mut out = [0u8; 128];
/// let mut head = ResponseHead::new(&mut out, HttpVersion::Http11, StatusCode::Ok);
/// head.header("Content-Type", b"text/plain").unwrap();
/// head.content_length(5).unwrap();
/// let head = head.finish().unwrap();
///
/// assert_eq!(head, b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\n");
/// ```
pub struct ResponseHead<'o> {
    out: &'o mut [u8],
    len: usize,
    version: HttpVersion,
    status: StatusCode,
}

impl<'o> ResponseHead<'o> {
    pub fn new(out: &'o mut [u8], version: HttpVersion, status: StatusCode) -> Self {
        Self {
            out,
            len: 0,
            version,
            status,
        }
    }

    pub fn set_status(&mut self, status: StatusCode) {
        self.status = status;
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Append a header. `value` may not contain CR, LF or any control
    /// character other than HTAB, so it can't inject other headers.
    pub fn header(&mut self, name: &str, value: &[u8]) -> Result<&mut Self, EncodeError> {
        if !is_token(name.as_bytes()) || value.iter().any(|&b| is_ctl(b) && b != b'\t') {
            return Err(EncodeError::InvalidHeader);
        }
        self.push_header(name.as_bytes(), value)?;
        Ok(self)
    }

    /// Append a `Content-Length` header
    pub fn content_length(&mut self, len: usize) -> Result<&mut Self, EncodeError> {
        let mut digits = [0u8; 20];
        let digits = format_decimal(len, &mut digits);
        self.push_header(b"Content-Length", digits)?;
        Ok(self)
    }

    /// Put the status-line in front of the headers and terminate the head,
    /// returning the encoded bytes
    pub fn finish(self) -> Result<&'o [u8], EncodeError> {
        let (code, reason) = self.status.fetch();
        if !(200..=999).contains(&code) || reason.bytes().any(is_ctl) {
            return Err(EncodeError::InvalidStatus);
        }
        let version: &[u8] = match self.version {
            HttpVersion::Http10 => b"HTTP/1.0 ",
            _ => b"HTTP/1.1 ",
        };
        let mut digits = [0u8; 20];
        let code = format_decimal(code as usize, &mut digits);
        let line_len = version.len() + code.len() + 1 + reason.len() + CRLF.len();
        let total = line_len + self.len + CRLF.len();
        if total > self.out.len() {
            return Err(EncodeError::BufferTooSmall);
        }

        self.out.copy_within(..self.len, line_len);
        let mut at = 0;
        for part in [version, code, b" ", reason.as_bytes(), CRLF] {
            self.out[at..at + part.len()].copy_from_slice(part);
            at += part.len();
        }
        self.out[total - CRLF.len()..total].copy_from_slice(CRLF);
        Ok(&self.out[..total])
    }

    fn push_header(&mut self, name: &[u8], value: &[u8]) -> Result<(), EncodeError> {
        let len = name.len() + 2 + value.len() + CRLF.len();
        let end = self
            .len
            .checked_add(len)
            .ok_or(EncodeError::BufferTooSmall)?;
        let dest = self
            .out
            .get_mut(self.len..end)
            .ok_or(EncodeError::BufferTooSmall)?;
        let mut at = 0;
        for part in [name, b": ", value, CRLF] {
            dest[at..at + part.len()].copy_from_slice(part);
            at += part.len();
        }
        self.len = end;
        Ok(())
    }
}

/// A response being built by a handler in the [`server`](crate::server)
/// loop. The head goes into the output buffer, the body is only borrowed and
/// written straight to the connection afterwards, so it can be a `'static`
/// slice or a part of the request without being copied.
///
/// Setters chain and never fail; the first invalid header is remembered and
/// the response is replaced by a `500` when it's sent. `Content-Length` and
/// `Connection` are set by the server, the handler may not set them.
pub struct Response<'o, 'b> {
    head: ResponseHead<'o>,
    body: &'b [u8],
    close: bool,
    error: Option<EncodeError>,
}

/// An encoded response, ready to be written
#[derive(Debug, PartialEq, Eq)]
pub struct Encoded<'o, 'b> {
    pub head: &'o [u8],
    pub body: &'b [u8],
    /// Whether the connection has to be closed after this response
    pub close: bool,
}

impl<'o, 'b> Response<'o, 'b> {
    /// A `200 OK` with an empty body
    pub fn new(out: &'o mut [u8], version: HttpVersion) -> Self {
        Self {
            head: ResponseHead::new(out, version, StatusCode::Ok),
            body: b"",
            close: false,
            error: None,
        }
    }

    pub fn status(&mut self, status: StatusCode) -> &mut Self {
        self.head.set_status(status);
        self
    }

    pub fn header(&mut self, name: &str, value: &[u8]) -> &mut Self {
        if self.error.is_some() {
            return self;
        }
        let reserved = ["content-length", "transfer-encoding", "connection"]
            .iter()
            .any(|reserved| name.eq_ignore_ascii_case(reserved));
        let result = if reserved {
            Err(EncodeError::InvalidHeader)
        } else {
            self.head.header(name, value).map(|_| ())
        };
        self.error = result.err();
        self
    }

    pub fn body(&mut self, body: &'b [u8]) -> &mut Self {
        self.body = body;
        self
    }

    /// Close the connection after this response
    pub fn close(&mut self) -> &mut Self {
        self.close = true;
        self
    }

    /// Add the framing headers and encode the head.
    ///
    /// `head_request` drops the body (a `HEAD` response only describes it)
    /// and `keep_alive` is whether the request asked for a persistent
    /// connection. No body or `Content-Length` is sent for `204` and `304`.
    pub fn finish(
        mut self,
        head_request: bool,
        keep_alive: bool,
    ) -> Result<Encoded<'o, 'b>, EncodeError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let close = self.close || !keep_alive;
        let bodiless = matches!(
            self.head.status(),
            StatusCode::NoContent | StatusCode::NotModified
        );
        if !bodiless {
            self.head.content_length(self.body.len())?;
        }
        match (self.head.version, close) {
            (HttpVersion::Http10, false) => self.head.push_header(b"Connection", b"keep-alive")?,
            (HttpVersion::Http10, true) => {}
            (_, true) => self.head.push_header(b"Connection", b"close")?,
            (_, false) => {}
        }
        Ok(Encoded {
            head: self.head.finish()?,
            body: if head_request || bodiless {
                b""
            } else {
                self.body
            },
            close,
        })
    }
}

/// Encode a bodiless error response that closes the connection
pub fn error_response(out: &mut [u8], status: StatusCode) -> Result<&[u8], EncodeError> {
    let mut head = ResponseHead::new(out, HttpVersion::Http11, status);
    head.content_length(0)?;
    head.push_header(b"Connection", b"close")?;
    head.finish()
}

/// Write `n` in decimal at the end of `buf`, returning the digits
fn format_decimal(mut n: usize, buf: &mut [u8; 20]) -> &[u8] {
    let mut start = buf.len();
    loop {
        start -= 1;
        buf[start] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    &buf[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_head() {
        let mut out = [0u8; 128];
        let mut head = ResponseHead::new(&mut out, HttpVersion::Http10, StatusCode::Ok);
        head.header("X-A", b"1").unwrap();
        head.set_status(StatusCode::NotFound);
        assert_eq!(
            head.finish().unwrap(),
            b"HTTP/1.0 404 Not Found\r\nX-A: 1\r\n\r\n"
        );
    }

    #[test]
    fn refuses_injection_and_bad_statuses() {
        let mut out = [0u8; 128];
        let mut head = ResponseHead::new(&mut out, HttpVersion::Http11, StatusCode::Ok);
        assert_eq!(
            head.header("X", b"a\r\nSet-Cookie: x").err(),
            Some(EncodeError::InvalidHeader)
        );
        assert_eq!(
            head.header("X Y", b"a").err(),
            Some(EncodeError::InvalidHeader)
        );
        assert!(head.header("X", b"a\tb").is_ok());

        for status in [
            StatusCode::Continue,
            StatusCode::Other(42, "Short"),
            StatusCode::Other(299, "Bad\r\nReason"),
        ] {
            let mut out = [0u8; 128];
            let head = ResponseHead::new(&mut out, HttpVersion::Http11, status);
            assert_eq!(head.finish().err(), Some(EncodeError::InvalidStatus));
        }
    }

    #[test]
    fn reports_a_full_buffer() {
        let mut out = [0u8; 16];
        let mut head = ResponseHead::new(&mut out, HttpVersion::Http11, StatusCode::Ok);
        head.header("X", b"1").unwrap();
        assert_eq!(head.finish().err(), Some(EncodeError::BufferTooSmall));

        let mut out = [0u8; 4];
        let mut head = ResponseHead::new(&mut out, HttpVersion::Http11, StatusCode::Ok);
        assert_eq!(
            head.header("Long", b"value").err(),
            Some(EncodeError::BufferTooSmall)
        );
    }

    #[test]
    fn response_framing() {
        let mut out = [0u8; 256];
        let mut res = Response::new(&mut out, HttpVersion::Http11);
        res.header("Content-Type", b"text/plain").body(b"hello");
        let encoded = res.finish(false, true).unwrap();
        assert_eq!(
            encoded.head,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\n\r\n"
        );
        assert_eq!((encoded.body, encoded.close), (&b"hello"[..], false));

        // HEAD keeps the length but drops the body, HTTP/1.1 close is explicit
        let mut out = [0u8; 256];
        let mut res = Response::new(&mut out, HttpVersion::Http11);
        res.body(b"hello");
        let encoded = res.finish(true, false).unwrap();
        assert_eq!(
            encoded.head,
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n"
        );
        assert_eq!((encoded.body, encoded.close), (&b""[..], true));

        // HTTP/1.0 keep-alive is explicit, 204 has no length
        let mut out = [0u8; 256];
        let mut res = Response::new(&mut out, HttpVersion::Http10);
        res.status(StatusCode::NoContent).body(b"ignored");
        let encoded = res.finish(false, true).unwrap();
        assert_eq!(
            encoded.head,
            b"HTTP/1.0 204 No Content\r\nConnection: keep-alive\r\n\r\n"
        );
        assert_eq!(encoded.body, b"");
    }

    #[test]
    fn handlers_cannot_set_framing_headers() {
        let mut out = [0u8; 256];
        let mut res = Response::new(&mut out, HttpVersion::Http11);
        res.header("content-length", b"1").header("X-Ok", b"1");
        assert_eq!(
            res.finish(false, true).err(),
            Some(EncodeError::InvalidHeader)
        );
    }

    #[test]
    fn error_responses() {
        let mut out = [0u8; 128];
        assert_eq!(
            error_response(&mut out, StatusCode::BadRequest).unwrap(),
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
    }
}
