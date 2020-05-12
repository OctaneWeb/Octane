use crate::constants::*;
use crate::time::Time;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum StatusCode {
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Response<'a> {
    pub status_code: StatusCode,
    pub body: &'a str,
    pub http_version: &'a str,
    pub headers: HashMap<String, String>,
}

impl<'a> Response<'a> {
    pub fn with_header(&mut self, key: &'a str, value: &'a str) -> &mut Self {
        self.headers.insert(key.to_owned(), value.to_owned());

        self
    }
    pub fn new(body: &'a str) -> Self {
        Response {
            status_code: StatusCode::Ok,
            body,
            http_version: "1.1",
            headers: HashMap::new(),
        }
    }

    pub fn default_headers(&mut self) -> &mut Self {
        self.headers
            .insert(String::from("Content-Length"), self.body.len().to_string());
        self.headers.insert(
            String::from("Cache-Control"),
            String::from("no-cache, private"),
        );
        if let Some(date) = Time::now() {
            self.headers.insert(String::from("Date"), date.format());
        }

        self.headers
            .insert(String::from("Content-Type"), String::from("text/plain"));
        // TODO: Add more default headers
        self
    }
    pub fn with_time(&'a mut self, stamp: i64) -> &mut Self {
        if let Some(time) = Time::now() {
            if let Some(with_stamp) = time.with_stamp(stamp) {
                self.headers
                    .insert(String::from("Date"), with_stamp.format());
            }
        }
        self
    }
    pub fn get_string(&mut self) -> String {
        let mut headers_str = String::from("");
        self.headers
            .iter()
            .for_each(|data| headers_str.push_str(&format!("{}:{}{}{}", data.0, SP, data.1, CRLF)));
        if self.body.trim().is_empty() {
            format!("{}{}", self.status_line(), headers_str)
        } else {
            format!("{}{}{}{}", self.status_line(), headers_str, CRLF, self.body)
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
    fn reason_phrase(&self) -> String {
        self.status_code.to_string().to_uppercase()
    }
    pub fn with_status(&mut self, code: StatusCode) -> &mut Self {
        self.status_code = code;
        self
    }
    pub fn with_http_version(&mut self, version: &'a str) -> &mut Self {
        self.http_version = version;
        self
    }
    fn status_code(&self) -> u16 {
        self.status_code as u16
    }
}
