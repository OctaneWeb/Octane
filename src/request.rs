use crate::constants::*;
#[cfg(feature = "cookies")]
use crate::cookie::Cookie;
use crate::path::PathBuf;
#[cfg(feature = "url_variables")]
pub use crate::path::Vars;
pub use crate::query::Query;
#[cfg(feature = "extended_queries")]
use crate::query::{parse_extended_query, QueryValue};
use crate::util::read_some;
use octane_http::http1x::{find_in_slice, MAX_HEAD_SIZE};
use octane_http::{HttpVersion, StatusCode};
use std::cfg;
#[cfg(feature = "extended_queries")]
use std::collections::HashMap;
#[cfg(not(feature = "raw_headers"))]
use std::marker::PhantomData;
use std::ops::Range;
use std::str;
use tokio::io::{AsyncRead, AsyncReadExt};

// The longest chunk-size line (the size and its extensions) a chunked body
// may have
const MAX_CHUNK_LINE: usize = 4096;

/// How the end of a request body is found (RFC 9112 section 6.3)
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum BodyFraming {
    /// `Content-Length` bytes, 0 when there is no body
    Length(usize),
    /// `Transfer-Encoding: chunked`
    Chunked,
}

pub use crate::method::RequestMethod;

/// The RequestLine struct represents the first
/// line of the http request, which contains
/// the http version, path and method of request
///
/// # Example
///
/// ```
/// use octane::prelude::*;
/// use octane::request::RequestMethod;
///
/// let mut app = Octane::new();
/// app.get("/",
///     route!(|req, res| {
///         assert_eq!(RequestMethod::Get, req.request_line.method);
///         Flow::Stop
///     }),
/// );
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RequestLine {
    /// The HTTP method which the request was made in
    pub method: RequestMethod,
    /// The path of the request
    pub path: PathBuf,
    /// The query string of the request, without the leading `?`. It is
    /// empty if the request had none
    pub query: String,
    /// Http version of the request
    pub version: HttpVersion,
}

impl RequestLine {
    /// Parses a request line str and returns a
    /// request line struct.
    pub(crate) fn parse(request_line: &str) -> Option<Self> {
        let mut toks = request_line.split(SP);
        let method = toks.next()?;
        let target = toks.next()?;
        let version = toks.next()?;
        Self::from_parts(method, target, version, toks.next().is_some())
    }

    fn from_parts(method: &str, target: &str, version: &str, has_extra: bool) -> Option<Self> {
        // only the path is used for routing, the query is kept apart
        let (path, query) = target.split_once('?').unwrap_or((target, ""));
        let path = PathBuf::parse_request(path).ok()?;
        // `get` avoids slicing through a multi-byte character and handles
        // versions shorter than the `HTTP/` prefix without panicking.
        let ver = version.get(5..).unwrap_or("");
        let enum_ver = match ver {
            "1.1" => HttpVersion::Http11,
            "1.0" => HttpVersion::Http10,
            "2.0" => HttpVersion::Http02,
            "0.9" => HttpVersion::Http09,
            _ => HttpVersion::HttpInvalid,
        };

        if cfg!(feature = "faithful") && (version.get(..5) != Some("HTTP/") || has_extra) {
            return None;
        }
        let request_method = match method {
            "POST" => RequestMethod::Post,
            "GET" => RequestMethod::Get,
            "DELETE" => RequestMethod::Delete,
            "PUT" => RequestMethod::Put,
            "OPTIONS" => RequestMethod::Options,
            "HEAD" => RequestMethod::Head,
            "TRACE" => RequestMethod::Trace,
            "CONNECT" => RequestMethod::Connect,
            "PATCH" => RequestMethod::Patch,
            _ => RequestMethod::None,
        };
        Some(Self {
            method: request_method,
            path,
            query: query.to_owned(),
            version: enum_ver,
        })
    }
}

/// Helper method to Parse request line from method url version parts
pub(crate) fn request_line_parse(method: &[u8], url: &[u8], version: &[u8]) -> Option<RequestLine> {
    match (
        str::from_utf8(method),
        str::from_utf8(url),
        str::from_utf8(version),
    ) {
        (Ok(method), Ok(url), Ok(version))
            if !method.contains(SP) && !url.contains(SP) && !version.contains(SP) =>
        {
            RequestLine::from_parts(method, url, version, false)
        }
        // Keep the exact legacy behavior for malformed fields containing a
        // space. Valid parser output takes the allocation-free branch above.
        (Ok(method), Ok(url), Ok(version)) => {
            RequestLine::parse(&format!("{} {} {}", method, url, version))
        }
        _ => None,
    }
}

/// One header field of a request, `name: value`, borrowed from the request.
/// The name is as the client sent it, the value has the whitespace around
/// it removed.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Header<'a> {
    name: &'a str,
    value: &'a str,
}

impl<'a> Header<'a> {
    /// The name of the header, as it was sent
    pub fn name(&self) -> &'a str {
        self.name
    }
    /// The value of the header
    pub fn value(&self) -> &'a str {
        self.value
    }
}

/// The headers of a request, a view of the request head: nothing is copied
/// out of it. Names are matched without regard to case (RFC 9110 section
/// 5.1), and are kept as the client sent them.
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get("/",
///     route!(|req, res| {
///         let agent = req.headers.get("User-Agent").unwrap_or("unknown");
///         res.send(agent);
///         Flow::Stop
///     }),
/// );
/// ```
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Headers<'a> {
    // every line was checked by `parse`, joined by CRLF, empty for none
    block: &'a str,
}

impl<'a> Headers<'a> {
    /// Check a header block, the lines between the request line and the
    /// empty line. Every field needs a token for a name and a value without
    /// control characters but HTAB, whatever the features: a lenient check
    /// is how framing headers get smuggled past a proxy.
    pub(crate) fn parse(block: &'a str) -> Option<Self> {
        octane_http::http1x::Headers::parse(block.as_bytes()).ok()?;
        Some(Self { block })
    }

    /// Headers for a block that was already checked by [`Headers::parse`]
    pub(crate) fn checked(block: &'a str) -> Self {
        Self { block }
    }

    /// Every header, in the order they were sent
    pub fn iter(&self) -> impl Iterator<Item = Header<'a>> + 'a {
        let block = self.block;
        block
            .split("\r\n")
            .filter(move |_| !block.is_empty())
            .filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                Some(Header {
                    name,
                    value: value.trim_matches(|c| c == SP || c == HT),
                })
            })
    }

    /// The value of the first header named `name`
    pub fn get(&self, name: &str) -> Option<&'a str> {
        self.get_all(name).next()
    }

    /// The values of every header named `name`, in order. A header that can
    /// be repeated, like `Accept`, is one list across all of them.
    pub fn get_all<'n>(&self, name: &'n str) -> impl Iterator<Item = &'a str> + 'n
    where
        'a: 'n,
    {
        self.iter()
            .filter(move |header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value)
    }

    /// Whether there is a header named `name`
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// How many headers there are
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.block.is_empty()
    }

    /// How many `Host` fields the request had. HTTP/1.1 requests need
    /// exactly one (RFC 9112 section 3.2)
    pub(crate) fn host_count(&self) -> usize {
        self.get_all("host").count()
    }

    /// Whether the `Connection` header lists `option`, like `close`
    pub(crate) fn connection_has(&self, option: &str) -> bool {
        self.get_all("connection").any(|value| {
            value.split(',').any(|member| {
                member
                    .trim_matches(|c| c == SP || c == HT)
                    .eq_ignore_ascii_case(option)
            })
        })
    }

    /// The body length declared by `Content-Length`, 0 if it isn't present
    /// (RFC 9112 section 6.3). Repeated identical values like `5, 5`, in one
    /// field or several, are accepted. A value that isn't a decimal number
    /// or values that disagree give a `400`, a length over `max` a `413`.
    pub(crate) fn content_length(&self, max: usize) -> Result<usize, StatusCode> {
        let mut digits: Option<&str> = None;
        for member in self
            .get_all("content-length")
            .flat_map(|value| value.split(','))
        {
            let member = member.trim_matches(|c| c == SP || c == HT);
            if member.is_empty() || !member.bytes().all(|b| b.is_ascii_digit()) {
                return Err(StatusCode::BadRequest);
            }
            // compare numbers rather than spellings, "05" and "5" agree
            let member = member.trim_start_matches('0');
            if digits.map_or(false, |digits| digits != member) {
                return Err(StatusCode::BadRequest);
            }
            digits = Some(member);
        }
        // anything past usize::MAX is too large anyway
        let length = digits.unwrap_or("").bytes().fold(0usize, |n, b| {
            n.saturating_mul(10).saturating_add(usize::from(b - b'0'))
        });
        if length > max {
            return Err(StatusCode::PayloadTooLarge);
        }
        Ok(length)
    }

    /// How the body of a request with these headers is delimited (RFC 9112
    /// section 6.3). `chunked` is the only transfer coding supported, and it
    /// has to be the only one: other codings give a `501`, and anything that
    /// leaves the length ambiguous gives a `400`, after which the connection
    /// must not be reused.
    pub(crate) fn body_framing(
        &self,
        version: HttpVersion,
        max: usize,
    ) -> Result<BodyFraming, StatusCode> {
        if !self.contains("transfer-encoding") {
            return self.content_length(max).map(BodyFraming::Length);
        }
        // Both headers are how requests get smuggled past a proxy that
        // reads the other one (RFC 9112 section 6.1), and HTTP/1.0 has no
        // transfer codings, so its framing can't be trusted either
        if self.contains("content-length") || version != HttpVersion::Http11 {
            return Err(StatusCode::BadRequest);
        }
        // the codings of every Transfer-Encoding field, in order
        let mut codings = self
            .get_all("transfer-encoding")
            .flat_map(|value| value.split(','))
            .map(|coding| coding.trim_matches(|c| c == SP || c == HT));
        let first = codings.next();
        // the last coding, and the first when there are more
        let (last, first) = match codings.last() {
            Some(last) => (Some(last), first),
            None => (first, None),
        };
        // chunked has to come last or the body has no end, and can't be
        // applied twice
        match last {
            Some(last) if last.eq_ignore_ascii_case("chunked") => {}
            _ => return Err(StatusCode::BadRequest),
        }
        match first {
            None => Ok(BodyFraming::Chunked),
            Some(coding) if coding.eq_ignore_ascii_case("chunked") => Err(StatusCode::BadRequest),
            Some(_) => Err(StatusCode::NotImplemented),
        }
    }
}

/// Represents a single request
///
/// # Example
///
/// ```
/// use octane::prelude::*;
/// use octane::request::RequestMethod;
///
/// let mut app = Octane::new();
/// app.get("/",
///     route!(|req, res| {
///         // The req here is not actually a
///         // Request but a MatchedRequest which
///         // implements deref to Request.
///         // req.request is the Request,
///         // you can directly use Request methods
///         // req
///         Flow::Stop
///     }),
/// );
/// ```
///
/// The request struct holds cookies (if enabled
/// in features) headers, the request body, the
/// request_line
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Request<'a> {
    /// The request_line is the first line of the http request, it has the version,
    /// request method, request_uri.
    pub request_line: RequestLine,
    /// Headers in the request
    pub headers: Headers<'a>,
    /// The body of the request
    pub body: &'a [u8],
    #[cfg(feature = "cookies")]
    /// Cookies in the request
    pub cookies: Vec<Cookie<'a>>,
}

impl<'a> Request<'a> {
    /// Parse a Request with request_line, headers
    /// and body and return a Request struct
    pub(crate) fn parse(
        request_line: RequestLine,
        headers: Headers<'a>,
        body: &'a [u8],
    ) -> Option<Self> {
        // borrowed from the head like the headers, from every Cookie field
        #[cfg(feature = "cookies")]
        let cookies = headers.get_all("cookie").flat_map(Cookie::parse).collect();
        Some(Self {
            request_line,
            headers,
            #[cfg(feature = "cookies")]
            cookies,
            body,
        })
    }
    /// Build a [`Request`] from an already-parsed request line, reading any
    /// body bytes that weren't already buffered alongside the head.
    ///
    /// `head` is where the header block is in `data`, already checked by
    /// [`Headers::parse`], and `framing` how its body is delimited. The
    /// headers are a view of `data`, made once the body is read: reading it
    /// may move the buffer. `body_start` is the offset in `data` immediately
    /// after the request head. Whatever of the body wasn't buffered yet is
    /// streamed off `reader` into that same allocation. A chunked body is
    /// decoded in place, so either way the body ends up as
    /// `data[body_start..end]` followed by any bytes that were sent after it
    /// (the next pipelined request). Returns the request and `end`. Bodies
    /// longer than `max_body_size` are refused with a `413`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn from_raw<T: AsyncRead + Unpin>(
        head: Range<usize>,
        framing: BodyFraming,
        request_line: RequestLine,
        data: &'a mut Vec<u8>,
        body_start: usize,
        mut reader: T,
        max_body_size: usize,
    ) -> Result<(Request<'a>, usize), StatusCode> {
        if body_start > data.len() || head.end > body_start {
            return Err(StatusCode::BadRequest);
        }
        let body_end = match framing {
            BodyFraming::Length(body_len) => {
                read_length(data, body_start, body_len, &mut reader).await?
            }
            BodyFraming::Chunked => {
                read_chunked(data, body_start, max_body_size, &mut reader).await?
            }
        };

        // nothing is read into the buffer from here on
        let data: &'a Vec<u8> = data;
        let block = str::from_utf8(&data[head]).map_err(|_| StatusCode::BadRequest)?;
        // Bytes already buffered or sent after the body are not part of it
        let body = data
            .get(body_start..body_end)
            .ok_or(StatusCode::BadRequest)?;
        let request = Self::parse(request_line, Headers::checked(block), body)
            .ok_or(StatusCode::BadRequest)?;
        Ok((request, body_end))
    }

    /// Whether the connection stays open after this request: HTTP/1.1
    /// unless `Connection: close`, HTTP/1.0 only with `Connection: keep-alive`
    /// (RFC 9112 section 9.3)
    pub(crate) fn keep_alive(&self) -> bool {
        match self.request_line.version {
            HttpVersion::Http11 => !self.headers.connection_has("close"),
            HttpVersion::Http10 => self.headers.connection_has("keep-alive"),
            _ => false,
        }
    }

    /// Get the first cookie with the given name. When several cookies share
    /// a name, browsers send the most specific one first
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    ///
    /// app.get("/", route_next!(|req, res| {
    ///     if let Some(session) = req.cookie("session") {
    ///         res.send(session.value());
    ///     }
    /// }));
    /// ```
    #[cfg(feature = "cookies")]
    pub fn cookie(&self, name: &str) -> Option<&Cookie<'a>> {
        self.cookies.iter().find(|cookie| cookie.name() == name)
    }

    /// Parse the query into its decoded `name=value` pairs, see [`Query`]
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    ///
    /// app.get("/", route_next!(|req, res| {
    ///     let parsed_query = req.get_query();
    ///     let value = parsed_query.get("value");
    /// }));
    /// ```
    pub fn get_query(&self) -> Query<'_> {
        Query::parse(&self.request_line.query)
    }

    /// Parse an `application/x-www-form-urlencoded` body, what an HTML
    /// `<form method="post">` sends, into its decoded `name=value` pairs.
    /// See [`Query`]. A body that isn't UTF-8 has no pairs: a form encodes
    /// everything outside ASCII as `%XX`.
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    ///
    /// app.post("/sign-up", route_stop!(|req, res| {
    ///     let form = req.get_form();
    ///     let name = form.get("name").unwrap_or("stranger");
    ///     res.send(format!("Welcome, {}", name));
    /// }));
    /// ```
    pub fn get_form(&self) -> Query<'_> {
        str::from_utf8(self.body)
            .map(Query::parse)
            .unwrap_or_default()
    }

    /// Parse the extended query and return the key value pairs in the form
    /// of an HashMap. See [`QueryValue`](../enum.QueryValue.html)
    #[cfg(feature = "extended_queries")]
    pub fn get_extended_query(&self) -> HashMap<String, QueryValue> {
        parse_extended_query(&self.request_line.query)
    }
}

// Read a `Content-Length` body starting at `start`, returning where it ends
async fn read_length<R: AsyncRead + Unpin>(
    data: &mut Vec<u8>,
    start: usize,
    len: usize,
    reader: &mut R,
) -> Result<usize, StatusCode> {
    let end = start.checked_add(len).ok_or(StatusCode::PayloadTooLarge)?;
    if end > data.len() {
        // grow the buffer as bytes arrive instead of allocating the
        // declared length up front
        let missing = (end - data.len()) as u64;
        reader
            .take(missing)
            .read_to_end(data)
            .await
            .map_err(|_| StatusCode::BadRequest)?;
        // the client hung up before sending the whole body
        if data.len() != end {
            return Err(StatusCode::BadRequest);
        }
    }
    Ok(end)
}

// Decode a chunked body (RFC 9112 section 7.1) starting at `start`, in place,
// returning where the decoded body ends.
//
// `data[start..end]` holds the body decoded so far and `data[pos..]` the bytes
// not decoded yet. Chunk data is moved down over the framing as it is read,
// which is never more bytes than were read, so decoding costs one copy of the
// body whatever the chunk sizes. Before reading more, the framing between
// `end` and `pos` is dropped, so the buffer holds the decoded body and at most
// one read of framing.
async fn read_chunked<R: AsyncRead + Unpin>(
    data: &mut Vec<u8>,
    start: usize,
    max_body_size: usize,
    reader: &mut R,
) -> Result<usize, StatusCode> {
    let mut end = start;
    let mut pos = start;
    loop {
        let line = read_line(data, end, &mut pos, MAX_CHUNK_LINE, reader).await?;
        let size = chunk_size(&data[line.0..line.1])?;
        if size == 0 {
            break;
        }
        if size > max_body_size - (end - start) {
            return Err(StatusCode::PayloadTooLarge);
        }
        let mut left = size;
        while left > 0 {
            if pos == data.len() {
                fill(data, end, &mut pos, reader).await?;
            }
            let n = left.min(data.len() - pos);
            data.copy_within(pos..pos + n, end);
            end += n;
            pos += n;
            left -= n;
        }
        // chunk data is followed by CRLF
        while data.len() - pos < 2 {
            fill(data, end, &mut pos, reader).await?;
        }
        if data[pos..pos + 2] != *B_CRLF {
            return Err(StatusCode::BadRequest);
        }
        pos += 2;
    }
    // The trailer section ends at an empty line. Trailer fields are checked,
    // but not merged into the headers (RFC 9112 section 7.1.2).
    let mut trailer_len = 0;
    loop {
        let (from, to) = read_line(data, end, &mut pos, MAX_HEAD_SIZE, reader).await?;
        if from == to {
            break;
        }
        trailer_len += to - from + B_CRLF.len();
        if trailer_len > MAX_HEAD_SIZE {
            return Err(StatusCode::RequestHeaderFieldsTooLarge);
        }
        if !is_trailer_field(&data[from..to]) {
            return Err(StatusCode::BadRequest);
        }
    }
    // what follows the body is the next request
    data.drain(end..pos);
    Ok(end)
}

// Find the line at `pos`, reading more until its CRLF arrives, and move `pos`
// past it. Returns where the line starts and ends, without the CRLF.
async fn read_line<R: AsyncRead + Unpin>(
    data: &mut Vec<u8>,
    end: usize,
    pos: &mut usize,
    max: usize,
    reader: &mut R,
) -> Result<(usize, usize), StatusCode> {
    loop {
        if let Some(len) = find_in_slice(&data[*pos..], B_CRLF) {
            if len > max {
                return Err(StatusCode::BadRequest);
            }
            let line = (*pos, *pos + len);
            *pos += len + B_CRLF.len();
            return Ok(line);
        }
        if data.len() - *pos > max {
            return Err(StatusCode::BadRequest);
        }
        fill(data, end, pos, reader).await?;
    }
}

// Drop the framing already decoded and read more of the body
async fn fill<R: AsyncRead + Unpin>(
    data: &mut Vec<u8>,
    end: usize,
    pos: &mut usize,
    reader: &mut R,
) -> Result<(), StatusCode> {
    data.drain(end..*pos);
    *pos = end;
    match read_some(reader, data).await {
        Ok(0) | Err(_) => Err(StatusCode::BadRequest),
        Ok(_) => Ok(()),
    }
}

// Whether `line` is a field line: a token name, a colon and a value with no
// control character but HTAB (RFC 9110 section 5). This is checked strictly
// whatever the features: a lenient check lets the bare LF in `0\r\n\n` start a
// "trailer" that swallows the request a proxy sends next, which smuggles the
// one hidden in that request's body (GHSA-2q7f-mf9f-5qhr in actix-http).
fn is_trailer_field(line: &[u8]) -> bool {
    let colon = match line.iter().position(|&b| b == b':') {
        Some(colon) => colon,
        None => return false,
    };
    let (name, value) = (&line[..colon], &line[colon + 1..]);
    str::from_utf8(name).map_or(false, is_token)
        && value
            .iter()
            .all(|&b| b == b'\t' || (b >= b' ' && b != 0x7f))
}

// Parse `chunk-size [ chunk-ext ]`. Extensions are allowed but ignored.
fn chunk_size(line: &[u8]) -> Result<usize, StatusCode> {
    let (size, ext) = match line.iter().position(|&b| b == b';') {
        Some(at) => line.split_at(at),
        None => (line, &[][..]),
    };
    // whitespace may come before the extensions (BWS), never before the size
    let size = match size.iter().rposition(|&b| b != b' ' && b != b'\t') {
        Some(last) => &size[..=last],
        None => return Err(StatusCode::BadRequest),
    };
    if !size.iter().all(u8::is_ascii_hexdigit) {
        return Err(StatusCode::BadRequest);
    }
    // HTAB is the only control character an extension may hold
    if ext.iter().any(|&b| (b < b' ' && b != b'\t') || b == 0x7f) {
        return Err(StatusCode::BadRequest);
    }
    size.iter().try_fold(0usize, |n, &b| {
        let digit = (b as char).to_digit(16).unwrap_or(0) as usize;
        n.checked_mul(16)
            .and_then(|n| n.checked_add(digit))
            // too large for any body that could be accepted
            .ok_or(StatusCode::PayloadTooLarge)
    })
}

/// The KeepAlive struct represents the value
/// parsed in the KeepAlive header. It holds the
/// timeout and max duration as a u64, (only http 1.0 and below)
#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct KeepAlive {
    timeout: Option<u64>,
    max: Option<u64>,
}

impl KeepAlive {
    /// The parse method takes a single valid
    /// `Keep-Alive` header and parses it to
    /// return a KeepAlive struct
    pub fn parse(header: &str) -> Self {
        let mut ret = Self {
            timeout: None,
            max: None,
        };
        for tok in header.split(',') {
            let trimmed = tok.trim();
            let eq_ind = match trimmed.find('=') {
                Some(v) => v,
                None => continue,
            };
            let (name, val_str) = trimmed.split_at(eq_ind);
            let val: u64 = match (&val_str[1..]).parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            match name {
                "timeout" => ret.timeout = Some(val),
                "max" => ret.max = Some(val),
                _ => continue,
            };
        }
        ret
    }
    // Returns the amount of timeout specified in the keep alive header
    pub fn timeout(&self) -> Option<u64> {
        self.timeout
    }
    #[allow(dead_code)]
    // Returns the amount of max requests specified in the keep alive header
    pub fn max(&self) -> Option<u64> {
        self.max
    }
}

/// The MatchedRequest is the struct which you see
/// when you have the `req` variable in the middleware closure.
/// It implements Deref to Request so you can use
/// Request methods/properties directly on it
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get("/",
///     route_next!(|req, res| {
///         // The req here is not actually a
///         // Request but a MatchedRequest which
///         // implements deref to Request.
///         // You can just directly use Request
///         // methods on it
///         let header = req.headers.get("Some-Header");
///     }),
/// );
/// ```
/// The struct also has the values of the url variables. This
/// requires the feature `url_variables` to be enabled.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MatchedRequest<'a> {
    /// The request coming from the client
    pub request: &'a Request<'a>,
    #[cfg(feature = "url_variables")]
    /// A Hashmap containing the variables specified in the url with their
    /// respective keys.
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    ///
    /// app.get("/foo/:var", // we used "var" as the identifier here
    ///     route_next!(|req, res| {
    ///         let some_header = req.headers.get("HeaderName");
    ///         res.with_type("application/json")
    ///            .send(req.vars.get("var").unwrap()); // we'll get the variable with the same keyword "var"
    ///     }),
    /// );
    /// ```
    pub vars: Vars<'a>,
}

impl<'a> std::ops::Deref for MatchedRequest<'a> {
    type Target = Request<'a>;

    fn deref(&self) -> &Self::Target {
        self.request
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn success_standard_request_line() {
        // Parsing should work as expected.
        let req = RequestLine::parse("POST /abc/def HTTP/1.1").unwrap();
        assert_eq!(req.method, RequestMethod::Post);
        assert_eq!(req.path, PathBuf::parse("/abc/def").ok().unwrap());
        assert_eq!(req.version, HttpVersion::Http11);
    }

    #[test]
    fn success_other_method() {
        let req = RequestLine::parse("PATCH /abc/def HTTP/1.1").unwrap();
        assert_eq!(req.method, RequestMethod::Patch);
        assert_eq!(req.path, PathBuf::parse("/abc/def").ok().unwrap());
        assert_eq!(req.version, HttpVersion::Http11);
    }

    #[test]
    fn byte_request_line_parser_matches_string_parser() {
        let cases = [
            ("GET", "/", "HTTP/1.1"),
            ("PATCH", "/a/b?x=1", "HTTP/1.0"),
            ("UNKNOWN", "/resource", "HTTP/9.9"),
            // Inputs containing spaces take the compatibility fallback.
            ("GET extra", "/", "HTTP/1.1"),
            ("GET", "/two paths", "HTTP/1.1"),
        ];
        for (method, target, version) in cases {
            let combined = format!("{} {} {}", method, target, version);
            assert_eq!(
                request_line_parse(method.as_bytes(), target.as_bytes(), version.as_bytes()),
                RequestLine::parse(&combined)
            );
        }
    }

    #[test]
    fn sucess_non_documented() {
        // Non-documented methods should also work.
        let req = RequestLine::parse("XYZ /abc/def HTTP/1.1").unwrap();
        assert_eq!(req.method, RequestMethod::None);
        assert_eq!(req.path, PathBuf::parse("/abc/def").ok().unwrap());
        assert_eq!(req.version, HttpVersion::Http11);
    }

    #[test]
    #[should_panic]
    #[cfg_attr(not(feature = "faithful"), ignore)]
    fn fail_extra_1() {
        // Extra clauses should error.
        RequestLine::parse("POST /abc/def HTTP/1.1 x").unwrap();
    }

    #[test]
    #[should_panic]
    #[cfg_attr(not(feature = "faithful"), ignore)]
    fn fail_extra_2() {
        // Extra clauses should error.
        RequestLine::parse("POST /a /b HTTP/1.1").unwrap();
    }

    #[test]
    #[should_panic]
    #[cfg_attr(not(feature = "faithful"), ignore)]
    fn fail_malformed_version() {
        // Malformed versions should error.
        RequestLine::parse("POST /abc/def HTDP/1.1").unwrap();
    }

    #[test]
    #[should_panic]
    fn fail_missing_clause() {
        // Missing clauses should error.
        RequestLine::parse("POST /abc/def").unwrap();
    }

    #[test]
    fn success_standard() {
        // Parsing should work as expected.
        let reqline = RequestLine::parse("POST /abc/def HTTP/1.1").unwrap();
        assert_eq!(reqline.method, RequestMethod::Post);
        assert_eq!(reqline.path, PathBuf::parse("/abc/def").ok().unwrap());
        assert_eq!(reqline.version, HttpVersion::Http11);
        let headers = Headers::parse(
            "Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        Accept: */*\r\n\
        Content-Length: 20\r\n\
        Content-Type: application/x-www-form-urlencoded",
        )
        .unwrap();
        assert_eq!(headers.get("host").unwrap(), "localhost:12345");
        assert_eq!(headers.get("user-agent").unwrap(), "curl/7.58.0");
        assert_eq!(headers.get("accept").unwrap(), "*/*");
        assert_eq!(headers.get("content-length").unwrap(), "20");
        assert_eq!(
            headers.get("content-type").unwrap(),
            "application/x-www-form-urlencoded"
        );
    }

    #[test]
    fn names_are_kept_as_sent_and_matched_without_case() {
        let headers = Headers::parse("HOst: localhost:12345\r\nUser-Agent: curl/7.58.0").unwrap();
        let fields: Vec<_> = headers.iter().map(|h| (h.name(), h.value())).collect();
        assert_eq!(
            fields,
            [("HOst", "localhost:12345"), ("User-Agent", "curl/7.58.0")]
        );
        assert_eq!(headers.get("host"), Some("localhost:12345"));
        assert_eq!(headers.get("USER-AGENT"), Some("curl/7.58.0"));
        assert_eq!(headers.len(), 2);
    }

    // The one field of a one line block
    fn field(line: &str) -> Option<(&str, &str)> {
        let headers = Headers::parse(line)?;
        let header = headers.iter().next()?;
        Some((header.name(), header.value()))
    }

    #[test]
    fn success_standard_header() {
        // Parsing should work as expected.
        let req = field("Referer: \t\t request://www.example.com/");
        assert_eq!(req, Some(("Referer", "request://www.example.com/")));
    }

    #[test]
    fn success_empty_value() {
        // Empty values are allowed.
        assert_eq!(field("Referer: \t\t "), Some(("Referer", "")));
    }

    #[test]
    fn malformed_fields_are_refused_whatever_the_features() {
        // no colon, no name, separators or control characters in the name,
        // control characters in the value
        for line in [
            "Referer",
            ": test",
            "Test Header: test",
            "Content-Length : 5",
            "X\x0b: 1",
            "X: a\x00b",
        ]
        .iter()
        {
            assert!(Headers::parse(line).is_none(), "{:?}", line);
        }
    }

    #[test]
    fn success_keepalive() {
        // Parsing should work as expected.
        let req = KeepAlive::parse("timeout=5, max=1000");
        assert_eq!(req.timeout(), Some(5));
        assert_eq!(req.max(), Some(1000));
    }

    #[test]
    fn success_keepalive_edge() {
        // Edge cases should work as expected.
        let req = KeepAlive::parse("timeout=,test,max=a, timeout=5");
        assert_eq!(req.timeout(), Some(5));
        assert_eq!(req.max(), None);
    }

    #[test]
    fn success_query_split() {
        // The query isn't part of the path.
        let req = RequestLine::parse("GET /a/b?x=1&y=2?z HTTP/1.1").unwrap();
        assert_eq!(req.path, PathBuf::parse("/a/b").ok().unwrap());
        assert_eq!(req.query, "x=1&y=2?z");
        assert_eq!(RequestLine::parse("GET /a HTTP/1.1").unwrap().query, "");
    }

    #[test]
    fn malformed_request_lines_do_not_panic() {
        // These used to panic: a version shorter than 5 bytes, a character
        // over byte 5 of the version and a path above the root.
        let short = RequestLine::parse("GET / X");
        let split = RequestLine::parse("GET / HTTP\u{e9}1.1");
        assert!(RequestLine::parse("GET /.. HTTP/1.1").is_none());
        if cfg!(feature = "faithful") {
            assert!(short.is_none() && split.is_none());
        }
    }

    #[test]
    fn success_no_headers() {
        // A request may have no header lines at all.
        assert!(Headers::parse("").unwrap().is_empty());
    }

    #[test]
    fn headers_accept_borrowed_blocks() {
        let block = String::from("Host: localhost\r\nX-Test: value");
        let headers = Headers::parse(block.as_str()).unwrap();
        assert_eq!(headers.get("host").unwrap(), "localhost");
        assert_eq!(headers.get("x-test").unwrap(), "value");
    }

    #[test]
    fn success_value_whitespace_and_tabs() {
        // Whitespace around a value is dropped, tabs inside it are kept.
        assert_eq!(field("X-Test:  a\tb \t"), Some(("X-Test", "a\tb")));
    }

    #[test]
    fn counts_host_fields() {
        let count = |block: &str| Headers::parse(block).unwrap().host_count();
        assert_eq!(count(""), 0);
        assert_eq!(count("Host: a"), 1);
        assert_eq!(count("Host: a\r\nX: 1\r\nhOST: a"), 2);
    }

    #[test]
    fn repeated_fields_are_all_kept() {
        let headers = Headers::parse("Cookie: a=1\r\nX: 0\r\ncookie: b=2").unwrap();
        assert_eq!(headers.get("cookie"), Some("a=1"));
        assert_eq!(
            headers.get_all("Cookie").collect::<Vec<_>>(),
            ["a=1", "b=2"]
        );
        #[cfg(feature = "cookies")]
        {
            let line = RequestLine::parse("GET / HTTP/1.1").unwrap();
            let request = Request::parse(line, headers, b"").unwrap();
            let cookies: Vec<_> = request
                .cookies
                .iter()
                .map(|c| (c.name(), c.value()))
                .collect();
            assert_eq!(cookies, [("a", "1"), ("b", "2")]);
        }
    }

    #[test]
    fn success_content_length() {
        let parse = |block: &str| Headers::parse(block).unwrap().content_length(100);
        assert_eq!(parse("Host: a"), Ok(0));
        assert_eq!(parse("Content-Length: 42"), Ok(42));
        assert_eq!(parse("Content-Length: 007"), Ok(7));
        assert_eq!(parse("Content-Length: 5\r\nContent-Length: 5"), Ok(5));
        assert_eq!(parse("Content-Length: 5, 5"), Ok(5));
    }

    #[test]
    fn fail_content_length() {
        let parse = |block: &str| Headers::parse(block).unwrap().content_length(100);
        let bad = Err(StatusCode::BadRequest);
        assert_eq!(parse("Content-Length: 5\r\nContent-Length: 6"), bad);
        assert_eq!(parse("Content-Length: +5"), bad);
        assert_eq!(parse("Content-Length: -1"), bad);
        assert_eq!(parse("Content-Length: 0x10"), bad);
        assert_eq!(parse("Content-Length: "), bad);
        assert_eq!(parse("Content-Length: 5,"), bad);
        // too large to represent, but still in disagreement
        let huge = "Content-Length: 99999999999999999999998, 99999999999999999999999";
        assert_eq!(parse(huge), bad);
        let too_large = Err(StatusCode::PayloadTooLarge);
        assert_eq!(parse("Content-Length: 101"), too_large);
        assert_eq!(parse("Content-Length: 99999999999999999999999"), too_large);
    }

    // `data` as `from_raw` gets it: the header block, then whatever of the
    // body was buffered with it
    fn buffered(block: &str, body: &[u8]) -> (Vec<u8>, Range<usize>, usize) {
        let mut data = block.as_bytes().to_vec();
        let head = 0..data.len();
        data.extend_from_slice(body);
        (data, head, block.len())
    }

    fn framing(block: &str, max: usize) -> Result<BodyFraming, StatusCode> {
        Headers::parse(block)
            .unwrap()
            .body_framing(HttpVersion::Http11, max)
    }

    #[tokio::test]
    async fn from_raw_appends_missing_body_to_the_request_buffer() {
        let block = "Content-Length: 5";
        let (mut data, head, start) = buffered(block, b"he");
        let (request, end) = Request::from_raw(
            head,
            framing(block, 16).unwrap(),
            RequestLine::parse("POST / HTTP/1.1").unwrap(),
            &mut data,
            start,
            &b"lloignored"[..],
            16,
        )
        .await
        .unwrap();

        assert_eq!(request.body, b"hello");
        assert_eq!(request.headers.get("content-length"), Some("5"));
        let body_pointer = request.body.as_ptr() as usize;
        drop(request);
        assert_eq!(body_pointer, data.as_ptr() as usize + start);
        assert_eq!(&data[start..], b"hello");
        assert_eq!(end, data.len());
    }

    #[tokio::test]
    async fn from_raw_excludes_already_buffered_trailing_bytes() {
        let block = "Content-Length: 2";
        let (mut data, head, start) = buffered(block, b"hello");
        let (request, end) = Request::from_raw(
            head,
            framing(block, 16).unwrap(),
            RequestLine::parse("POST / HTTP/1.1").unwrap(),
            &mut data,
            start,
            &b""[..],
            16,
        )
        .await
        .unwrap();

        assert_eq!(request.body, b"he");
        drop(request);
        // the rest is the next request
        assert_eq!(&data[end..], b"llo");
    }

    #[tokio::test]
    async fn oversized_and_incomplete_bodies_are_refused() {
        // too large is known from the headers, before anything is read
        assert_eq!(
            framing("Content-Length: 6", 5),
            Err(StatusCode::PayloadTooLarge)
        );

        let block = "Content-Length: 5";
        let (mut data, head, start) = buffered(block, b"he");
        let incomplete = Request::from_raw(
            head,
            framing(block, 5).unwrap(),
            RequestLine::parse("POST / HTTP/1.1").unwrap(),
            &mut data,
            start,
            &b"l"[..],
            5,
        )
        .await;
        assert!(matches!(incomplete, Err(StatusCode::BadRequest)));
        drop(incomplete);
        assert_eq!(&data[start..], b"hel");
    }

    // Hands out at most `step` bytes per read, like a slow client
    struct Trickle<'a> {
        bytes: &'a [u8],
        step: usize,
    }

    impl AsyncRead for Trickle<'_> {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let n = self.step.min(self.bytes.len()).min(buf.remaining());
            let (read, rest) = self.bytes.split_at(n);
            buf.put_slice(read);
            self.bytes = rest;
            std::task::Poll::Ready(Ok(()))
        }
    }

    // Decode `wire` as a chunked body that arrives `step` bytes at a time
    // after `buffered`, returning the body and what came after it: the bytes
    // left in the buffer followed by those that weren't read at all
    async fn chunked(
        buffered: &[u8],
        wire: &[u8],
        step: usize,
        max: usize,
    ) -> Result<(Vec<u8>, Vec<u8>), StatusCode> {
        let block = "Transfer-Encoding: chunked";
        let (mut data, head, start) = self::buffered(block, buffered);
        let mut reader = Trickle { bytes: wire, step };
        let (request, end) = Request::from_raw(
            head,
            BodyFraming::Chunked,
            RequestLine::parse("POST / HTTP/1.1").unwrap(),
            &mut data,
            start,
            &mut reader,
            max,
        )
        .await?;
        let body = request.body.to_vec();
        assert_eq!(request.headers.get("transfer-encoding"), Some("chunked"));
        drop(request);
        assert_eq!(&data[..start], block.as_bytes());
        assert_eq!(&data[start..end], &body[..]);
        let mut rest = data[end..].to_vec();
        rest.extend_from_slice(reader.bytes);
        Ok((body, rest))
    }

    #[tokio::test]
    async fn chunked_bodies_are_decoded_whatever_the_fragmentation() {
        let wire: &[u8] = b"5\r\nhello\r\n1;name=\"v\"\r\n,\r\nA \t;x\r\n world!!!!\r\n0\r\nX-Trailer: 1\r\n\r\nGET / HTTP/1.1\r\n";
        let want = (b"hello, world!!!!".to_vec(), b"GET / HTTP/1.1\r\n".to_vec());
        for step in 1..=wire.len() {
            // all of it arriving later, a byte at a time and so on
            assert_eq!(
                chunked(b"", wire, step, 64).await,
                Ok(want.clone()),
                "step {}",
                step
            );
        }
        for split in 0..=wire.len() {
            // some of it buffered with the head, the rest read later
            let (buffered, rest) = wire.split_at(split);
            assert_eq!(
                chunked(buffered, rest, 3, 64).await,
                Ok(want.clone()),
                "split {}",
                split
            );
        }
        // trailer values may hold tabs and bytes over 0x7f
        let trailers = b"0\r\nX-Sum: a\tb \xc3\xa9\r\nX-Empty:\r\n\r\n";
        assert_eq!(
            chunked(trailers, b"", 1, 0).await,
            Ok((Vec::new(), Vec::new()))
        );
        // an empty body
        let empty = chunked(b"0\r\n\r\n", b"", 1, 0).await;
        assert_eq!(empty, Ok((Vec::new(), Vec::new())));
    }

    #[tokio::test]
    async fn malformed_chunked_bodies_are_refused() {
        let bad = Err(StatusCode::BadRequest);
        let cases: &[&[u8]] = &[
            // no size, not hex, a sign or whitespace before it
            b"\r\nhello\r\n0\r\n\r\n",
            b"g\r\nhello\r\n0\r\n\r\n",
            b"+5\r\nhello\r\n0\r\n\r\n",
            b" 5\r\nhello\r\n0\r\n\r\n",
            b"0x5\r\nhello\r\n0\r\n\r\n",
            // bare LF line ends
            b"5\nhello\r\n0\r\n\r\n",
            b"5\r\nhello\n0\r\n\r\n",
            // data longer than its size
            b"5\r\nhello!\r\n0\r\n\r\n",
            // a control character in an extension
            b"5;a\x01\r\nhello\r\n0\r\n\r\n",
            // a malformed trailer field
            b"5\r\nhello\r\n0\r\nno colon\r\n\r\n",
            // a bare LF after the last chunk can't start a trailer that
            // swallows the next request (GHSA-2q7f-mf9f-5qhr in actix-http)
            b"0\r\n\nGET /a:b HTTP/1.1\r\nHost: a\r\n\r\n",
            b"0\r\n\nX-Smuggled: 1\r\n\r\n",
            b"0\r\nX-Bad\x01: 1\r\n\r\n",
            b"0\r\nX-Ok: a\x00b\r\n\r\n",
            b"0\r\nBad Name: 1\r\n\r\n",
            // the client hangs up before the end
            b"5\r\nhel",
            b"5\r\nhello\r\n0\r\n",
        ];
        for wire in cases {
            let got = chunked(b"", wire, 2, 64).await;
            assert_eq!(got, bad, "{:?}", String::from_utf8_lossy(wire));
        }
        // more than max_body_size, however it is split up
        let too_large = Err(StatusCode::PayloadTooLarge);
        assert_eq!(
            chunked(b"", b"6\r\nhello!\r\n0\r\n\r\n", 4, 5).await,
            too_large
        );
        assert_eq!(
            chunked(b"", b"3\r\nabc\r\n3\r\ndef\r\n0\r\n\r\n", 4, 5).await,
            too_large
        );
        let overflow = b"ffffffffffffffffffff\r\n";
        assert_eq!(chunked(b"", overflow, 4, 5).await, too_large);
    }

    #[test]
    fn body_framing_follows_rfc_9112() {
        let framing =
            |block: &str, version| Headers::parse(block).unwrap().body_framing(version, 100);
        let v11 = HttpVersion::Http11;
        assert_eq!(framing("", v11), Ok(BodyFraming::Length(0)));
        assert_eq!(
            framing("Content-Length: 5", v11),
            Ok(BodyFraming::Length(5))
        );
        assert_eq!(
            framing("Transfer-Encoding: chunked", v11),
            Ok(BodyFraming::Chunked)
        );
        assert_eq!(
            framing("Transfer-Encoding: CHUNKED", v11),
            Ok(BodyFraming::Chunked)
        );
        let bad = Err(StatusCode::BadRequest);
        // smuggling: both framings, or chunked on HTTP/1.0
        let both = "Transfer-Encoding: chunked\r\nContent-Length: 5";
        assert_eq!(framing(both, v11), bad);
        let http10 = framing("Transfer-Encoding: chunked", HttpVersion::Http10);
        assert_eq!(http10, bad);
        // chunked not last, twice, or nothing at all
        assert_eq!(framing("Transfer-Encoding: chunked, gzip", v11), bad);
        assert_eq!(framing("Transfer-Encoding: chunked, chunked", v11), bad);
        let repeated = "Transfer-Encoding: chunked\r\nTransfer-Encoding: chunked";
        assert_eq!(framing(repeated, v11), bad);
        assert_eq!(framing("Transfer-Encoding: gzip", v11), bad);
        assert_eq!(framing("Transfer-Encoding: ", v11), bad);
        // codings that aren't supported
        let gzip = framing("Transfer-Encoding: gzip, chunked", v11);
        assert_eq!(gzip, Err(StatusCode::NotImplemented));
    }

    #[test]
    fn keep_alive_follows_the_version_and_connection_header() {
        let keep_alive = |line: &str, block: &str| {
            Request::parse(
                RequestLine::parse(line).unwrap(),
                Headers::parse(block).unwrap(),
                b"",
            )
            .unwrap()
            .keep_alive()
        };
        assert!(keep_alive("GET / HTTP/1.1", ""));
        assert!(keep_alive("GET / HTTP/1.1", "Connection: keep-alive"));
        assert!(!keep_alive("GET / HTTP/1.1", "Connection: close"));
        assert!(!keep_alive("GET / HTTP/1.1", "Connection: Upgrade, Close"));
        assert!(!keep_alive("GET / HTTP/1.0", ""));
        assert!(keep_alive("GET / HTTP/1.0", "Connection: Keep-Alive"));
    }
}
