//! Entry points for the fuzz targets in `octane_http/fuzz`, behind the
//! `fuzzing` feature. This is not a stable API.

#[cfg(feature = "cookies")]
use crate::cookie::{Cookie, CookieError};
use crate::path::PathBuf;
#[cfg(feature = "extended_queries")]
use crate::query::QueryValue;
use crate::request::{Headers, RequestLine};
use crate::server::Octane;
use octane_http::StatusCode;
use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::runtime::{Builder, Runtime};

/// Parse a request line from the method, url and version fields, like the
/// server does with the fields split off by the raw parser
pub fn request_line(method: &[u8], url: &[u8], version: &[u8]) -> Option<RequestLine> {
    crate::request::request_line_parse(method, url, version)
}

/// Parse a header block, the lines between the request line and the empty line
pub fn headers(block: &str) -> Option<Headers> {
    Headers::parse(block.to_owned())
}

/// The body length the server reads for a request with `headers`
pub fn content_length(headers: &Headers, max: usize) -> Result<usize, StatusCode> {
    headers.content_length(max)
}

/// Normalise a path into its chunks, `None` if it climbs above the root
pub fn path(path: &str) -> Option<Vec<String>> {
    PathBuf::parse(path).ok().map(|path| path.chunks)
}

/// Parse a query string
pub fn query(query: &str) -> HashMap<String, String> {
    crate::query::parse_query(query)
}

/// Parse a query string with arrays and objects
#[cfg(feature = "extended_queries")]
pub fn extended_query(query: &str) -> HashMap<String, QueryValue> {
    crate::query::parse_extended_query(query)
}

/// Decode one query string component
pub fn query_decode(component: &str) -> String {
    crate::query::unescape_hex(component)
}

/// Parse a request `Cookie` header
#[cfg(feature = "cookies")]
pub fn cookies(header: &str) -> Vec<Cookie<'_>> {
    Cookie::parse(header)
}

/// The `Set-Cookie` header value for `cookie`
#[cfg(feature = "cookies")]
pub fn set_cookie(cookie: &Cookie<'_>) -> Result<String, CookieError> {
    cookie.serialise()
}

/// Serve one connection whose client sends `input`, handed to the server
/// `chunk` bytes per read, and return everything the server wrote back
pub fn serve(app: &Arc<Octane>, input: &[u8], chunk: usize) -> Vec<u8> {
    thread_local! {
        static RUNTIME: Runtime = Builder::new_current_thread()
            .build()
            .expect("Unable to build tokio runtime");
    }
    let output = Arc::new(Mutex::new(Vec::new()));
    let stream = MemStream {
        input,
        chunk: chunk.max(1),
        output: Arc::clone(&output),
    };
    RUNTIME.with(|runtime| {
        // writes to the in-memory stream can't fail
        let _ = runtime.block_on(Octane::serve_http1x(stream, Arc::clone(app)));
    });
    let output = output.lock().unwrap();
    output.clone()
}

// A connection held in memory, reads hand out `input` `chunk` bytes at a
// time and writes are collected in `output`
struct MemStream<'a> {
    input: &'a [u8],
    chunk: usize,
    output: Arc<Mutex<Vec<u8>>>,
}

impl AsyncRead for MemStream<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let n = self.chunk.min(self.input.len()).min(buf.remaining());
        let (read, rest) = self.input.split_at(n);
        buf.put_slice(read);
        self.input = rest;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for MemStream<'_> {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.output.lock().unwrap().extend_from_slice(data);
        Poll::Ready(Ok(data.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
