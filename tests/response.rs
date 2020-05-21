extern crate octane;
use octane::responder::Response;
use octane::constants::StatusCode;

#[test]
fn success_standard() {
    // default response should provide OK 200 Code
    let req = Response::new(b"").get_string();
    assert_eq!(String::from_utf8(req).unwrap(), "HTTP/1.1 200 OK\r\n\r\n");
}

#[test]
fn response_with_status_code_different() {
    // Reponse with different status codes should work
    let req = Response::new(b"")
        .with_status(StatusCode::Created)
        .get_string();
    
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
        .with_status(StatusCode::Created)
        .get_string();
    assert_eq!(
        String::from_utf8(req).unwrap(),
        "HTTP/x.y 201 CREATED\r\n\r\n"
    );
}
