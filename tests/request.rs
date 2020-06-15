use octane::request::*;

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let reqline = RequestLine::parse("POST /abc/def HTTP/1.1").unwrap();
    assert_eq!(reqline.method, RequestMethod::Post);
    assert_eq!(reqline.path, "/abc/def");
    assert_eq!(reqline.version, HttpVersion::Http11);
    let headers = Headers::parse(
        "Host: localhost:12345\r\n\
        User-Agent: curl/7.58.0\r\n\
        Accept: */*\r\n\
        Content-Length: 20\r\n\
        Content-Type: application/x-www-form-urlencoded",
    ).unwrap();
    assert_eq!(
        headers.get("host").unwrap(),
        "localhost:12345"
    );
    assert_eq!(
        headers.get("user-agent").unwrap(),
        "curl/7.58.0"
    );
    assert_eq!(headers.get("accept").unwrap(), "*/*");
    assert_eq!(
        headers.get("content-length").unwrap(),
        "20"
    );
    assert_eq!(
        headers.get("content-type").unwrap(),
        "application/x-www-form-urlencoded"
    );
}

#[test]
#[cfg(feature = "raw_headers")]
fn success_raw_headers() {
    // Parsing should work as expected.
    let headers = Headers::parse(
        "HOst: localhost:12345\r\n\
        User-Agent: curl/7.58.0"
    )
    .unwrap();
    assert_eq!(headers.raw[0].name, "HOst");
    assert_eq!(headers.raw[0].value, "localhost:12345");
    assert_eq!(headers.raw[1].name, "User-Agent");
    assert_eq!(headers.raw[1].value, "curl/7.58.0");
}
