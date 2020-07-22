use crate::constants::*;
use crate::file_handler::FileHandler;
use crate::time::Time;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

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
    pub fn with_header(&mut self, key: &str, value: String) -> &mut Self {
        self.headers.insert(key.to_owned(), value);

        self
    }
    pub fn new(body: &[u8]) -> Self {
        Response {
            status_code: StatusCode::Ok,
            body: body.to_vec(),
            http_version: "1.1".to_owned(),
            headers: HashMap::new(),
        }
    }
    pub fn send(&mut self, body: &'static str) {
        let heading_one = b"<!DOCTYPE html><html><head></head><body>";
        let heading_two = b"</body></html>";
        let data = &[heading_one, body.as_bytes(), heading_two].concat();
        self.body = data.to_vec();
    }

    pub fn default_headers(&mut self) -> &mut Self {
        self.headers
            .insert(String::from("Content-Length"), self.body.len().to_string());
        if let Some(date) = Time::now() {
            self.headers.insert(String::from("Date"), date.format());
        }
        self.headers
            .insert(String::from("Content-Type"), String::from("text/html"));
        // TODO: Add more default headers
        self
    }
    pub fn with_time(&mut self, stamp: i64) -> &mut Self {
        if let Some(time) = Time::now() {
            if let Some(with_stamp) = time.with_stamp(stamp) {
                self.headers
                    .insert(String::from("Date"), with_stamp.format());
            }
        }
        self
    }
    pub fn get_data(&self) -> Vec<u8> {
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
    pub async fn send_file(&mut self, file: PathBuf) -> std::io::Result<()> {
        if let Some(file) = FileHandler::handle_file(&file)? {
            let mime_type = file.get_mime_type();
            self.with_header("Content-Type", mime_type);
            self.body = file.contents;
        } else {
            self.declare_error(StatusCode::NotFound)?.default_headers();
        }
        Ok(())
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
    pub fn declare_error(&mut self, error_kind: StatusCode) -> std::io::Result<&mut Self> {
        self.status_code = error_kind;
        if error_kind == StatusCode::NotFound {
            self.body = FileHandler::get_404_file()?;
        }
        Ok(self)
    }
    fn reason_phrase(&self) -> String {
        self.status_code.to_string().to_uppercase()
    }
    pub fn status(&mut self, code: StatusCode) -> &mut Self {
        self.status_code = code;
        self
    }
    pub fn with_http_version(&mut self, version: &'static str) -> &mut Self {
        self.http_version = version.to_owned();
        self
    }
    fn status_code(&self) -> i32 {
        self.status_code.into()
    }
}
