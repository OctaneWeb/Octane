//! An HTTP/1.1 server as a WASIp3 component.
//!
//! `octane::wasip3::serve` creates the socket, accepts connections and runs
//! each one as its own task, so this file only routes. The handler is async:
//! while one request waits on a timer or streams its body, the others are
//! served.
//!
//! Routes:
//! - `GET /` replies at once
//! - `GET /sleep?ms=N` waits N ms on the host clock before replying
//! - `GET /stream?chunks=N&ms=M` writes the body in N chunks, M ms apart
//! - `POST /echo` replies with the request body

use octane::portable::config::Config;
use octane::portable::http1x::{Method, Request};
use octane::wasip3::{serve, Responder};
use octane::StatusCode;
use std::net::SocketAddr;
use wasip3::clocks::monotonic_clock;

/// 16 KiB per request (head and body), 1 KiB per response head
const CONFIG: Config<{ 16 * 1024 }, 1024> = Config::new();

wasip3::cli::command::export!(Server);

struct Server;

impl wasip3::exports::cli::run::Guest for Server {
    async fn run() -> Result<(), ()> {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        serve(CONFIG, addr, |addr| println!("listening on http://{}", addr), handle)
            .await
            .map_err(|error| eprintln!("cannot serve on {}: {:?}", addr, error))
    }
}

async fn handle(req: &Request<'_>, res: &mut Responder<'_>) {
    println!("{} {}", req.method, req.target);
    let query = |name: &str| -> Option<u64> {
        req.query_pairs()
            .find(|(key, _)| *key == name)
            .and_then(|(_, value)| value.parse().ok())
    };
    let text = [("Content-Type", "text/plain")];

    // a failed send means the peer left or the response was invalid; the
    // server closes the connection either way
    let _ = match (req.method, req.path) {
        (Method::Get | Method::Head, "/") => {
            res.send(StatusCode::Ok, &text, b"hello from a wasip3 component\n").await
        }
        (Method::Get, "/sleep") => {
            let ms = query("ms").unwrap_or(1000).min(60_000);
            sleep_ms(ms).await;
            res.send(StatusCode::Ok, &text, format!("slept {} ms\n", ms).as_bytes()).await
        }
        (Method::Get | Method::Head, "/stream") => {
            let chunks = query("chunks").unwrap_or(5).min(100);
            let delay = query("ms").unwrap_or(200).min(10_000);
            stream(res, chunks, delay).await
        }
        (Method::Post, "/echo") => res.send(StatusCode::Ok, &[], req.body).await,
        // nothing sent: the server answers 404
        _ => Ok(()),
    };
}

/// Send `chunks` lines, `delay_ms` apart, as one response body
async fn stream(
    res: &mut Responder<'_>,
    chunks: u64,
    delay_ms: u64,
) -> Result<(), octane::wasip3::SendError> {
    let line = |i: u64| format!("chunk {} of {}\n", i + 1, chunks);
    let len = (0..chunks).map(|i| line(i).len()).sum();
    let mut body = res.stream(StatusCode::Ok, &[("Content-Type", "text/plain")], len).await?;
    for i in 0..chunks {
        if i > 0 {
            sleep_ms(delay_ms).await;
        }
        body.write(line(i).as_bytes()).await?;
    }
    Ok(())
}

async fn sleep_ms(ms: u64) {
    monotonic_clock::wait_for(ms * 1_000_000).await;
}
