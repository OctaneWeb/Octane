use super::raw_request::RawRequest1x;
use super::{HeadTerminatorSearch, MAX_HEAD_SIZE};
use crate::StatusCode;

use core::future::Future;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::io;
use std::vec::Vec;
use tokio::io::{AsyncRead, ReadBuf};

// 8 kb static read buffer, the standard chunk size across nginx and friends.
const BUFFER_SIZE: usize = 8192;

/// Poll one read without retaining an 8 KiB scratch array in every suspended
/// reader future. The temporary storage lives only on the executor's stack
/// while this closure is actively being polled, and `MaybeUninit` avoids
/// clearing bytes that the I/O source immediately overwrites.
struct ReadChunk<'a, R> {
    reader: &'a mut R,
    data: &'a mut Vec<u8>,
    read_limit: usize,
}

impl<R> Future for ReadChunk<'_, R>
where
    R: AsyncRead + Unpin,
{
    type Output = io::Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        debug_assert!(this.read_limit <= BUFFER_SIZE);
        let mut storage = [MaybeUninit::<u8>::uninit(); BUFFER_SIZE];
        let mut buf = ReadBuf::uninit(&mut storage[..this.read_limit]);

        match Pin::new(&mut *this.reader).poll_read(cx, &mut buf) {
            Poll::Ready(Ok(())) => {
                let bytes = buf.filled();
                let count = bytes.len();
                this.data.extend_from_slice(bytes);
                Poll::Ready(Ok(count))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn read_chunk<'a, R>(
    reader: &'a mut R,
    data: &'a mut Vec<u8>,
    read_limit: usize,
) -> ReadChunk<'a, R>
where
    R: AsyncRead + Unpin,
{
    ReadChunk {
        reader,
        data,
        read_limit,
    }
}

/// Reads and parses the head of an HTTP/1.x request straight off an
/// [`AsyncRead`] source, with zero copies of the parsed fields.
///
/// Bytes are streamed into the caller-owned `data` buffer until the
/// `CRLFCRLF` head terminator is seen (or the peer hangs up). The returned
/// [`RawRequest1x`] then borrows directly from `data` — the method, url,
/// version, header block and already-buffered body are all sub-slices of it.
///
/// The reader is borrowed rather than consumed, so the caller still owns it
/// afterwards and can stream the rest of the body off the same source.
pub struct Http1xReader;

impl Http1xReader {
    pub async fn new<'a, R>(
        reader: &mut R,
        data: &'a mut Vec<u8>,
    ) -> Result<RawRequest1x<'a>, StatusCode>
    where
        R: AsyncRead + Unpin,
    {
        let mut terminator = HeadTerminatorSearch::default();

        // Keep reading until the head is complete. We only need the head
        // buffered to parse the request; the caller can stream the rest of the
        // body afterwards from the same reader.
        loop {
            // Only the first MAX_HEAD_SIZE bytes can contain an admissible
            // terminator. This also bounds work when a caller supplies a large
            // pre-populated buffer containing an already-buffered body.
            let searchable_len = data.len().min(MAX_HEAD_SIZE);
            if let Some(head_end) = terminator.find(&data[..searchable_len]) {
                // Reuse the delimiter position so the zero-copy parser does
                // not make a second pass over a potentially 64 KiB head.
                return RawRequest1x::parse_with_head_end(&data[..], head_end);
            }

            // At exactly the limit, an incomplete delimiter can only finish
            // beyond it. Checking before the next read keeps the cap exact and
            // independent of the source's chunk size.
            if searchable_len == MAX_HEAD_SIZE {
                return Err(StatusCode::RequestHeaderFieldsTooLarge);
            }

            let read_limit = BUFFER_SIZE.min(MAX_HEAD_SIZE - data.len());
            let n = read_chunk(reader, data, read_limit)
                .await
                .map_err(|_| StatusCode::BadRequest)?;

            if n == 0 {
                // EOF before a complete head arrived.
                return Err(StatusCode::BadRequest);
            }
        }
    }
}

/// Free-function form of [`Http1xReader::new`], kept for call sites that read
/// a request without holding onto a reader type.
pub async fn http_1x_reader<'a, R>(
    reader: &mut R,
    data: &'a mut Vec<u8>,
) -> Result<RawRequest1x<'a>, StatusCode>
where
    R: AsyncRead + Unpin,
{
    Http1xReader::new(reader, data).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DOUBLE_CRLF;
    use std::vec::Vec;

    struct ChunkedReader<'a> {
        remaining: &'a [u8],
        max_chunk: usize,
        reads: usize,
    }

    impl<'a> ChunkedReader<'a> {
        fn new(bytes: &'a [u8], max_chunk: usize) -> Self {
            assert!(max_chunk > 0);
            Self {
                remaining: bytes,
                max_chunk,
                reads: 0,
            }
        }
    }

    impl AsyncRead for ChunkedReader<'_> {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            let count = this
                .remaining
                .len()
                .min(this.max_chunk)
                .min(buf.remaining());
            let (chunk, remaining) = this.remaining.split_at(count);
            buf.put_slice(chunk);
            this.remaining = remaining;
            this.reads += 1;
            Poll::Ready(Ok(()))
        }
    }

    fn request_with_head_len(head_len: usize) -> Vec<u8> {
        const PREFIX: &[u8] = b"GET / HTTP/1.1\r\nX: ";
        assert!(head_len >= PREFIX.len() + DOUBLE_CRLF.len());

        let mut request = Vec::with_capacity(head_len);
        request.extend_from_slice(PREFIX);
        request.resize(head_len - DOUBLE_CRLF.len(), b'a');
        request.extend_from_slice(DOUBLE_CRLF);
        request
    }

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(fut)
    }

    #[test]
    fn reads_a_request_split_across_chunks() {
        // A `&[u8]` is an `AsyncRead`; reading from it advances the slice, so
        // this exercises the multi-`read` accumulation loop.
        let raw = b"GET /hi HTTP/1.1\r\nHost: a\r\n\r\nbody-bytes";
        let mut data = Vec::new();
        let mut reader = &raw[..];

        let req = block_on(Http1xReader::new(&mut reader, &mut data)).unwrap();

        assert_eq!(req.request_method, b"GET");
        assert_eq!(req.request_url, b"/hi");
        assert_eq!(req.headers, b"Host: a");
        assert_eq!(req.body_remainder, b"body-bytes");
    }

    #[test]
    fn errors_when_peer_hangs_up_before_head_completes() {
        let raw = b"GET / HTTP/1.1\r\nHost: a\r\n"; // no terminating CRLFCRLF
        let mut data = Vec::new();
        let mut reader = &raw[..];

        let result = block_on(Http1xReader::new(&mut reader, &mut data));

        assert_eq!(result, Err(StatusCode::BadRequest));
    }

    #[test]
    fn accepts_a_head_exactly_at_the_limit_for_any_chunking() {
        let request = request_with_head_len(MAX_HEAD_SIZE);

        for chunk_size in [1, 17, BUFFER_SIZE, usize::MAX] {
            let mut reader = ChunkedReader::new(&request, chunk_size);
            let mut data = Vec::new();

            let parsed = block_on(Http1xReader::new(&mut reader, &mut data)).unwrap();
            assert_eq!(parsed.request_method, b"GET");
            assert!(parsed.body_remainder.is_empty());
            assert_eq!(data.len(), MAX_HEAD_SIZE, "chunk size {}", chunk_size);
        }
    }

    #[test]
    fn rejects_a_head_one_byte_over_the_limit_for_any_chunking() {
        let request = request_with_head_len(MAX_HEAD_SIZE + 1);

        for chunk_size in [1, 17, BUFFER_SIZE, usize::MAX] {
            let mut reader = ChunkedReader::new(&request, chunk_size);
            let mut data = Vec::new();

            let result = block_on(Http1xReader::new(&mut reader, &mut data));
            assert_eq!(
                result,
                Err(StatusCode::RequestHeaderFieldsTooLarge),
                "chunk size {}",
                chunk_size
            );
            assert_eq!(data.len(), MAX_HEAD_SIZE, "chunk size {}", chunk_size);
        }
    }

    #[test]
    fn does_not_charge_an_already_buffered_body_to_the_head_limit() {
        let head = b"POST / HTTP/1.1\r\nContent-Length: 131072\r\n\r\n";
        let body_len = MAX_HEAD_SIZE * 2;
        let mut data = Vec::with_capacity(head.len() + body_len);
        data.extend_from_slice(head);
        data.resize(head.len() + body_len, b'x');
        let mut reader = ChunkedReader::new(&[], 1);

        let parsed = block_on(Http1xReader::new(&mut reader, &mut data)).unwrap();

        assert_eq!(parsed.body_remainder.len(), body_len);
        assert_eq!(reader.reads, 0);
    }

    #[test]
    fn suspended_reader_does_not_retain_the_scratch_buffer() {
        let raw = b"";
        let mut data = Vec::new();
        let mut reader = &raw[..];
        let future = Http1xReader::new(&mut reader, &mut data);
        let future_size = std::mem::size_of_val(&future);

        assert!(
            future_size < 1024,
            "reader future unexpectedly retained {} bytes",
            future_size
        );
    }
}
