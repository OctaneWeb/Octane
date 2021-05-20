use crate::request::Request;
use http10::*;
use http11::*;
use octane_http::HttpVersion;
use octane_http::StatusCode;
use std::time::Duration;

pub mod http10;
pub mod http11;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeepAliveState {
    Close,
    UserDefined,
    Particular(Duration),
}

pub struct Http<'a> {
    pub request: &'a Request<'a>,
    pub err_code: Option<StatusCode>,
    pub keep_alive: KeepAliveState,
}

impl<'a> Http<'a> {
    pub fn validate(request: &'a Request<'a>) -> Self {
        let mut handler = Self {
            request,
            err_code: None,
            keep_alive: KeepAliveState::Close,
        };
        match request.request_line.version {
            HttpVersion::Http11 => http11_check(&mut handler),
            HttpVersion::Http10 => http10_check(&mut handler),
            _ => (),
        }
        handler
    }
    pub fn set(&mut self, err_code: StatusCode) {
        self.err_code = Some(err_code)
    }
    pub fn set_keepalive(&mut self, keep_alive: KeepAliveState) {
        self.keep_alive = keep_alive
    }
    pub fn is_malformed(&self) -> bool {
        self.err_code.is_some()
    }
    #[allow(dead_code)]
    pub fn keep_alive(&self) -> bool {
        !(self.keep_alive == KeepAliveState::Close)
    }
}
