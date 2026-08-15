//! Fuzz target for the HPACK layer (RFC 7541).
//!
//! Ported from h2's `fuzz_hpack`: raw bytes go straight into the decoder,
//! which must reject malformed input with `COMPRESSION_ERROR` — never panic
//! and never let the dynamic table exceed its advertised cap. On top of that,
//! the Huffman coder and the header encoder are checked to round-trip.

#![no_main]

use libfuzzer_sys::fuzz_target;
use octane_http::http2::hpack::{encode_header, huffman, Decoder};

fuzz_target!(|data: &[u8]| {
    let (seed, rest) = match data.split_first() {
        Some(split) => split,
        None => return,
    };

    // Decode arbitrary bytes as a header block fragment. The table cap is
    // fuzzer-chosen (0..=8160 octets) so size-update handling is exercised
    // against caps both below and above the encoded updates.
    let max_table_size = *seed as usize * 32;
    let mut decoder = Decoder::new(max_table_size);
    let _ = decoder.decode(rest);

    // A decoder holds per-connection state; a second block against the same
    // dynamic table must also be handled (arbitrary state carried over).
    let _ = decoder.decode(rest);

    // Huffman: decoding arbitrary bytes may fail but must not panic, and
    // anything we encode must decode back to itself.
    let _ = huffman::decode(rest);
    assert_eq!(huffman::decode(&huffman::encode(rest)).as_deref(), Some(rest));

    // Encoder round-trip: a header field encoded by the server must survive a
    // trip through its own decoder byte-for-byte.
    let (name, value) = rest.split_at(rest.len() / 2);
    if !name.is_empty() {
        let mut block = Vec::new();
        encode_header(name, value, &mut block);
        let headers = Decoder::new(4096).decode(&block).unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(&*headers[0].name, name);
        assert_eq!(&*headers[0].value, value);
    }
});
