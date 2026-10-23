//! Fuzz target for the request-line parser.
//!
//! The first two input bytes give the lengths of the method and url fields
//! and the rest is the version, so each field can hold anything the raw
//! parser would hand over: spaces, control characters, broken UTF-8. Parsing
//! must never panic, and an accepted request line must give the router a
//! normalised path, the query kept apart, and the method and version read
//! exactly from their fields.

#![no_main]

use libfuzzer_sys::fuzz_target;
use octane::fuzzing;
use octane::request::RequestMethod;
use octane_http::HttpVersion;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let rest = &data[2..];
    let (method, rest) = rest.split_at((data[0] as usize).min(rest.len()));
    let (url, version) = rest.split_at((data[1] as usize).min(rest.len()));

    let line = match fuzzing::request_line(method, url, version) {
        Some(line) => line,
        None => return,
    };
    // only UTF-8 fields are accepted
    let method = std::str::from_utf8(method).unwrap();
    let url = std::str::from_utf8(url).unwrap();
    let version = std::str::from_utf8(version).unwrap();

    // `faithful` accepts exactly three space separated tokens and an
    // `HTTP/` version
    assert!(!method.contains(' ') && !url.contains(' ') && !version.contains(' '));
    assert!(version.starts_with("HTTP/"));

    // the query isn't part of the routed path
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    assert_eq!(line.query, query);

    // the router sees normalised chunks, and normalising them again is a
    // no-op; a path without dot or empty segments passes through unchanged
    let chunks = &line.path.chunks;
    for chunk in chunks {
        assert!(!chunk.is_empty() && chunk != "." && chunk != "..");
        assert!(!chunk.contains('/') && !chunk.contains('?'));
    }
    assert_eq!(fuzzing::path(&chunks.join("/")).as_ref(), Some(chunks));
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.iter().all(|s| *s != "." && *s != "..") {
        assert_eq!(segments, *chunks);
    }

    let expected_method = match method {
        "GET" => RequestMethod::Get,
        "POST" => RequestMethod::Post,
        "PUT" => RequestMethod::Put,
        "DELETE" => RequestMethod::Delete,
        "OPTIONS" => RequestMethod::Options,
        "HEAD" => RequestMethod::Head,
        "TRACE" => RequestMethod::Trace,
        "CONNECT" => RequestMethod::Connect,
        "PATCH" => RequestMethod::Patch,
        _ => RequestMethod::None,
    };
    assert_eq!(line.method, expected_method);
    let expected_version = match version {
        "HTTP/1.1" => HttpVersion::Http11,
        "HTTP/1.0" => HttpVersion::Http10,
        "HTTP/2.0" => HttpVersion::Http02,
        "HTTP/0.9" => HttpVersion::Http09,
        _ => HttpVersion::HttpInvalid,
    };
    assert_eq!(line.version, expected_version);
});
