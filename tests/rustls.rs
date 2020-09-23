#![cfg(feature = "rustls")]
use octane::prelude::*;
use reqwest::{Certificate, ClientBuilder};
use std::fs::File;
use std::io::Read;
use std::net::IpAddr;
mod common;

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
        let mut buf = Vec::new();
        File::open("templates/cert.pem")
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
        let cert = Certificate::from_pem(&buf).unwrap();
        let local_addr = IpAddr::from([0, 0, 0, 0]);
        let client = ClientBuilder::new()
            .local_address(local_addr)
            .use_native_tls()
            .no_proxy()
            .no_trust_dns()
            .add_root_certificate(cert)
            .danger_accept_invalid_certs(true)
            .danger_accept_invalid_hostnames(true)
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
    })
}
