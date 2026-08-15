//! Fuzz target scaffold for QUIC / HTTP/3.
//!
//! Octane has no QUIC implementation yet (there is no `quic` module in
//! `octane_http`). This target is wired into the fuzz crate so that the day a
//! QUIC packet/varint parser lands, fuzzing it is a one-line change here —
//! point the body at the parser's entry point, mirroring how `http2_frame`
//! drives `RawFrame::parse`.
//!
//! Suggested first hooks once the module exists:
//!   * long/short packet header parsing (RFC 9000 §17)
//!   * variable-length integer decoding (RFC 9000 §16) + round-trip
//!   * HTTP/3 frame parsing and QPACK (RFC 9114 / 9204), mirroring
//!     `http2_frame` / `http2_hpack`

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|_data: &[u8]| {
    // TODO(quic): octane_http::quic::Packet::parse(_data) once it exists.
});
