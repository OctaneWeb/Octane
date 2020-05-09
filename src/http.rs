use crate::constants::*;
use std::cfg;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RequestMethod<'a> {
    Options,
    Get,
    Head,
    Post,
    Put,
    Delete,
    Trace,
    Connect,
    Other(&'a str),
}

pub struct RequestLine<'a> {
    pub method: RequestMethod<'a>,
    pub path: &'a str,
    pub version: &'a str,
}

impl<'a> RequestLine<'a> {
    pub fn parse(request_line: &'a str) -> Option<Self> {
        let mut toks = request_line.split(SP);
        let method = match toks.next() {
            Some(v) => v,
            None => return None,
        };
        let path = match toks.next() {
            Some(v) => v,
            None => return None,
        };
        let version = match toks.next() {
            Some(v) => v,
            None => return None,
        };
        let (first, ver) = version.split_at(5);

        if cfg!(feature = "faithful") && (first != "HTTP/" || toks.next().is_some()) {
            return None;
        }
        let request_method = match method {
            "POST" => RequestMethod::Post,
            "GET" => RequestMethod::Get,
            "DELETE" => RequestMethod::Delete,
            "PUT" => RequestMethod::Put,
            "OPTIONS" => RequestMethod::Options,
            "HEAD" => RequestMethod::Head,
            "TRACE" => RequestMethod::Trace,
            "CONNECT" => RequestMethod::Connect,
            _ => RequestMethod::Other(method),
        };
        Some(Self {
            method: request_method,
            path,
            version: ver,
        })
    }
}

pub struct Header<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

impl<'a> Header<'a> {
    pub fn parse(header: &'a str) -> Option<Self> {
        let mut toks = header.splitn(2, ':');
        let name = match toks.next() {
            Some(v) => v,
            None => return None,
        };
        if name.len() == 0 {
            return None;
        }
        if cfg!(feature = "faithful") {
            for c in name.chars() {
                if TOKEN_CHARS.get(&c).is_none() {
                    return None;
                }
            }
        }
        let value = match toks.next() {
            Some(v) => v,
            None => return None,
        }
        .trim_start_matches(|c| c == SP || c == HT);
        if cfg!(feature = "faithful") {
            if value.chars().any(is_ctl) {
                return None;
            }
        }
        Some(Self {
            name: name,
            value: value,
        })
    }
}
