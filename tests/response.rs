extern crate octane;
use octane::responder;
use octane::time;
mod common;

#[test]
fn success_standard() {
    // default response should provide OK 200 Code
    let req = responder::Response::new("").get_string();
    assert_eq!(req, "HTTP/1.1 200 OK\r\n");
}

#[test]
fn response_with_status_code_different() {
    // Reponse with different status codes should work
    let req = responder::Response::new("")
        .with_status(responder::StatusCode::Created)
        .get_string();
    assert_eq!(req, "HTTP/1.1 201 CREATED\r\n");
}

#[test]
fn response_with_different_http_version() {
    // Reponse with different status codes should work
    let req = responder::Response::new("")
        .with_http_version("x.y")
        .with_status(responder::StatusCode::Created)
        .get_string();
    assert_eq!(req, "HTTP/x.y 201 CREATED\r\n");
}
