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
    common::run(app, || async {
        let client = reqwest::Client::new();
        assert_eq!(
            string,
            client
                .get(&path!(""))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .post(&path!(""))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .put(&path!(""))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .patch(&path!(""))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .delete(&path!(""))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
        assert_eq!(
            string,
            client
                .head(&path!(""))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
    });
}
