extern crate octane;
use octane::http;

mod common;

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let req = http::Request::parse(
        "POST /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        Accept: */*\r\n\
        Content-Length: 20\r\n\
        Content-Type: application/x-www-form-urlencoded\r\n\
        \r\n\
        field1=a%2Fb&field2=",
    )
    .unwrap();
    assert_eq!(req.method, http::RequestMethod::Post);
    assert_eq!(req.path, "/abc/def");
    assert_eq!(req.version, "1.1");
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
    assert_eq!(req.body, "field1=a%2Fb&field2=");
}

#[test]
fn success_binary_body() {
    // Response body should be able to have binary data.
    let bin_vec = (0..255).collect::<Vec<u8>>();
    let bin = String::from_utf8_lossy(&bin_vec[..]);
    let bod = format!(
        "POST /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        \r\n\
        {}",
        bin
    );
    let req = http::Request::parse(&bod[..]).unwrap();
    assert_eq!(req.body, bin);
}

#[test]
fn success_no_body() {
    // Requests with no body should not have a body.
    let req = http::Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        \r\n",
    )
    .unwrap();
    assert_eq!(req.body, "");
}

#[test]
fn success_same_header() {
    // Requests with no body should not have a body.
    let req = http::Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        Header: a\r\n\
        Header: b\r\n\
        \r\n",
    )
    .unwrap();
    assert_eq!(*req.headers.get("header").unwrap(), "a, b".to_string());
}


#[test]
#[cfg(feature = "raw_headers")]
fn success_raw_headers() {
    // Parsing should work as expected.
    let req = http::Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        HOst: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        \r\n",
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
    http::Request::parse(
        "\r\nGET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        \r\n",
    )
    .unwrap();
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "faithful"), ignore)]
fn fail_no_empty_line() {
    // Parsing should require an empty line at the end.
    // Will complete when body parsing is added.
    http::Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n",
    )
    .unwrap();
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "faithful"), ignore)]
fn fail_no_ending_crlf() {
    // Parsing should require a crlf at the end of every header.
    // Will complete when body parsing is added.
    http::Request::parse(
        "GET /abc/def HTTP/1.1\r\n\
        Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0",
    )
    .unwrap();
}
