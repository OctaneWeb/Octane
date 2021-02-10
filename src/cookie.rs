use crate::deref;
use cookie::Cookie as CookieRs;
use cookie::CookieBuilder;

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
///         res.cookie(octane::cookie::Cookie::new("name", "value")).send("Cookie has been set!");
///         let sent_cookie = &req.request.cookies;
///         // Do stuff with cookies
///         Flow::Stop
///     }),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Cookie<'a> {
    cookie: CookieRs<'a>,
}

impl Cookie<'_> {
    /// Creates a new `Cookie` instance with the given `name` and `value`.
    /// 
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let cookie = octane::cookie::Cookie::new("name", "value");
    /// println!("{:?}", cookie);
    /// ```
    pub fn new<'a>(name: &'a str, value: &'a str) -> Cookie<'a> {
        Cookie {
            cookie: CookieRs::new(name, value),
        }
    }
    /// Creates a new `cookie` instance from a `cookie` in the `cookie-rs` library.
    /// 
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    /// use cookie::Cookie as CookieRs;
    ///
    /// let cookie_rs_cookie = CookieRs::new("name", "value");
    /// let octane_cookie = octane::cookie::Cookie::from(cookie_rs_cookie);
    /// println!("{:?}", octane_cookie);
    /// ```
    pub fn from<'a>(cookie: CookieRs<'a>) -> Cookie<'a> {
        Cookie {
            cookie,
        }
    }
    /// Returns a `CookieBuilder` from the `cookie-rs` library to build a `Cookie`.
    /// 
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    /// use cookie::Cookie as CookieRs;
    ///
    /// let cookie = octane::cookie::Cookie::from(
    ///     octane::cookie::Cookie::build("name", "value")
    ///         .http_only(true)
    ///         .finish()
    /// );
    /// println!("{:?}", cookie);
    /// ```
    pub fn build<'a>(name: &'a str, value: &'a str) -> CookieBuilder<'a> {
        CookieRs::build(name, value)
    }
    // Parse a Cookie header value and create a Vec with all the 
    // cookies in the header
    pub(crate) fn parse<'a>(header: &'a str) -> Vec<Cookie<'a>> {
        let mut cookies_vec = Vec::new();
        let header_cookies = header.split("; ");
        for tok in header_cookies {
            cookies_vec.push(Cookie {
                cookie: CookieRs::parse(tok).unwrap()
            });
        }
        cookies_vec
    }
    // Prepare the `Set-Cookie` Header string from the values
    // in the HashMap
    pub(crate) fn serialise(&self) -> String {
        format!("Set-Cookie: {}", self.cookie.to_string())
    }
}

impl Eq for Cookie<'_> {}

deref!(Cookie<'a>, CookieRs<'a>, cookie);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn cookie_new() {
        let cookie = Cookie::new("name", "value");
        assert_eq!(("name", "value"), cookie.cookie.name_value());
    }

    #[test]
    pub fn cookie_parsing() {
        // basic parsing should work
        let cookie_string = "name1=value1; name2=value2";
        let cookies = Cookie::parse(cookie_string);
        let correct_cookies = vec![
            Cookie::from(
                Cookie::build("name1", "value1").finish(),
            ),
            Cookie::from(
                Cookie::build("name2", "value2").finish(),
            )
        ];
        assert!(cookies == correct_cookies);
    }

    #[test]
    pub fn cookie_serialize() {
        // basic serializing should work
        let cookie = Cookie::from(
            Cookie::build("name", "value")
                .http_only(true)
                .finish()
        );
        assert_eq!("Set-Cookie: name=value; HttpOnly", cookie.serialise());
    }
}
