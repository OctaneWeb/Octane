use octane::path::*;

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let chunks = PathBuf::parse("/asdf//foo/a/b/c/../d").unwrap();
    assert_eq!(chunks.chunks, vec!["asdf", "foo", "a", "b", "d"]);
}

#[test]
#[cfg(feature = "url_variables")]
fn success_matching() {
    // Parsing should work as expected.
    let path1 = PathBuf::parse("/asdf/:var/foo/").unwrap();
    let path2 = PathBuf::parse("asdf/test/foo/").unwrap();
    let path3 = PathBuf::parse("/asdf/test/foo").unwrap();
    let path4 = PathBuf::parse("/asdf/test/bad").unwrap();
    assert_eq!(
        path1.check_matches(&path2).unwrap()["var"],
        "test".to_string()
    );
    assert_eq!(
        path1.check_matches(&path3).unwrap()["var"],
        "test".to_string()
    );
    assert!(path1.check_matches(&path4).is_none());
}

#[test]
#[cfg(not(feature = "url_variables"))]
fn success_matching() {
    // Parsing should work as expected.
    let path1 = PathBuf::parse("/asdf/test/foo/").unwrap();
    let path2 = PathBuf::parse("asdf/test/foo").unwrap();
    let path3 = PathBuf::parse("asdf/test/bad").unwrap();
    path1.check_matches(&path2).unwrap();
    assert!(path1.check_matches(&path3).is_none());
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

#[test]
#[cfg(feature = "url_variables")]
fn success_tree() {
    let mut node: PathNode<i32> = PathNode::new();
    let path1 = PathBuf::parse("/asdf/:var/foo/").unwrap();
    let path2 = PathBuf::parse("asdf/test/foo/").unwrap();
    let path3 = PathBuf::parse("/asdf/test/foo").unwrap();
    let path4 = PathBuf::parse("/asdf/test/bad").unwrap();
    let path5 = PathBuf::parse("/asdf/test/nope").unwrap();
    node.insert(path1.clone(), 1);
    node.insert(path4.clone(), 4);
    assert!(node.get(&path5).is_empty());
    let matched = node.get(&path2).remove(0);
    assert_eq!(*matched.data, 1);
    assert_eq!(matched.vars.get("var").unwrap(), "test");
    let matched = node.get(&path3).remove(0);
    assert_eq!(*matched.data, 1);
    assert_eq!(matched.vars.get("var").unwrap(), "test");
    let matched = node.get(&path4).remove(0);
    assert_eq!(*matched.data, 4);
    assert!(matched.vars.is_empty());
}

#[test]
#[cfg(not(feature = "url_variables"))]
fn success_tree() {
    let mut node: PathNode<i32> = PathNode::new();
    let path1 = PathBuf::parse("asdf/test/foo/").unwrap();
    let path2 = PathBuf::parse("/asdf/test/bad").unwrap();
    node.insert(path1.clone(), 1);
    assert!(node.get(&path2).is_empty());
    let matched = node.get(&path1).remove(0);
    assert_eq!(*matched.data, 1);
}
