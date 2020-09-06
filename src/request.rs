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

/// Holds the type of request method, like GET
/// POST etc.
#[derive(Debug, PartialEq, Eq, Clone, Hash, Copy)]
pub enum RequestMethod {
    Options,
    Get,
    Head,
    Post,
    Put,
    Delete,
    Trace,
    Connect,
    All,
    None,
}

impl RequestMethod {
    /// Get all the values of the enum in an
    /// array. Useful for iterating and populating
    /// HashMap that holds closures for different
    /// different methods.
    pub fn values() -> [Self; 10] {
        use RequestMethod::*;
        [
            Options, Get, Head, Post, Put, Delete, Trace, Connect, All, None,
        ]
    }
    /// Return false if the RequestMethod has the
    /// variant `None` else return true
    pub fn is_some(&self) -> bool {
        if let Self::None = self {
            false
        } else {
            true
        }
    }
}
/// Holds the http versions you can match the
/// variants by doing a comparison with the version
/// in the request_line
///
/// # Example
///
/// ```no_run
/// use octane::Octane;
/// use octane::{route, router::{Flow, Route}};
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
    Http11,
    Http10,
    Http02,
    Http09,
    HttpInvalid,
}

impl HttpVersion {
    /// Returns the version in string like "1.1"
    /// or "1.0" etc
    pub fn get_version_string(self) -> String {
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
/// use octane::Octane;
/// use octane::{route, router::{Flow, Route}};
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
    pub fn parse(request_line: &str) -> Option<Self> {
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
/// and holds the key and value both. You
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
    /// Parses the `key: value` header unit
    /// str and returns a Header struct
    pub fn parse(header: String) -> Option<Self> {
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
/// use octane::Octane;
/// use octane::{route, router::{Flow, Route}};
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
    pub fn parse(request: String) -> Option<Self> {
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

/// Helper function for extracting some headers
pub fn parse_without_body(data: &str) -> Option<(RequestLine, Headers)> {
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
/// use octane::Octane;
/// use octane::{route, router::{Flow, Route}};
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
    /// The requestline is the first line of the http request, it has the version,
    /// request method, request_uri.
    pub request_line: RequestLine,
    /// Headers in a request
    pub headers: Headers,
    /// The body of the request
    pub body: &'a [u8],
    #[cfg(feature = "cookies")]
    /// Cookies in a request
    pub cookies: Cookies,
}

impl<'a> Request<'a> {
    /// Parse a Request with request_line, headers
    /// and body and return a Request struct
    pub fn parse(request_line: RequestLine, headers: Headers, body: &'a [u8]) -> Option<Self> {
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
/// timeout and max duration as a u64, only http 1.0 and below
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct KeepAlive {
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
    /// Returns the ammount of timeout specified in the keep alive header
    pub fn timeout(&self) -> Option<u64> {
        self.timeout
    }
    /// Returns the ammount of max requests specified in the keep alive header
    pub fn max(&self) -> Option<u64> {
        self.max
    }
}

/// The MatchedRequest is the struct which you see
/// when you have the `req` variable in the closure
/// It implements Deref to Request so you can use
/// Request methods/properties directly on it
///
/// # Example
///
/// ```no_run
/// use octane::Octane;
/// use octane::{route, router::{Flow, Route}};
/// use octane::request::RequestMethod;
///
/// let mut app = Octane::new();
/// app
/// .get("/",
///     route!(|req, res| {
///         // The req here is not actually a
///         // Request but a MatchedRequest which
///         // implements deref to Request.
///         // You can just directly use Request
///         // methods on it
///         let header = req.headers.get("Some-Header");
///         Flow::Stop
///     }),
/// );
/// ```
/// The struct also have the values of the url
/// variables
/// TODO: Add a url variable example
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MatchedRequest<'a> {
    /// The request comming from a client
    pub request: Request<'a>,
    #[cfg(feature = "url_variables")]
    /// A Hashmap containing the variables specified in the url with their respective keys
    pub vars: HashMap<&'a str, &'a str>,
}

deref!(MatchedRequest<'a>, Request<'a>, request);
deref!(Headers, HashMap<String, String>, parsed);
