#![cfg(feature = "std")]
//! Graceful shutdown: idle connections close, a request in flight finishes.
use octane::prelude::*;
use std::io::Write;
use std::time::{Duration, Instant};
mod common;

const PORT: u16 = 8095;

#[test]
pub fn shutdown_finishes_requests_and_closes_idle_connections() {
    let mut app = Octane::new();
    app.get("/", route_stop!(|req, res| res.send("ok")))
        .unwrap();
    app.get(
        "/slow",
        route_async_stop!(|req, res| {
            tokio::time::sleep(Duration::from_millis(300)).await;
            res.send("done");
        }),
    )
    .unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = runtime.spawn(async move {
        app.listen_with_shutdown(PORT, || {}, async {
            let _ = stopped.await;
        })
        .await
        .is_ok()
    });
    common::wait_for(PORT);

    // an idle keep-alive connection, and one with a request in flight
    let mut idle = common::connect(PORT);
    idle.write_all(b"GET / HTTP/1.1\r\nHost: a\r\n\r\n")
        .unwrap();
    assert_eq!("ok", common::read_response(&mut idle, false).text());
    let mut busy = common::connect(PORT);
    busy.write_all(b"GET /slow HTTP/1.1\r\nHost: a\r\n\r\n")
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let start = Instant::now();
    stop.send(()).unwrap();
    // the idle one closes right away
    assert!(common::is_closed(&mut idle));
    assert!(start.elapsed() < Duration::from_millis(250));
    // the busy one is answered, told the connection closes, then closed
    let res = common::read_response(&mut busy, false);
    assert_eq!(
        ("done", Some("close")),
        (res.text(), res.header("connection"))
    );
    assert!(common::is_closed(&mut busy));
    // and listening ends once both are gone
    assert!(runtime.block_on(server).unwrap());
    assert!(std::net::TcpStream::connect(("127.0.0.1", PORT)).is_err());
}
