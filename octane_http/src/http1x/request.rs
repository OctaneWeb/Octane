use super::headers::{is_token, Headers};
use super::raw_request::RawRequest1x;
use crate::{HttpVersion, StatusCode};
use core::fmt;
use core::str;

/// The request method. Methods without a variant of their own are kept as
/// [`Method::Other`], borrowed from the request.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Method<'a> {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
    Other(&'a str),
}

impl<'a> Method<'a> {
    /// Parse a method token. Methods are case-sensitive (RFC 9110 section
    /// 9.1), so `get` is an unknown method rather than `GET`.
    pub fn parse(method: &'a [u8]) -> Result<Self, StatusCode> {
        if !is_token(method) {
            return Err(StatusCode::BadRequest);
        }
        Ok(match method {
            b"GET" => Self::Get,
            b"HEAD" => Self::Head,
            b"POST" => Self::Post,
            b"PUT" => Self::Put,
            b"DELETE" => Self::Delete,
            b"CONNECT" => Self::Connect,
            b"OPTIONS" => Self::Options,
            b"TRACE" => Self::Trace,
            b"PATCH" => Self::Patch,
            // tokens are ASCII, so this never fails
            other => Self::Other(str::from_utf8(other).map_err(|_| StatusCode::BadRequest)?),
        })
    }

    pub fn as_str(&self) -> &'a str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Connect => "CONNECT",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
            Self::Patch => "PATCH",
            Self::Other(method) => method,
        }
    }
}

impl fmt::Display for Method<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A fully validated HTTP/1.x request, borrowing every field from the buffer
/// it was parsed out of. Building one never allocates.
///
/// ```
/// use octane_http::http1x::{Method, Request};
/// use octane_http::HttpVersion;
///
/// let buf = b"GET /a/b?x=1 HTTP/1.1\r\nHost: example.com\r\n\r\n";
/// let req = Request::parse(buf).unwrap();
///
/// assert_eq!(req.method, Method::Get);
/// assert_eq!(req.path, "/a/b");
/// assert_eq!(req.query, "x=1");
/// assert_eq!(req.version, HttpVersion::Http11);
/// assert_eq!(req.headers.get_str("host"), Some("example.com"));
/// ```
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Request<'a> {
    pub method: Method<'a>,
    /// The request-target exactly as it was sent
    pub target: &'a str,
    /// The path of the target, without its query. `*` for an asterisk-form
    /// `OPTIONS` request, and the path part of an absolute-form target.
    /// It is not percent-decoded.
    pub path: &'a str,
    /// The query, without the leading `?`. Empty if there is none
    pub query: &'a str,
    /// [`HttpVersion::Http10`] or [`HttpVersion::Http11`]
    pub version: HttpVersion,
    pub headers: Headers<'a>,
    /// The body bytes that were buffered alongside the head. The request's
    /// declared body may be shorter (the rest is the next pipelined request)
    /// or longer (the rest hasn't been read yet), see
    /// [`RequestBuffer`](super::RequestBuffer) for a reader that handles both.
    pub body: &'a [u8],
}

impl<'a> Request<'a> {
    /// Parse a complete request head out of `buf`
    pub fn parse(buf: &'a [u8]) -> Result<Self, StatusCode> {
        Self::from_raw(RawRequest1x::parse(buf)?)
    }

    /// Validate the fields that [`RawRequest1x`] only split apart.
    ///
    /// Besides the syntax, this enforces what the core needs to frame
    /// requests safely: only HTTP/1.0 and HTTP/1.1 are served (`505`
    /// otherwise), HTTP/1.1 needs exactly one `Host` and `Transfer-Encoding`
    /// isn't supported (`501`), so a body is always delimited by
    /// `Content-Length`.
    pub fn from_raw(raw: RawRequest1x<'a>) -> Result<Self, StatusCode> {
        let method = Method::parse(raw.request_method)?;
        let version = parse_version(raw.request_version)?;
        let target = parse_target(raw.request_url)?;
        let (path, query) = split_target(target, &method)?;
        let headers = Headers::parse(raw.headers)?;

        if version == HttpVersion::Http11 && headers.get_all("host").count() != 1 {
            return Err(StatusCode::BadRequest);
        }
        if headers.contains("transfer-encoding") {
            return Err(StatusCode::NotImplemented);
        }

        Ok(Self {
            method,
            target,
            path,
            query,
            version,
            headers,
            body: raw.body_remainder,
        })
    }

    /// The non-empty segments of the path, skipping `.` segments.
    /// `..` segments never get here, they are refused while parsing.
    pub fn segments(&self) -> impl Iterator<Item = &'a str> {
        self.path
            .split('/')
            .filter(|segment| !segment.is_empty() && *segment != ".")
    }

    /// The `name=value` pairs of the query, not percent-decoded. A pair
    /// without `=` has an empty value.
    pub fn query_pairs(&self) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| pair.split_once('=').unwrap_or((pair, "")))
    }

    /// Whether the connection stays open after this request: HTTP/1.1
    /// unless `Connection: close`, HTTP/1.0 only with `Connection: keep-alive`
    pub fn keep_alive(&self) -> bool {
        match self.version {
            HttpVersion::Http11 => !self.headers.connection_has("close"),
            _ => self.headers.connection_has("keep-alive"),
        }
    }
}

fn parse_version(version: &[u8]) -> Result<HttpVersion, StatusCode> {
    match version {
        b"HTTP/1.1" => Ok(HttpVersion::Http11),
        b"HTTP/1.0" => Ok(HttpVersion::Http10),
        [b'H', b'T', b'T', b'P', b'/', major, b'.', minor]
            if major.is_ascii_digit() && minor.is_ascii_digit() =>
        {
            Err(StatusCode::HttpVersionNotSupported)
        }
        _ => Err(StatusCode::BadRequest),
    }
}

/// A request-target may only hold visible ASCII (RFC 9112 section 3.2)
fn parse_target(target: &[u8]) -> Result<&str, StatusCode> {
    if target.is_empty() || !target.iter().all(|&b| (0x21..=0x7e).contains(&b)) {
        return Err(StatusCode::BadRequest);
    }
    str::from_utf8(target).map_err(|_| StatusCode::BadRequest)
}

fn split_target<'a>(
    target: &'a str,
    method: &Method<'_>,
) -> Result<(&'a str, &'a str), StatusCode> {
    // asterisk-form, only meaningful for OPTIONS
    if target == "*" {
        return match method {
            Method::Options => Ok(("*", "")),
            _ => Err(StatusCode::BadRequest),
        };
    }
    // authority-form, only for CONNECT; the "path" is the authority
    if let Method::Connect = method {
        return Ok((target, ""));
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let path = if path.starts_with('/') {
        path
    } else {
        // absolute-form, keep the path after the scheme and authority
        let rest = strip_prefix_ignore_case(path, "http://")
            .or_else(|| strip_prefix_ignore_case(path, "https://"))
            .ok_or(StatusCode::BadRequest)?;
        match rest.find('/') {
            Some(slash) => &rest[slash..],
            None => "/",
        }
    };
    // `..` can't be resolved without a stack of segments, refuse it rather
    // than hand a handler a path that climbs out of where it points
    if path.split('/').any(|segment| segment == "..") {
        return Err(StatusCode::BadRequest);
    }
    Ok((path, query))
}

fn strip_prefix_ignore_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let head = value.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        Some(&value[prefix.len()..])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_request() {
        let req = Request::parse(b"POST /a/./b?x=1&y HTTP/1.1\r\nHost: h\r\n\r\nbody").unwrap();
        assert_eq!(req.method, Method::Post);
        assert_eq!(req.target, "/a/./b?x=1&y");
        assert_eq!(req.path, "/a/./b");
        assert_eq!(req.query, "x=1&y");
        assert_eq!(req.body, b"body");

        let mut segments = req.segments();
        assert_eq!(segments.next(), Some("a"));
        assert_eq!(segments.next(), Some("b"));
        assert_eq!(segments.next(), None);

        let mut pairs = req.query_pairs();
        assert_eq!(pairs.next(), Some(("x", "1")));
        assert_eq!(pairs.next(), Some(("y", "")));
        assert_eq!(pairs.next(), None);
    }

    #[test]
    fn unknown_methods_are_kept() {
        let req = Request::parse(b"PURGE / HTTP/1.0\r\n\r\n").unwrap();
        assert_eq!(req.method, Method::Other("PURGE"));
        assert_eq!(req.method.as_str(), "PURGE");
    }

    #[test]
    fn target_forms() {
        let req = Request::parse(b"GET http://h/x?q HTTP/1.1\r\nHost: h\r\n\r\n").unwrap();
        assert_eq!((req.path, req.query), ("/x", "q"));
        let req = Request::parse(b"GET HTTPS://h HTTP/1.1\r\nHost: h\r\n\r\n").unwrap();
        assert_eq!(req.path, "/");
        let req = Request::parse(b"OPTIONS * HTTP/1.1\r\nHost: h\r\n\r\n").unwrap();
        assert_eq!(req.path, "*");
        let req = Request::parse(b"CONNECT h:443 HTTP/1.1\r\nHost: h\r\n\r\n").unwrap();
        assert_eq!(req.path, "h:443");

        let bad = Err(StatusCode::BadRequest);
        assert_eq!(Request::parse(b"GET * HTTP/1.1\r\nHost: h\r\n\r\n"), bad);
        assert_eq!(Request::parse(b"GET x HTTP/1.1\r\nHost: h\r\n\r\n"), bad);
        assert_eq!(
            Request::parse(b"GET /a/../.. HTTP/1.1\r\nHost: h\r\n\r\n"),
            bad
        );
        assert_eq!(
            Request::parse(b"GET /\x7f HTTP/1.1\r\nHost: h\r\n\r\n"),
            bad
        );
    }

    #[test]
    fn versions() {
        assert_eq!(
            Request::parse(b"GET / HTTP/2.0\r\n\r\n"),
            Err(StatusCode::HttpVersionNotSupported)
        );
        assert_eq!(
            Request::parse(b"GET / HTDP/1.1\r\n\r\n"),
            Err(StatusCode::BadRequest)
        );
        assert_eq!(
            Request::parse(b"GET / HTTP/1.10\r\n\r\n"),
            Err(StatusCode::BadRequest)
        );
    }

    #[test]
    fn framing_rules() {
        let bad = Err(StatusCode::BadRequest);
        // HTTP/1.1 needs exactly one Host, HTTP/1.0 doesn't
        assert_eq!(Request::parse(b"GET / HTTP/1.1\r\n\r\n"), bad);
        assert_eq!(
            Request::parse(b"GET / HTTP/1.1\r\nHost: a\r\nHost: b\r\n\r\n"),
            bad
        );
        assert!(Request::parse(b"GET / HTTP/1.0\r\n\r\n").is_ok());
        assert_eq!(
            Request::parse(b"POST / HTTP/1.1\r\nHost: a\r\nTransfer-Encoding: chunked\r\n\r\n"),
            Err(StatusCode::NotImplemented)
        );
        assert_eq!(Request::parse(b"G@T / HTTP/1.1\r\nHost: a\r\n\r\n"), bad);
    }

    #[test]
    fn keep_alive() {
        let parse = |buf: &'static [u8]| Request::parse(buf).unwrap().keep_alive();
        assert!(parse(b"GET / HTTP/1.1\r\nHost: a\r\n\r\n"));
        assert!(!parse(
            b"GET / HTTP/1.1\r\nHost: a\r\nConnection: close\r\n\r\n"
        ));
        assert!(!parse(b"GET / HTTP/1.0\r\n\r\n"));
        assert!(parse(b"GET / HTTP/1.0\r\nConnection: Keep-Alive\r\n\r\n"));
    }
}
