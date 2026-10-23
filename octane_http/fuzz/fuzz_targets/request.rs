//! End-to-end fuzz target: one connection through the whole HTTP/1.x stack.
//!
//! The first input byte picks how many bytes each read hands the server, so
//! heads and bodies arrive in arbitrary fragments, and the rest is what the
//! client sends. It goes through the head reader, the request-line, header,
//! `Content-Length`, query and cookie parsers, the router and the response
//! writer of a small app whose routes reflect request data (path, query,
//! cookies, headers, body) back into headers, cookies and bodies.
//!
//! Any panic is a crash. The server must always answer with exactly one well
//! framed response, with a body as long as its `Content-Length` and no header
//! line the app didn't set, whatever the request smuggled into the values.

#![no_main]

use libfuzzer_sys::fuzz_target;
use octane::cookie::Cookie;
use octane::fuzzing;
use octane::prelude::*;
use std::sync::{Arc, OnceLock};

// every header the server or the app below can send
const HEADERS: &[&str] = &[
    "Content-Length",
    "Content-Type",
    "Date",
    "Location",
    "Set-Cookie",
    "X-Path",
    "X-Echo",
];

fn app() -> &'static Arc<Octane> {
    static APP: OnceLock<Arc<Octane>> = OnceLock::new();
    APP.get_or_init(|| {
        let mut app = Octane::new();
        app.settings.max_body_size = 64;
        app.add(route_next!(|req, res| {
            res.set("X-Path", &req.request_line.path.to_string());
        }))
        .unwrap();
        app.get("/", route_stop!(|req, res| res.send("ok")))
            .unwrap();
        app.head("/", route_stop!(|req, res| res.send("ok")))
            .unwrap();
        app.get(
            "/redirect",
            route_stop!(|req, res| {
                let next = req.get_query().get("next").cloned().unwrap_or_default();
                res.redirect(&next).send("");
            }),
        )
        .unwrap();
        app.get(
            "/cookies",
            route_stop!(|req, res| {
                let set = req.get_query().get("set").cloned().unwrap_or_default();
                let pairs: Vec<String> = req
                    .cookies
                    .iter()
                    .map(|c| format!("{}={}", c.name(), c.value()))
                    .collect();
                res.cookie(Cookie::new("echo", set))
                    .cookie(Cookie::build("sid", "1").path("/").http_only(true))
                    .send(pairs.join("\n"));
            }),
        )
        .unwrap();
        app.post(
            "/echo",
            route_stop!(|req, res| {
                let echo = req.headers.get("x-echo").cloned().unwrap_or_default();
                res.set("X-Echo", &echo).send(req.body);
            }),
        )
        .unwrap();
        app.get(
            "/vars/:a/:b",
            route_stop!(|req, res| res.send(format!("{:?}", req.vars))),
        )
        .unwrap();
        Arc::new(app)
    })
}

fuzz_target!(|data: &[u8]| {
    let (seed, wire) = match data.split_first() {
        Some(split) => split,
        None => return,
    };
    let out = fuzzing::serve(app(), wire, *seed as usize % 17 + 1);

    let end = out
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("no complete response head");
    let head = std::str::from_utf8(&out[..end]).expect("head is not utf-8");
    let body = &out[end + 4..];

    let mut lines = head.split("\r\n");
    let status = lines.next().unwrap();
    assert!(status.starts_with("HTTP/1.1 "), "{:?}", status);
    assert!(
        status[9..12].bytes().all(|b| b.is_ascii_digit()),
        "{:?}",
        status
    );

    let mut content_length = None;
    for line in lines {
        // CR and LF only ever appear together, ending a line, and HTAB is
        // the only other control character a value may hold
        let is_ctl = |c: char| (c <= '\x1f' || c == '\x7f') && c != '\t';
        assert!(!line.contains(is_ctl), "{:?}", line);
        let (name, value) = line.split_once(": ").expect("header line without a name");
        assert!(HEADERS.contains(&name), "unexpected header {:?}", line);
        if name == "Content-Length" {
            assert!(content_length.is_none(), "two Content-Length headers");
            content_length = Some(value.parse::<usize>().expect("bad Content-Length"));
        }
    }
    assert_eq!(
        Some(body.len()),
        content_length,
        "body doesn't match Content-Length"
    );
});
