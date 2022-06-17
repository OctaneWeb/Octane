use crate::response::Body;

use crate::{CRLF, SP};

use http::response::Response;
use http::Version;
use log::error;
use tokio::io::AsyncReadExt;

pub async fn serialise_res(res: Response<Body>) -> String {
    let status_line = serialise_status_line(&res);

    let mut headers = serialise_header(&res);

    headers.push_str(CRLF);

    let mut body = String::new();

    let mut reader = res.into_body().get_reader();

    reader
        .read_to_string(&mut body)
        .await
        .map_err(|e| error!("Body reading error: {:?}", e));

    format!("{}{}{}", status_line, headers, body)
}

pub fn serialise_status_line(res: &Response<Body>) -> String {
    let mut status_line = String::with_capacity(45);

    status_line.push_str(version_to_string(res.version()));
    status_line.push(SP as char);
    status_line.push_str(res.status().as_str());
    status_line.push(SP as char);
    status_line.push_str(res.status().canonical_reason().unwrap());
    status_line.push_str(CRLF);
    status_line
}

pub fn serialise_header(res: &Response<Body>) -> String {
    let mut headers = String::new();

    for header in res.headers().iter() {
        headers.push_str(header.0.as_str());
        headers.push(':');
        headers.push_str(header.1.to_str().expect("Add error checking here"));
        headers.push_str(CRLF);
    }
    headers
}

pub fn version_to_string(version: Version) -> &'static str {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2",
        Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/1.1",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BoxReader;

    use http::header::{HOST, LOCATION};
    use http::response::Response;
    use http::status::StatusCode;
    use http::version::Version;
    use http::HeaderValue;

    use std::io::Cursor;

    #[test]
    fn normal_header_gen() {
        let mut res: Response<String> = Response::default();
        res.headers_mut()
            .insert(HOST, HeaderValue::from_static("world"));
        res.headers_mut()
            .insert(LOCATION, HeaderValue::from_static("hello"));

        let eq = "host:world\r\nlocation:hello\r\n";

        assert_eq!(
            eq,
            serialise_header(&Response::new(Body::Sized(
                eq.len(),
                Box::new(Cursor::new(eq.as_bytes())) as BoxReader
            )))
        );
    }

    #[test]
    fn normal_status_line_gen() {
        let mut res = Response::new("Body");
        *res.status_mut() = StatusCode::NON_AUTHORITATIVE_INFORMATION;
        *res.version_mut() = Version::HTTP_09;

        let eq = "HTTP/0.9 203 Non Authoritative Information\r\n";

        assert_eq!(
            eq,
            serialise_status_line(&Response::new(Body::Sized(
                eq.len(),
                Box::new(Cursor::new(eq.as_bytes())) as BoxReader
            )))
        );
    }

    #[test]
    async fn normal_response_gen() {
        let mut res = Response::new("Body");
        *res.status_mut() = StatusCode::OK;
        res.headers_mut()
            .insert(HOST, HeaderValue::from_static("world"));
        res.headers_mut()
            .insert(LOCATION, HeaderValue::from_static("hello"));

        let eq = "HTTP/1.1 200 OK\r\nhost:world\r\nlocation:hello\r\n\r\nBody";

        assert_eq!(
            eq,
            serialise_res(
                Response::new(Body::Sized(
                    eq.len(),
                    Box::new(Cursor::new(eq.as_bytes())) as BoxReader
                ))
                .await
            )
        );
    }
}
