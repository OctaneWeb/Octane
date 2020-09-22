use octane::prelude::*;
mod common;

pub fn basic_server_hello() {
    let mut app = Octane::new();
    app.get(
        "/",
        route_next!(|req, res| {
            res.send("Hello, World");
        }),
    )
    .unwrap();
    common::run(app, || async {
        assert_eq!("Hello, World", common::client_request(&path!("")).await);
    });
}
