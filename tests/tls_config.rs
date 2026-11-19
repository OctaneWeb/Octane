#![cfg(any(feature = "rustls", feature = "openSSL"))]
//! TLS problems are returned by `listen`, and TLS that isn't set up
//! doesn't start.
use octane::prelude::*;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::timeout;
mod common;

// `listen` with `app`, which should fail right away
fn listen_error(app: Octane, port: u16) -> String {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        let listening = timeout(Duration::from_secs(5), app.listen(port, || {}));
        match listening.await {
            Ok(Err(error)) => error.to_string(),
            Ok(Ok(())) => panic!("listen returned without an error"),
            Err(_) => panic!("listen started serving instead of failing"),
        }
    })
}

#[test]
pub fn missing_certificate_is_an_error() {
    let mut app = Octane::new();
    app.ssl(8445)
        .key("templates/does-not-exist.pem")
        .cert("templates/does-not-exist.pem");
    let error = listen_error(app, 8099);
    assert!(
        error.contains("can't load the TLS certificate"),
        "{}",
        error
    );
    // nothing was left listening
    assert!(std::net::TcpStream::connect(("127.0.0.1", 8099)).is_err());
}

#[test]
pub fn half_configured_tls_is_an_error() {
    let mut app = Octane::new();
    app.ssl(8446).key("templates/key.pem");
    let error = listen_error(app, 8100);
    assert!(error.contains("both a key and a certificate"), "{}", error);
}

#[test]
pub fn tls_without_a_certificate_is_off() {
    let mut app = Octane::new();
    app.settings.ssl.port = 8447;
    app.get("/", route_stop!(|req, res| res.send("plain")))
        .unwrap();
    // with the TLS feature on but no key or certificate, only HTTP is served
    common::run(app, 8101, || {
        assert_eq!("plain", common::request(8101, "GET", "/", &[]).text());
        assert!(std::net::TcpStream::connect(("127.0.0.1", 8447)).is_err());
    });
}
