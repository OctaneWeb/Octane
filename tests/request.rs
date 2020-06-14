extern crate octane;
use octane::request::{HttpVersion, Request, RequestMethod};

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let req = Request::parse(
        "POST /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        Accept: */*\r\n\
        Content-Length: 20\r\n\
        Content-Type: application/x-www-form-urlencoded\r\n\
        \r\n\
        field1=a%2Fb&field2="
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(req.request_line.method, RequestMethod::Post);
    assert_eq!(req.request_line.path, "/abc/def");
    assert_eq!(req.request_line.version, HttpVersion::Http11);
    assert_eq!(
        *req.headers.get("host").unwrap(),
        "localhost:12345".to_string()
    );
    assert_eq!(
        *req.headers.get("user-agent").unwrap(),
        "curl/7.58.0".to_string()
    );
    assert_eq!(*req.headers.get("accept").unwrap(), "*/*".to_string());
    assert_eq!(
        *req.headers.get("content-length").unwrap(),
        "20".to_string()
    );
    assert_eq!(
        *req.headers.get("content-type").unwrap(),
        "application/x-www-form-urlencoded".to_string()
    );
    assert_eq!(req.body, b"field1=a%2Fb&field2=");
}

#[test]
#[cfg(feature = "cookies")]
fn success_cookies() {
    // Parsing should work as expected.
    let req = Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        Cookie: a=1; b=2\r\n\
        \r\n"
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(req.cookies["a"], "1".to_string());
    assert_eq!(req.cookies["b"], "2".to_string());
}

#[test]
#[cfg(feature = "cookies")]
fn success_cookies_no_header() {
    // Having no Cookie header should yield an empty hashmap.
    let req = Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        \r\n"
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(req.cookies.len(), 0);
}

#[test]
fn success_binary_body() {
    // Response body should be able to have binary data.
    let mut bod: Vec<u8> = b"POST /abc/def HTTP/1.1\r\n\
          Host: localhost:12345\r\n\
          \r\n"
        .to_vec();
    bod.extend(0..255);
    let req = Request::parse(&bod[..]).unwrap();
    assert_eq!(req.body, &(0..255).collect::<Vec<u8>>()[..]);
}

#[test]
fn success_no_body() {
    // Requests with no body should not have a body.
    let req = Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        \r\n"
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(req.body, b"");
}

#[test]
fn success_same_header() {
    // Requests with no body should not have a body.
    let req = Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        Header: a\r\n\
        Header: b\r\n\
        \r\n"
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(*req.headers.get("header").unwrap(), "a, b".to_string());
}

#[test]
#[cfg(feature = "raw_headers")]
fn success_raw_headers() {
    // Parsing should work as expected.
    let req = Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        HOst: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        \r\n"
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(req.raw_headers[0].name, "HOst");
    assert_eq!(req.raw_headers[0].value, "localhost:12345");
    assert_eq!(req.raw_headers[1].name, "User-Agent");
    assert_eq!(req.raw_headers[1].value, "curl/7.58.0");
}

#[test]
fn success_empty_lines() {
    // Parsing should ignore leading empty lines.
    Request::parse(
        "\r\nGET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        \r\n"
            .as_bytes(),
    )
    .unwrap();
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "faithful"), ignore)]
fn fail_no_empty_line() {
    // Parsing should require an empty line at the end.
    // Will complete when body parsing is added.
    Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n"
            .as_bytes(),
    )
    .unwrap();
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "faithful"), ignore)]
fn fail_no_ending_crlf() {
    // Parsing should require a crlf at the end of every header.
    // Will complete when body parsing is added.
    Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0"
            .as_bytes(),
    )
    .unwrap();
}
