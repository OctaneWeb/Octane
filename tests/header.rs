#[cfg(feature = "cookies")]
use octane::cookies::Cookies;
use octane::request::{Header, KeepAlive};

#[test]
fn success_standard() {
    // Parsing should work as expected.
    let req = Header::parse("Referer: \t\t request://www.example.com/".to_string()).unwrap();
    assert_eq!(req.name, "Referer");
    assert_eq!(req.value, "request://www.example.com/");
}

#[test]
fn success_empty_value() {
    // Empty values are allowed.
    let req = Header::parse("Referer: \t\t ".to_string()).unwrap();
    assert_eq!(req.name, "Referer");
    assert_eq!(req.value, "");
}

#[test]
#[should_panic]
fn fail_no_value() {
    // Having no value should fail.
    Header::parse("Referer".to_string()).unwrap();
}

#[test]
#[should_panic]
fn fail_empty_name() {
    // Having no name should fail.
    Header::parse(": test".to_string()).unwrap();
}

#[test]
#[should_panic]
#[cfg_attr(not(feature = "faithful"), ignore)]
fn fail_malformed_name() {
    // Having separators in the name should fail.
    Header::parse("Test Header: test".to_string()).unwrap();
}

#[test]
fn success_keepalive() {
    // Parsing should work as expected.
    let req = KeepAlive::parse("timeout=5, max=1000");
    assert_eq!(req.timeout, Some(5));
    assert_eq!(req.max, Some(1000));
}

#[test]
fn success_keepalive_edge() {
    // Edge cases should work as expected.
    let req = KeepAlive::parse("timeout=,test,max=a, timeout=5");
    assert_eq!(req.timeout, Some(5));
    assert_eq!(req.max, None);
}

#[cfg(feature = "cookies")]
#[test]
fn success_cookies() {
    // Parsing should work as expected.
    let cookies = Cookies::parse("a=asdf; b=fdsa; c=; d=x=5");
    assert_eq!(cookies.get("a"), Some(&"asdf".to_string()));
    assert_eq!(cookies.get("b"), Some(&"fdsa".to_string()));
    assert_eq!(cookies.get("c"), Some(&"".to_string()));
    assert_eq!(cookies.get("d"), Some(&"x=5".to_string()));
}

#[cfg(feature = "cookies")]
#[test]
fn seriialise_cookies() {
    let cookies = Cookies::parse("a=asdf");
    assert_eq!(cookies.serialise(), "Set-Cookie:a=asdf\r\n");
}
