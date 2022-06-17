use http::Error;

type ReqResult<T> = Result<T, Error>;

// constants
const CR: u8 = 13;
const LF: u8 = 10;
const SP: u8 = 32;
const HT: u8 = 9;
const QUOTE: u8 = 34;
const CRLF: &str = "\r\n";

// Modules
mod proto;
// mod server;
mod stream;

pub mod config;
pub mod request;
// pub mod response;

pub use http::header::{AsHeaderName, IntoHeaderName};
pub use http::status::StatusCode;
pub use http::version::Version;
pub use http::HeaderValue;

// pub use crate::response::BoxReader;
