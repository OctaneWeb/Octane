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

#[test]
pub fn url_variables_are_decoded() {
    const PORT: u16 = 8097;
    let mut app = Octane::new();
    app.get(
        "/users/:name",
        route_stop!(|req, res| {
            res.send(req.vars.get("name").unwrap());
        }),
    )
    .unwrap();
    app.get("/café", route_stop!(|req, res| res.send("coffee")))
        .unwrap();
    common::run(app, PORT, || {
        let res = common::request(PORT, "GET", "/users/Ada%20Lovelace", &[]);
        assert_eq!("Ada Lovelace", res.text());
        // `+` is only a space in queries and forms
        let res = common::request(PORT, "GET", "/users/a+b", &[]);
        assert_eq!("a+b", res.text());
        // an encoded slash stays in the variable
        let res = common::request(PORT, "GET", "/users/a%2Fb", &[]);
        assert_eq!("a/b", res.text());
        // literal routes match the decoded path too
        let res = common::request(PORT, "GET", "/caf%C3%A9", &[]);
        assert_eq!("coffee", res.text());
        // an encoded `..` is a dot segment, not a variable
        let res = common::request(PORT, "GET", "/users/x/%2e%2e/Ada", &[]);
        assert_eq!("Ada", res.text());
    });
}
