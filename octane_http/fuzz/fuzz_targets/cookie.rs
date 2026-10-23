//! Fuzz target for cookies: the request `Cookie` header parser and the
//! `Set-Cookie` serialiser.
//!
//! The input is used twice. As a raw `Cookie` header, every cookie parsed
//! from it must be a token name with a value free of control characters,
//! borrowed from the header, and each one that is valid to send must survive
//! a trip through `Set-Cookie` and back. As a source of structured data, it
//! builds a cookie with arbitrary attributes through the public builder,
//! whose `Set-Cookie` header must agree with `validate`, never hold a
//! character that could break the header line, keep every attribute that was
//! set and parse back to the same name and value.

#![no_main]

use libfuzzer_sys::arbitrary::{self, Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use octane::cookie::{Cookie, SameSite};
use octane::fuzzing;
use std::time::{Duration, UNIX_EPOCH};

fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

fn is_ctl(c: char) -> bool {
    c <= '\x1f' || c == '\x7f'
}

fn is_ows(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fuzz_target!(|data: &[u8]| {
    if let Ok(header) = std::str::from_utf8(data) {
        check_parse(header);
    }
    let _ = check_build(&mut Unstructured::new(data));
});

fn check_parse(header: &str) {
    let cookies = fuzzing::cookies(header);
    let (start, end) = (
        header.as_ptr() as usize,
        header.as_ptr() as usize + header.len(),
    );
    for cookie in &cookies {
        let (name, value) = cookie.name_value();
        assert!(!name.is_empty() && name.bytes().all(is_tchar), "{:?}", name);
        assert!(
            !value.chars().any(is_ctl) && !value.contains(';'),
            "{:?}",
            value
        );
        assert_eq!(name, name.trim_matches(is_ows));
        // nothing was copied
        for s in [name, value] {
            let p = s.as_ptr() as usize;
            assert!(p >= start && p + s.len() <= end);
        }
        if let Ok(set_cookie) = fuzzing::set_cookie(&Cookie::new(name, value)) {
            assert_eq!(set_cookie, format!("{}={}", name, value));
            let back = fuzzing::cookies(&set_cookie);
            assert_eq!(back.len(), 1);
            assert_eq!(back[0].name_value(), (name, unquote(value)));
        }
    }
    // the header is split on `;` only, so there can't be more cookies than pairs
    assert!(cookies.len() <= header.split(';').count());
}

fn check_build(u: &mut Unstructured<'_>) -> arbitrary::Result<()> {
    let name = <&str>::arbitrary(u)?;
    let value = <&str>::arbitrary(u)?;
    let mut builder = Cookie::build(name, value);
    if bool::arbitrary(u)? {
        builder = builder.path(<&str>::arbitrary(u)?);
    }
    if bool::arbitrary(u)? {
        builder = builder.domain(<&str>::arbitrary(u)?);
    }
    if bool::arbitrary(u)? {
        builder = builder.max_age(Duration::from_secs(u64::arbitrary(u)?));
    }
    if bool::arbitrary(u)? {
        // up to ~35000 years past the epoch, beyond what Expires can carry
        let secs = u64::arbitrary(u)? % (1 << 40);
        builder = builder.expires(UNIX_EPOCH + Duration::from_secs(secs));
    }
    builder = match u8::arbitrary(u)? % 4 {
        0 => builder.same_site(SameSite::Strict),
        1 => builder.same_site(SameSite::Lax),
        2 => builder.same_site(SameSite::None),
        _ => builder,
    };
    let cookie = builder
        .secure(bool::arbitrary(u)?)
        .http_only(bool::arbitrary(u)?)
        .partitioned(bool::arbitrary(u)?)
        .finish();

    let header = match (cookie.validate(), fuzzing::set_cookie(&cookie)) {
        (Ok(()), Ok(header)) => header,
        (Err(a), Err(b)) => {
            assert_eq!(a, b);
            return Ok(());
        }
        (valid, header) => panic!("validate: {:?}, set_cookie: {:?}", valid, header),
    };
    assert!(!header.chars().any(is_ctl), "{:?}", header);

    // it starts with the pair and parses back to it
    let pair = format!("{}={}", name, value);
    assert!(header == pair || header.starts_with(&format!("{}; ", pair)));
    let back = fuzzing::cookies(&header);
    assert_eq!(back[0].name_value(), (name, unquote(value)));

    // every attribute that was set is there
    let attrs: Vec<&str> = header.split("; ").skip(1).collect();
    let has = |attr: &str| attrs.iter().any(|a| *a == attr);
    assert_eq!(has("Secure"), cookie.secure());
    assert_eq!(has("HttpOnly"), cookie.http_only());
    assert_eq!(has("Partitioned"), cookie.partitioned());
    if let Some(path) = cookie.path() {
        assert!(has(&format!("Path={}", path)));
    }
    if let Some(domain) = cookie.domain() {
        assert!(has(&format!("Domain={}", domain)));
    }
    if let Some(max_age) = cookie.max_age() {
        assert!(has(&format!("Max-Age={}", max_age.as_secs())));
    }
    match cookie.same_site() {
        Some(SameSite::Strict) => assert!(has("SameSite=Strict")),
        Some(SameSite::Lax) => assert!(has("SameSite=Lax")),
        Some(SameSite::None) => assert!(has("SameSite=None")),
        None => assert!(!header.contains("SameSite")),
    }
    if cookie.expires().is_some() {
        // an IMF-fixdate like "Tue, 15 Sep 2026 08:30:05 GMT"
        let expires = attrs.iter().find(|a| a.starts_with("Expires=")).unwrap();
        let date = &expires["Expires=".len()..];
        assert_eq!(date.len(), 29, "{:?}", date);
        assert!(date.ends_with(" GMT"), "{:?}", date);
    }

    // the rules browsers enforce hold for everything that is sent
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("__secure-") || cookie.same_site() == Some(SameSite::None) {
        assert!(cookie.secure());
    }
    if lower.starts_with("__host-") {
        assert!(cookie.secure() && cookie.path() == Some("/") && cookie.domain().is_none());
    }
    Ok(())
}
