use crate::constants::*;
use crate::{default, deref};
use std::collections::HashMap;

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
        let header = {
            if header.contains(":") {
                header.split(":").collect::<Vec<&str>>().last().unwrap().trim()
            } else {
                header
            }
        };
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
}
