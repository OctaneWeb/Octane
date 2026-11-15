use crate::constants::*;
#[cfg(feature = "cookies")]
use crate::cookie::Cookie;
use crate::file_handler::FileHandler;
use crate::time::Time;
use octane_http::{HttpVersion, StatusCode};
#[cfg(feature = "json")]
use serde::Serialize;
use std::borrow::Cow;
use std::error::Error;
#[cfg(feature = "cookies")]
use std::fmt;
use std::fmt::Write;
use std::io::Cursor;
use std::marker::PhantomData;
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncReadExt};

pub(crate) type BoxReader = Box<dyn AsyncRead + Unpin + Send>;

pub(crate) enum ResBody {
    None,
    Bytes(Vec<u8>),
    Static(&'static [u8]),
    Sized(usize, BoxReader),
}

impl ResBody {
    pub fn get_reader(self) -> BoxReader {
        match self {
            // never send more than the Content-Length we announced
            ResBody::Sized(len, reader) => Box::new(reader.take(len as u64)) as BoxReader,
            ResBody::Bytes(body) => Box::new(Cursor::new(body)) as BoxReader,
            ResBody::Static(body) => Box::new(Cursor::new(body)) as BoxReader,
            ResBody::None => Box::new(Cursor::new(Vec::new())) as BoxReader,
        }
    }
    pub fn is_some(&self) -> bool {
        !matches!(self, ResBody::None)
    }
}

/// The response struct contains the data which is
/// to be send on a request. The struct has several
/// methods to modify the contents.
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
///         // access res (response) here
///         Flow::Next
///     }),
/// );
/// ```
pub struct Response<'a> {
    body: ResBody,
    /// The status code the response will contain
    pub status_code: StatusCode,
    /// Length of the content which will be sent as the response
    pub content_len: Option<usize>,
    /// Http version which the response will use, `1.1` when empty
    pub http_version: String,
    /// Custom headers which will be sent with the response
    pub headers: ResponseHeaders,
    /// Content-Type charset
    pub charset: Option<String>,
    /// Cookies that will be sent with the response
    #[cfg(feature = "cookies")]
    pub cookies: Vec<Cookie<'a>>,
    // add `Content-Length` and `Date` to the head, unless already set
    auto_headers: bool,
    // `Content-Length` is the length of the body, whatever was set
    framed: bool,
    marker: PhantomData<&'a ()>,
}

impl<'a> Response<'a> {
    /// Adds appends a custom header with the headers
    /// that will be sent.
    ///
    /// **Note**: This will overwrite the header with the
    /// same name
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
    ///         res
    ///             .set("header-name", "header-value")
    ///             .send("HELLO");
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    pub fn set(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.set(key.to_owned(), value.to_owned());
        self
    }
    /// Get the header value by name
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
    ///         res.send("Hello, world");
    ///         assert_eq!(res.get("Content-Type"), Some("text/html"));
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    pub fn get(&self, field: &str) -> Option<&str> {
        self.headers.get(field)
    }
    /// Puts the given text to the body and send it
    /// as html by default
    ///
    /// # Example
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send("HELLO");
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    ///
    /// ```
    pub fn send<T: AsRef<[u8]>>(&mut self, body: T) {
        self.set_body(body.as_ref().to_vec());
    }
    /// Sets an owned response body without copying it. Prefer this when the
    /// body is already a `Vec<u8>` or `String`.
    pub fn send_owned<T: Into<Vec<u8>>>(&mut self, body: T) {
        self.set_body(body.into());
    }
    /// Automatically set headers like date, content
    /// length, and sent content header to "text/html"
    /// if no content header is sent
    pub fn default_headers(&mut self) -> &mut Self {
        if let Some(content_len) = self.content_len {
            self.headers.set("Content-Length", content_len.to_string());
        }
        if let Some(date) = Time::now_http_date() {
            self.headers.set("Date", date);
        }
        self.default_type();
        self
    }
    // `Content-Type: text/html`, unless a type was set
    fn default_type(&mut self) {
        if !self.headers.contains("Content-Type") {
            match &self.charset {
                Some(charset) => {
                    let mut format = String::from("text/html");
                    write!(&mut format, " ;charset={:?}", charset)
                        .expect("writing a charset to a String cannot fail");
                    self.headers.set("Content-Type", format);
                }
                None => self.headers.set("Content-Type", "text/html"),
            }
        }
    }
    /// Modify the `Content-Type` header as passed
    /// in the argument
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.with_type("json").send(r#"{"server": "Octane"}"#);
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    /// ```
    pub fn with_type(&mut self, _type: &'static str) -> &mut Self {
        // TODO:
        // res.with_type("json") => application/json
        // res.with_type("application/json") => application/json
        self.headers.set("Content-Type", _type);
        self
    }
    /// Consume the response and get the final formed http
    /// response that the server will send in bytes
    pub fn get_data(self) -> (String, BoxReader) {
        let (head, body) = self.into_parts();
        (head, body.get_reader())
    }
    /// Send a file as the response, automatically detect the
    /// mime type and set the headers accordingly
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    /// use std::path::PathBuf;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send_file("templates/index.html").expect("file not found");
    ///             assert_eq!(res.get("Content-Type"),  Some("text/html"));
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    ///
    /// ```
    pub fn send_file(&mut self, file: &str) -> Result<Option<()>, Box<dyn Error>> {
        let file = FileHandler::handle_file(&PathBuf::from(file))?;
        self.headers
            .set("Content-Type", FileHandler::mime_type(file.extension));
        let len = file.meta.len() as usize;
        self.content_len = Some(len);
        self.auto_headers = true;
        self.body = ResBody::Sized(len, Box::new(file.file) as BoxReader);
        Ok(Some(()))
    }

    /// Converts the structure to a json string and sends
    /// it as the response with the mime type `application/json`.
    /// The structure which will be passed should implement
    /// `ToJSON` from `octane_macros::convert`
    ///
    /// TODO: add a example here with a struct that implements
    /// ToJSON and then do res.json(structure)
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    /// use std::path::PathBuf;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             // add example here
    ///             // assert_eq!(res.get("Content-Type"), Some("application/json"));
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    ///
    /// ```
    #[cfg(feature = "json")]
    pub fn json<T: Serialize>(&mut self, structure: T) -> serde_json::Result<()> {
        let body = serde_json::to_vec(&structure)?;
        let len = body.len();
        self.body = ResBody::Bytes(body);
        self.content_len = Some(len);
        self.with_type("application/json");
        self.auto_headers = true;
        Ok(())
    }
    /// Set the status code from the status code enum
    ///
    /// # Example
    ///
    /// ```
    /// use octane::prelude::*;
    /// use octane::StatusCode;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.status(StatusCode::NotFound).send("Page not found");
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    ///
    /// ```
    pub fn status(&mut self, code: StatusCode) -> &mut Self {
        self.status_code = code;
        self
    }
    /// Sets the http version specified, to specify a version
    /// the version type should be variant of HttpVersion
    pub fn http_version(&mut self, version: HttpVersion) -> &mut Self {
        self.http_version = version.to_string();
        self
    }
    /// Tells if the headers are sent or not
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
    ///         assert_eq!(false, res.headers_sent());
    ///         res.send("Hello, World");
    ///         assert_eq!(true, res.headers_sent());
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    pub fn headers_sent(&self) -> bool {
        !self.headers.is_empty()
    }
    /// Sets the http `Content-Disposition` header field
    /// to `attachment`
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
    ///         res.attachment();
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    pub fn attachment(&mut self) -> &mut Self {
        self.set("Content-Disposition", "attachment");
        self
    }
    /// Sets the http `Content-Disposition` header field
    /// to `attachment` with the filename and automatically
    /// updates the content type with the extension
    /// provided in the filename
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
    ///         res.attachment_with_filename("image.png");
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    pub fn attachment_with_filename(&mut self, file_name: &'static str) -> &mut Self {
        let extension = FileHandler::get_extension(&PathBuf::from(file_name));
        self.set(
            "Content-Disposition",
            &format!("attachment; filename = {:?}", file_name),
        );
        self.set("Content-Type", &extension);
        self
    }
    /// Sets the Location header with a status code `302 FOUND`
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
    ///         res.redirect("/").send("Taking you to home");
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    pub fn redirect(&mut self, location: &str) -> &mut Self {
        self.status(StatusCode::Found);
        self.set("Location", location);
        self
    }
    /// Creates a cookie with the specified name
    /// and value. This method requires `cookies`
    /// feature, which is enabled in default feature
    ///
    /// Cookies that fail [`Cookie::validate`](../cookie/struct.Cookie.html#method.validate)
    /// are not sent.
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
    #[cfg(feature = "cookies")]
    pub fn cookie<C: Into<Cookie<'a>>>(&mut self, cookie: C) -> &mut Response<'a> {
        self.cookies.push(cookie.into());
        self
    }
    /// Sets the content type charset
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
    ///         res.charset("utf-8").send("Hello"); // the header is now Content-Type: text/html; charset=utf-8
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    pub fn charset(&mut self, charset: &str) -> &mut Self {
        self.charset = Some(charset.to_owned());
        self
    }
    pub(crate) fn has_body(&self) -> bool {
        self.body.is_some()
    }
    /// Make `Content-Length` the length of the body that will be sent. On a
    /// persistent connection it is the only thing telling the client where
    /// the response ends, so it can't be missing (`send_file` doesn't set
    /// it) or disagree with the body.
    pub(crate) fn frame(&mut self) {
        let len = match &self.body {
            ResBody::None => 0,
            ResBody::Bytes(body) => body.len(),
            ResBody::Static(body) => body.len(),
            ResBody::Sized(len, _) => *len,
        };
        self.content_len = Some(len);
        self.framed = true;
    }
    // Creates a new response from a slice
    pub(crate) fn new_from_slice<T: AsRef<[u8]>>(body: T) -> Self {
        let body_slice = body.as_ref();
        Self::with_body(ResBody::Bytes(body_slice.to_vec()), Some(body_slice.len()))
    }
    // Creates a response backed by static bytes without copying the body.
    pub(crate) fn new_from_static(body: &'static [u8]) -> Self {
        Self::with_body(ResBody::Static(body), Some(body.len()))
    }
    // Creates a new response with empty body
    pub(crate) fn new_empty() -> Self {
        Response {
            status_code: StatusCode::Ok,
            body: ResBody::None,
            content_len: None,
            http_version: String::new(),
            headers: ResponseHeaders::new(),
            charset: None,
            #[cfg(feature = "cookies")]
            cookies: Vec::new(),
            auto_headers: false,
            framed: false,
            marker: PhantomData,
        }
    }
    // Generates a new empty response
    fn with_body(body: ResBody, content_len: Option<usize>) -> Self {
        Response {
            status_code: StatusCode::Ok,
            body,
            content_len,
            http_version: String::new(),
            headers: ResponseHeaders::new(),
            charset: None,
            #[cfg(feature = "cookies")]
            cookies: Vec::new(),
            auto_headers: false,
            framed: false,
            marker: PhantomData,
        }
    }
    fn set_body(&mut self, body: Vec<u8>) {
        let len = body.len();
        self.body = ResBody::Bytes(body);
        self.content_len = Some(len);
        self.auto_headers = true;
        self.default_type();
    }
    pub(crate) fn into_parts(self) -> (String, ResBody) {
        let mut head = Vec::with_capacity(128);
        self.write_head(&mut head);
        // header values are `&str` and only ASCII bytes get replaced
        let head = String::from_utf8(head).expect("a response head is UTF-8");
        (head, self.body)
    }
    pub(crate) fn into_body(self) -> ResBody {
        self.body
    }
    /// Whether the status allows content: 1xx, `204 No Content` and
    /// `304 Not Modified` never have any (RFC 9110 section 6.4.1), so a
    /// client reads the next response right after the head
    pub(crate) fn status_has_body(&self) -> bool {
        let (code, _) = self.status_code.fetch();
        !(code < 200 || code == 204 || code == 304)
    }
    /// Append the status line and headers to `out`
    pub(crate) fn write_head(&self, out: &mut Vec<u8>) {
        let (status_code, reason_phrase) = self.status_code.fetch();
        out.extend_from_slice(b"HTTP/");
        if self.http_version.is_empty() {
            out.extend_from_slice(b"1.1");
        } else {
            out.extend_from_slice(self.http_version.as_bytes());
        }
        out.push(b' ');
        push_number(out, status_code as usize);
        out.push(b' ');
        out.extend(reason_phrase.bytes().map(|b| b.to_ascii_uppercase()));
        out.extend_from_slice(CRLF.as_bytes());

        // no length either: it is forbidden for 1xx and 204, and for 304
        // could only be the length of the 200 response, which isn't known
        let has_body = self.status_has_body();
        let (mut has_length, mut has_date) = (false, false);
        for (name, value) in self.headers.iter() {
            if name.eq_ignore_ascii_case("content-length") {
                if self.framed || !has_body {
                    continue;
                }
                has_length = true;
            } else if name.eq_ignore_ascii_case("date") {
                has_date = true;
            }
            push_header(out, name, value);
        }
        if has_body && (self.framed || (self.auto_headers && !has_length)) {
            if let Some(len) = self.content_len {
                out.extend_from_slice(b"Content-Length: ");
                push_number(out, len);
                out.extend_from_slice(CRLF.as_bytes());
            }
        }
        if self.auto_headers && !has_date {
            Time::with_http_date(|date| {
                out.extend_from_slice(b"Date: ");
                out.extend_from_slice(date.as_bytes());
                out.extend_from_slice(CRLF.as_bytes());
            });
        }
        // push cookies, one Set-Cookie line each, leaving out the ones
        // that would make an invalid header (see `Cookie::validate`)
        #[cfg(feature = "cookies")]
        self.cookies
            .iter()
            .filter_map(|cookie| cookie.serialise().ok())
            .for_each(|cookie| push_header(out, "Set-Cookie", &cookie));
        out.extend_from_slice(CRLF.as_bytes());
    }
}

/// The headers a response is sent with, in the order they were set. Names
/// are matched without regard to case, and setting a header replaces one of
/// the same name. Static names and values, like the ones octane sets itself,
/// aren't copied.
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get("/", route_stop!(|req, res| {
///     res.headers.set("Cache-Control", "no-store");
///     res.headers.set("X-Request-Path", req.request_line.path.to_string());
///     res.send("ok");
/// }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResponseHeaders {
    fields: Vec<(Cow<'static, str>, Cow<'static, str>)>,
}

impl ResponseHeaders {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }
    /// The value of the header `name`
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_ref())
    }
    /// Set the header `name` to `value`, replacing it if it was set
    pub fn set<N, V>(&mut self, name: N, value: V)
    where
        N: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        let (name, value) = (name.into(), value.into());
        match self
            .fields
            .iter_mut()
            .find(|(key, _)| key.eq_ignore_ascii_case(&name))
        {
            Some(field) => *field = (name, value),
            None => self.fields.push((name, value)),
        }
    }
    /// Remove the header `name`, returning its value if it was set
    pub fn remove(&mut self, name: &str) -> Option<Cow<'static, str>> {
        let at = self
            .fields
            .iter()
            .position(|(key, _)| key.eq_ignore_ascii_case(name))?;
        Some(self.fields.remove(at).1)
    }
    /// Whether the header `name` is set
    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }
    /// Every header, in the order it was first set
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fields
            .iter()
            .map(|(name, value)| (name.as_ref(), value.as_ref()))
    }
    pub fn len(&self) -> usize {
        self.fields.len()
    }
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

// Push `n` in decimal
fn push_number(out: &mut Vec<u8>, mut n: usize) {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[at..]);
}

// Push a `name: value` line. A name that isn't a token is skipped and control
// characters in the value, CR and LF included, are replaced by spaces, so no
// value can start another header line or end the head early.
fn push_header(out: &mut Vec<u8>, name: &str, value: &str) {
    if !is_token(name) {
        return;
    }
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(b": ");
    let is_ctl = |byte: u8| (byte <= b'\x1f' && byte != b'\t') || byte == b'\x7f';
    if value.bytes().any(is_ctl) {
        out.extend(value.bytes().map(|b| if is_ctl(b) { b' ' } else { b }));
    } else {
        out.extend_from_slice(value.as_bytes());
    }
    out.extend_from_slice(CRLF.as_bytes());
}

#[cfg(feature = "cookies")]
impl Default for Response<'_> {
    fn default() -> Self {
        Self::new_from_slice(b"")
    }
}

#[cfg(feature = "cookies")]
impl fmt::Debug for Response<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status_code", &self.status_code)
            .field("headers", &self.headers)
            .field("http_version", &self.http_version)
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use octane_http::HttpVersion;
    use tokio::io::AsyncReadExt;

    async fn data_to_string(mut data: (String, BoxReader)) -> String {
        let mut ret = data.0;
        data.1
            .read_to_string(&mut ret)
            .await
            .expect("cannot read to string");
        ret
    }

    #[tokio::test]
    async fn success_standard() {
        // default response should provide OK 200 Code
        let req = data_to_string(Response::new_from_slice(b"").get_data()).await;
        assert_eq!(req, "HTTP/1.1 200 OK\r\n\r\n");
    }

    #[tokio::test]
    async fn response_with_status_code_different() {
        // Reponse with different status codes should work
        let mut req = Response::new_from_slice(b"");
        req.status(StatusCode::Created);

        assert_eq!(
            data_to_string(req.get_data()).await,
            "HTTP/1.1 201 CREATED\r\n\r\n"
        );
    }

    #[tokio::test]
    async fn response_with_different_http_version() {
        // Reponse with different status codes should work
        let mut req = Response::new_from_slice(b"");

        req.http_version(HttpVersion::Http10)
            .status(StatusCode::Created);
        assert_eq!(
            data_to_string(req.get_data()).await,
            "HTTP/1.0 201 CREATED\r\n\r\n"
        );
    }

    #[test]
    fn header_values_cannot_add_lines() {
        // CR and LF in values become spaces, invalid names are skipped
        let mut res = Response::new_empty();
        res.redirect("/home\r\nX-Injected: yes")
            .set("Bad Name", "x")
            .send("");
        let (head, _) = res.get_data();
        assert!(head.contains("\r\nLocation: /home  X-Injected: yes\r\n"));
        assert!(!head.contains("\r\nX-Injected"));
        assert!(!head.contains("Bad Name"));
    }

    #[tokio::test]
    async fn body_never_exceeds_content_length() {
        let mut res = Response::new_empty();
        res.send("hello");
        res.body = ResBody::Sized(2, Box::new(Cursor::new(b"hello".to_vec())));
        assert!(data_to_string(res.get_data()).await.ends_with("\r\n\r\nhe"));
    }

    #[cfg(feature = "json")]
    #[tokio::test]
    async fn json_body_is_owned_and_sized() {
        let mut res = Response::new_empty();
        res.json([1, 2]).unwrap();
        let data = data_to_string(res.get_data()).await;

        assert!(data.contains("\r\nContent-Length: 5\r\n"));
        assert!(data.contains("\r\nContent-Type: application/json\r\n"));
        assert!(data.ends_with("\r\n\r\n[1,2]"));
    }

    #[test]
    fn send_owned_reuses_the_body_allocation() {
        let body = Vec::with_capacity(64);
        let allocation = body.as_ptr();
        let mut res = Response::new_empty();
        res.send_owned(body);

        match &res.body {
            ResBody::Bytes(body) => assert_eq!(body.as_ptr(), allocation),
            _ => panic!("owned response body was not stored as bytes"),
        }
    }

    #[cfg(feature = "cookies")]
    #[tokio::test]
    async fn cookies_get_their_own_header_lines() {
        let mut res = Response::new_empty();
        res.cookie(Cookie::new("a", "1"))
            .cookie(Cookie::new("b", "x\r\ny"))
            .send("body");
        let data = data_to_string(res.get_data()).await;
        assert!(data.contains("\r\nSet-Cookie: a=1\r\n"));
        // an invalid cookie is left out
        assert!(!data.contains("Set-Cookie: b="));
        assert!(data.ends_with("\r\n\r\nbody"));
    }
}
