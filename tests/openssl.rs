#![cfg(feature = "openSSL")]
use octane::prelude::*;
use reqwest::ClientBuilder;
mod common;

#[test]
pub fn basic_https_hello_world_openssl() {
    let mut app = Octane::new();
    app.ssl(8000)
        .key("templates/key.pem")
        .cert("templates/cert.pem");
    let string = "Hello, World";
    app.get(
        "/",
        route_next!(|req, res| {
            res.send(string);
        }),
    )
    .unwrap();
    common::run(app, || async {
        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();
        assert_eq!(
            string,
            client
                .get("https://0.0.0.0:8000/")
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        );
    });
}
