#![cfg(feature = "std")]
use octane::prelude::*;
mod common;

const PORT: u16 = 8098;

#[test]
pub fn form_bodies_are_decoded() {
    let mut app = Octane::new();
    app.post(
        "/sign",
        route_stop!(|req, res| {
            let form = req.get_form();
            let name = form.get("name").unwrap_or("");
            let note = form.get("note").unwrap_or("");
            res.send(format!("{}|{}", name, note));
        }),
    )
    .unwrap();
    common::run(app, PORT, || {
        let body = "name=Ada+Lovelace&note=1%2B1%3D2+%F0%9F%A6%80";
        let request = format!(
            "POST /sign HTTP/1.1\r\nHost: localhost\r\n\
             Content-Type: application/x-www-form-urlencoded\r\n\
             Content-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let res = common::raw_request(PORT, request.as_bytes());
        assert_eq!("Ada Lovelace|1+1=2 \u{1F980}", res.text());
    });
}
