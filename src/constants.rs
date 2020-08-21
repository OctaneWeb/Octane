use crate::middlewares::Closures;
use crate::path::PathNode;
use crate::request::RequestMethod;
use octane_macros::status_codes;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::iter::FromIterator;
use std::sync::Mutex;

pub const SP: char = ' ';
pub const HT: char = '\t';
pub const CR: char = '\r';
pub const LF: char = '\n';
pub const CRLF: &str = "\r\n";
pub const B_CRLF: &[u8] = b"\r\n";
pub const WEEKS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
pub const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sept", "Oct", "Nov", "Dec",
];
pub const LEAPOCH: i64 = 946684800i64 + 86400 * (31 + 29);
pub const DAYS_PER_400Y: i64 = 365 * 400 + 97;
pub const DAYS_PER_100Y: i64 = 365 * 100 + 24;
pub const DAYS_PER_4Y: i64 = 365 * 4 + 1;
pub static DAYS_IN_MONTH: [i64; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];
pub const BUF_SIZE: usize = 512;
pub type Paths = HashMap<RequestMethod, PathNode<Closures>>;
// TOKEN_CHARS needs to be defined somehow
lazy_static! {
    pub static ref TOKEN_CHARS: HashSet<char> = HashSet::from_iter(
        "!#$%&'*+-.0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ^_`abcdefghijklmnopqrstuvwxyz|~".chars()
    );
    pub static ref CLOSURES: Mutex<Paths> = Mutex::new(HashMap::new());
}

pub fn with_lock<F, T>(f: F) -> T
where
    F: FnOnce(&mut HashMap<RequestMethod, PathNode<Closures>>) -> T,
{
    let mut lock = CLOSURES.lock().unwrap();
    f(&mut *lock)
}

pub fn is_ctl(c: char) -> bool {
    c < '\x1f' || c == '\x7f'
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

#[macro_export]
macro_rules! deref {
    ( $struct : ident<$($gen: tt),+>, $target : ty, $body : ident ) => {
        impl<$($gen),+> std::ops::Deref for $struct <$($gen),+> {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$body
            }
        }
    };
    ( $struct : ty, $target : ty, $body : ident ) => {
        impl std::ops::Deref for $struct {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$body
            }
        }
    };

}

#[macro_export]
macro_rules! default {
    ( $struct : ident<$($gen: tt),+> ) => {
        impl<$($gen),+> Default for $struct <$($gen),+> {
            fn default() -> Self {
                Self::new()
            }
        }
    };
    ( $struct : ty ) => {
        impl Default for $struct {
            fn default() -> Self {
                Self::new()
            }
        }
    };
    ( $struct : ty, $args : expr ) => {
        impl Default for $struct {
            fn default() -> Self {
                Self::new($args)
            }
        }
    };
}
