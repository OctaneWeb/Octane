//! Fuzz target for path normalisation (`.`, `..` and empty segments).
//!
//! Normalising must never panic or climb above the root, the chunks it
//! returns must be plain segments, and normalising them again must be a
//! no-op.

#![no_main]

use libfuzzer_sys::fuzz_target;
use octane::fuzzing;

fuzz_target!(|data: &[u8]| {
    let path = match std::str::from_utf8(data) {
        Ok(path) => path,
        Err(_) => return,
    };
    let chunks = match fuzzing::path(path) {
        Some(chunks) => chunks,
        // only a `..` with nothing left to pop is refused
        None => return assert!(path.split('/').any(|s| s == "..")),
    };
    for chunk in &chunks {
        assert!(!chunk.is_empty() && chunk != "." && chunk != "..");
        assert!(!chunk.contains('/'));
    }
    assert_eq!(fuzzing::path(&chunks.join("/")), Some(chunks.clone()));
    // each `..` removes one segment, so the chunks are never more than the
    // plain segments minus the `..`s
    let plain = path
        .split('/')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .count();
    let parents = path.split('/').filter(|s| *s == "..").count();
    assert_eq!(chunks.len(), plain - parents);
});
