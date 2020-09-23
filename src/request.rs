use crate::constants::*;
#[cfg(feature = "cookies")]
use crate::cookies::Cookies;
use crate::deref;
use crate::path::is_ctl;
use crate::path::PathBuf;
use crate::util::Spliterator;
use std::cfg;
use std::collections::HashMap;
#[cfg(not(feature = "raw_headers"))]
use std::marker::PhantomData;
use std::str;
use std::string::ToString;

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
        if let Self::None = self {
            false
        } else {
            true
        }
    }
}
/// Holds the http versions. You can match the
/// variants by doing a comparison with the version
/// in the request_line
///
/// # Example
///
/// ```no_run
/// use octane::prelude::*;
/// use octane::request::HttpVersion;
///
/// let mut app = Octane::new();
/// app
/// .get("/",
///     route!(|req, res| {
///        if req.request_line.version == HttpVersion::Http11 {
///            // do something
///         }
///         Flow::Stop
///     }),
/// );
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HttpVersion {
    #[doc(hidden)]
    Http11,
    #[doc(hidden)]
    Http10,
    #[doc(hidden)]
    Http02,
    #[doc(hidden)]
    Http09,
    #[doc(hidden)]
    HttpInvalid,
}

impl ToString for HttpVersion {
    fn to_string(&self) -> std::string::String {
        match self {
            Self::Http11 => "1.1",
            Self::Http10 => "1.0",
            Self::Http09 => "0.9",
            Self::Http02 => "0.2",
            _ => "",
        }
        .to_owned()
    }
}
/// The RequestLine struct represents the first
/// line of the http request, which contains
/// the http version, path and method of request
///
/// # Example
///
/// ```no_run
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
    /// Http version of the request
    pub version: HttpVersion,
}

impl RequestLine {
    /// Parses a request line str and returns a
    /// request line struct.
    pub(crate) fn parse(request_line: &str) -> Option<Self> {
        let mut toks = request_line.split(SP);
        let method = toks.next()?;
        let path = match PathBuf::parse(toks.next()?) {
            Ok(val) => val,
            Err(e) => panic!("{:?}", e),
        };
        let version = toks.next()?;
        let (first, ver) = version.split_at(5);
        let enum_ver = match ver {
            "1.1" => HttpVersion::Http11,
            "1.0" => HttpVersion::Http10,
            "2.0" => HttpVersion::Http02,
            "0.9" => HttpVersion::Http09,
            _ => HttpVersion::HttpInvalid,
        };

        if cfg!(feature = "faithful") && (first != "HTTP/" || toks.next().is_some()) {
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
            version: enum_ver,
        })
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
    pub(crate) fn parse(header: String) -> Option<Self> {
        let mut toks = header.splitn(2, ':');
        let name = toks.next()?;
        if name.is_empty() {
            return None;
        }
        if cfg!(feature = "faithful") {
            for c in name.chars() {
                TOKEN_CHARS.get(&c)?;
            }
        }
        let value = toks.next()?.trim_start_matches(|c| c == SP || c == HT);
        if cfg!(feature = "faithful") && value.chars().any(is_ctl) {
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
/// ```no_run
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
    pub(crate) fn parse(request: String) -> Option<Self> {
        let toks = Spliterator::new(request.as_bytes(), B_CRLF);
        let mut headers: HashMap<String, String> = HashMap::new();
        #[cfg(feature = "raw_headers")]
        let mut raw_headers: Vec<Header> = Vec::new();
        for tok in toks {
            let parsed = Header::parse(match str::from_utf8(tok) {
                Ok(s) => s.to_owned(),
                Err(_) => return None,
            })?;
            headers
                .entry(parsed.name.to_ascii_lowercase())
                .and_modify(|v| *v = format!("{}, {}", v, parsed.value))
                .or_insert_with(|| parsed.value.to_owned());
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
}

// Helper function for extracting some headers
pub(crate) fn parse_without_body(data: &str) -> Option<(RequestLine, Headers)> {
    let n = data.find("\r\n")?;
    let (line, rest) = data.split_at(n);
    let request_line = RequestLine::parse(line)?;
    let headers = Headers::parse((&rest[2..]).to_owned())?;
    Some((request_line, headers))
}

/// Represents a single request
///
/// # Example
///
/// ```no_run
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
    pub cookies: Cookies,
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
        let cookies: Cookies;
        #[cfg(feature = "cookies")]
        if let Some(v) = headers.get("cookie") {
            cookies = Cookies::parse(v);
        } else {
            cookies = Default::default();
        }
        Some(Self {
            request_line,
            headers,
            #[cfg(feature = "cookies")]
            cookies,
            body,
        })
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
/// when you have the `req` variable in the closure.
/// It implements Deref to Request so you can use
/// Request methods/properties directly on it
///
/// # Example
///
/// ```no_run
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
    /// ```no_run
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
}
