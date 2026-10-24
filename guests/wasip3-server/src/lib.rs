//! An HTTP/1.1 server as a WASIp3 component.
//!
//! wasi:sockets hands out connections as a `stream<tcp-socket>`, each
//! connection's bytes as a `stream<u8>` in and a `stream<u8>` out. Every
//! connection runs as its own task with `spawn_local`, so while one awaits a
//! read, a timer or a write the others make progress, all on one thread.
//!
//! octane_http does no I/O: bytes read off the stream go into a
//! `RequestBuffer`, the response head is encoded by `Response`, and this file
//! moves the bytes between them and the streams.
//!
//! Routes:
//! - `GET /` replies at once
//! - `GET /sleep?ms=N` waits N ms on the host clock before replying
//! - `GET /stream?chunks=N&ms=M` writes the body in N chunks, M ms apart
//! - `POST /echo` replies with the request body

use octane_http::http1x::{Method, Request, RequestBuffer, Status};
use octane_http::response::{error_response, Response, ResponseHead};
use octane_http::{HttpVersion, StatusCode};
use std::cell::Cell;
use std::rc::Rc;
use wasip3::clocks::monotonic_clock;
use wasip3::sockets::types::{IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, TcpSocket};
use wasip3::wit_bindgen::rt::async_support::{StreamResult, StreamWriter};
use wasip3::{spawn_local, wit_stream};

const PORT: u16 = 8080;
/// Bytes buffered per connection for a request, head and body together
const INPUT_SIZE: usize = 16 * 1024;
/// Bytes for a response head
const OUTPUT_SIZE: usize = 1024;
/// Largest single read off the socket stream
const READ_CHUNK: usize = 4096;

wasip3::cli::command::export!(Server);

struct Server;

impl wasip3::exports::cli::run::Guest for Server {
    async fn run() -> Result<(), ()> {
        let listener = TcpSocket::create(IpAddressFamily::Ipv4).map_err(|e| fail("create", e))?;
        listener
            .bind(IpSocketAddress::Ipv4(Ipv4SocketAddress {
                port: PORT,
                address: (127, 0, 0, 1),
            }))
            .map_err(|e| fail("bind", e))?;
        let mut connections = listener.listen().map_err(|e| fail("listen", e))?;
        println!("listening on http://127.0.0.1:{}", PORT);

        // how many connections are being served right now, to show the
        // tasks overlapping
        let open = Rc::new(Cell::new(0usize));
        let mut id = 0usize;
        while let Some(socket) = connections.next().await {
            id += 1;
            let open = Rc::clone(&open);
            spawn_local(async move {
                open.set(open.get() + 1);
                println!("[conn {}] open ({} concurrent)", id, open.get());
                serve(socket, id).await;
                open.set(open.get() - 1);
                println!("[conn {}] closed ({} concurrent)", id, open.get());
            });
        }
        Ok(())
    }
}

fn fail(what: &str, error: impl std::fmt::Debug) {
    eprintln!("{} failed: {:?}", what, error);
}

/// Serve one connection until either side closes it
async fn serve(socket: TcpSocket, id: usize) {
    let (mut incoming, _receive_done) = socket.receive();
    let (mut outgoing, outgoing_reader) = wit_stream::new::<u8>();
    // the host drains `outgoing_reader` into the socket while this future
    // is pending, it resolves once the stream is closed
    let send_done = socket.send(outgoing_reader);

    let mut input = vec![0u8; INPUT_SIZE];
    let mut output = vec![0u8; OUTPUT_SIZE];
    let mut requests = RequestBuffer::new(&mut input);

    loop {
        let close = match requests.poll() {
            Ok(Status::Complete(request)) => {
                println!("[conn {}] {} {}", id, request.method, request.target);
                respond(&request, &mut outgoing, &mut output).await
            }
            Ok(Status::Partial) => {
                let spare = requests.unfilled();
                let want = spare.len().min(READ_CHUNK);
                let (result, bytes) = incoming.read(Vec::with_capacity(want)).await;
                match result {
                    StreamResult::Complete(_) if !bytes.is_empty() => {
                        spare[..bytes.len()].copy_from_slice(&bytes);
                        requests.advance(bytes.len());
                        continue;
                    }
                    // a zero length read can't make progress, treat it as EOF
                    _ => true,
                }
            }
            Err(status) => {
                if let Ok(head) = error_response(&mut output, status) {
                    write(&mut outgoing, head).await;
                }
                true
            }
        };
        if close {
            break;
        }
        requests.consume();
    }

    // closing the writer ends the outgoing stream, then wait for the host to
    // flush it to the socket
    drop(outgoing);
    if let Err(error) = send_done.await {
        eprintln!("[conn {}] send failed: {:?}", id, error);
    }
}

/// Route one request and write its response, returning whether to close
async fn respond(
    request: &Request<'_>,
    outgoing: &mut StreamWriter<u8>,
    output: &mut [u8],
) -> bool {
    let query = |name: &str| -> Option<u64> {
        request
            .query_pairs()
            .find(|(key, _)| *key == name)
            .and_then(|(_, value)| value.parse().ok())
    };

    match (request.method, request.path) {
        (Method::Get, "/stream") => {
            let chunks = query("chunks").unwrap_or(5).min(100);
            let delay = query("ms").unwrap_or(200);
            stream_chunks(request, outgoing, output, chunks, delay).await
        }
        (Method::Get, "/sleep") => {
            let ms = query("ms").unwrap_or(1000).min(60_000);
            sleep_ms(ms).await;
            let body = format!("slept {} ms\n", ms);
            send(request, outgoing, output, |res| {
                res.header("Content-Type", b"text/plain").body(body.as_bytes());
            })
            .await
        }
        (Method::Get | Method::Head, "/") => {
            send(request, outgoing, output, |res| {
                res.header("Content-Type", b"text/plain")
                    .body(b"hello from a wasip3 component\n");
            })
            .await
        }
        (Method::Post, "/echo") => {
            send(request, outgoing, output, |res| {
                res.body(request.body);
            })
            .await
        }
        _ => {
            send(request, outgoing, output, |res| {
                res.status(StatusCode::NotFound).body(b"not found\n");
            })
            .await
        }
    }
}

/// Encode a complete response with octane's `Response` and write it
async fn send<'b, F>(
    request: &Request<'_>,
    outgoing: &mut StreamWriter<u8>,
    output: &mut [u8],
    build: F,
) -> bool
where
    F: for<'o> FnOnce(&mut Response<'o, 'b>),
{
    let mut res = Response::new(output, request.version);
    build(&mut res);
    let encoded = match res.finish(request.method == Method::Head, request.keep_alive()) {
        Ok(encoded) => encoded,
        Err(_) => {
            // the handler set an invalid header; answer with a 500 and close
            if let Ok(head) = error_response(output, StatusCode::InternalServerError) {
                write(outgoing, head).await;
            }
            return true;
        }
    };
    let close = encoded.close;
    if !(write(outgoing, encoded.head).await && write(outgoing, encoded.body).await) {
        return true;
    }
    close
}

/// Write the head, then the body a chunk at a time with a delay between
/// chunks, so the response is produced while other connections run
async fn stream_chunks(
    request: &Request<'_>,
    outgoing: &mut StreamWriter<u8>,
    output: &mut [u8],
    chunks: u64,
    delay_ms: u64,
) -> bool {
    let chunk_len = |i: u64| format!("chunk {} of {}\n", i + 1, chunks).len();
    let total: usize = (0..chunks).map(chunk_len).sum();

    let mut head = ResponseHead::new(output, request.version, StatusCode::Ok);
    let close = !request.keep_alive();
    let head = (|| {
        head.header("Content-Type", b"text/plain")?;
        head.content_length(total)?;
        if close && request.version == HttpVersion::Http11 {
            head.header("Connection", b"close")?;
        }
        head.finish()
    })();
    let head = match head {
        Ok(head) => head,
        Err(_) => return true,
    };
    if !write(outgoing, head).await {
        return true;
    }
    if request.method == Method::Head {
        return close;
    }
    for i in 0..chunks {
        if i > 0 {
            sleep_ms(delay_ms).await;
        }
        if !write(outgoing, format!("chunk {} of {}\n", i + 1, chunks).as_bytes()).await {
            return true;
        }
    }
    close
}

/// Write all of `bytes`, false if the peer went away
async fn write(outgoing: &mut StreamWriter<u8>, bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return true;
    }
    // the stream ABI takes ownership of the buffer, so this is the one copy
    outgoing.write_all(bytes.to_vec()).await.is_empty()
}

async fn sleep_ms(ms: u64) {
    monotonic_clock::wait_for(ms * 1_000_000).await;
}
