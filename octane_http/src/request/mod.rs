use crate::request::request_line::parse_request_line;

use std::io;

use bytes::BytesMut;
use http::method::InvalidMethod;
use http::request::Request as HttpRequest;
use http::status::StatusCode;
use http::uri::InvalidUri;
use http::Error;
use tokio_util::codec::Decoder;

mod headers;
mod request_line;

pub type ReqResult<T> = Result<T, RequestError>;

#[derive(Debug)]
pub enum RequestError {
    StatusCodeErr(StatusCode),
    ParsingErr(Error),
}

struct RawRequest<T> {
    req: HttpRequest<T>,
}

impl<T> RawRequest<T> {}

impl From<StatusCode> for RequestError {
    fn from(code: StatusCode) -> Self {
        RequestError::StatusCodeErr(code)
    }
}

impl From<Error> for RequestError {
    fn from(err: Error) -> Self {
        RequestError::ParsingErr(err)
    }
}

impl From<InvalidUri> for RequestError {
    fn from(err: InvalidUri) -> Self {
        RequestError::ParsingErr(Error::from(err))
    }
}

impl From<InvalidMethod> for RequestError {
    fn from(err: InvalidMethod) -> Self {
        RequestError::ParsingErr(Error::from(err))
    }
}

// impl<T> Decoder for RawRequest<T> {
//     type Item = HttpRequest<T>;
//     type Error = io::Error;

//     fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
//         let request_line = parse_request_line(src);

//         Ok(Some())
//     }
// }
