use crate::constants::*;
#[cfg(feature = "cookies")]
use crate::cookies::Cookies;
use crate::file_handler::FileHandler;
use crate::request::HttpVersion;
use crate::time::Time;
use octane_json::convert::ToJSON;
use octane_macros::status_codes;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io::Cursor;
use std::path::PathBuf;
use tokio::io::AsyncRead;

pub(crate) type BoxReader = Box<dyn AsyncRead + Unpin + Send>;

/// The response struct contains the data which is
/// to be send on a request. The struct has several
/// methods to modify the contents.
///
/// # Example
///
/// ```no_run
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
pub struct Response {
    body: BoxReader,
    /// The status code the response will contain
    pub status_code: StatusCode,
    /// Length of the content which will be sent as the response
    pub content_len: Option<usize>,
    /// Http version which the response will use
    pub http_version: String,
    /// Custom headers which will be sent with the response
    pub headers: HashMap<String, String>,
    /// Content-Type charset
    pub charset: Option<String>,
    #[cfg(feature = "cookies")]
    /// Cookies that will be sent with the response
    pub cookies: Cookies,
}

impl Response {
    /// Adds appends a custom header with the headers
    /// that will be sent.
    ///
    /// **Note**: This will overwrite the header with the
    /// same name
    ///
    /// # Example
    ///
    /// ```no_run
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
        self.headers.insert(key.to_owned(), value.to_owned());
        self
    }
    /// Get the header value by name
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(|req, res| {
    ///         res.send("Hello, world");
    ///         assert_eq!(res.get("Content-Type"),  Some(&"text/html".to_owned()));
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    pub fn get(&mut self, field: &'static str) -> Option<&String> {
        self.headers.get(field)
    }
    /// Creates a new response from a slice
    pub fn new_from_slice<T: AsRef<[u8]>>(body: T) -> Self {
        let body_slice = body.as_ref();
        Self::new(
            Box::new(Cursor::new(body_slice.to_vec())) as BoxReader,
            Some(body_slice.len()),
        )
    }
    /// Generates a new empty response. A new response is
    /// already created for you, so you don't have to call this
    /// method
    pub fn new(body: BoxReader, content_len: Option<usize>) -> Self {
        Response {
            status_code: StatusCode::Ok,
            body,
            content_len,
            http_version: "1.1".to_owned(),
            headers: HashMap::new(),
            charset: None,
            #[cfg(feature = "cookies")]
            cookies: Cookies::new(),
        }
    }
    /// Puts the given text to the body and send it
    /// as html by default
    ///
    /// # Example
    /// ```no_run
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
        let body_slice = body.as_ref();
        self.body = Box::new(Cursor::new(body_slice.to_vec())) as BoxReader;
        self.content_len = Some(body_slice.len());
        self.default_headers();
    }
    /// Automatically set headers like date, content
    /// length, and sent content header to "text/html"
    /// if no content header is sent
    pub fn default_headers(&mut self) -> &mut Self {
        if let Some(x) = self.content_len {
            self.headers
                .insert("Content-Length".to_string(), x.to_string());
        }
        if let Some(date) = Time::now() {
            self.headers.insert("Date".to_string(), date.format());
        }
        if self.headers.get("Content-Type").is_none() {
            let mut format = String::from("text/html");
            if let Some(charset) = &self.charset {
                format.push_str(&format!(" ;charset={:?}", charset));
            }
            self.set("Content-Type", &format);
        }
        self
    }
    /// Modify the `Content-Type` header as passed
    /// in the argument
    ///
    /// # Example
    /// ```no_run
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
        self.set("Content-Type", _type);
        self
    }
    /// Consume the response and get the final formed http
    /// response that the server will send in bytes
    pub fn get_data(self) -> (String, BoxReader) {
        (
            format!("{}{}{}", self.status_line(), self.headers(), CRLF),
            self.body,
        )
    }
    /// Send a file as the response, automatically detect the
    /// mime type and set the headers accordingly
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::prelude::*;
    /// use std::path::PathBuf;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send_file("templates/index.html").expect("file not found");
    ///             assert_eq!(res.get("Content-Type"),  Some(&"text/html".to_owned()));
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    ///
    /// ```
    pub fn send_file(&mut self, file: &str) -> Result<Option<()>, Box<dyn Error>> {
        let file = FileHandler::handle_file(&PathBuf::from(file))?;
        self.headers.insert(
            "Content-Type".to_string(),
            FileHandler::mime_type(file.extension),
        );
        self.content_len = Some(file.meta.len() as usize);
        self.body = Box::new(file.file) as BoxReader;
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
    /// ```no_run
    /// use octane::prelude::*;
    /// use std::path::PathBuf;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             // add example here
    ///             // assert_eq!(res.get("Content-Type"),  Some(&"application/json".to_owned()));
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    ///
    /// ```
    pub fn json<T: ToJSON>(&mut self, structure: T) {
        self.body = Box::new(Cursor::new(
            structure
                .to_json_string()
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
        )) as BoxReader;
        self.with_type("application/json");
        self.default_headers();
    }
    /// Set the status code from the status code enum
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::prelude::*;
    /// use octane::responder::StatusCode;
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
    /// ```no_run
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
    /// ```no_run
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
    /// ```no_run
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
    /// ```no_run
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
    /// # Example
    ///
    /// ```no_run
    /// use octane::prelude::*;
    ///
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(|req, res| {
    ///         res.cookie("name", "value").send("Cookie has been set!");
    ///         if let Some(value) = req.request.cookies.get("name") {
    ///             println!("{:?}", value); // value
    ///         }
    ///         Flow::Stop
    ///     }),
    /// );
    /// ```
    #[cfg(feature = "cookies")]
    pub fn cookie(&mut self, name: &str, value: &str) -> &mut Self {
        self.cookies.set(name, value);
        self
    }
    /// Sets the content type charset
    ///
    /// # Example
    ///
    /// ```no_run
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
    fn reason_phrase(&self) -> String {
        self.status_code.to_string().to_uppercase()
    }
    fn status_code(&self) -> i32 {
        self.status_code.into()
    }
    fn status_line(&self) -> String {
        format!(
            "{}/{}{}{}{}{}{}",
            "HTTP",
            self.http_version,
            SP,
            self.status_code(),
            SP,
            self.reason_phrase(),
            CRLF
        )
    }
    fn headers(&self) -> String {
        let mut headers_str = String::new();
        // push normal headers
        self.headers
            .iter()
            .for_each(|data| headers_str.push_str(&format!("{}:{}{}{}", data.0, SP, data.1, CRLF)));
        // push cookies
        #[cfg(feature = "cookies")]
        {
            headers_str.push_str(&self.cookies.serialise());
        }
        headers_str
    }
}

status_codes! {
    100 "Continue"
    101 "Switching Protocol"
    102 "Processing"
    103 "Early Hints"
    200 "OK"
    201 "Created"
    202 "Accepted"
    203 "Non-Authoritative Information"
    204 "No Content"
    205 "Reset Content"
    206 "Partial Content"
    207 "Multi-Status"
    208 "Already Reported"
    226 "IM Used"
    300 "Multiple Choice"
    301 "Moved Permanently"
    302 "Found"
    303 "See Other"
    304 "Not Modified"
    307 "Temporary Redirect"
    308 "Permanent Redirect"
    400 "Bad Request"
    401 "Unauthorized"
    402 "Payment Required"
    403 "Forbidden"
    404 "Not Found"
    405 "Method Not Allowed"
    406 "Not Acceptable"
    407 "Proxy Authentication Required"
    408 "Request Timeout"
    409 "Conflict"
    410 "Gone"
    411 "Length Required"
    412 "Precondition Failed"
    413 "Payload Too Large"
    414 "URI Too Long"
    415 "Unsupported Media Type"
    416 "Range Not Satisfiable"
    417 "Expectation Failed"
    418 "I'm a teapot"
    421 "Misdirected Request"
    422 "Unprocessable Entity"
    423 "Locked"
    424 "Failed Dependency"
    425 "Too Early"
    426 "Upgrade Required"
    428 "Precondition Required"
    429 "Too Many Requests"
    431 "Request Header Fields Too Large"
    451 "Unavailable For Legal Reasons"
    500 "Internal Server Error"
    501 "Not Implemented"
    502 "Bad Gateway"
    503 "Service Unavailable"
    504 "Gateway Timeout"
    505 "HTTP Version Not Supported"
    506 "Variant Also Negotiates"
    507 "Insufficient Storage"
    508 "Loop Detected"
    510 "Not Extended"
    511 "Network Authentication Required"
}

impl Into<i32> for StatusCode {
    fn into(self) -> i32 {
        let (n, _) = self.fetch();
        n
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (_, s) = self.fetch();
        write!(f, "{}", s)
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new_from_slice(b"")
    }
}

impl fmt::Debug for Response {
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
    use crate::request::HttpVersion;
    use tokio::io::AsyncReadExt;

    async fn data_to_string(mut data: (String, BoxReader)) -> String {
        let mut ret = data.0;
        data.1
            .read_to_string(&mut ret)
            .await
            .expect("cannot read to string");
        ret
    }

    #[crate::test]
    async fn success_standard() {
        // default response should provide OK 200 Code
        let req = data_to_string(Response::new_from_slice(b"").get_data()).await;
        assert_eq!(req, "HTTP/1.1 200 OK\r\n\r\n");
    }

    #[crate::test]
    async fn response_with_status_code_different() {
        // Reponse with different status codes should work
        let mut req = Response::new_from_slice(b"");
        req.status(StatusCode::Created);

        assert_eq!(
            data_to_string(req.get_data()).await,
            "HTTP/1.1 201 CREATED\r\n\r\n"
        );
    }

    #[crate::test]
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
}
