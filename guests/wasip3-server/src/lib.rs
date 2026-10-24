//! An HTTP/1.1 server as a WASIp3 component, routed by octane's router.
//!
//! Routes are added with the same `Route` trait as on octane's tokio server
//! and `octane::wasip3::serve` does the sockets, connections and buffers.
//! Handlers are async: while one waits on a timer or streams its body, the
//! other connections are served.
//!
//! Routes:
//! - every request is logged by a middleware
//! - `GET /` replies at once
//! - `GET /hello/:name` greets `name`, a url variable
//! - `GET /sleep?ms=N` waits N ms on the host clock before replying
//! - `GET /stream?chunks=N&ms=M` writes the body in N chunks, M ms apart
//! - `POST /echo` replies with the request body
//! - anything else gets a 404 from the server

use octane::portable::config::Config;
use octane::wasip3::{handler, serve, Flow, MatchedRequest, Responder, Route, Router, SendError};
use octane::StatusCode;
use std::net::SocketAddr;
use wasip3::clocks::monotonic_clock;

/// 16 KiB per request (head and body), 1 KiB per response head
const CONFIG: Config<{ 16 * 1024 }, 1024> = Config::new();

const TEXT: [(&str, &str); 1] = [("Content-Type", "text/plain")];

wasip3::cli::command::export!(Server);

struct Server;

impl wasip3::exports::cli::run::Guest for Server {
    async fn run() -> Result<(), ()> {
        let router = routes().map_err(|error| eprintln!("invalid route: {}", error))?;
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        serve(
            CONFIG,
            addr,
            |addr| println!("listening on http://{}", addr),
            router,
        )
        .await
        .map_err(|error| eprintln!("cannot serve on {}: {:?}", addr, error))
    }
}

fn routes() -> Result<Router, octane::wasip3::InvalidPathError> {
    let mut router = Router::new();

    router.add(handler(async |req, _res| {
        println!("{} {}", req.method, req.target);
        Flow::Next
    }))?;

    router.get(
        "/",
        handler(async |_req, res| {
            let _ = res
                .send(StatusCode::Ok, &TEXT, b"hello from a wasip3 component\n")
                .await;
            Flow::Stop
        }),
    )?;

    router.get(
        "/hello/:name",
        handler(async |req, res| {
            let name = req.vars.get("name").copied().unwrap_or("there");
            let _ = res
                .send(
                    StatusCode::Ok,
                    &TEXT,
                    format!("hello, {}\n", name).as_bytes(),
                )
                .await;
            Flow::Stop
        }),
    )?;

    router.get(
        "/sleep",
        handler(async |req, res| {
            let ms = query(&req, "ms").unwrap_or(1000).min(60_000);
            sleep_ms(ms).await;
            let _ = res
                .send(
                    StatusCode::Ok,
                    &TEXT,
                    format!("slept {} ms\n", ms).as_bytes(),
                )
                .await;
            Flow::Stop
        }),
    )?;

    router.get(
        "/stream",
        handler(async |req, res| {
            let chunks = query(&req, "chunks").unwrap_or(5).min(100);
            let delay = query(&req, "ms").unwrap_or(200).min(10_000);
            let _ = stream(res, chunks, delay).await;
            Flow::Stop
        }),
    )?;

    router.post(
        "/echo",
        handler(async |req, res| {
            let _ = res.send(StatusCode::Ok, &[], req.body).await;
            Flow::Stop
        }),
    )?;

    Ok(router)
}

fn query(req: &MatchedRequest<'_>, name: &str) -> Option<u64> {
    req.query_pairs()
        .find(|(key, _)| *key == name)
        .and_then(|(_, value)| value.parse().ok())
}

/// Send `chunks` lines, `delay_ms` apart, as one response body
async fn stream(res: &mut Responder<'_>, chunks: u64, delay_ms: u64) -> Result<(), SendError> {
    let line = |i: u64| format!("chunk {} of {}\n", i + 1, chunks);
    let len = (0..chunks).map(|i| line(i).len()).sum();
    let mut body = res.stream(StatusCode::Ok, &TEXT, len).await?;
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
