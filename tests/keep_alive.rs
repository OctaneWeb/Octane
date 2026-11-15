#![cfg(feature = "std")]
//! Persistent connections: several requests per connection, pipelined
//! requests, chunked request bodies and the things that end a connection.
use octane::prelude::*;
use octane::StatusCode;
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;
mod common;

const PORT: u16 = 8090;
const IDLE_PORT: u16 = 8091;

fn app() -> Octane {
    let mut app = Octane::new();
    app.settings.max_body_size = 16;
    app.get("/", route_next!(|req, res| res.send("ok")))
        .unwrap();
    app.head("/", route_next!(|req, res| res.send("ok")))
        .unwrap();
    app.post("/echo", route_next!(|req, res| res.send(req.body)))
        .unwrap();
    // statuses that never have content, sent with some anyway
    app.get(
        "/not-modified",
        route_next!(|req, res| res.status(StatusCode::NotModified).send("stale")),
    )
    .unwrap();
    app.get(
        "/no-content",
        route_next!(|req, res| res.status(StatusCode::NoContent).send("nothing")),
    )
    .unwrap();
    app
}

const GET: &[u8] = b"GET / HTTP/1.1\r\nHost: a\r\n\r\n";

#[test]
pub fn connections_carry_many_requests() {
    common::run(app(), PORT, || {
        // one request after another on the same connection
        let mut stream = common::connect(PORT);
        for _ in 0..3 {
            stream.write_all(GET).unwrap();
            let res = common::read_response(&mut stream, false);
            assert_eq!((200, "ok"), (res.status, res.text()));
            assert_eq!(None, res.header("connection"));
        }

        // pipelined: everything sent at once, answered in order, including
        // a body-less HEAD, a 404 and a chunked body between the others
        let mut stream = common::connect(PORT);
        let mut pipeline = Vec::new();
        pipeline.extend_from_slice(GET);
        pipeline.extend_from_slice(b"HEAD / HTTP/1.1\r\nHost: a\r\n\r\n");
        pipeline.extend_from_slice(b"GET /missing HTTP/1.1\r\nHost: a\r\n\r\n");
        pipeline.extend_from_slice(
            b"POST /echo HTTP/1.1\r\nHost: a\r\nTransfer-Encoding: chunked\r\n\r\n\
              5\r\nhello\r\n7;ext=1\r\n, world\r\n0\r\nX-Trailer: t\r\n\r\n",
        );
        pipeline
            .extend_from_slice(b"POST /echo HTTP/1.1\r\nHost: a\r\nContent-Length: 3\r\n\r\nabc");
        pipeline.extend_from_slice(GET);
        stream.write_all(&pipeline).unwrap();
        let ok = common::read_response(&mut stream, false);
        assert_eq!((200, "ok"), (ok.status, ok.text()));
        let head = common::read_response(&mut stream, true);
        assert_eq!(
            (200, Some("2")),
            (head.status, head.header("content-length"))
        );
        assert_eq!(404, common::read_response(&mut stream, false).status);
        let chunked = common::read_response(&mut stream, false);
        assert_eq!((200, "hello, world"), (chunked.status, chunked.text()));
        assert_eq!("abc", common::read_response(&mut stream, false).text());
        assert_eq!("ok", common::read_response(&mut stream, false).text());

        // a client may send an empty line before a request (RFC 9112 2.2)
        stream.write_all(b"\r\n").unwrap();
        stream.write_all(GET).unwrap();
        assert_eq!("ok", common::read_response(&mut stream, false).text());
        // even when its CR and LF arrive apart
        for part in [&b"\r"[..], b"\n", GET].iter() {
            stream.write_all(part).unwrap();
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!("ok", common::read_response(&mut stream, false).text());

        // a request split over several writes is put back together
        let parts: [&[u8]; 4] = [
            b"POST /echo HTTP/1.1\r\nHo",
            b"st: a\r\nTransfer-Encoding: chunked\r\n\r\n3",
            b"\r\nab",
            b"c\r\n0\r\n\r\n",
        ];
        for part in parts.iter() {
            stream.write_all(part).unwrap();
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!("abc", common::read_response(&mut stream, false).text());
    });
}

#[test]
pub fn connections_end_when_asked_or_when_framing_is_lost() {
    common::run(app(), PORT + 2, || {
        let port = PORT + 2;
        let closes = |request: &[u8], status: u16| {
            let mut stream = common::connect(port);
            stream.write_all(request).unwrap();
            let res = common::read_response(&mut stream, false);
            assert_eq!(status, res.status, "{:?}", String::from_utf8_lossy(request));
            assert_eq!(Some("close"), res.header("connection"));
            assert!(common::is_closed(&mut stream));
        };
        // asked for
        closes(
            b"GET / HTTP/1.1\r\nHost: a\r\nConnection: close\r\n\r\n",
            200,
        );
        closes(b"GET / HTTP/1.0\r\n\r\n", 200);
        // the request can't be framed, so nothing after it can be either
        let smuggled = b"POST /echo HTTP/1.1\r\nHost: a\r\nContent-Length: 3\r\n\
                         Transfer-Encoding: chunked\r\n\r\n0\r\n\r\n";
        closes(smuggled, 400);
        let gzip = b"POST /echo HTTP/1.1\r\nHost: a\r\nTransfer-Encoding: gzip, chunked\r\n\r\n";
        closes(gzip, 501);
        let bad_chunk =
            b"POST /echo HTTP/1.1\r\nHost: a\r\nTransfer-Encoding: chunked\r\n\r\nz\r\n";
        closes(bad_chunk, 400);
        let too_large = b"POST /echo HTTP/1.1\r\nHost: a\r\nTransfer-Encoding: chunked\r\n\r\n\
                          11\r\n12345678901234567\r\n0\r\n\r\n";
        closes(too_large, 413);
        closes(b"GET / HTTP/1.1\r\n\r\n", 400);

        // HTTP/1.0 keeps the connection only when it asks to
        let mut stream = common::connect(port);
        for _ in 0..2 {
            stream
                .write_all(b"GET / HTTP/1.0\r\nConnection: keep-alive\r\n\r\n")
                .unwrap();
            let res = common::read_response(&mut stream, false);
            assert_eq!(
                ("ok", Some("keep-alive")),
                (res.text(), res.header("connection"))
            );
        }
    });
}

#[test]
pub fn a_path_without_the_method_is_405() {
    common::run(app(), PORT + 6, || {
        let mut stream = common::connect(PORT + 6);
        // `/` has GET and HEAD routes, `/echo` only POST
        stream
            .write_all(
                b"DELETE / HTTP/1.1\r\nHost: a\r\n\r\n\
                  GET /echo HTTP/1.1\r\nHost: a\r\n\r\n\
                  GET /missing HTTP/1.1\r\nHost: a\r\n\r\n\
                  GET / HTTP/1.1\r\nHost: a\r\n\r\n",
            )
            .unwrap();
        let res = common::read_response(&mut stream, false);
        assert_eq!((405, Some("GET, HEAD")), (res.status, res.header("allow")));
        let res = common::read_response(&mut stream, false);
        assert_eq!((405, Some("POST")), (res.status, res.header("allow")));
        // the connection carries on, it's the request that was refused
        let res = common::read_response(&mut stream, false);
        assert_eq!((404, None), (res.status, res.header("allow")));
        assert_eq!("ok", common::read_response(&mut stream, false).text());
    });
}

#[test]
pub fn bodiless_statuses_send_no_body() {
    common::run(app(), PORT + 5, || {
        // whatever the handler sent, the next response follows the head
        let mut stream = common::connect(PORT + 5);
        stream
            .write_all(
                b"GET /not-modified HTTP/1.1\r\nHost: a\r\n\r\n\
                  GET /no-content HTTP/1.1\r\nHost: a\r\n\r\n\
                  GET / HTTP/1.1\r\nHost: a\r\n\r\n",
            )
            .unwrap();
        for status in [304, 204].iter() {
            let res = common::read_response(&mut stream, true);
            assert_eq!(*status, res.status);
            assert_eq!(None, res.header("content-length"));
        }
        assert_eq!("ok", common::read_response(&mut stream, false).text());
    });
}

#[test]
pub fn a_refused_upload_still_gets_its_response() {
    common::run(app(), PORT + 4, || {
        for _ in 0..5 {
            let mut stream = common::connect(PORT + 4);
            // over max_body_size, refused before the body is read
            stream
                .write_all(b"POST /echo HTTP/1.1\r\nHost: a\r\nContent-Length: 4194304\r\n\r\n")
                .unwrap();
            // the client is still sending when the answer comes
            let chunk = vec![b'x'; 64 * 1024];
            for _ in 0..8 {
                if stream.write_all(&chunk).is_err() {
                    break;
                }
            }
            let res = common::read_response(&mut stream, false);
            assert_eq!((413, Some("close")), (res.status, res.header("connection")));
            // and the stream ends, rather than being reset over the unread
            // body, which can destroy the response before it is read
            // (RFC 9112 section 9.6)
            assert!(common::is_closed(&mut stream), "the connection was reset");
        }
    });
}

#[test]
pub fn expect_continue_is_answered_before_the_body_is_read() {
    common::run(app(), PORT + 3, || {
        let mut stream = common::connect(PORT + 3);
        stream
            .write_all(b"POST /echo HTTP/1.1\r\nHost: a\r\nContent-Length: 5\r\nExpect: 100-continue\r\n\r\n")
            .unwrap();
        let interim = common::read_response(&mut stream, true);
        assert_eq!(100, interim.status);
        stream.write_all(b"hello").unwrap();
        assert_eq!("hello", common::read_response(&mut stream, false).text());

        // not for a body that is going to be refused
        stream
            .write_all(b"POST /echo HTTP/1.1\r\nHost: a\r\nContent-Length: 99\r\nExpect: 100-continue\r\n\r\n")
            .unwrap();
        assert_eq!(413, common::read_response(&mut stream, false).status);
    });
}

#[test]
pub fn idle_connections_are_closed() {
    let mut app = app();
    app.settings.keep_alive = Some(Duration::from_millis(200));
    common::run(app, IDLE_PORT, || {
        let mut stream = common::connect(IDLE_PORT);
        stream.write_all(GET).unwrap();
        assert_eq!("ok", common::read_response(&mut stream, false).text());
        // still open within the idle time
        thread::sleep(Duration::from_millis(50));
        stream.write_all(GET).unwrap();
        assert_eq!("ok", common::read_response(&mut stream, false).text());
        // and closed after it, without a response
        let mut rest = Vec::new();
        stream.read_to_end(&mut rest).unwrap();
        assert!(rest.is_empty());
    });

    // without keep-alive every response closes its connection
    let mut app = Octane::new();
    app.settings.keep_alive = None;
    app.get("/", route_next!(|req, res| res.send("ok")))
        .unwrap();
    common::run(app, IDLE_PORT + 1, || {
        let mut stream = common::connect(IDLE_PORT + 1);
        stream.write_all(GET).unwrap();
        let res = common::read_response(&mut stream, false);
        assert_eq!(
            ("ok", Some("close")),
            (res.text(), res.header("connection"))
        );
        assert!(common::is_closed(&mut stream));
    });
}
