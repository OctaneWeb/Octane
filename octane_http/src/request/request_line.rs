use crate::request::{ReqResult, RequestError};
use crate::stream::StreamHelper;
use crate::{CR, LF, SP};

use std::convert::TryFrom;

use http::method::Method;
use http::request::{Parts, Request};
use http::status::StatusCode;
use http::uri::Uri;
use http::version::Version;

fn parse_http_version(version: &[u8]) -> Result<Version, StatusCode> {
    match version {
        b"HTTP/0.9" => Ok(Version::HTTP_09),
        b"HTTP/1.0" => Ok(Version::HTTP_10),
        b"HTTP/1.1" => Ok(Version::HTTP_11),
        b"HTTP/2" => Ok(Version::HTTP_2),
        b"HTTP/3" => Ok(Version::HTTP_3),
        _ => Err(StatusCode::HTTP_VERSION_NOT_SUPPORTED),
    }
}

pub fn parse_request_line(stream: &[u8]) -> ReqResult<Parts> {
    let mut request = Request::new("");
    let mut helper = StreamHelper { stream };

    let method_traverse = helper.get_till(&SP)?;
    *request.method_mut() = Method::try_from(method_traverse.0)?;

    let uri_traverse = helper.get_till(&SP)?;
    *request.uri_mut() = Uri::try_from(uri_traverse.0)?;

    let version_traverse = helper.get_till(&CR)?;
    *request.version_mut() = parse_http_version(version_traverse.0)?;

    if stream.last() != Some(&LF) {
        return Err(RequestError::StatusCodeErr(StatusCode::BAD_REQUEST));
    }

    Ok(request.into_parts().0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CR, LF, SP};

    #[test]
    fn basic() {
        let stream = format!(
            "GET{}example.com{}HTTP/1.1{}{}",
            SP as char, SP as char, CR as char, LF as char
        );

        let parts = parse_request_line(&mut stream.as_bytes()).expect("Cannot parse request line");

        assert_eq!(parts.method, Method::GET);
        assert_eq!(parts.uri, "example.com".parse::<Uri>().unwrap());
        assert_eq!(parts.version, Version::HTTP_11);
    }

    #[test]
    #[should_panic]
    fn without_lf() {
        let stream = format!(
            "GET{}example.com{}HTTP/1.1{}",
            SP as char, SP as char, CR as char
        );

        parse_request_line(&mut stream.as_bytes()).expect("Cannot parse request line");
    }
}
