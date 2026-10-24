//! A transport-agnostic HTTP/1.x server loop that never allocates.
//!
//! The loop only needs something that can read and write bytes, a
//! [`Connection`]. Accepting connections is left to the platform: a Linux
//! socket, a WASI socket, a smoltcp/embassy TCP socket or an in-memory pipe
//! all work the same way.
//!
//! ```
//! use octane_http::config::Config;
//! use octane_http::http1x::Method;
//! use octane_http::server::Connection;
//! use octane_http::StatusCode;
//!
//! // A connection that replays a request and records the response
//! struct Pipe<'a> {
//!     input: &'a [u8],
//!     output: [u8; 256],
//!     written: usize,
//! }
//!
//! impl Connection for Pipe<'_> {
//!     type Error = ();
//!
//!     fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
//!         let n = self.input.len().min(buf.len());
//!         buf[..n].copy_from_slice(&self.input[..n]);
//!         self.input = &self.input[n..];
//!         Ok(n)
//!     }
//!
//!     fn write_all(&mut self, buf: &[u8]) -> Result<(), ()> {
//!         self.output[self.written..self.written + buf.len()].copy_from_slice(buf);
//!         self.written += buf.len();
//!         Ok(())
//!     }
//! }
//!
//! let mut conn = Pipe {
//!     input: b"GET /hello HTTP/1.1\r\nHost: a\r\nConnection: close\r\n\r\n",
//!     output: [0; 256],
//!     written: 0,
//! };
//! // the buffer sizes have to be chosen, there is no default without `std`
//! let config = Config::<1024, 256>::new();
//!
//! config.serve(&mut conn, |req, res| {
//!     match (req.method, req.path) {
//!         (Method::Get, "/hello") => res.header("Content-Type", b"text/plain").body(b"Hello from no_std!"),
//!         _ => res.status(StatusCode::NotFound).body(b"Not Found"),
//!     };
//! })
//! .unwrap();
//!
//! assert!(conn.output[..conn.written].ends_with(b"\r\n\r\nHello from no_std!"));
//! ```

use crate::config::{Buffers, Config};
use crate::http1x::{Method, Request, RequestBuffer, Status};
use crate::response::{error_response, Response};
use crate::StatusCode;

/// A bidirectional byte stream, typically one accepted TCP connection
pub trait Connection {
    type Error;

    /// Read into `buf`, returning how many bytes were read. `0` means the
    /// peer closed its side of the connection.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Write all of `buf`
    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error>;

    /// Flush anything the transport buffered, called after every response
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<C: Connection + ?Sized> Connection for &mut C {
    type Error = C::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        (**self).read(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        (**self).write_all(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        (**self).flush()
    }
}

/// Adapts any `std::io::Read + Write` stream, like a `std::net::TcpStream`
/// (also on WASI), into a [`Connection`]
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct StdIo<T>(pub T);

#[cfg(feature = "std")]
impl<T: std::io::Read + std::io::Write> Connection for StdIo<T> {
    type Error = std::io::Error;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        loop {
            match self.0.read(buf) {
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                result => return result,
            }
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.0.write_all(buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.0.flush()
    }
}

impl<const INPUT: usize, const OUTPUT: usize> Config<INPUT, OUTPUT> {
    /// Serve requests on `conn` until either side closes it, with the
    /// buffers on the stack ([`Config::stack_size`] bytes).
    ///
    /// Requests that don't parse or don't fit in `INPUT` get an error
    /// response and the connection is closed: a malformed request gets a
    /// `400`, as does an HTTP/1.1 request without exactly one `Host` header.
    /// Pipelined requests are answered in order.
    ///
    /// Timeouts are the transport's job: a `read` that never returns stalls
    /// this connection.
    pub fn serve<C, H>(&self, conn: C, handler: H) -> Result<(), C::Error>
    where
        C: Connection,
        H: for<'r, 'o> FnMut(&Request<'r>, &mut Response<'o, 'r>),
    {
        let mut buffers = self.buffers();
        self.serve_with(conn, &mut buffers, handler)
    }

    /// Like [`Config::serve`], with buffers that live elsewhere, like a
    /// `static` on a target whose stack is too small to hold them
    pub fn serve_with<C, H>(
        &self,
        conn: C,
        buffers: &mut Buffers<INPUT, OUTPUT>,
        handler: H,
    ) -> Result<(), C::Error>
    where
        C: Connection,
        H: for<'r, 'o> FnMut(&Request<'r>, &mut Response<'o, 'r>),
    {
        serve_connection(conn, &mut buffers.input, &mut buffers.output, handler)
    }
}

fn serve_connection<C, H>(
    mut conn: C,
    input: &mut [u8],
    output: &mut [u8],
    mut handler: H,
) -> Result<(), C::Error>
where
    C: Connection,
    H: for<'r, 'o> FnMut(&Request<'r>, &mut Response<'o, 'r>),
{
    let mut requests = RequestBuffer::new(input);
    loop {
        let close = match requests.poll() {
            Ok(Status::Complete(request)) => respond(&mut conn, output, &request, &mut handler)?,
            Ok(Status::Partial) => {
                let n = conn.read(requests.unfilled())?;
                if n == 0 {
                    // the peer is done, whether or not it was mid-request
                    return Ok(());
                }
                requests.advance(n);
                continue;
            }
            Err(status) => {
                send_error(&mut conn, output, status)?;
                true
            }
        };
        if close {
            return Ok(());
        }
        requests.consume();
    }
}

/// Run the handler and write its response, returning whether to close
fn respond<'r, C, H>(
    conn: &mut C,
    output: &mut [u8],
    request: &Request<'r>,
    handler: &mut H,
) -> Result<bool, C::Error>
where
    C: Connection,
    H: for<'o> FnMut(&Request<'r>, &mut Response<'o, 'r>),
{
    let mut response = Response::new(output, request.version);
    handler(request, &mut response);
    match response.finish(request.method == Method::Head, request.keep_alive()) {
        Ok(encoded) => {
            conn.write_all(encoded.head)?;
            if !encoded.body.is_empty() {
                conn.write_all(encoded.body)?;
            }
            conn.flush()?;
            Ok(encoded.close)
        }
        Err(_) => {
            send_error(conn, output, StatusCode::InternalServerError)?;
            Ok(true)
        }
    }
}

fn send_error<C: Connection>(
    conn: &mut C,
    output: &mut [u8],
    status: StatusCode,
) -> Result<(), C::Error> {
    // an output buffer too small for even this gets the connection closed
    // without a response
    if let Ok(head) = error_response(output, status) {
        conn.write_all(head)?;
        conn.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds `input` in `chunk` sized reads and collects what is written
    struct Pipe<'a> {
        input: &'a [u8],
        chunk: usize,
        output: [u8; 2048],
        written: usize,
    }

    impl<'a> Pipe<'a> {
        fn new(input: &'a [u8], chunk: usize) -> Self {
            Self {
                input,
                chunk,
                output: [0; 2048],
                written: 0,
            }
        }

        fn written(&self) -> &[u8] {
            &self.output[..self.written]
        }
    }

    impl Connection for Pipe<'_> {
        type Error = ();

        fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
            let n = self.input.len().min(buf.len()).min(self.chunk);
            buf[..n].copy_from_slice(&self.input[..n]);
            self.input = &self.input[n..];
            Ok(n)
        }

        fn write_all(&mut self, buf: &[u8]) -> Result<(), ()> {
            let end = self.written + buf.len();
            self.output
                .get_mut(self.written..end)
                .ok_or(())?
                .copy_from_slice(buf);
            self.written = end;
            Ok(())
        }
    }

    fn serve(input: &[u8], chunk: usize, input_len: usize) -> Pipe<'_> {
        let mut conn = Pipe::new(input, chunk);
        let mut input = [0u8; 512];
        let mut output = [0u8; 256];
        serve_connection(
            &mut conn,
            &mut input[..input_len],
            &mut output,
            |req, res| {
                match (req.method, req.path) {
                    (Method::Get | Method::Head, "/") => res.body(b"root"),
                    (Method::Post, "/echo") => res.body(req.body),
                    (Method::Get, "/bad") => res.header("Bad Name", b"x"),
                    _ => res.status(StatusCode::NotFound),
                };
            },
        )
        .unwrap();
        conn
    }

    #[test]
    fn serves_pipelined_keep_alive_requests() {
        let input = b"GET / HTTP/1.1\r\nHost: a\r\n\r\n\
POST /echo HTTP/1.1\r\nHost: a\r\nContent-Length: 4\r\n\r\nping\
HEAD / HTTP/1.1\r\nHost: a\r\nConnection: close\r\n\r\n\
GET /never HTTP/1.1\r\nHost: a\r\n\r\n";
        let expected: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nroot\
HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nping\
HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\n";
        for chunk in [1, 7, usize::MAX] {
            assert_eq!(
                serve(input, chunk, 512).written(),
                expected,
                "chunk {}",
                chunk
            );
        }
    }

    #[test]
    fn errors_close_the_connection() {
        let bad_request =
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        assert_eq!(
            serve(
                b"GET / HTTP/1.1\r\n\r\nGET / HTTP/1.1\r\nHost: a\r\n\r\n",
                64,
                512
            )
            .written(),
            &bad_request[..]
        );
        assert_eq!(
            serve(b"GET /bad HTTP/1.1\r\nHost: a\r\n\r\n", 64, 512).written(),
            &b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"[..]
        );
        assert_eq!(
            serve(
                b"POST /echo HTTP/1.1\r\nHost: a\r\nContent-Length: 100\r\n\r\n",
                64,
                64
            )
            .written(),
            &b"HTTP/1.1 413 Payload Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"[..]
        );
    }

    #[test]
    fn http11_needs_exactly_one_host() {
        let bad_request: &[u8] =
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        for request in [
            &b"GET / HTTP/1.1\r\n\r\n"[..],
            b"GET / HTTP/1.1\r\nHost: a\r\nHost: a\r\n\r\n",
            b"GET / HTTP/1.1\r\nHost: a\r\nX: 1\r\nhOST: b\r\n\r\n",
        ] {
            assert_eq!(serve(request, 64, 512).written(), bad_request);
        }
        // HTTP/1.0 predates Host
        assert!(serve(b"GET / HTTP/1.0\r\n\r\n", 64, 512)
            .written()
            .starts_with(b"HTTP/1.0 200 OK"));
    }

    #[test]
    fn config_bounds_requests_by_its_input_size() {
        const CONFIG: Config<128, 128> = Config::new();
        let mut buffers = CONFIG.buffers();
        let handler = |req: &Request<'_>, res: &mut Response<'_, '_>| {
            res.body(if req.body.is_empty() {
                b"empty"
            } else {
                b"body"
            });
        };

        let mut conn = Pipe::new(
            b"GET / HTTP/1.1\r\nHost: a\r\nConnection: close\r\n\r\n",
            64,
        );
        CONFIG.serve_with(&mut conn, &mut buffers, handler).unwrap();
        assert!(conn.written().ends_with(b"\r\n\r\nempty"));

        // 47 bytes of head leave 81 for the body
        let mut conn = Pipe::new(
            b"POST / HTTP/1.1\r\nHost: a\r\nContent-Length: 82\r\n\r\n",
            64,
        );
        CONFIG.serve(&mut conn, handler).unwrap();
        assert!(conn
            .written()
            .starts_with(b"HTTP/1.1 413 Payload Too Large"));
    }

    #[test]
    fn http10_closes_by_default() {
        assert_eq!(
            serve(b"GET / HTTP/1.0\r\n\r\nGET / HTTP/1.0\r\n\r\n", 64, 512).written(),
            &b"HTTP/1.0 200 OK\r\nContent-Length: 4\r\n\r\nroot"[..]
        );
    }

    #[test]
    fn peer_closing_mid_request_ends_quietly() {
        assert_eq!(serve(b"GET / HTTP/1.1\r\nHo", 64, 512).written(), b"");
    }

    #[cfg(feature = "std")]
    #[test]
    fn serves_a_std_tcp_stream() {
        use std::io::{Read, Write};
        use std::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            crate::config::DefaultConfig::new()
                .serve(StdIo(stream), |_, res| {
                    res.body(b"over tcp");
                })
                .unwrap();
        });

        let mut client = TcpStream::connect(addr).unwrap();
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: a\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut response = std::vec::Vec::new();
        client.read_to_end(&mut response).unwrap();
        server.join().unwrap();
        assert!(response.ends_with(b"\r\n\r\nover tcp"));
    }
}
