use octane::cookies::*;

#[test]
pub fn cookie_new() {
    let mut cookies = Cookies::new();
    // basic value settings should work
    cookies.set("key", "value");
    assert_eq!(1, cookies.len());
}

#[test]
pub fn cookie_parsing() {
    // basic parsing should work
    let cookie_string = "Cookie: first_key=value; second_key=value; third_value=value";
    let cookies = Cookies::parse(cookie_string);
    assert_eq!(3, cookies.len());
}

#[test]
pub fn cookie_serialize() {
    // basic serializing should work
    let cookie_string = "first_key=value; second_key=value; third_key=value";
    let mut cookies = Cookies::parse(cookie_string);
    cookies.set("forth_key", "value");
    // we cannot know the order of the cookies so we put in the len
    // the string should be like
    // Set-Cookie:forth_key=value\r\nSet-Cookie:first_key=value\r\nSet-Cookie:third_key=value\r\nSet-Cookie:second_key=value\r\n
    assert_eq!(113, cookies.serialise().len());
}
