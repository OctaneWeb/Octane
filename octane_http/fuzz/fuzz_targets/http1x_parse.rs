//! Fuzz target for the HTTP/1.x head parser.
//!
//! Feeds arbitrary bytes to `RawRequest1x::parse` and, when parsing succeeds,
//! checks the invariants the rest of the server relies on — request-line
//! splitting, the header block and the buffered body remainder. The parser
//! must never panic (slice-index or otherwise) regardless of input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use octane_http::http1x::{find_in_slice, raw_request::RawRequest1x};
use octane_http::{CRLF, DOUBLE_CRLF};

fuzz_target!(|data: &[u8]| {
    let req = match RawRequest1x::parse(data) {
        Ok(req) => req,
        Err(_) => return,
    };

    // Structural guarantees on success: the request-line fields are non-empty.
    assert!(!req.request_method.is_empty());
    assert!(!req.request_url.is_empty());
    assert!(!req.request_version.is_empty());

    // Zero-copy guarantee: every field is a sub-slice of the input buffer.
    let start = data.as_ptr() as usize;
    let end = start + data.len();
    for field in [
        req.request_method,
        req.request_url,
        req.request_version,
        req.headers,
        req.body_remainder,
    ] {
        let p = field.as_ptr() as usize;
        assert!(p >= start && p + field.len() <= end);
    }

    // Request-line: the parsed pieces joined by the separators the parser
    // consumed must reproduce the start of the input exactly.
    let mut request_line = Vec::new();
    request_line.extend_from_slice(req.request_method);
    request_line.push(b' ');
    request_line.extend_from_slice(req.request_url);
    request_line.push(b' ');
    request_line.extend_from_slice(req.request_version);
    assert!(data.starts_with(&request_line));

    // The method and url stopped at the first SP, so they contain none; the
    // version stopped at the first CRLF, so it contains none.
    assert!(!req.request_method.contains(&b' '));
    assert!(!req.request_url.contains(&b' '));
    assert!(find_in_slice(req.request_version, CRLF).is_none());

    // Header block: the head ends at the FIRST CRLFCRLF, so the header block
    // itself can never contain one, and each CRLF-separated header line is a
    // sub-slice of the input like everything else.
    assert!(find_in_slice(req.headers, DOUBLE_CRLF).is_none());
    for line in req.headers.split(|&b| b == b'\n') {
        let p = line.as_ptr() as usize;
        assert!(p >= start && p + line.len() <= end);
    }

    // Body: everything after the head terminator is the body remainder, byte
    // for byte — it must be exactly the tail of the input.
    assert!(data.ends_with(req.body_remainder));
    let head_len = data.len() - req.body_remainder.len();
    assert!(data[..head_len].ends_with(DOUBLE_CRLF));

    // The parsed pieces can never exceed the input.
    let accounted = req.request_method.len()
        + req.request_url.len()
        + req.request_version.len()
        + req.headers.len()
        + req.body_remainder.len();
    assert!(accounted <= data.len());
});
