extern crate octane;
use octane::path::PathBuf;

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let chunks = PathBuf::parse("/asdf//foo/a/b/c/../d").unwrap();
    assert_eq!(chunks.chunks, vec!["asdf", "foo", "a", "b", "d"]);
}

#[test]
#[should_panic]
fn fail_traversal() {
    // Too many ..s should error.
    PathBuf::parse("/asdf/../..").unwrap();
}
