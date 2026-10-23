//! Fuzz target for the header-block parser and `Content-Length`.
//!
//! The input is a header block, the lines between the request line and the
//! empty line. Parsing must never panic, every accepted field must be well
//! formed, repeated fields must be merged in order, and the body length must
//! follow RFC 9112 section 6.3: missing means 0, repeats must agree, anything
//! but digits is a 400 and a length over the limit is a 413.

#![no_main]

use libfuzzer_sys::fuzz_target;
use octane::fuzzing;
use octane::StatusCode;
use std::collections::HashMap;

const MAX_BODY: usize = 1 << 20;

fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

fn is_ows(c: char) -> bool {
    c == ' ' || c == '\t'
}

fuzz_target!(|data: &[u8]| {
    let block = match std::str::from_utf8(data) {
        Ok(block) => block,
        Err(_) => return,
    };
    let headers = match fuzzing::headers(block) {
        Some(headers) => headers,
        None => return,
    };

    // every line became one field, in order, holding exactly what was sent
    let lines: Vec<&str> = if block.is_empty() {
        Vec::new()
    } else {
        block.split("\r\n").collect()
    };
    assert_eq!(headers.raw.len(), lines.len());
    for (field, line) in headers.raw.iter().zip(&lines) {
        let (name, value) = (field.name(), field.value());
        assert!(!name.is_empty() && name.bytes().all(is_tchar), "{:?}", name);
        assert!(!value
            .chars()
            .any(|c| (c <= '\x1f' || c == '\x7f') && c != '\t'));
        let (line_name, line_value) = line.split_once(':').unwrap();
        assert_eq!(name, line_name);
        assert_eq!(value, line_value.trim_matches(is_ows));
    }

    // repeated fields are joined in order, cookies with "; "
    let mut joined: HashMap<String, String> = HashMap::new();
    for field in &headers.raw {
        let name = field.name().to_ascii_lowercase();
        let sep = if name == "cookie" { "; " } else { ", " };
        joined
            .entry(name)
            .and_modify(|v| {
                v.push_str(sep);
                v.push_str(&field.value());
            })
            .or_insert_with(|| field.value());
    }
    assert_eq!(headers.parsed, joined);

    let length = fuzzing::content_length(&headers, MAX_BODY);
    let value = match headers.parsed.get("content-length") {
        Some(value) => value,
        None => return assert_eq!(length, Ok(0)),
    };
    let members: Vec<&str> = value.split(',').map(|m| m.trim_matches(is_ows)).collect();
    let digits_only = members
        .iter()
        .all(|m| !m.is_empty() && m.bytes().all(|b| b.is_ascii_digit()));
    let numbers: Vec<&str> = members.iter().map(|m| m.trim_start_matches('0')).collect();
    let agree = numbers.windows(2).all(|pair| pair[0] == pair[1]);
    let number = numbers[0];
    let too_large = number.len() > 7 || number.parse::<usize>().unwrap_or(0) > MAX_BODY;
    match length {
        Ok(n) => {
            assert!(digits_only && agree && !too_large, "{:?} -> {}", value, n);
            assert_eq!(n, number.parse::<usize>().unwrap_or(0));
        }
        Err(StatusCode::BadRequest) => assert!(!digits_only || !agree, "{:?}", value),
        Err(StatusCode::PayloadTooLarge) => {
            assert!(digits_only && agree && too_large, "{:?}", value)
        }
        Err(other) => panic!("unexpected {:?} for {:?}", other, value),
    }
});
