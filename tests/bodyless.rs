#![cfg(feature = "std")]
//! A route that answers without a body still answers the request.
use octane::prelude::*;
use octane::StatusCode;
mod common;

const PORT: u16 = 8096;

#[test]
pub fn responses_without_a_body() {
    let mut app = Octane::new();
    // middleware that stops answers, one that passes doesn't
    app.add(route!(|req, res| {
        if req.headers.contains("x-block") {
            res.status(StatusCode::Unauthorized);
            Flow::Stop
        } else {
            res.set("X-Seen", "yes");
            Flow::Next
        }
    }))
    .unwrap();
    app.get(
        "/redirect",
        route_stop!(|req, res| {
            res.redirect("/");
        }),
    )
    .unwrap();
    app.get(
        "/next-redirect",
        route_next!(|req, res| {
            res.redirect("/");
        }),
    )
    .unwrap();
    app.delete(
        "/item",
        route_stop!(|req, res| {
            res.status(StatusCode::NoContent);
        }),
    )
    .unwrap();

    common::run(app, PORT, || {
        let res = common::request(PORT, "GET", "/redirect", &[]);
        assert_eq!(302, res.status);
        assert_eq!(Some("/"), res.header("location"));
        assert_eq!(Some("0"), res.header("content-length"));
        assert!(res.body.is_empty());

        // a route that ran answers even when it passes the request on
        let res = common::request(PORT, "GET", "/next-redirect", &[]);
        assert_eq!(302, res.status);

        let res = common::request(PORT, "DELETE", "/item", &[]);
        assert_eq!(204, res.status);
        assert_eq!(None, res.header("content-length"));

        let res = common::request(PORT, "GET", "/missing", &["X-Block: 1"]);
        assert_eq!(401, res.status);

        // only middleware ran: still nothing at this path
        let res = common::request(PORT, "GET", "/missing", &[]);
        assert_eq!(404, res.status);
        let res = common::request(PORT, "POST", "/redirect", &[]);
        assert_eq!(405, res.status);
        assert_eq!(Some("GET"), res.header("allow"));
    });
}
