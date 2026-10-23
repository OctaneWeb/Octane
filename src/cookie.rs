//! Cookies (RFC 6265): reading the request `Cookie` header and writing
//! `Set-Cookie` response headers.
//!
//! Parsing is lenient so that cookies set by browsers or scripts still come
//! through, while serialising is strict so the server never sends a header
//! that a browser would reject or misread.

use crate::constants::{HT, SP, TOKEN_CHARS};
use crate::path::is_ctl;
use crate::time::Time;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// The latest instant `Expires` can carry, 9999-12-31 23:59:59 GMT, as the
// IMF-fixdate format only has room for a four digit year
const MAX_EXPIRES: u64 = 253_402_300_799;

/// Represents a cookie, a name and a value along with the attributes used
/// when it is sent in a `Set-Cookie` header. Cookies parsed from a request
/// only carry a name and a value.
///
/// Cookies are behind the feature `cookies` which is included
/// in the default one
///
/// # Example
///
/// ```
/// use octane::prelude::*;
/// use octane::cookie::Cookie;
///
/// let mut app = Octane::new();
/// app.get(
///     "/",
///     route!(|req, res| {
///         let visits = req
///             .cookie("visits")
///             .map_or(0, |c| c.value().parse::<u32>().unwrap_or(0));
///         res.cookie(Cookie::new("visits", (visits + 1).to_string()))
///             .send("Cookie has been set!");
///         Flow::Stop
///     }),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie<'a> {
    name: Cow<'a, str>,
    value: Cow<'a, str>,
    expires: Option<SystemTime>,
    max_age: Option<Duration>,
    domain: Option<Cow<'a, str>>,
    path: Option<Cow<'a, str>>,
    secure: bool,
    http_only: bool,
    same_site: Option<SameSite>,
    partitioned: bool,
}

/// The `SameSite` attribute of a cookie, which decides whether it is
/// sent along with cross-site requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SameSite {
    /// Only sent with same-site requests
    Strict,
    /// Also sent when navigating to the site from another one
    Lax,
    /// Sent with all requests, browsers require `Secure` with it
    None,
}

/// The reason a cookie can't be sent in a `Set-Cookie` header, returned
/// by [`Cookie::validate`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieError {
    /// The name is empty or isn't a token
    InvalidName,
    /// The value has characters outside the RFC 6265 cookie-octet set
    InvalidValue,
    /// The path has a control character or a `;`
    InvalidPath,
    /// The domain isn't a host name
    InvalidDomain,
    /// The cookie must be `Secure`, as browsers drop `SameSite=None`,
    /// `Partitioned`, `__Secure-` and `__Host-` cookies otherwise
    NotSecure,
    /// A `__Host-` cookie must have `Path=/` and no `Domain`
    HostPrefix,
}

impl<'a> Cookie<'a> {
    /// Creates a new `Cookie` instance with the given `name` and `value`.
    ///
    /// # Example
    ///
    /// ```
    /// use octane::cookie::Cookie;
    ///
    /// let cookie = Cookie::new("name", "value");
    /// assert_eq!(cookie.name_value(), ("name", "value"));
    /// ```
    pub fn new<N, V>(name: N, value: V) -> Self
    where
        N: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>,
    {
        Cookie {
            name: name.into(),
            value: value.into(),
            expires: None,
            max_age: None,
            domain: None,
            path: None,
            secure: false,
            http_only: false,
            same_site: None,
            partitioned: false,
        }
    }
    /// Returns a `CookieBuilder` to build a `Cookie` with attributes.
    ///
    /// # Example
    ///
    /// ```
    /// use octane::cookie::{Cookie, SameSite};
    ///
    /// let cookie = Cookie::build("session", "abc123")
    ///     .path("/")
    ///     .http_only(true)
    ///     .secure(true)
    ///     .same_site(SameSite::Lax)
    ///     .finish();
    /// assert!(cookie.validate().is_ok());
    /// ```
    pub fn build<N, V>(name: N, value: V) -> CookieBuilder<'a>
    where
        N: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>,
    {
        CookieBuilder {
            cookie: Cookie::new(name, value),
        }
    }
    /// The name of the cookie
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The value of the cookie, without the double quotes around a quoted
    /// value when it was parsed from a request
    pub fn value(&self) -> &str {
        &self.value
    }
    /// The name and the value of the cookie
    pub fn name_value(&self) -> (&str, &str) {
        (&self.name, &self.value)
    }
    /// The `Expires` attribute
    pub fn expires(&self) -> Option<SystemTime> {
        self.expires
    }
    /// The `Max-Age` attribute
    pub fn max_age(&self) -> Option<Duration> {
        self.max_age
    }
    /// The `Domain` attribute
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }
    /// The `Path` attribute
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }
    /// Whether the `Secure` attribute is set
    pub fn secure(&self) -> bool {
        self.secure
    }
    /// Whether the `HttpOnly` attribute is set
    pub fn http_only(&self) -> bool {
        self.http_only
    }
    /// The `SameSite` attribute
    pub fn same_site(&self) -> Option<SameSite> {
        self.same_site
    }
    /// Whether the `Partitioned` attribute is set
    pub fn partitioned(&self) -> bool {
        self.partitioned
    }
    /// Checks that the cookie can be sent in a `Set-Cookie` header: the name
    /// must be a token, the value made of RFC 6265 cookie-octets (optionally
    /// inside double quotes), the path free of control characters and `;`,
    /// and the domain a host name. It also checks the rules browsers enforce
    /// before storing a cookie, see [`CookieError`].
    ///
    /// Invalid cookies are left out of the response, so to store arbitrary
    /// data in a value, encode it first (with base64 for example).
    pub fn validate(&self) -> Result<(), CookieError> {
        if !is_token(&self.name) {
            return Err(CookieError::InvalidName);
        }
        if !unquote(&self.value).bytes().all(is_cookie_octet) {
            return Err(CookieError::InvalidValue);
        }
        if !self.path.as_deref().map_or(true, is_path_value) {
            return Err(CookieError::InvalidPath);
        }
        if !self.domain.as_deref().map_or(true, is_domain_value) {
            return Err(CookieError::InvalidDomain);
        }
        let host_prefix = has_prefix(&self.name, "__Host-");
        if !self.secure
            && (host_prefix
                || has_prefix(&self.name, "__Secure-")
                || self.same_site == Some(SameSite::None)
                || self.partitioned)
        {
            return Err(CookieError::NotSecure);
        }
        if host_prefix && (self.domain.is_some() || self.path.as_deref() != Some("/")) {
            return Err(CookieError::HostPrefix);
        }
        Ok(())
    }
    // Parse a request `Cookie` header (RFC 6265 section 5.4) and create a
    // Vec with all the cookies in it, borrowing from the header. Pairs are
    // split on `;` with or without the space after it, whitespace around
    // names and values and the quotes around a quoted value are dropped. A
    // pair without `=`, with a name that isn't a token or with a control
    // character in its value is skipped rather than failing the whole header.
    // Order and repeated names are kept, browsers send the most specific
    // cookie with a name first.
    pub(crate) fn parse(header: &'a str) -> Vec<Cookie<'a>> {
        header
            .split(';')
            .filter_map(|pair| {
                let (name, value) = pair.split_once('=')?;
                let name = trim_ows(name);
                let value = unquote(trim_ows(value));
                if is_token(name) && !value.chars().any(is_ctl) {
                    Some(Cookie::new(name, value))
                } else {
                    None
                }
            })
            .collect()
    }
    // Prepare the `Set-Cookie` header value, if the cookie is valid
    pub(crate) fn serialise(&self) -> Result<String, CookieError> {
        self.validate()?;
        let mut cookie = format!("{}={}", self.name, self.value);
        if let Some(expires) = self.expires {
            let secs = expires
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
                .min(MAX_EXPIRES);
            if let Some(time) = Time::time(secs as i64) {
                cookie.push_str("; Expires=");
                cookie.push_str(&time.format());
            }
        }
        if let Some(max_age) = self.max_age {
            cookie.push_str(&format!("; Max-Age={}", max_age.as_secs()));
        }
        if let Some(domain) = &self.domain {
            cookie.push_str("; Domain=");
            cookie.push_str(domain);
        }
        if let Some(path) = &self.path {
            cookie.push_str("; Path=");
            cookie.push_str(path);
        }
        if self.secure {
            cookie.push_str("; Secure");
        }
        if self.http_only {
            cookie.push_str("; HttpOnly");
        }
        if let Some(same_site) = self.same_site {
            cookie.push_str(match same_site {
                SameSite::Strict => "; SameSite=Strict",
                SameSite::Lax => "; SameSite=Lax",
                SameSite::None => "; SameSite=None",
            });
        }
        if self.partitioned {
            cookie.push_str("; Partitioned");
        }
        Ok(cookie)
    }
}

/// A builder for a [`Cookie`] with attributes, see [`Cookie::build`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieBuilder<'a> {
    cookie: Cookie<'a>,
}

impl<'a> CookieBuilder<'a> {
    /// Sets the `Expires` attribute
    pub fn expires(mut self, expires: SystemTime) -> Self {
        self.cookie.expires = Some(expires);
        self
    }
    /// Sets the `Max-Age` attribute, sent in whole seconds
    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.cookie.max_age = Some(max_age);
        self
    }
    /// Sets the `Domain` attribute
    pub fn domain<D: Into<Cow<'a, str>>>(mut self, domain: D) -> Self {
        self.cookie.domain = Some(domain.into());
        self
    }
    /// Sets the `Path` attribute
    pub fn path<P: Into<Cow<'a, str>>>(mut self, path: P) -> Self {
        self.cookie.path = Some(path.into());
        self
    }
    /// Sets the `Secure` attribute
    pub fn secure(mut self, secure: bool) -> Self {
        self.cookie.secure = secure;
        self
    }
    /// Sets the `HttpOnly` attribute
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.cookie.http_only = http_only;
        self
    }
    /// Sets the `SameSite` attribute
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.cookie.same_site = Some(same_site);
        self
    }
    /// Sets the `Partitioned` attribute
    pub fn partitioned(mut self, partitioned: bool) -> Self {
        self.cookie.partitioned = partitioned;
        self
    }
    /// Returns the built cookie
    pub fn finish(self) -> Cookie<'a> {
        self.cookie
    }
}

impl<'a> From<CookieBuilder<'a>> for Cookie<'a> {
    fn from(builder: CookieBuilder<'a>) -> Cookie<'a> {
        builder.finish()
    }
}

impl fmt::Display for CookieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CookieError::InvalidName => "cookie name is empty or not a token",
            CookieError::InvalidValue => "cookie value has characters outside the cookie-octet set",
            CookieError::InvalidPath => "cookie path has a control character or a `;`",
            CookieError::InvalidDomain => "cookie domain is not a host name",
            CookieError::NotSecure => "cookie must be Secure",
            CookieError::HostPrefix => "__Host- cookie must have Path=/ and no Domain",
        })
    }
}

impl Error for CookieError {}

fn trim_ows(s: &str) -> &str {
    s.trim_matches(|c| c == SP || c == HT)
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn is_token(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| TOKEN_CHARS.contains(&c))
}

// cookie-octet: printable US-ASCII except `"`, `,`, `;` and `\`
fn is_cookie_octet(b: u8) -> bool {
    matches!(b, 0x21 | 0x23..=0x2B | 0x2D..=0x3A | 0x3C..=0x5B | 0x5D..=0x7E)
}

// path-value: any CHAR except CTLs or `;`
fn is_path_value(path: &str) -> bool {
    path.bytes().all(|b| (0x20..0x7f).contains(&b) && b != b';')
}

// domain-value: a host name, browsers ignore a leading dot
fn is_domain_value(domain: &str) -> bool {
    let domain = domain.strip_prefix('.').unwrap_or(domain);
    !domain.is_empty()
        && domain
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
}

// browsers match cookie name prefixes case-insensitively
fn has_prefix(name: &str, prefix: &str) -> bool {
    name.as_bytes()
        .get(..prefix.len())
        .map_or(false, |start| start.eq_ignore_ascii_case(prefix.as_bytes()))
}

#[cfg(test)]
mod test {
    use super::*;

    fn pairs<'a>(cookies: &'a [Cookie<'a>]) -> Vec<(&'a str, &'a str)> {
        cookies.iter().map(Cookie::name_value).collect()
    }

    #[test]
    pub fn cookie_new() {
        let cookie = Cookie::new("name", "value");
        assert_eq!(("name", "value"), cookie.name_value());
        let owned = Cookie::new("name", String::from("value"));
        assert_eq!(cookie, owned);
    }

    #[test]
    pub fn cookie_parsing() {
        // basic parsing should work
        let cookie_string = "name1=value1; name2=value2";
        let cookies = Cookie::parse(cookie_string);
        let correct_cookies = vec![
            Cookie::from(Cookie::build("name1", "value1").finish()),
            Cookie::from(Cookie::build("name2", "value2").finish()),
        ];
        assert!(cookies == correct_cookies);
    }

    #[test]
    pub fn cookie_parsing_separators_and_whitespace() {
        let cookies = Cookie::parse("a=1;b=2 ;  c = 3\t;;d=");
        assert_eq!(
            pairs(&cookies),
            [("a", "1"), ("b", "2"), ("c", "3"), ("d", "")]
        );
    }

    #[test]
    pub fn cookie_parsing_quotes_and_equals() {
        let cookies = Cookie::parse(r#"q="quoted"; b64=YWJj==; e="""#);
        assert_eq!(
            pairs(&cookies),
            [("q", "quoted"), ("b64", "YWJj=="), ("e", "")]
        );
    }

    #[test]
    pub fn cookie_parsing_skips_malformed_pairs() {
        // no `=`, empty name, name that isn't a token, control character
        let cookies = Cookie::parse("foo; =v; a b=1; ok=1; c=\u{1}; d=1");
        assert_eq!(pairs(&cookies), [("ok", "1"), ("d", "1")]);
        assert!(Cookie::parse("").is_empty());
        assert!(Cookie::parse("a=1; ").len() == 1);
    }

    #[test]
    pub fn cookie_parsing_is_lenient_with_values() {
        // browsers send values set by scripts as they are
        let cookies = Cookie::parse(r#"json={"a":1,"b":2}; s=a b; u=é"#);
        assert_eq!(
            pairs(&cookies),
            [("json", r#"{"a":1,"b":2}"#), ("s", "a b"), ("u", "é")]
        );
    }

    #[test]
    pub fn cookie_parsing_keeps_order_and_duplicates() {
        let cookies = Cookie::parse("id=1; id=2");
        assert_eq!(pairs(&cookies), [("id", "1"), ("id", "2")]);
    }

    #[test]
    pub fn cookie_serialize() {
        // basic serializing should work
        let cookie = Cookie::from(Cookie::build("name", "value").http_only(true).finish());
        assert_eq!(Ok("name=value; HttpOnly".to_owned()), cookie.serialise());
    }

    #[test]
    pub fn cookie_serialize_all_attributes() {
        let cookie = Cookie::build("id", "\"a1\"")
            .expires(UNIX_EPOCH + Duration::from_secs(1789461005))
            .max_age(Duration::from_secs(3600))
            .domain("example.com")
            .path("/app")
            .secure(true)
            .http_only(true)
            .same_site(SameSite::None)
            .partitioned(true)
            .finish();
        assert_eq!(
            cookie.serialise().unwrap(),
            "id=\"a1\"; Expires=Tue, 15 Sep 2026 08:30:05 GMT; Max-Age=3600; \
             Domain=example.com; Path=/app; Secure; HttpOnly; SameSite=None; Partitioned"
        );
    }

    #[test]
    pub fn cookie_serialize_clamps_expires() {
        let epoch = Cookie::build("a", "1").expires(UNIX_EPOCH).finish();
        assert!(epoch
            .serialise()
            .unwrap()
            .ends_with("Expires=Thu, 01 Jan 1970 00:00:00 GMT"));
        let far = Cookie::build("a", "1")
            .expires(UNIX_EPOCH + Duration::from_secs(u32::MAX as u64 * 100))
            .finish();
        assert!(far
            .serialise()
            .unwrap()
            .ends_with("Expires=Fri, 31 Dec 9999 23:59:59 GMT"));
    }

    #[test]
    pub fn cookie_serialize_rejects_invalid() {
        let check = |cookie: CookieBuilder<'_>| cookie.finish().serialise().unwrap_err();
        assert_eq!(check(Cookie::build("", "v")), CookieError::InvalidName);
        assert_eq!(check(Cookie::build("a b", "v")), CookieError::InvalidName);
        assert_eq!(
            check(Cookie::build("a", "x\r\nX: y")),
            CookieError::InvalidValue
        );
        assert_eq!(check(Cookie::build("a", "x;y")), CookieError::InvalidValue);
        assert_eq!(check(Cookie::build("a", "x y")), CookieError::InvalidValue);
        assert_eq!(check(Cookie::build("a", "\"")), CookieError::InvalidValue);
        assert_eq!(
            check(Cookie::build("a", "v").path("/; Domain=evil")),
            CookieError::InvalidPath
        );
        assert_eq!(
            check(Cookie::build("a", "v").domain("evil.com/x")),
            CookieError::InvalidDomain
        );
        assert_eq!(
            check(Cookie::build("a", "v").same_site(SameSite::None)),
            CookieError::NotSecure
        );
        assert_eq!(
            check(Cookie::build("__secure-a", "v")),
            CookieError::NotSecure
        );
        assert_eq!(
            check(Cookie::build("__Host-a", "v").secure(true).path("/x")),
            CookieError::HostPrefix
        );
        assert!(Cookie::build("__Host-a", "v")
            .secure(true)
            .path("/")
            .finish()
            .validate()
            .is_ok());
    }

    #[test]
    pub fn cookie_round_trip() {
        let sent = Cookie::build("token", "\"abc\"").path("/").finish();
        let header = sent.serialise().unwrap();
        let received = Cookie::parse(&header);
        assert_eq!(received[0].name_value(), ("token", "abc"));
    }
}
