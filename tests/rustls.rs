#![cfg(feature = "rustls")]
use curl::easy::Easy;
use octane::prelude::*;
mod common;

#[test]
pub fn basic_https_hello_world_rustls() {
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
        let mut easy = Easy::new();
        easy.url("https://0.0.0.0:8000/").unwrap();
        easy.ssl_verify_peer(false).unwrap();
        easy.ssl_verify_host(false).unwrap();
        let mut transfer = easy.transfer();
        transfer
            .write_function(|data| {
                assert_eq!(data, string.as_bytes());
                Ok(data.len())
            })
            .unwrap();
        transfer.perform().unwrap();
    })
}
