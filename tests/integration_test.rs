extern crate rusty_ws;
use rusty_ws::http;

mod common;

#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn request_line_success() {
    // Parsing should work as expected.
    let req = http::RequestLine::parse("POST /abc/def HTTP/1.1").unwrap();
    assert_eq!(req.method, http::RequestMethod::Post);
    assert_eq!(req.path, "/abc/def");
    assert_eq!(req.version, "1.1");
}

#[test]
fn request_line_success_other_method() {
    // Non-documented methods should also work.
    let req = http::RequestLine::parse("PATCH /abc/def HTTP/1.1").unwrap();
    assert_eq!(req.method, http::RequestMethod::Other("PATCH"));
    assert_eq!(req.path, "/abc/def");
    assert_eq!(req.version, "1.1");
}

#[test]
#[should_panic]
fn request_line_fail_extra_1() {
    // Extra clauses should error.
    http::RequestLine::parse("POST /abc/def HTTP/1.1 x").unwrap();
}

#[test]
#[should_panic]
fn request_line_fail_extra_2() {
    // Extra clauses should error.
    http::RequestLine::parse("POST /a /b HTTP/1.1").unwrap();
}

#[test]
#[should_panic]
fn request_line_fail_malformed_version() {
    // Malformed versions should error.
    http::RequestLine::parse("POST /abc/def HTDP/1.1").unwrap();
}

#[test]
#[should_panic]
fn request_line_fail_missing_clause() {
    // Missing clauses should error.
    http::RequestLine::parse("POST /abc/def").unwrap();
}

#[test]
fn header_success() {
    // Parsing should work as expected.
    let req = http::Header::parse("Referer: \t\t http://www.example.com/").unwrap();
    assert_eq!(req.name, "Referer");
    assert_eq!(req.value, "http://www.example.com/");
}

#[test]
fn header_success_empty_value() {
    // Empty values are allowed.
    let req = http::Header::parse("Referer: \t\t ").unwrap();
    assert_eq!(req.name, "Referer");
    assert_eq!(req.value, "");
}

#[test]
#[should_panic]
fn header_fail_no_value() {
    // Having no value should fail.
    http::Header::parse("Referer").unwrap();
}

#[test]
#[should_panic]
fn header_fail_empty_name() {
    // Having no name should fail.
    http::Header::parse(": test").unwrap();
}

#[test]
#[should_panic]
fn header_fail_malformed_name() {
    // Having separators in the name should fail.
    http::Header::parse("Test Header: test").unwrap();
}