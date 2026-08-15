//! Fuzz target for the HTTP/2 framing layer.
//!
//! Ported from the shape of h2's `fuzz_e2e`: a mock connection whose read
//! sizes are driven by the fuzz input feeds `Http2FrameReader`, and every
//! frame that parses is pushed through the typed payload parsers (DATA,
//! HEADERS, CONTINUATION, RST_STREAM, SETTINGS, PRIORITY) plus the
//! header serialisation round-trip. None of it may panic on any input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use octane_http::http2::frame::{FrameHeader, RawFrame, FRAME_HEADER_LEN};
use octane_http::http2::payload::{
    ContinuationPayload, DataPayload, HeadersPayload, Priority, RstStream,
};
use octane_http::http2::settings::{Settings, SettingsIter};
use octane_http::http2::{starts_with_preface, Http2FrameReader, PREFACE};

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

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

/// Everything the server does with a frame once it has one, with the
/// invariants each step guarantees.
fn exercise_frame(frame: &RawFrame<'_>, settings: &mut Settings) {
    let header = frame.header;

    // The payload is exactly as long as the header advertised.
    assert_eq!(frame.payload.len(), header.length as usize);

    // Serialisation round-trip: writing the header back out and re-parsing it
    // must give the same header (the reserved stream-id bit is masked on both
    // paths).
    let mut out = [0u8; FRAME_HEADER_LEN];
    header.write_to(&mut out);
    assert_eq!(FrameHeader::parse(&out), Ok(header));

    // Typed payload views re-slice the payload; whatever they return must
    // stay inside it, and none of them may panic.
    let start = frame.payload.as_ptr() as usize;
    let end = start + frame.payload.len();
    let mut check_in_bounds = |slice: &[u8]| {
        let p = slice.as_ptr() as usize;
        assert!(p >= start && p + slice.len() <= end);
    };

    match header.frame_type.as_u8() {
        0x0 => {
            if let Ok(data) = DataPayload::parse(frame) {
                check_in_bounds(data.data);
            }
        }
        0x1 => {
            if let Ok(headers) = HeadersPayload::parse(frame) {
                check_in_bounds(headers.header_block_fragment);
            }
        }
        0x2 => {
            let _ = Priority::parse(frame.payload);
        }
        0x3 => {
            let _ = RstStream::parse(frame);
        }
        0x4 => {
            if let Ok(iter) = SettingsIter::new(frame.payload) {
                // Iterating arbitrary parameters must not panic.
                for _param in iter {}
            }
            // Applying a whole SETTINGS frame validates ack/stream-id/values.
            let _ = settings.apply_frame(frame);
        }
        0x9 => {
            if let Ok(cont) = ContinuationPayload::parse(frame) {
                check_in_bounds(cont.header_block_fragment);
            }
        }
        _ => {}
    }
}

fuzz_target!(|data: &[u8]| {
    let (seed, wire) = match data.split_first() {
        Some(split) => split,
        None => return,
    };
    let chunk = (*seed as usize % 31) + 1;

    // Sync path: parse frames back-to-back straight out of the raw input,
    // exactly as a buffer of pipelined frames would be consumed.
    {
        let mut settings = Settings::default();
        let mut rest = wire;
        while let Ok(frame) = RawFrame::parse(rest) {
            exercise_frame(&frame, &mut settings);
            rest = &rest[FRAME_HEADER_LEN + frame.payload.len()..];
        }
        let _ = starts_with_preface(wire);
    }

    // Async path: the same bytes arriving over a connection in fuzzer-chosen
    // fragments, behind the mandatory client preface.
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut full = Vec::with_capacity(PREFACE.len() + wire.len());
        full.extend_from_slice(PREFACE);
        full.extend_from_slice(wire);
        let mut io = ChunkedIo { input: &full, chunk };

        if Http2FrameReader::read_preface(&mut io).await.is_err() {
            return;
        }

        let mut settings = Settings::default();
        let mut buffered = Vec::new();
        // The frame reader validates the advertised length against
        // SETTINGS_MAX_FRAME_SIZE before buffering a payload.
        while let Ok(frame) =
            Http2FrameReader::read_frame(&mut io, &mut buffered, settings.max_frame_size).await
        {
            let mut fresh = Settings::default();
            exercise_frame(&frame, &mut fresh);
            // `apply_frame` is only defined for SETTINGS frames; the caller
            // (like a real connection loop) dispatches by type first.
            if frame.header.frame_type.as_u8() == 0x4 {
                let _ = settings.apply_frame(&frame);
            }
        }
    });
});
