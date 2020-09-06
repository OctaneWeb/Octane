use crate::constants::*;
use crate::{default, deref};
use std::collections::HashMap;

/// Represents the cookies, cookies are stored
/// with the name and values. By default you have
/// yourself a `Cookies` field in `Response`.
///
/// Cookies are behind a feature but are included
/// in the default one
///
/// # Example
///
/// ```no_run
/// use octane::{route, router::{Flow, Route}};
/// use octane::server::Octane;
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
    /// Return a new empty `Cookies` instance
    pub fn new() -> Self {
        Cookies {
            cookies: HashMap::new(),
        }
    }
    /// Parse a Cookie header value and populate the
    /// HashMap with value and keys
    pub fn parse(header: &str) -> Self {
        let mut hashmap: HashMap<String, String> = HashMap::new();
        for tok in header.split("; ") {
            let eq_ind = match tok.find('=') {
                Some(v) => v,
                None => continue,
            };
            let (first, second) = tok.split_at(eq_ind);
            hashmap.insert(first.to_owned(), second[1..].to_owned());
        }
        Self { cookies: hashmap }
    }
    /// Insert a cookie with the `key` being the name and
    /// `value` being the value of the cookie. This is called
    /// when you do `res.cookie("name", "value")`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    /// Prepare the `Set-Cookie` Header string from the values
    /// in the HashMap
    pub fn serialise(&self) -> String {
        let mut cookies_str = String::new();
        for cookie in self.iter() {
            cookies_str.push_str(&format!("Set-Cookie:{}={}{}", cookie.0, cookie.1, CRLF))
        }
        cookies_str
    }
}

deref!(Cookies, HashMap<String, String>, cookies);

default!(Cookies);
