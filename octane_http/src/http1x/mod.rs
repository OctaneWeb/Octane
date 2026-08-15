use crate::http1x::raw_request::RawRequest1x;
use crate::{StatusCode, DOUBLE_CRLF};

use tokio::io::{AsyncRead, AsyncReadExt};

pub mod raw_request;

// 8 kb static read buffer, the standard chunk size across nginx and friends.
const BUFFER_SIZE: usize = 8192;

// Upper bound on the request head (request-line + headers). A peer that never
// sends the CRLFCRLF terminator must not be able to grow `data` without limit.
const MAX_HEAD_SIZE: usize = 64 * 1024;

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
        let mut buf = [0u8; BUFFER_SIZE];

        // Keep reading until the head is complete. We only need the head
        // buffered to parse the request; the caller can stream the rest of the
        // body afterwards from the same reader.
        while find_in_slice(&data[..], DOUBLE_CRLF).is_none() {
            if data.len() > MAX_HEAD_SIZE {
                return Err(StatusCode::RequestHeaderFieldsTooLarge);
            }

            let n = reader
                .read(&mut buf)
                .await
                .map_err(|_| StatusCode::BadRequest)?;

            if n == 0 {
                // EOF before a complete head arrived.
                break;
            }

            data.extend_from_slice(&buf[..n]);
        }

        RawRequest1x::parse(&data[..])
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

pub fn find_in_slice<T: Eq>(haystack: &[T], needle: &[T]) -> Option<usize> {
    // naive algorithm only meant for small needles
    if needle.len() > haystack.len() {
        return None;
    }
    for i in 0..=haystack.len() - needle.len() {
        let mut matching = true;
        for j in 0..needle.len() {
            if haystack[i + j] != needle[j] {
                matching = false;
                break;
            }
        }
        if matching {
            return Some(i);
        }
    }
    None
}

// Helper function for extracting some headers
pub(crate) fn parse_without_body(data: &str) -> Option<(&str, String)> {
    let n = data.find("\r\n")?;
    let (line, rest) = data.split_at(n);
    let request_line = line;
    let headers = (&rest[2..]).to_owned();
    Some((request_line, headers))
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn find_in_slice_locates_and_misses() {
        assert_eq!(find_in_slice(b"abcXYdef", b"XY"), Some(3));
        assert_eq!(find_in_slice(b"abcdef", b"XY"), None);
        assert_eq!(find_in_slice(b"a", b"abc"), None);
    }
}
