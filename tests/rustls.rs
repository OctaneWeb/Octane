#![cfg(feature = "rustls")]
use octane::prelude::*;
mod common;

#[test]
pub fn basic_https_hello_world_rustls() {
    let mut app = Octane::new();
    app.ssl(8443)
        .key("templates/svr.pem")
        .cert("templates/cert.pem");
    let string = "Hello, World";
    app.get(
        "/",
        route_next!(|req, res| {
            res.send(string);
        }),
    )
    .unwrap();
    common::run(app, 8086, || {
        common::wait_for(8443);
        assert_eq!(string, common::https_get(8443, "/"));
    })
}
