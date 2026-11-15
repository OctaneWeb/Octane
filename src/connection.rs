//! The connection the HTTP/1.x server reads requests from and writes
//! responses to. Responses are held back while there are requests left to
//! answer and go out together once the server would wait on the client, so
//! the answers to a batch of pipelined requests cost one write rather than
//! one each.
use std::io;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

// Past this many held bytes, writing doesn't wait for the server to run out
// of requests. Writes at least this large on their own skip the buffer, and
// a buffer that grew past it is released once written, rather than kept for
// as long as the connection stays open.
const WRITE_BUFFER: usize = 64 * 1024;

pub(crate) struct Connection<S> {
    io: S,
    // held output, `out[written..]` is what hasn't gone out yet
    out: Vec<u8>,
    written: usize,
    // something was written to `io` since it was last flushed, which over
    // TLS can still be sitting in a buffer
    unflushed: bool,
}

impl<S: AsyncWrite + Unpin> Connection<S> {
    pub(crate) fn new(io: S) -> Self {
        Self {
            io,
            out: Vec::new(),
            written: 0,
            unflushed: false,
        }
    }

    /// Whether there's output that hasn't reached the client yet
    pub(crate) fn has_pending(&self) -> bool {
        !self.out.is_empty() || self.unflushed
    }

    /// The output held for the client, to append a response to. Anything
    /// not written yet stays ahead of it.
    pub(crate) fn buffer(&mut self) -> &mut Vec<u8> {
        &mut self.out
    }

    // Write and flush everything held
    fn poll_send(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        while self.written < self.out.len() {
            let n = ready!(Pin::new(&mut self.io).poll_write(cx, &self.out[self.written..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
            }
            self.written += n;
            self.unflushed = true;
        }
        // keeps its allocation for the next responses, unless one of them
        // made it large
        if self.out.capacity() > WRITE_BUFFER {
            self.out = Vec::new();
        } else {
            self.out.clear();
        }
        self.written = 0;
        if self.unflushed {
            ready!(Pin::new(&mut self.io).poll_flush(cx))?;
            self.unflushed = false;
        }
        Poll::Ready(Ok(()))
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for Connection<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // everything answered so far goes out before waiting on the client,
        // which may be waiting for it before it sends more
        ready!(this.poll_send(cx))?;
        Pin::new(&mut this.io).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Connection<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if this.out.len() >= WRITE_BUFFER || (buf.len() >= WRITE_BUFFER && !this.out.is_empty()) {
            ready!(this.poll_send(cx))?;
        }
        if buf.len() >= WRITE_BUFFER {
            // nothing is held now, so the order is kept without copying it
            let n = ready!(Pin::new(&mut this.io).poll_write(cx, buf))?;
            this.unflushed = true;
            return Poll::Ready(Ok(n));
        }
        this.out.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut().poll_send(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_send(cx))?;
        Pin::new(&mut this.io).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Records every write and flush, and reads hand out `input`
    #[derive(Default)]
    struct Recorder {
        input: Vec<u8>,
        writes: Vec<Vec<u8>>,
        flushes: usize,
        shut: bool,
    }

    impl AsyncRead for Recorder {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let n = self.input.len().min(buf.remaining());
            let rest = self.input.split_off(n);
            buf.put_slice(&self.input);
            self.input = rest;
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for Recorder {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.writes.push(buf.to_vec());
            Poll::Ready(Ok(buf.len()))
        }
        fn poll_flush(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            self.flushes += 1;
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            self.shut = true;
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn responses_go_out_together_before_the_next_read() {
        let mut conn = Connection::new(Recorder {
            input: b"next".to_vec(),
            ..Recorder::default()
        });
        for response in [&b"one "[..], b"two ", b"three"].iter() {
            conn.write_all(response).await.unwrap();
        }
        assert!(conn.io.writes.is_empty());
        let mut buf = [0u8; 4];
        conn.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"next");
        assert_eq!(conn.io.writes, vec![b"one two three".to_vec()]);
        assert_eq!(conn.io.flushes, 1);
        // nothing held, nothing to flush
        conn.flush().await.unwrap();
        assert_eq!(conn.io.flushes, 1);
    }

    #[tokio::test]
    async fn a_large_buffer_is_released_once_written() {
        let mut conn = Connection::new(Recorder::default());
        // many small responses add up past the limit, without one of them
        // finding the buffer full
        for _ in 0..(WRITE_BUFFER / 1000 + 1) {
            conn.write_all(&[b'x'; 1000]).await.unwrap();
        }
        assert!(conn.io.writes.is_empty() && conn.out.len() > WRITE_BUFFER);
        conn.flush().await.unwrap();
        assert_eq!(conn.out.capacity(), 0);
        // a small one keeps its buffer
        conn.write_all(b"small").await.unwrap();
        conn.flush().await.unwrap();
        assert!(conn.out.capacity() > 0);
    }

    #[tokio::test]
    async fn large_writes_keep_their_order_and_skip_the_buffer() {
        let mut conn = Connection::new(Recorder::default());
        let large = vec![b'x'; WRITE_BUFFER];
        conn.write_all(b"head").await.unwrap();
        conn.write_all(&large).await.unwrap();
        conn.write_all(b"tail").await.unwrap();
        assert_eq!(conn.io.writes, vec![b"head".to_vec(), large]);
        conn.shutdown().await.unwrap();
        assert_eq!(conn.io.writes.last().unwrap(), b"tail");
        assert!(conn.io.shut);
    }
}
