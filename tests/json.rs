extern crate octane;
use octane::json::{self, Value};

#[test]
fn success_string() {
    // Parsing should work as expected.
    let (string, rest) = json::parse_string(r#""as\n\t\u0041d\"f"foo"#).unwrap();
    assert_eq!(string, "as\n\tAd\"f".to_string());
    assert_eq!(rest, "foo");
}

#[test]
fn failure_string() {
    // No closing quote should result in an error.
    assert!(json::parse_string(r#""asdffoo"#).is_none());
    // Unicode escapes should handle bad cases.
    assert!(json::parse_string(r#""abc\u004""#).is_none());
    assert!(json::parse_string(r#""abc\ux123""#).is_none());
    // Non-existent escapes should error.
    assert!(json::parse_string(r#""\c""#).is_none());
}

#[test]
fn success_bool() {
    // Parsing should work as expected.
    assert_eq!((true, " asdf"), json::parse_bool("true asdf").unwrap());
    assert_eq!((false, " asdf"), json::parse_bool("false asdf").unwrap());
    assert!(json::parse_bool("asdf").is_none());
}
#[test]
fn success_null() {
    // Parsing should work as expected.
    assert_eq!(((), " asdf"), json::parse_null("null asdf").unwrap());
    assert!(json::parse_null("asdf").is_none());
}