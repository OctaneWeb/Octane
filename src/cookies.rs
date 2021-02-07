use crate::constants::*;
use crate::{default, deref};
use std::collections::HashMap;
use std::collections::HashSet;

/// Represents the cookies, cookies are stored
/// with the name and values. By default you have
/// yourself a `Cookies` field in `Response`.
///
/// Cookies are behind the feature `cookies` which is included
/// in the default one
///
/// # Example
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route!(|req, res| {
///         res.cookie("name", "value").send("Cookie has been set!");
///         if let Some(value) = req.request.cookies.get("name") {
///             // access value here
///         }
///         Flow::Stop
///     }),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookies {
    cookies: HashMap<String, String>,
}

impl Cookies {
    // Return a new empty `Cookies` instance
    pub(crate) fn new() -> Self {
        Cookies {
            cookies: HashMap::new(),
        }
    }
    // Parse a Cookie header value and populate the
    // HashMap with value and keys
    pub(crate) fn parse(header: &str) -> Self {
        let mut cookies: HashMap<String, String> = HashMap::new();
        for tok in header.split("; ") {
            let eq_ind = match tok.find('=') {
                Some(v) => v,
                None => continue,
            };
            let (first, second) = tok.split_at(eq_ind);
            cookies.insert(first.to_owned(), second[1..].to_owned());
        }
        Self { cookies }
    }
    /// Insert a cookie with the `key` being the name and
    /// `value` being the value of the cookie. This is called
    /// when you do `res.cookie("name", "value")`
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(|req, res| {
    ///         res.cookie("name", "value").send("Cookie has been set!");
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    pub fn set(&mut self, key: &str, value: &str) {
        self.cookies.insert(key.to_owned(), value.to_owned());
    }
    // Prepare the `Set-Cookie` Header string from the values
    // in the HashMap
    pub(crate) fn serialise(&self) -> String {
        let mut cookies_str = String::new();
        for cookie in self.iter() {
            cookies_str.push_str(&format!("Set-Cookie:{}={}{}", cookie.0, cookie.1, CRLF))
        }
        cookies_str
    }
}

deref!(Cookies, HashMap<String, String>, cookies);

default!(Cookies);
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn cookie_new() {
        let mut cookies = Cookies::new();
        // basic value settings should work
        cookies.set("key", "value");
        let mut correct_cookies = HashMap::new();
        correct_cookies.insert("key".to_owned(), "value".to_owned());
        assert_eq!(cookies.cookies, correct_cookies);
    }

    #[test]
    pub fn cookie_parsing() {
        // basic parsing should work
        let cookie_string = "first_key=value1; second_key=value2; third_key=value3";
        let cookies = Cookies::parse(cookie_string);
        let mut correct_cookies = HashMap::new();
        correct_cookies.insert("first_key".to_owned(), "value1".to_owned());
        correct_cookies.insert("second_key".to_owned(), "value2".to_owned());
        correct_cookies.insert("third_key".to_owned(), "value3".to_owned());
        assert_eq!(cookies.cookies, correct_cookies);
    }

    #[test]
    pub fn cookie_serialize() {
        // basic serializing should work
        let cookie_string = "first_key=value; second_key=value; third_key=value";
        let mut cookies = Cookies::parse(cookie_string);
        cookies.set("fourth_key", "value");
        // Serialised cookies should be something like
        // Set-Cookie:fourth_key=value\r\nSet-Cookie:first_key=value\r\nSet-Cookie:third_key=value\r\nSet-Cookie:second_key=value\r\n
        let mut cookies_serialisation = HashSet::new();
        let serialised = cookies.serialise();
        for cookie in serialised.lines() {
            cookies_serialisation.insert(cookie);
        }
        let mut correct_serialisation = HashSet::new();
        correct_serialisation.insert("Set-Cookie:first_key=value");
        correct_serialisation.insert("Set-Cookie:second_key=value");
        correct_serialisation.insert("Set-Cookie:third_key=value");
        correct_serialisation.insert("Set-Cookie:fourth_key=value");
        assert_eq!(cookies_serialisation, correct_serialisation);
    }
}
