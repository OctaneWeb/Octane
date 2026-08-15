//! Fuzz target for the streaming HTTP/1.x reader.
//!
//! Ported from the `MockIo` idea in h2's `fuzz_e2e`: the first input byte
//! drives the size of every read the mock connection yields, so the fuzzer
//! explores how `Http1xReader` accumulates a head that arrives in arbitrary
//! fragments (including 1-byte reads that split CRLFCRLF across chunks).

#![no_main]

use libfuzzer_sys::fuzz_target;
use octane_http::http1x::{raw_request::RawRequest1x, Http1xReader};

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

/// An `AsyncRead` that hands out the wire bytes `chunk` bytes at a time.
struct ChunkedIo<'a> {
    input: &'a [u8],
    chunk: usize,
}

impl<'a> AsyncRead for ChunkedIo<'a> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let n = self.chunk.min(self.input.len()).min(buf.remaining());
        let (give, rest) = self.input.split_at(n);
        buf.put_slice(give);
        self.input = rest;
        Poll::Ready(Ok(()))
    }
}

fuzz_target!(|data: &[u8]| {
    let (seed, wire) = match data.split_first() {
        Some(split) => split,
        None => return,
    };
    let chunk = (*seed as usize % 17) + 1;

    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut io = ChunkedIo { input: wire, chunk };
        let mut buffered = Vec::new();

        // The returned request borrows `buffered`, so copy its fields out
        // before inspecting the buffer again.
        let owned = Http1xReader::new(&mut io, &mut buffered).await.ok().map(|req| {
            (
                req.request_method.to_vec(),
                req.request_url.to_vec(),
                req.request_version.to_vec(),
                req.headers.to_vec(),
                req.body_remainder.to_vec(),
            )
        });

        // The reader only ever buffers bytes the connection produced, in order.
        assert!(wire.starts_with(&buffered));

        if let Some((method, url, version, headers, body_remainder)) = owned {
            // Whatever the chunking, the reader's answer must agree with a
            // direct parse of the bytes it buffered.
            let direct = RawRequest1x::parse(&buffered).unwrap();
            assert_eq!(method, direct.request_method);
            assert_eq!(url, direct.request_url);
            assert_eq!(version, direct.request_version);
            assert_eq!(headers, direct.headers);
            assert_eq!(body_remainder, direct.body_remainder);

            // And the head must agree with a direct parse of the whole wire
            // input — chunking can only affect how much body is buffered,
            // never how the head is split.
            let whole = RawRequest1x::parse(wire).unwrap();
            assert_eq!(method, whole.request_method);
            assert_eq!(url, whole.request_url);
            assert_eq!(version, whole.request_version);
            assert_eq!(headers, whole.headers);
        }
    });
});
