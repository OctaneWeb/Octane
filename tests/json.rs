use octane::json::{self, FromJSON, ToJSON, Value};
pub mod common;
use crate::common::*;
use std::convert::TryFrom;

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

#[test]
fn success_number() {
    // Parsing should work as expected.
    assert_eq!(
        (-1.23e-2, "asdf"),
        json::parse_number("-1.23e-2asdf").unwrap()
    );
    assert!(json::parse_number("1..").is_none());
}

#[test]
fn success_element() {
    // Parsing should work as expected.
    let (val, rest) = json::parse_element(" 123 asdf").unwrap();
    assert_eq!(rest, "asdf");
    assert!(val.is_number());
    assert!(!val.is_string());
    assert!(approx_equal(*val.as_number().unwrap(), 123.0));
}

#[test]
fn success_object() {
    // Parsing should work as expected.
    let (obj, rest) = json::parse_object(r#"{"a" : 1.0 , "b": "two", "c": {"x": 3}, "d": true, "e": false, "f": null, "g": [true, false]}asdf"#).unwrap();
    assert_eq!(u64::try_from(obj["a"].clone()).unwrap(), 1u64);
    assert_eq!(*obj["b"].as_string().unwrap(), "two".to_string());
    assert!(approx_equal(
        *obj["c"].as_object().unwrap()["x"].as_number().unwrap(),
        3.0
    ));
    assert_eq!(*obj["d"].as_boolean().unwrap(), true);
    assert_eq!(*obj["e"].as_boolean().unwrap(), false);
    assert_eq!(obj["f"].as_null().unwrap(), ());
    assert!(obj["g"]
        .as_array()
        .unwrap()
        .into_iter()
        .map(|v| *v.as_boolean().unwrap())
        .eq(vec![true, false].into_iter()));
    assert_eq!(rest, "asdf");
}

#[test]
fn success_serialize() {
    // Values should be converted to valid JSON..
    let s = r#"{"a" : 1.0 , "b": "two", "c": {"x": 3}, "d": true, "e": false, "f": null, "g": [true, false]}"#;
    let parsed = Value::parse(s).unwrap();
    assert_eq!(parsed, Value::parse(&parsed.to_json().unwrap()).unwrap());
}

#[test]
fn failure_object() {
    // Bad cases should be handled.
    assert!(json::parse_object(r#"{"#).is_none());
    assert!(json::parse_object(r#"{"a":}"#).is_none());
    assert!(json::parse_object(r#"{"a":,}"#).is_none());
    assert!(json::parse_object(r#"{a:1}"#).is_none());
    assert!(json::parse_object(r#"{"a":1,}"#).is_none());
}

#[derive(FromJSON, Debug, Clone, PartialEq, Eq)]
struct JSONable<T: Clone>
where
    T: Copy,
{
    x: T,
    y: String,
    z: Vec<i32>,
}

#[derive(FromJSON, Debug, Clone, PartialEq, Eq)]
struct NoWhere<T: Clone + Copy>
{
    x: T,
    y: String,
    z: Vec<i32>,
}

#[test]
fn success_derive() {
    // The derive macro should work.
    let obj = JSONable::<i32>::from_json(
        Value::parse(r#"{"x": 1, "y": "asdf", "z": [1, 2, 3]}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(obj.x, 1);
    assert_eq!(obj.y, "asdf".to_string());
    assert_eq!(obj.z, vec![1, 2, 3]);
    let obj = NoWhere::<i32>::from_json(
        Value::parse(r#"{"x": 1, "y": "asdf", "z": [1, 2, 3]}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(obj.x, 1);
    assert_eq!(obj.y, "asdf".to_string());
    assert_eq!(obj.z, vec![1, 2, 3]);
}

#[test]
fn fail_derive() {
    // The derive macro should error when converting decimals to integers.
    assert!(JSONable::<i32>::from_json(
        Value::parse(r#"{"x": 1.1, "y": "asdf", "z": [1, 2, 3]}"#).unwrap()
    )
    .is_none());
    // Missing fields should error.
    assert!(
        JSONable::<i32>::from_json(Value::parse(r#"{"x": 1, "y": "asdf"}"#).unwrap()).is_none()
    );
    // Extra fields should error.
    assert!(JSONable::<i32>::from_json(
        Value::parse(r#"{"x": 1, "y": "asdf", "z": [1, 2, 3], "foo": "bar"}"#).unwrap()
    )
    .is_none());
}
