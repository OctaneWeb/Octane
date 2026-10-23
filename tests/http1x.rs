#![cfg(feature = "cookies")]
use octane::cookie::Cookie;
use octane::prelude::*;
use std::fs;
mod common;

const PORT: u16 = 8088;

fn app() -> Octane {
    let mut app = Octane::new();
    app.settings.max_body_size = 16;
    app.get("/", route_next!(|req, res| res.send("ok")))
        .unwrap();
    app.post("/echo", route_next!(|req, res| res.send(req.body)))
        .unwrap();
    app.get(
        "/query",
        route_next!(|req, res| {
            res.send(req.get_query().get("q").cloned().unwrap_or_default());
        }),
    )
    .unwrap();
    app.get(
        "/redirect",
        route_next!(|req, res| {
            let next = req.get_query().get("next").cloned().unwrap_or_default();
            res.redirect(&next).send("");
        }),
    )
    .unwrap();
    app.get(
        "/cookies",
        route_next!(|req, res| {
            let pairs: Vec<String> = req
                .cookies
                .iter()
                .map(|c| format!("{}={}", c.name(), c.value()))
                .collect();
            res.cookie(Cookie::build("sid", "1").http_only(true))
                .send(pairs.join(","));
        }),
    )
    .unwrap();
    app.get(
        "/file",
        route_next!(|req, res| {
            res.send_file("templates/test.html").unwrap();
        }),
    )
    .unwrap();
    app
}

#[test]
pub fn http1x_regressions() {
    common::run(app(), PORT, || {
        // these used to panic: a path above the root, and versions that are
        // too short or split inside a character, which only `faithful` rejects
        let above_root = b"GET /.. HTTP/1.1\r\nHost: a\r\n\r\n";
        assert_eq!(400, common::raw_request(PORT, above_root).status);
        let versions: [&[u8]; 2] = [b"GET / X\r\n\r\n", "GET / HTTP\u{e9}1.1\r\n\r\n".as_bytes()];
        for raw in versions.iter() {
            let status = common::raw_request(PORT, raw).status;
            assert!(status == 400 || !cfg!(feature = "faithful"));
        }

        // a request doesn't need any header lines
        let res = common::raw_request(PORT, b"GET / HTTP/1.0\r\n\r\n");
        assert_eq!((200, "ok"), (res.status, res.text()));

        // the body length is checked before reading the body
        let too_long = ["Content-Length: 99999999999999999999"];
        assert_eq!(
            413,
            common::request(PORT, "POST", "/echo", &too_long).status
        );
        let conflicting = ["Content-Length: 1", "Content-Length: 2"];
        assert_eq!(
            400,
            common::request(PORT, "POST", "/echo", &conflicting).status
        );
        let signed = ["Content-Length: +1"];
        assert_eq!(400, common::request(PORT, "POST", "/echo", &signed).status);

        // bytes past Content-Length aren't part of the body
        let raw = b"POST /echo HTTP/1.1\r\nHost: a\r\nContent-Length: 2\r\n\r\nhello";
        assert_eq!("he", common::raw_request(PORT, raw).text());

        // the query isn't part of the routed path and decodes as UTF-8
        let res = common::request(PORT, "GET", "/query?q=caf%C3%A9+au+lait", &[]);
        assert_eq!("café au lait", res.text());

        // CR and LF in a value can't add response headers
        let res = common::request(PORT, "GET", "/redirect?next=/x%0d%0aX-Injected:%20yes", &[]);
        assert_eq!(302, res.status);
        assert!(!res.head.contains("\r\nX-Injected"));

        // malformed cookie pairs are skipped and Set-Cookie ends its line
        let res = common::request(PORT, "GET", "/cookies", &["Cookie: foo; =v; a=1;b=2"]);
        assert!(res.head.contains("\r\nSet-Cookie: sid=1; HttpOnly"));
        assert_eq!("a=1,b=2", res.text());

        // files are sent in full
        let res = common::request(PORT, "GET", "/file", &[]);
        assert_eq!(fs::read("templates/test.html").unwrap(), res.body);
    });
}
