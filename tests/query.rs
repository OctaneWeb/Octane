#![cfg(feature = "query_strings")]
use octane::query;
extern crate octane;

mod common;

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let query = query::parse_query("abc=def&ab=ab%20cd%4A");
    assert_eq!(query["abc"], "def");
    assert_eq!(query["ab"], "ab cdJ");
}

#[test]
fn success_blank_query() {
    // Queries without an "=" should be blank.
    let query = query::parse_query("a&abc=def");
    assert_eq!(query["a"], "");
    assert_eq!(query["abc"], "def");
}

#[test]
fn success_weird_hex() {
    // Queries should be able to handle weird hex escapes.
    let query = query::parse_query("a=%0%41&b=%41%0&c=%%41&d=A%5");
    assert_eq!(query["a"], "%0A");
    assert_eq!(query["b"], "A%0");
    assert_eq!(query["c"], "%A");
    assert_eq!(query["d"], "A%5");
}

#[test]
#[cfg_attr(not(feature = "faithful"), ignore)]
fn success_no_name() {
    // Queries with no name should not insert anything into the hashmap.
    assert_eq!(query::parse_query("").len(), 0);
    assert_eq!(query::parse_query("=x").len(), 0);
    assert_eq!(query::parse_query("=x&=y").len(), 0);
}
