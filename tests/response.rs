use octane::constants::StatusCode;
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
    let req = Response::new(b"").status(StatusCode::Created).get_data();

    assert_eq!(
        String::from_utf8(req).unwrap(),
        "HTTP/1.1 201 CREATED\r\n\r\n"
    );
}

#[test]
fn response_with_different_http_version() {
    // Reponse with different status codes should work
    let req = Response::new(b"")
        .with_http_version("x.y")
        .status(StatusCode::Created)
        .get_data();
    assert_eq!(
        String::from_utf8(req).unwrap(),
        "HTTP/x.y 201 CREATED\r\n\r\n"
    );
}
