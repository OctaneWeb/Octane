use crate::constants::*;
use std::cfg;
use std::collections::HashMap;

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

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Request<'a> {
    pub method: RequestMethod<'a>,
    pub path: &'a str,
    pub version: &'a str,
    pub headers: HashMap<String, String>,
    #[cfg(feature = "raw_headers")]
    pub raw_headers: Vec<Header<'a>>,
}

impl<'a> Request<'a> {
    pub fn parse(request: &'a str) -> Option<Self> {
        let mut toks = request.split(CRLF).skip_while(|v| v.len() == 0);
        let line = match toks.next().map(RequestLine::parse).flatten() {
            Some(v) => v,
            None => return None,
        };
        let mut headers: HashMap<String, String> = HashMap::new();
        let mut raw_headers: Vec<Header> = Vec::new();
        for tok in toks.by_ref() {
            if tok.len() == 0 {
                break;
            }
            let parsed = match Header::parse(tok) {
                Some(v) => v,
                None => return None,
            };
            headers
                .entry(parsed.name.to_ascii_lowercase())
                .and_modify(|v| *v = format!("{}, {}", v, parsed.value))
                .or_insert(parsed.value.to_string());
            if cfg!(feature = "raw_headers") {
                raw_headers.push(parsed);
            }
        }
        Some(Self {
            method: line.method,
            path: line.path,
            version: line.version,
            headers: headers,
            #[cfg(feature = "raw_headers")]
            raw_headers: raw_headers,
        })
    }
}
