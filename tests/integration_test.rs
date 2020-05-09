extern crate rusty_ws;
use rusty_ws::http;

mod common;

#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn request_line_success() {
    let req = http::Request::new("POST /abc/def HTTP/1.1");
    assert_eq!(req.method, http::RequestMethod::Post);
    assert_eq!(req.path, "/abc/def");
    assert_eq!(req.version, "1.1");
}

#[test]
#[should_panic]
fn request_line_fail_1() {
    http::Request::new("POST /abc/def HTTP/1.1 x");
}

#[test]
#[should_panic]
fn request_line_fail_2() {
    http::Request::new("POST /abc/def HTDP/1.1");
}

#[test]
#[should_panic]
fn request_line_fail_3() {
    http::Request::new("POST /abc/def");
}
