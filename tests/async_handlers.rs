#![cfg(feature = "std")]
//! Async handlers wait without holding a worker thread.
use octane::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
mod common;

const PORT: u16 = 8093;
const WAIT: Duration = Duration::from_millis(200);
const CONCURRENT: usize = 64;

#[test]
pub fn async_handlers_wait_concurrently() {
    let mut app = Octane::new();
    let served = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&served);
    // the body borrows the captured counter across the await
    app.get(
        "/wait/:ms",
        route_async_stop!(|req, res| {
            let ms = req
                .vars
                .get("ms")
                .and_then(|ms| ms.parse().ok())
                .unwrap_or(0);
            tokio::time::sleep(Duration::from_millis(ms)).await;
            counter.fetch_add(1, Ordering::SeqCst);
            res.send(ms.to_string());
        }),
    )
    .unwrap();
    // a sync handler next to them still answers
    app.get("/now", route_stop!(|req, res| res.send("now")))
        .unwrap();

    common::run(app, PORT, || {
        let start = Instant::now();
        let clients: Vec<_> = (0..CONCURRENT)
            .map(|_| {
                thread::spawn(|| {
                    let path = format!("/wait/{}", WAIT.as_millis());
                    common::request(PORT, "GET", &path, &[]).text().to_owned()
                })
            })
            .collect();
        // answered while every async handler is still waiting
        assert_eq!("now", common::request(PORT, "GET", "/now", &[]).text());
        for client in clients {
            assert_eq!(WAIT.as_millis().to_string(), client.join().unwrap());
        }
        let elapsed = start.elapsed();
        assert!(elapsed >= WAIT, "answered before waiting: {:?}", elapsed);
        // blocking a worker per request would take CONCURRENT / workers waits
        assert!(
            elapsed < Duration::from_secs(1),
            "async handlers didn't overlap: {:?}",
            elapsed
        );
        assert_eq!(CONCURRENT, served.load(Ordering::SeqCst));
    });
}
