use octane::json::{parse, FromJSON, ToJSON, Value};

#[test]
fn success_string() {
    // Parsing should work as expected.
    let (string, rest) = parse::parse_string(r#""as\n\t\u0041d\"f"foo"#).unwrap();
    assert_eq!(string, "as\n\tAd\"f".to_string());
    assert_eq!(rest, "foo");
}

#[test]
fn failure_string() {
    // No closing quote should result in an error.
    assert!(parse::parse_string(r#""asdffoo"#).is_none());
    // Unicode escapes should handle bad cases.
    assert!(parse::parse_string(r#""abc\u004""#).is_none());
    assert!(parse::parse_string(r#""abc\ux123""#).is_none());
    // Non-existent escapes should error.
    assert!(parse::parse_string(r#""\c""#).is_none());
}

#[test]
fn success_bool() {
    // Parsing should work as expected.
    assert_eq!((true, " asdf"), parse::parse_bool("true asdf").unwrap());
    assert_eq!((false, " asdf"), parse::parse_bool("false asdf").unwrap());
    assert!(parse::parse_bool("asdf").is_none());
}

#[test]
fn success_null() {
    // Parsing should work as expected.
    assert_eq!(((), " asdf"), parse::parse_null("null asdf").unwrap());
    assert!(parse::parse_null("asdf").is_none());
}

#[test]
fn success_number() {
    // Parsing should work as expected.
    assert_eq!(
        (Value::Float(-1.23e-2), "asdf"),
        parse::parse_int_or_float("-1.23e-2asdf").unwrap()
    );
    assert!(parse::parse_int_or_float("1..").is_none());
}

#[test]
fn success_element() {
    // Parsing should work as expected.
    let (val, rest) = parse::parse_element(" 123 asdf").unwrap();
    assert_eq!(rest, "asdf");
    assert!(val.is_integer());
    assert!(!val.is_string());
    assert_eq!(*val.as_integer().unwrap(), 123);
}

#[test]
fn success_object() {
    // Parsing should work as expected.
    let (obj, rest) = parse::parse_object(r#"{"a" : 1.0 , "b": "two", "c": {"x": 3}, "d": true, "e": false, "f": null, "g": [true, false]}asdf"#).unwrap();
    assert_eq!(*obj["a"].as_float().unwrap(), 1.0);
    assert_eq!(*obj["b"].as_string().unwrap(), "two".to_string());
    assert_eq!(
        *obj["c"].as_object().unwrap()["x"].as_integer().unwrap(),
        3
    );
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
    assert_eq!(parsed, Value::parse(&parsed.to_string()).unwrap());
}

#[test]
fn failure_object() {
    // Bad cases should be handled.
    assert!(parse::parse_object(r#"{"#).is_none());
    assert!(parse::parse_object(r#"{"a":}"#).is_none());
    assert!(parse::parse_object(r#"{"a":,}"#).is_none());
    assert!(parse::parse_object(r#"{a:1}"#).is_none());
    assert!(parse::parse_object(r#"{"a":1,}"#).is_none());
}

#[derive(FromJSON, ToJSON, Debug, Clone, PartialEq, Eq)]
struct JSONable<T: Clone>
where
    T: Copy,
{
    x: T,
    y: String,
    z: Vec<i32>,
}

#[derive(FromJSON, ToJSON, Debug, Clone, PartialEq, Eq)]
struct NoWhere<T: Clone + Copy> {
    x: T,
    y: String,
    z: Vec<i32>,
}

#[derive(FromJSON, ToJSON, Debug, Clone, PartialEq, Eq)]
struct TupleStruct(i32, String, Vec<TupleStruct>);

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
    assert_eq!(
        obj,
        JSONable::from_json(obj.clone().to_json().unwrap()).unwrap()
    );
    let obj = NoWhere::<i32>::from_json(
        Value::parse(r#"{"x": 1, "y": "asdf", "z": [1, 2, 3]}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(obj.x, 1);
    assert_eq!(obj.y, "asdf".to_string());
    assert_eq!(obj.z, vec![1, 2, 3]);
    assert_eq!(
        obj,
        NoWhere::from_json(obj.clone().to_json().unwrap()).unwrap()
    );
    let val = Value::parse(r#"[1, "asdf", [[2, "foo", []], [3, "bar", []]]]"#).unwrap();
    let arr = TupleStruct::from_json(val).unwrap();
    assert_eq!(arr.0, 1);
    assert_eq!(arr.1, "asdf".to_string());
    assert_eq!(arr.2[0].0, 2);
    assert_eq!(arr.2[0].1, "foo".to_string());
    assert_eq!(arr.2[0].2, vec![]);
    assert_eq!(arr.2[1].0, 3);
    assert_eq!(arr.2[1].1, "bar".to_string());
    assert_eq!(arr.2[1].2, vec![]);
    assert_eq!(
        arr,
        TupleStruct::from_json(arr.clone().to_json().unwrap()).unwrap()
    );
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
