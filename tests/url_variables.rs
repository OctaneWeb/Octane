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
            res.send("test");
            assert_eq!(value, *req.vars.get("var").unwrap());
        }),
    )
    .unwrap();
    common::run(app, || async {
        common::client_request(&path!(format!("foo/{}", value))).await;
    });
}
