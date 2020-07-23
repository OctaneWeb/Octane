#![cfg(all(feature = "query_strings", feature = "extended_queries"))]
use octane::query::{parse_extended_query, QueryValue};

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let query = parse_extended_query(
        "abc=def&ab=ab%20cd%4A&arr[]=x&arr[]=y&arr[]=z&obj[a]=x&obj[b]=y&obj[c]=z",
    );
    assert_eq!(query["abc"], QueryValue::Str("def".to_string()));
    assert_eq!(query["ab"], QueryValue::Str("ab cdJ".to_string()));
    assert_eq!(
        query["arr"],
        QueryValue::Arr(["x", "y", "z"].iter().map(|v| v.to_string()).collect())
    );
    let obj = match query["obj"].clone() {
        QueryValue::Obj(v) => v,
        _ => panic!("Query string did not have an object where expected."),
    };
    assert_eq!(obj["a"], "x");
    assert_eq!(obj["b"], "y");
    assert_eq!(obj["c"], "z");
}

#[test]
fn success_ignore_incompatible() {
    // If different types are specified, strings take precedence, then the one coming first takes precedence.
    let query = parse_extended_query("a[]=2&a[x]=3&a=1&b[]=1&b[x]=2&c[x]=1&c[]=2");
    assert_eq!(query["a"], QueryValue::Str("1".to_string()));
    assert_eq!(query["b"], QueryValue::Arr(vec!["1".to_string()]));
    let obj = match query["c"].clone() {
        QueryValue::Obj(v) => v,
        _ => panic!("Query string did not have an object where expected."),
    };
    assert_eq!(obj["x"], "1".to_string());
}
