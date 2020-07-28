use crate::constants::*;
use crate::file_handler::FileHandler;
use crate::request::HttpVersion;
use crate::time::Time;
use octane_json::convert::ToJSON;
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
///                 Flow::Next
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
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    /// Asks for the Returns the value of the header
    /// key and returns the value of the field
    ///
    /// # Example
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    /// Generates a new empty response, usually
    /// you should not be using this method directly.
    pub fn new(body: &[u8]) -> Self {
        Response {
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
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    pub fn send(&mut self, body: &'static str) {
        self.body = body.as_bytes().to_vec();
        self.default_headers();
    }
    /// Automatically set headers like date, content
    /// length, and sent content to "text/html" if no
    /// content header is sent
    pub fn default_headers(&mut self) -> &mut Self {
        self.headers
            .insert("Content-Length".to_string(), self.body.len().to_string());
        if let Some(date) = Time::now() {
            self.headers.insert("Date".to_string(), date.format());
        }
        if self.headers.get("Content-Type").is_none() {
            self.set("Content-Type", "text/html");
        }
        self
    }
    /// Modify the `Content-Type` header as passed
    /// in the argument
    ///
    /// # Example
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    /// Send a file as the response, automatically detect the
    /// mime type and set the headers accordingly
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
    /// use std::path::PathBuf;
    /// let mut app = Octane::new();
    /// app.get(
    ///     "/",
    ///     route!(
    ///         |req, res| {
    ///             res.send_file(PathBuf::from("templates/index.html"));
    ///             assert_eq!(res.get("Content-Type"),  Some(&"text/html".to_owned()));
    ///             Flow::Stop
    ///         }
    ///     ),
    /// );
    ///
    /// ```
    pub async fn send_file(&mut self, file: PathBuf) -> Result<Option<()>> {
        if let Some(file) = FileHandler::handle_file(&file)? {
            self.headers.insert(
                "Content-Type".to_string(),
                FileHandler::mime_type(file.extension),
            );
            self.body = file.contents;
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }
    /// Converts the structure to a json string and sends
    /// it as the response with the mime type `application/json`.
    /// The structure which will be passed, should implement
    /// `ToJSON` from `octane_macros::convert`
    ///
    /// TODO: add a example here with a struct that implements
    /// ToJSON and then do res.json(structure)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
        self.body = structure
            .to_json_string()
            .unwrap_or(String::new())
            .as_bytes()
            .to_vec();
        self.with_type("application/json");
        self.default_headers();
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
    ///                 res.status(StatusCode::NotFound).send("Page not found");
    ///                 Flow::Stop
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
    /// Sets the http version specified, to specify a version
    /// the versios type should be variant of HttpVersion
    pub fn http_version(&mut self, version: HttpVersion) -> &mut Self {
        self.http_version = version.get_version_string();
        self
    }
    /// Tells if the headers are sent or not
    ///
    /// # Example
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
        self.headers.len() != 0
    }
    /// Sets the http `Content-Disposition` header field
    /// to `attachment`
    ///
    /// # Example
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    pub fn attachment(&mut self) {
        self.set("Content-Disposition", "attachment");
    }
    /// Sets the http `Content-Disposition` header field
    /// to `attachment` with the filename and automatically
    /// updates the content type with the extension
    /// provided in the filename
    ///
    /// # Example
    /// ```no_run
    /// use octane::{route, router::{Flow, Route}};
    /// use octane::server::Octane;
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
    pub fn attachment_with_filename(&mut self, file_name: &'static str) {
        let extension = FileHandler::get_extension(&PathBuf::from(file_name));
        self.set(
            "Content-Disposition",
            &format!("attachment; filename = {:?}", file_name),
        );
        self.set("Content-Type", &extension);
    }
    /// Sets the location header to the field specified
    pub fn location(&mut self, field: &'static str) {
        self.set("Location", field);
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
}
