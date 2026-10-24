//! Serving HTTP/1.x from a WASIp3 component over `wasi:sockets`.
//!
//! Available with the `wasm` feature when building for `wasm32-wasip3`.
//! [`serve`] creates the listening socket and runs every accepted connection
//! as its own task, so connections are served concurrently on the
//! component's single thread: while one waits on its socket, a timer or a
//! streamed body, the others make progress.
//!
//! Handlers are async and answer through a [`Responder`], either with a
//! whole body ([`Responder::send`]) or by streaming one
//! ([`Responder::stream`]).
//!
//! ```ignore
//! use octane_http::config::Config;
//! use octane_http::http1x::Request;
//! use octane_http::wasip3::{serve, Responder};
//! use octane_http::StatusCode;
//!
//! const CONFIG: Config<{ 16 * 1024 }, 1024> = Config::new();
//!
//! async fn handle(req: &Request<'_>, res: &mut Responder<'_>) {
//!     let _ = res.send(StatusCode::Ok, &[("Content-Type", "text/plain")], b"hi").await;
//! }
//!
//! // inside the component's async `run` export
//! serve(CONFIG, "127.0.0.1:8080".parse().unwrap(), |addr| println!("on {}", addr), handle).await
//! ```
//!
//! Parsing stays allocation-free. Each connection's [`Buffers`] are boxed,
//! since a task outlives the stack frame that spawned it, and the stream ABI
//! takes ownership of what is written, so writes copy into a `Vec`.

use crate::config::{Buffers, Config};
use crate::http1x::{Method, Request, RequestBuffer, Status};
use crate::response::{error_response, EncodeError, Response};
use crate::{HttpVersion, StatusCode};
use ::wasip3::sockets::types::{
    IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress, TcpSocket,
};
use ::wasip3::wit_bindgen::rt::async_support::{StreamResult, StreamWriter};
use ::wasip3::{spawn_local, wit_stream};
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

/// The `wasi:sockets` error code
pub use ::wasip3::sockets::types::ErrorCode;

/// Largest single read off a connection's stream
const READ_CHUNK: usize = 4096;

/// Listen on `addr` and serve every connection until the listener closes.
///
/// `on_listen` is called with the bound address once the socket listens,
/// which tells the port when `addr` asks for port 0. Errors creating,
/// binding or listening on the socket are returned; a failing connection
/// only ends that connection.
pub async fn serve<const INPUT: usize, const OUTPUT: usize, L, H>(
    config: Config<INPUT, OUTPUT>,
    addr: SocketAddr,
    on_listen: L,
    handler: H,
) -> Result<(), ErrorCode>
where
    L: FnOnce(SocketAddr),
    H: AsyncFn(&Request<'_>, &mut Responder<'_>) + 'static,
{
    let family = match addr {
        SocketAddr::V4(_) => IpAddressFamily::Ipv4,
        SocketAddr::V6(_) => IpAddressFamily::Ipv6,
    };
    let listener = TcpSocket::create(family)?;
    listener.bind(to_wasi(addr))?;
    let mut connections = listener.listen()?;
    on_listen(from_wasi(listener.get_local_address()?));

    let handler = Rc::new(handler);
    while let Some(socket) = connections.next().await {
        let handler = Rc::clone(&handler);
        spawn_local(async move {
            // the error only concerns this connection
            let _ = serve_connection(config, socket, &*handler).await;
        });
    }
    Ok(())
}

/// Serve one accepted connection until either side closes it, for callers
/// that set up the listener themselves
pub async fn serve_connection<const INPUT: usize, const OUTPUT: usize, H>(
    config: Config<INPUT, OUTPUT>,
    socket: TcpSocket,
    handler: &H,
) -> Result<(), ErrorCode>
where
    H: AsyncFn(&Request<'_>, &mut Responder<'_>),
{
    let (mut incoming, _receive_done) = socket.receive();
    let (mut outgoing, outgoing_reader) = wit_stream::new::<u8>();
    // the host drains the stream into the socket while this is pending
    let send_done = socket.send(outgoing_reader);

    let mut buffers = Box::new(config.buffers());
    let Buffers { input, output } = &mut *buffers;
    let mut requests = RequestBuffer::new(&mut input[..]);

    loop {
        let close = match requests.poll() {
            Ok(Status::Complete(request)) => {
                let mut responder = Responder::new(&mut output[..], &mut outgoing, &request);
                handler(&request, &mut responder).await;
                responder.finish().await
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
                if let Ok(head) = error_response(&mut output[..], status) {
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

    // closing the writer ends the stream, then wait for the host to flush it
    drop(outgoing);
    send_done.await
}

/// Why a response couldn't be sent
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SendError {
    /// A response was already sent for this request
    AlreadySent,
    /// The head couldn't be encoded, a `500` was sent instead
    Encode(EncodeError),
    /// More body bytes were written than the declared length
    BodyTooLong,
    /// The peer closed the connection
    Closed,
}

/// Answers one request. A handler that doesn't send anything gets a `404`
/// sent for it.
pub struct Responder<'c> {
    output: &'c mut [u8],
    outgoing: &'c mut StreamWriter<u8>,
    version: HttpVersion,
    head_request: bool,
    keep_alive: bool,
    sent: bool,
    close: bool,
    /// Declared body bytes a streamed response still owes
    owed: usize,
}

impl<'c> Responder<'c> {
    fn new(
        output: &'c mut [u8],
        outgoing: &'c mut StreamWriter<u8>,
        request: &Request<'_>,
    ) -> Self {
        Self {
            output,
            outgoing,
            version: request.version,
            head_request: request.method == Method::Head,
            keep_alive: request.keep_alive(),
            sent: false,
            close: false,
            owed: 0,
        }
    }

    /// Send a response with the whole `body`. `Content-Length` and
    /// `Connection` are set by the server.
    pub async fn send(
        &mut self,
        status: StatusCode,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<(), SendError> {
        self.claim()?;
        let mut response = build(self.output, self.version, status, headers);
        response.body(body);
        let encoded = match response.finish(self.head_request, self.keep_alive) {
            Ok(encoded) => encoded,
            Err(error) => return Err(self.fail(error).await),
        };
        self.close = encoded.close;
        if !(write(self.outgoing, encoded.head).await && write(self.outgoing, encoded.body).await) {
            self.close = true;
            return Err(SendError::Closed);
        }
        Ok(())
    }

    /// Send the head of a response with a `body_len` bytes long body and
    /// return a writer for the body. Writing fewer bytes than declared
    /// closes the connection once the handler returns.
    pub async fn stream(
        &mut self,
        status: StatusCode,
        headers: &[(&str, &str)],
        body_len: usize,
    ) -> Result<BodyWriter<'_, 'c>, SendError> {
        self.claim()?;
        let response = build(self.output, self.version, status, headers);
        // HEAD responses and 204/304 describe a body without sending it
        let discard = self.head_request || response.is_bodiless();
        let encoded = match response.finish_streamed(body_len, self.keep_alive) {
            Ok(encoded) => encoded,
            Err(error) => return Err(self.fail(error).await),
        };
        self.close = encoded.close;
        if !write(self.outgoing, encoded.head).await {
            self.close = true;
            return Err(SendError::Closed);
        }
        self.owed = if discard { 0 } else { body_len };
        Ok(BodyWriter {
            responder: self,
            remaining: body_len,
            discard,
        })
    }

    /// Whether a response was sent
    pub fn is_sent(&self) -> bool {
        self.sent
    }

    /// Mark the request as answered, only once
    fn claim(&mut self) -> Result<(), SendError> {
        if self.sent {
            return Err(SendError::AlreadySent);
        }
        self.sent = true;
        Ok(())
    }

    /// Answer an unencodable response with a `500` and close
    async fn fail(&mut self, error: EncodeError) -> SendError {
        self.close = true;
        if let Ok(head) = error_response(self.output, StatusCode::InternalServerError) {
            write(self.outgoing, head).await;
        }
        SendError::Encode(error)
    }

    /// Send a `404` if nothing was sent, returning whether to close
    async fn finish(mut self) -> bool {
        if !self.sent {
            let _ = self.send(StatusCode::NotFound, &[], b"").await;
        }
        self.close || self.owed > 0
    }
}

/// Writes the body of a [`Responder::stream`] response
pub struct BodyWriter<'r, 'c> {
    responder: &'r mut Responder<'c>,
    remaining: usize,
    discard: bool,
}

impl BodyWriter<'_, '_> {
    /// Write the next part of the body
    pub async fn write(&mut self, bytes: &[u8]) -> Result<(), SendError> {
        if bytes.len() > self.remaining {
            return Err(SendError::BodyTooLong);
        }
        self.remaining -= bytes.len();
        if self.discard {
            return Ok(());
        }
        if !write(self.responder.outgoing, bytes).await {
            self.responder.close = true;
            return Err(SendError::Closed);
        }
        self.responder.owed = self.remaining;
        Ok(())
    }

    /// Body bytes still to write
    pub fn remaining(&self) -> usize {
        self.remaining
    }
}

/// A response head with the handler's status and headers. Invalid headers
/// surface as an error when it's encoded.
fn build<'o, 'b>(
    output: &'o mut [u8],
    version: HttpVersion,
    status: StatusCode,
    headers: &[(&str, &str)],
) -> Response<'o, 'b> {
    let mut response = Response::new(output, version);
    response.status(status);
    for (name, value) in headers {
        response.header(name, value.as_bytes());
    }
    response
}

/// Write all of `bytes`, false if the peer went away
async fn write(outgoing: &mut StreamWriter<u8>, bytes: &[u8]) -> bool {
    bytes.is_empty() || outgoing.write_all(bytes.to_vec()).await.is_empty()
}

fn to_wasi(addr: SocketAddr) -> IpSocketAddress {
    match addr {
        SocketAddr::V4(addr) => {
            let [a, b, c, d] = addr.ip().octets();
            IpSocketAddress::Ipv4(Ipv4SocketAddress {
                port: addr.port(),
                address: (a, b, c, d),
            })
        }
        SocketAddr::V6(addr) => {
            let [a, b, c, d, e, f, g, h] = addr.ip().segments();
            IpSocketAddress::Ipv6(Ipv6SocketAddress {
                port: addr.port(),
                flow_info: addr.flowinfo(),
                address: (a, b, c, d, e, f, g, h),
                scope_id: addr.scope_id(),
            })
        }
    }
}

fn from_wasi(addr: IpSocketAddress) -> SocketAddr {
    match addr {
        IpSocketAddress::Ipv4(addr) => {
            let (a, b, c, d) = addr.address;
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(a, b, c, d), addr.port))
        }
        IpSocketAddress::Ipv6(addr) => {
            let (a, b, c, d, e, f, g, h) = addr.address;
            SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::new(a, b, c, d, e, f, g, h),
                addr.port,
                addr.flow_info,
                addr.scope_id,
            ))
        }
    }
}
