//! The allocation-free core served over `std::net`, which also works on
//! WASI preview 2:
//!
//! ```sh
//! cargo run --example portable_server
//! cargo build --example portable_server --target wasm32-wasip2
//! wasmtime serve -S cli -S inherit-network target/wasm32-wasip2/debug/examples/portable_server.wasm
//! ```
//!
//! On a platform without `std`, implement `Connection` for its socket type
//! and call `CONFIG.serve` the same way.
use octane_http::config::Config;
use octane_http::http1x::Method;
use octane_http::server::StdIo;
use octane_http::StatusCode;
use std::net::TcpListener;

// 16 KiB for requests, 1 KiB for response heads, 17 KiB of stack in all
const CONFIG: Config<{ 16 * 1024 }, 1024> = Config::new();

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("listening on http://{}", listener.local_addr()?);

    // one connection at a time, so one pair of buffers is enough
    let mut buffers = CONFIG.buffers();
    for stream in listener.incoming() {
        let result = CONFIG.serve_with(StdIo(stream?), &mut buffers, |req, res| {
            match (req.method, req.path) {
                (Method::Get, "/hello") => res
                    .header("Content-Type", b"text/plain")
                    .body(b"Hello from no_std!"),
                (Method::Post, "/echo") => res.body(req.body),
                _ => res.status(StatusCode::NotFound).body(b"Not Found"),
            };
        });
        if let Err(error) = result {
            eprintln!("connection error: {}", error);
        }
    }
    Ok(())
}
