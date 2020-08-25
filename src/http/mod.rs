use crate::request::{HttpVersion, Request};
use crate::responder::StatusCode;
use std::time::Duration;

use http10::*;
use http11::*;

pub mod http10;
pub mod http11;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeepAliveState {
    Close,
    UserDefined,
    Particular(Duration),
}

pub struct Validator<'a> {
    pub request: &'a Request<'a>,
    pub err_code: Option<StatusCode>,
    pub keep_alive: KeepAliveState,
}

impl<'a> Validator<'a> {
    pub fn validate(request: &'a Request<'a>) -> Self {
        let mut handler = Validator {
            request: request,
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
    pub fn keep_alive(&self) -> bool {
        if self.keep_alive == KeepAliveState::Close {
            false
        } else {
            true
        }
    }
}
