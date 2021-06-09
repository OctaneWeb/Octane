use octane_macros::status_codes;
use std::fmt;

mod con;
pub mod http1x;
/// Holds the http versions. You can match the
/// variants by doing a comparison with the version
/// in the request_line
///
/// # Example
///
/// ```
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

status_codes! {
    100 "Continue"
    101 "Switching Protocol"
    102 "Processing"
    103 "Early Hints"
    200 "OK"
    201 "Created"
    202 "Accepted"
    203 "Non-Authoritative Information"
    204 "No Content"
    205 "Reset Content"
    206 "Partial Content"
    207 "Multi-Status"
    208 "Already Reported"
    226 "IM Used"
    300 "Multiple Choice"
    301 "Moved Permanently"
    302 "Found"
    303 "See Other"
    304 "Not Modified"
    307 "Temporary Redirect"
    308 "Permanent Redirect"
    400 "Bad Request"
    401 "Unauthorized"
    402 "Payment Required"
    403 "Forbidden"
    404 "Not Found"
    405 "Method Not Allowed"
    406 "Not Acceptable"
    407 "Proxy Authentication Required"
    408 "Request Timeout"
    409 "Conflict"
    410 "Gone"
    411 "Length Required"
    412 "Precondition Failed"
    413 "Payload Too Large"
    414 "URI Too Long"
    415 "Unsupported Media Type"
    416 "Range Not Satisfiable"
    417 "Expectation Failed"
    418 "I'm a teapot"
    421 "Misdirected Request"
    422 "Unprocessable Entity"
    423 "Locked"
    424 "Failed Dependency"
    425 "Too Early"
    426 "Upgrade Required"
    428 "Precondition Required"
    429 "Too Many Requests"
    431 "Request Header Fields Too Large"
    451 "Unavailable For Legal Reasons"
    500 "Internal Server Error"
    501 "Not Implemented"
    502 "Bad Gateway"
    503 "Service Unavailable"
    504 "Gateway Timeout"
    505 "HTTP Version Not Supported"
    506 "Variant Also Negotiates"
    507 "Insufficient Storage"
    508 "Loop Detected"
    510 "Not Extended"
    511 "Network Authentication Required"
}

impl Into<i32> for StatusCode {
    fn into(self) -> i32 {
        let (n, _) = self.fetch();
        n
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (_, s) = self.fetch();
        write!(f, "{}", s)
    }
}
