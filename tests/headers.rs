#![cfg(feature = "std")]
use octane::prelude::*;

mod common;

#[test]
pub fn basic_header_value_retrival() {
    let mut app = Octane::new();
    let string = "My Rust Program";
    app.get(
        "/",
        route_next!(|req, res| {
            res.send(req.headers.get("user-agent").unwrap());
        }),
    )
    .unwrap();
    common::run(app, 8083, || {
        let res = common::request(8083, "GET", "/", &["User-Agent: My Rust Program"]);
        assert_eq!(string, res.text());
    });
}
