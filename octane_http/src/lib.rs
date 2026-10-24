//! The HTTP/1.x protocol core of Octane.
//!
//! The crate is `#![no_std]` and never allocates: requests are parsed in
//! place out of a caller-provided buffer and responses are encoded into one.
//! Only the transport differs between platforms, so the same code serves on
//! Linux, WASI or a bare-metal TCP stack. See [`config`] for sizing the
//! buffers, [`server`] for the serve loop and [`server::Connection`] for the
//! transport trait.
//!
//! The `wasm` feature adds the `wasip3` module on `wasm32-wasip3`: a
//! server over `wasi:sockets` that creates the listener and serves each
//! connection as a concurrent task.
//!
//! The `std` feature (on by default) adds the tokio based
//! [`http1x::Http1xReader`] used by the `octane` server and
//! [`server::StdIo`], an adapter for any `std::io::Read + Write` stream.
#![no_std]

#[cfg(any(feature = "std", test))]
extern crate std;

#[cfg(all(feature = "wasm", target_os = "wasi", target_env = "p3"))]
extern crate alloc;

use core::fmt;
use octane_macros::status_codes;

pub const SP: char = ' ';
pub const HT: char = '\t';
pub const CR: char = '\r';
pub const LF: char = '\n';
pub const CRLF: &[u8; 2] = b"\r\n";
pub const DOUBLE_CRLF: &[u8; 4] = b"\r\n\r\n";

pub mod config;
mod eater;
pub mod http1x;
pub mod response;
pub mod server;
#[cfg(all(feature = "wasm", target_os = "wasi", target_env = "p3"))]
pub mod wasip3;
/// Holds the http versions. You can match the
/// variants by doing a comparison with the version
/// in the request_line
///
/// # Example
///
/// ```ignore
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
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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

impl HttpVersion {
    /// The version number as it appears after `HTTP/`, empty for
    /// [`HttpVersion::HttpInvalid`]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http11 => "1.1",
            Self::Http10 => "1.0",
            Self::Http09 => "0.9",
            Self::Http02 => "0.2",
            _ => "",
        }
    }
}

impl fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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
