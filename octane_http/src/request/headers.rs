use std::convert::TryFrom;

use crate::request::{ReqResult, RequestError};
use crate::stream::StreamHelper;
use crate::{CR, LF};

use http::header::{HeaderMap, HeaderName, HeaderValue};
use http::status::StatusCode;

fn parse_header(mut stream: &[u8]) -> ReqResult<(HeaderName, HeaderValue)> {
    let mut helper = StreamHelper { stream };

    let name_traverse = helper.get_till(&b':')?;

    let header_name = HeaderName::try_from(StreamHelper::trim(name_traverse.0))
        .map_err(|_| RequestError::StatusCodeErr(StatusCode::BAD_REQUEST))?;

    let value_traverse = helper.get_till(&CR)?;

    let header_value = HeaderValue::try_from(StreamHelper::trim(value_traverse.0))
        .map_err(|_| RequestError::StatusCodeErr(StatusCode::BAD_REQUEST))?;

    let _finish = helper.get_till(&b'\n')?;

    Ok((header_name, header_value))
}

fn parse_headers(stream: &[u8]) -> ReqResult<HeaderMap> {
    let mut peekable = stream.iter().peekable();
    let mut map = HeaderMap::new();
    let mut helper = StreamHelper { stream };

    while let Some(byte) = peekable.next() {
        if byte == &CR && peekable.next_if(|&x| x == &LF).is_some() {
            // This could be either the end of an header decleration
            // or it can be the start of the body
            // First let's check if it's the end of headers
            if peekable.next_if(|&x| x == &CR).is_some()
                && peekable.next_if(|&x| x == &LF).is_some()
            {
                // body starts
                return Ok(map);
            } else {
                println!("{:?}", std::str::from_utf8(stream).unwrap());
                let mut header_key_value = helper.get_till(&CR)?;
                let parsed = parse_header(header_key_value.0)?;

                map.insert(parsed.0, parsed.1);
            }
        }
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_individual_header() {
        let header = " Content-Length : 420 ";
        let parsed = parse_header(&mut header.as_bytes()).unwrap();
        assert_eq!(parsed.0, HeaderName::from_static("Content-Length"));
        assert_eq!(parsed.1, HeaderValue::from_static("420"));
    }
}
