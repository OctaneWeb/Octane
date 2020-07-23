use octane::constants::StatusCode;
use octane::request::HttpVersion;
use octane::responder::Response;

#[test]
fn success_standard() {
    // default response should provide OK 200 Code
    let req = Response::new(b"").get_data();
    assert_eq!(String::from_utf8(req).unwrap(), "HTTP/1.1 200 OK\r\n\r\n");
}

#[test]
fn response_with_status_code_different() {
    // Reponse with different status codes should work
    let mut req = Response::new(b"");
    req.status(StatusCode::Created);

    assert_eq!(
        String::from_utf8(req.get_data()).unwrap(),
        "HTTP/1.1 201 CREATED\r\n\r\n"
    );
}

#[test]
fn response_with_different_http_version() {
    // Reponse with different status codes should work
    let mut req = Response::new(b"");

    req.http_version(HttpVersion::Http10)
        .status(StatusCode::Created);
    assert_eq!(
        String::from_utf8(req.get_data()).unwrap(),
        "HTTP/1.0 201 CREATED\r\n\r\n"
    );
}
