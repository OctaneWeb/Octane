extern crate octane;
use octane::path::PathBuf;

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let chunks = PathBuf::parse("/asdf//foo/a/b/c/../d").unwrap();
    assert_eq!(chunks.chunks, vec!["asdf", "foo", "a", "b", "d"]);
}

#[test]
fn success_matching() {
    // Parsing should work as expected.
    let path1 = PathBuf::parse("/asdf/:var/foo/").unwrap();
    let path2 = PathBuf::parse("asdf/test/foo/").unwrap();
    let path3 = PathBuf::parse("/asdf/test/foo").unwrap();
    let path4 = PathBuf::parse("/asdf/test/bad").unwrap();
    assert_eq!(path1.check_matches(&path2).unwrap()["var"], "test".to_string());
    assert_eq!(path1.check_matches(&path3).unwrap()["var"], "test".to_string());
    assert!(path1.check_matches(&path4).is_none());
}

#[test]
fn success_subtraction() {
    // Parsing should work as expected.
    let path1 = PathBuf::parse("/a/b/c/d/").unwrap();
    let path2 = PathBuf::parse("/a/b/").unwrap();
    let path3 = PathBuf::parse("/c/d").unwrap();
    let path4 = PathBuf::parse("a/:x/c").unwrap();
    assert_eq!(path1.subtract(&path2).unwrap().chunks, vec!["c", "d"]);
    assert!(path1.subtract(&path3).is_none());
    assert!(path1.subtract(&path4).is_none());
}

#[test]
#[should_panic]
fn fail_traversal() {
    // Too many ..s should error.
    PathBuf::parse("/asdf/../..").unwrap();
}
