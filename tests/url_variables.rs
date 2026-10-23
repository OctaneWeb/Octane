#![cfg(feature = "url_variables")]
mod common;
use octane::prelude::*;

#[test]
pub fn basic_url_variables() {
    let mut app = Octane::new();
    let value = "bar";
    app.get(
        "/foo/:var",
        route_next!(|req, res| {
            res.send(req.vars.get("var").unwrap());
        }),
    )
    .unwrap();
    common::run(app, 8085, || {
        let res = common::request(8085, "GET", &format!("/foo/{}", value), &[]);
        assert_eq!(value, res.text());
    });
}
