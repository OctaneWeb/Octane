use octane::prelude::*;
mod common;

#[test]
pub fn basic_server_hello() {
    let mut app = Octane::new();
    app.get(
        "/",
        route_next!(|req, res| {
            res.send("Hello, World");
        }),
    )
    .unwrap();
    common::run(app, 8081, || {
        assert_eq!(
            "Hello, World",
            common::request(8081, "GET", "/", &[]).text()
        );
    });
}

#[test]
pub fn basic_server_x_req() {
    let mut app = Octane::new();
    let string = "Hello, World";
    app.get(
        "/",
        route_next!(|req, res| {
            res.send(string);
        }),
    )
    .unwrap();
    app.post(
        "/",
        route_next!(|req, res| {
            res.send(string);
        }),
    )
    .unwrap();
    app.put(
        "/",
        route_next!(|req, res| {
            res.send(string);
        }),
    )
    .unwrap();
    app.patch(
        "/",
        route_next!(|req, res| {
            res.send(string);
        }),
    )
    .unwrap();
    app.head(
        "/",
        route_next!(|req, res| {
            res.send(string);
        }),
    )
    .unwrap();
    app.delete(
        "/",
        route_next!(|req, res| {
            res.send(string);
        }),
    )
    .unwrap();
    common::run(app, 8082, || {
        for method in &["GET", "POST", "PUT", "PATCH", "DELETE"] {
            assert_eq!(string, common::request(8082, method, "/", &[]).text());
        }
        // TODO: HEAD responses still carry the body, they must not
        assert_eq!(200, common::request(8082, "HEAD", "/", &[]).status);
    });
}
