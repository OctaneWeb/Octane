use crate::constants::*;
use crate::file_handler::FileHandler;
use crate::request::HttpVersion;
use crate::time::Time;
use std::collections::HashMap;
use std::fmt;
use std::io::Result;
use std::path::PathBuf;

/// The response struct contains the data which is
/// to be send on a request. The struct has several
/// methods to modify the contents.
///
/// # Example
///
/// ```no_run
/// use octane::server::Octane;
/// use octane::{route, router::{Flow, Route}};
///
/// #[tokio::main]
/// async fn main() {
///     let mut app = Octane::new();
///     app.get(
///         "/",
///         route!(
///             |req, res| {
///                 // access res (response) here
///             }
///         ),
///     );
///
///     app.listen(8080).await.expect("Cannot establish connection");
/// }
/// ```
pub struct Response {
    pub status_code: StatusCode,
    pub body: Vec<u8>,
    pub http_version: String,
    pub headers: HashMap<String, String>,
    pub has_body: bool,
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status_code", &self.status_code)
            .field("http_version", &self.http_version)
            .field("body", &std::str::from_utf8(&self.body))
            .finish()
    }
}

impl Response {
    /// Adds appends a custom header with the headers
    /// that will be sent.
    ///
    /// **Note**: Will overwrite the header with the
    /// same name
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use octane::{route, router::{Flow, Route}};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut app = Octane::new();
    ///     app.get(
    ///         "/",
    ///         route!(
    ///             |req, res| {
    ///                res
    ///                .with_header("header-name", "header-value")
    ///                .send("HELLO");
    ///             }
    ///         ),
    ///     );
    ///
    ///     app.listen(8080).await.expect("Cannot establish connection");
    /// }
    /// ```
    pub fn with_header(&mut self, key: &'static str, value: &'static str) -> &mut Self {
        self.headers.insert(key.to_owned(), value.to_owned());

        self
    }
    /// Generates a new response empty response, usually
    /// you should not be using this method.
    pub fn new(body: &[u8]) -> Self {
        Response {
            has_body: false,
            status_code: StatusCode::Ok,
            body: body.to_vec(),
            http_version: "1.1".to_owned(),
            headers: HashMap::new(),
        }
    }
    /// Puts the given text to the body and send it
    /// as html by default
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use octane::{route, router::{Flow, Route}};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut app = Octane::new();
    ///     app.get(
    ///         "/",
    ///         route!(
    ///             |req, res| {
    ///                res.send("HELLO");
    ///             }
    ///         ),
    ///     );
    ///
    ///     app.listen(8080).await.expect("Cannot establish connection");
    /// }
    /// ```
    pub fn send(&mut self, body: &'static str) {
        self.body = body.as_bytes().to_vec();
        self.has_body = true;
        self.default_headers();
    }
    /// Add some default headers like time, content-type
    pub fn default_headers(&mut self) -> &mut Self {
        self.headers
            .insert("Content-Length".to_string(), self.body.len().to_string());
        if let Some(date) = Time::now() {
            self.headers.insert("Date".to_string(), date.format());
        }
        if let None = self.headers.get("Content-Type") {
            self.with_header("Content-Type", "text/html");
        }
        self
    }
    /// Modify the Content-Type header to fit the data
    /// which is being send
    /// # Example
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use octane::{route, router::{Flow, Route}};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut app = Octane::new();
    ///     app.get(
    ///         "/",
    ///         route!(
    ///             |req, res| {
    ///                res.with_type("json").send(r#"{"server": "Octane"}"#);
    ///             }
    ///         ),
    ///     );
    ///
    ///     app.listen(8080).await.expect("Cannot establish connection");
    /// }
    /// ```
    pub fn with_type(&mut self, _type: &'static str) -> &mut Self {
        // TODO:
        // res.with_type("json") => application/json
        // res.with_type("application/json") => application/json
        self.with_header("Content-Type", _type);
        self
    }
    /// Consume the response and get the final formed http
    /// response that the server will send in bytes
    pub fn get_data(self) -> Vec<u8> {
        let mut headers_str = String::from("");
        self.headers
            .iter()
            .for_each(|data| headers_str.push_str(&format!("{}:{}{}{}", data.0, SP, data.1, CRLF)));
        [
            format!("{}{}{}", self.status_line(), headers_str, CRLF).as_bytes(),
            &self.body,
        ]
        .concat()
    }
    /// Determine the content type and server the file
    /// contents as the response
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use octane::{route, router::{Flow, Route}};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut app = Octane::new();
    ///     app.get(
    ///         "/",
    ///         route!(
    ///             |req, res| {
    ///                res.with_type("json").send(r#"{"server": "Octane"}"#);
    ///             }
    ///         ),
    ///     );
    ///
    ///     app.listen(8080).await.expect("Cannot establish connection");
    /// }
    /// ```
    pub async fn send_file(&mut self, file: PathBuf) -> Result<Option<()>> {
        if let Some(file) = FileHandler::handle_file(&file)? {
            self.has_body = true;
            self.headers
                .insert("Content-Type".to_string(), file.get_mime_type());
            self.body = file.contents;
            Ok(Some(()))
        } else {
            Ok(None)
        }
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

    /// Set the status code from the status code enum
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::server::Octane;
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::constants::StatusCode;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut app = Octane::new();
    ///     app.get(
    ///         "/",
    ///         route!(
    ///             |req, res| {
    ///                res.status(StatusCode::NotFound).send("Page not found");
    ///             }
    ///         ),
    ///     );
    ///
    ///     app.listen(8080).await.expect("Cannot establish connection");
    /// }
    /// ```
    pub fn status(&mut self, code: StatusCode) -> &mut Self {
        self.status_code = code;
        self
    }
    pub fn http_version(&mut self, version: HttpVersion) -> &mut Self {
        self.http_version = version.get_version_string();
        self
    }
    fn reason_phrase(&self) -> String {
        self.status_code.to_string().to_uppercase()
    }
    fn status_code(&self) -> i32 {
        self.status_code.into()
    }
}
