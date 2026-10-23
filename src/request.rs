use crate::constants::*;
#[cfg(feature = "cookies")]
use crate::cookie::Cookie;
use crate::deref;
use crate::path::is_ctl;
use crate::path::PathBuf;
use crate::query::parse_query;
#[cfg(feature = "extended_queries")]
use crate::query::{parse_extended_query, QueryValue};
use crate::util::Spliterator;
use octane_http::{HttpVersion, StatusCode};
use std::cfg;
use std::collections::HashMap;
#[cfg(not(feature = "raw_headers"))]
use std::marker::PhantomData;
use std::str;
use std::string::ToString;
use tokio::io::{AsyncRead, AsyncReadExt};

/// Holds the type of request method, like GET,
/// POST etc.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub enum RequestMethod {
    #[doc(hidden)]
    Options,
    #[doc(hidden)]
    Get,
    #[doc(hidden)]
    Head,
    #[doc(hidden)]
    Post,
    #[doc(hidden)]
    Put,
    #[doc(hidden)]
    Delete,
    #[doc(hidden)]
    Trace,
    #[doc(hidden)]
    Connect,
    #[doc(hidden)]
    Patch,
    #[doc(hidden)]
    All,
    #[doc(hidden)]
    None,
}

impl RequestMethod {
    /// Return false if the RequestMethod is `None`, otherwise it's true
    pub fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }
}

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
        let path = PathBuf::parse(path).ok()?;
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

/// The header structure represents a parsed value
/// of unit header that looks like `key: value`
/// and holds both the key and value. You
/// shouldn't use it directly as the parsing has
/// been done for you and all the headers are
/// available in the `Headers` struct which you
/// get as a field in the `req` variable in
/// closures
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Header {
    name: String,
    value: String,
}

impl Header {
    /// Parses a `key: value` header string and
    /// returns a Header struct
    pub(crate) fn parse(header: impl AsRef<str>) -> Option<Self> {
        let header = header.as_ref();
        let mut toks = header.splitn(2, ':');
        let name = toks.next()?;
        if name.is_empty() {
            return None;
        }
        if cfg!(feature = "faithful") && !is_token(name) {
            return None;
        }
        // whitespace around a field value isn't part of it, and HTAB is the
        // only control character allowed inside one
        let value = toks.next()?.trim_matches(|c| c == SP || c == HT);
        if cfg!(feature = "faithful") && value.chars().any(|c| is_ctl(c) && c != HT) {
            return None;
        }
        Some(Self {
            name: name.to_owned(),
            value: value.to_owned(),
        })
    }
    /// Return the name of the header
    pub fn name(&self) -> String {
        self.name.to_string()
    }
    /// Return the value of the header
    pub fn value(&self) -> String {
        self.value.to_string()
    }
}

/// The `Headers` struct holds _all_ the headers
/// a request might have in raw form (if the
/// feature is enabled) and in a HashMap with key
/// and value of the type `String`
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
///         let some_header = req.headers.get("HeaderName");
///         Flow::Stop
///     }),
/// );
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Headers {
    /// Headers, serialized to a HashMap
    pub parsed: HashMap<String, String>,
    #[cfg(feature = "raw_headers")]
    /// Contains headers in raw form
    pub raw: Vec<Header>,
    #[cfg(not(feature = "raw_headers"))]
    raw: PhantomData<()>,
}

impl Headers {
    /// Parse all the headers on a request
    pub(crate) fn parse(request: impl AsRef<str>) -> Option<Self> {
        let request = request.as_ref();
        let mut toks = Spliterator::new(request.as_bytes(), B_CRLF);
        // an empty block is a request without any header lines
        toks.finished = request.is_empty();
        let mut headers: HashMap<String, String> = HashMap::new();
        #[cfg(feature = "raw_headers")]
        let mut raw_headers: Vec<Header> = Vec::new();
        for tok in toks {
            let parsed = Header::parse(str::from_utf8(tok).ok()?)?;
            #[cfg(feature = "raw_headers")]
            {
                let name = parsed.name.to_ascii_lowercase();
                // cookie pairs are joined with "; " (RFC 6265 section 5.4)
                let sep = if name == "cookie" { "; " } else { ", " };
                headers
                    .entry(name)
                    .and_modify(|v| {
                        v.push_str(sep);
                        v.push_str(&parsed.value);
                    })
                    .or_insert_with(|| parsed.value.to_owned());
            }
            #[cfg(not(feature = "raw_headers"))]
            {
                // Without raw-header preservation, reuse the allocations made
                // by `Header::parse` instead of cloning both strings.
                let Header { mut name, value } = parsed;
                name.make_ascii_lowercase();
                let sep = if name == "cookie" { "; " } else { ", " };
                match headers.entry(name) {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        entry.get_mut().push_str(sep);
                        entry.get_mut().push_str(&value);
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(value);
                    }
                }
            }
            #[cfg(feature = "raw_headers")]
            raw_headers.push(parsed);
        }
        Some(Self {
            parsed: headers,
            #[cfg(feature = "raw_headers")]
            raw: raw_headers,
            #[cfg(not(feature = "raw_headers"))]
            raw: PhantomData,
        })
    }

    /// The body length declared by the `Content-Length` header, 0 if it
    /// isn't present (RFC 9112 section 6.3). Repeated identical values like
    /// `5, 5` are accepted. A value that isn't a decimal number or values
    /// that disagree give a `400`, a length over `max` gives a `413`.
    pub(crate) fn content_length(&self, max: usize) -> Result<usize, StatusCode> {
        let value = match self.get("content-length") {
            Some(value) => value,
            None => return Ok(0),
        };
        let mut digits: Option<&str> = None;
        for member in value.split(',') {
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

    /// Build `Headers` from already-decoded name/value pairs, as produced by
    /// the HTTP/2 HPACK decoder. Names are lowercased and duplicates are joined
    /// (with `; ` for `cookie` per RFC 9113 §8.2.3, `, ` otherwise); no header
    /// block is re-serialized.
    pub(crate) fn from_pairs(pairs: &[(String, String)]) -> Self {
        let mut parsed: HashMap<String, String> = HashMap::new();
        #[cfg(feature = "raw_headers")]
        let mut raw: Vec<Header> = Vec::new();
        for (name, value) in pairs {
            let lower = name.to_ascii_lowercase();
            let sep = if lower == "cookie" { "; " } else { ", " };
            parsed
                .entry(lower)
                .and_modify(|v| {
                    v.push_str(sep);
                    v.push_str(value);
                })
                .or_insert_with(|| value.clone());
            #[cfg(feature = "raw_headers")]
            raw.push(Header {
                name: name.clone(),
                value: value.clone(),
            });
        }
        Self {
            parsed,
            #[cfg(feature = "raw_headers")]
            raw,
            #[cfg(not(feature = "raw_headers"))]
            raw: PhantomData,
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
    pub headers: Headers,
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
        headers: Headers,
        body: &'a [u8],
    ) -> Option<Self> {
        #[cfg(feature = "cookies")]
        let cookies: Vec<Cookie<'static>> = headers.get("cookie").map_or_else(Vec::new, |value| {
            Cookie::parse(value)
                .into_iter()
                .map(|cookie| Cookie::new(cookie.name().to_owned(), cookie.value().to_owned()))
                .collect()
        });
        Some(Self {
            request_line,
            headers,
            #[cfg(feature = "cookies")]
            cookies,
            body,
        })
    }
    /// Build a [`Request`] from an already-parsed request line and header set,
    /// reading any body bytes that weren't already buffered alongside the head.
    ///
    /// `body_start` is the offset in `data` immediately after the request head.
    /// If `Content-Length` calls for more bytes than are already buffered, the
    /// rest is streamed off `reader` into that same allocation. Bodies longer
    /// than `max_body_size` are refused with a `413` before anything is read.
    pub(crate) async fn from_raw<T: AsyncRead + Unpin>(
        headers: Headers,
        request_line: RequestLine,
        data: &'a mut Vec<u8>,
        body_start: usize,
        mut reader: T,
        max_body_size: usize,
    ) -> Result<Request<'a>, StatusCode> {
        let body_len = headers.content_length(max_body_size)?;
        let buffered_body_len = data
            .len()
            .checked_sub(body_start)
            .ok_or(StatusCode::BadRequest)?;
        let body_end = body_start
            .checked_add(body_len)
            .ok_or(StatusCode::PayloadTooLarge)?;

        if body_len > buffered_body_len {
            // grow the buffer as bytes arrive instead of allocating the
            // declared length up front
            let missing = (body_len - buffered_body_len) as u64;
            (&mut reader)
                .take(missing)
                .read_to_end(data)
                .await
                .map_err(|_| StatusCode::BadRequest)?;
            // the client hung up before sending the whole body
            if data.len() != body_end {
                return Err(StatusCode::BadRequest);
            }
        }

        // Bytes already buffered or sent after the declared length are not
        // part of this request body.
        let body = data
            .get(body_start..body_end)
            .ok_or(StatusCode::BadRequest)?;
        Self::parse(request_line, headers, body).ok_or(StatusCode::BadRequest)
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

    /// Parse the query and return the key value pairs in the form
    /// of an HashMap
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
    pub fn get_query(&self) -> HashMap<String, String> {
        parse_query(&self.request_line.query)
    }

    /// Parse the extended query and return the key value pairs in the form
    /// of an HashMap. See [`QueryValue`](../enum.QueryValue.html)
    #[cfg(feature = "extended_queries")]
    pub fn get_extended_query(&self) -> HashMap<String, QueryValue> {
        parse_extended_query(&self.request_line.query)
    }
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
    pub request: Request<'a>,
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
    pub vars: HashMap<&'a str, &'a str>,
}

deref!(MatchedRequest<'a>, Request<'a>, request);
deref!(Headers, HashMap<String, String>, parsed);

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
        Content-Type: application/x-www-form-urlencoded"
                .to_string(),
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
    #[cfg(feature = "raw_headers")]
    fn success_raw_headers() {
        // Parsing should work as expected.
        let headers = Headers::parse(
            "HOst: localhost:12345\r\n\
        User-Agent: curl/7.58.0"
                .to_string(),
        )
        .unwrap();
        assert_eq!(headers.raw[0].name(), "HOst");
        assert_eq!(headers.raw[0].value(), "localhost:12345");
        assert_eq!(headers.raw[1].name(), "User-Agent");
        assert_eq!(headers.raw[1].value(), "curl/7.58.0");
    }

    #[test]
    fn success_standard_header() {
        // Parsing should work as expected.
        let req = Header::parse("Referer: \t\t request://www.example.com/".to_string()).unwrap();
        assert_eq!(req.name(), "Referer");
        assert_eq!(req.value(), "request://www.example.com/");
    }

    #[test]
    fn success_empty_value() {
        // Empty values are allowed.
        let req = Header::parse("Referer: \t\t ".to_string()).unwrap();
        assert_eq!(req.name(), "Referer");
        assert_eq!(req.value(), "");
    }

    #[test]
    #[should_panic]
    fn fail_no_value() {
        // Having no value should fail.
        Header::parse("Referer".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    fn fail_empty_name() {
        // Having no name should fail.
        Header::parse(": test".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    #[cfg_attr(not(feature = "faithful"), ignore)]
    fn fail_malformed_name() {
        // Having separators in the name should fail.
        Header::parse("Test Header: test".to_string()).unwrap();
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
        assert!(Headers::parse(String::new()).unwrap().is_empty());
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
        let req = Header::parse("X-Test:  a\tb \t".to_string()).unwrap();
        assert_eq!(req.value(), "a\tb");
    }

    #[test]
    fn success_cookie_headers_joined_with_semicolons() {
        let headers = Headers::parse("Cookie: a=1\r\nCookie: b=2".to_string()).unwrap();
        assert_eq!(headers.get("cookie").unwrap(), "a=1; b=2");
    }

    #[test]
    fn success_content_length() {
        let parse = |block: &str| {
            Headers::parse(block.to_string())
                .unwrap()
                .content_length(100)
        };
        assert_eq!(parse("Host: a"), Ok(0));
        assert_eq!(parse("Content-Length: 42"), Ok(42));
        assert_eq!(parse("Content-Length: 007"), Ok(7));
        assert_eq!(parse("Content-Length: 5\r\nContent-Length: 5"), Ok(5));
        assert_eq!(parse("Content-Length: 5, 5"), Ok(5));
    }

    #[test]
    fn fail_content_length() {
        let parse = |block: &str| {
            Headers::parse(block.to_string())
                .unwrap()
                .content_length(100)
        };
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

    #[tokio::test]
    async fn from_raw_appends_missing_body_to_the_request_buffer() {
        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(b"HEADhe");
        let request = Request::from_raw(
            Headers::parse("Content-Length: 5").unwrap(),
            RequestLine::parse("POST / HTTP/1.1").unwrap(),
            &mut data,
            4,
            &b"lloignored"[..],
            16,
        )
        .await
        .unwrap();

        assert_eq!(request.body, b"hello");
        let body_pointer = request.body.as_ptr() as usize;
        drop(request);
        assert_eq!(body_pointer, data.as_ptr() as usize + 4);
        assert_eq!(data, b"HEADhello");
    }

    #[tokio::test]
    async fn from_raw_excludes_already_buffered_trailing_bytes() {
        let mut data = b"HEADhello".to_vec();
        let request = Request::from_raw(
            Headers::parse("Content-Length: 2").unwrap(),
            RequestLine::parse("POST / HTTP/1.1").unwrap(),
            &mut data,
            4,
            &b""[..],
            16,
        )
        .await
        .unwrap();

        assert_eq!(request.body, b"he");
        drop(request);
        assert_eq!(data, b"HEADhello");
    }

    #[tokio::test]
    async fn from_raw_rejects_oversized_and_incomplete_bodies() {
        let mut oversized_data = b"HEAD".to_vec();
        let oversized = Request::from_raw(
            Headers::parse("Content-Length: 6").unwrap(),
            RequestLine::parse("POST / HTTP/1.1").unwrap(),
            &mut oversized_data,
            4,
            &b"unused"[..],
            5,
        )
        .await;
        assert!(matches!(oversized, Err(StatusCode::PayloadTooLarge)));
        drop(oversized);
        assert_eq!(oversized_data, b"HEAD");

        let mut incomplete_data = b"HEADhe".to_vec();
        let incomplete = Request::from_raw(
            Headers::parse("Content-Length: 5").unwrap(),
            RequestLine::parse("POST / HTTP/1.1").unwrap(),
            &mut incomplete_data,
            4,
            &b"l"[..],
            5,
        )
        .await;
        assert!(matches!(incomplete, Err(StatusCode::BadRequest)));
        drop(incomplete);
        assert_eq!(incomplete_data, b"HEADhel");
    }
}
