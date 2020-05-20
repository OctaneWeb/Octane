extern crate octane;
use octane::request::Header;

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let req = Header::parse("Referer: \t\t request://www.example.com/").unwrap();
    assert_eq!(req.name, "Referer");
    assert_eq!(req.value, "request://www.example.com/");
}

#[test]
fn success_empty_value() {
    // Empty values are allowed.
    let req = Header::parse("Referer: \t\t ").unwrap();
    assert_eq!(req.name, "Referer");
    assert_eq!(req.value, "");
}

#[test]
#[should_panic]
fn fail_no_value() {
    // Having no value should fail.
    Header::parse("Referer").unwrap();
}

#[test]
#[should_panic]
fn fail_empty_name() {
    // Having no name should fail.
    Header::parse(": test").unwrap();
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "faithful"), ignore)]
fn fail_malformed_name() {
    // Having separators in the name should fail.
    Header::parse("Test Header: test").unwrap();
}
