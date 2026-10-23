//! Fuzz target for query strings and the component decoder.
//!
//! Parsing arbitrary query strings, plain or extended (`a[]=1&b[k]=2`), must
//! never panic and must keep every named pair. The decoder is checked against
//! two encoders: every byte as `%XX` (either case), and the form encoding
//! browsers use (space as `+`, unreserved bytes as they are). Both must decode
//! back to the original text.

#![no_main]

use libfuzzer_sys::fuzz_target;
use octane::fuzzing;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(input) => input,
        Err(_) => return,
    };

    let query = fuzzing::query(input);
    let _ = fuzzing::extended_query(input);
    for pair in input.split('&') {
        let name = pair.split_once('=').map_or(pair, |(name, _)| name);
        if !name.is_empty() {
            assert!(
                query.contains_key(&fuzzing::query_decode(name)),
                "{:?}",
                pair
            );
        }
    }

    // text without escapes comes back unchanged
    if !input.contains(|c| c == '%' || c == '+') {
        assert_eq!(fuzzing::query_decode(input), input);
    }

    let upper: String = input.bytes().map(|b| format!("%{:02X}", b)).collect();
    assert_eq!(fuzzing::query_decode(&upper), input);
    let lower: String = input.bytes().map(|b| format!("%{:02x}", b)).collect();
    assert_eq!(fuzzing::query_decode(&lower), input);

    let form: String = input
        .bytes()
        .map(|b| match b {
            b' ' => "+".to_owned(),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'*' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect();
    assert_eq!(fuzzing::query_decode(&form), input);
});
