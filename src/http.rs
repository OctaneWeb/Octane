use crate::constants::*;

pub struct RequestLine {
    pub method: String,
    pub path: String,
    pub version: String,
}

impl RequestLine {
    pub fn parse(request_line: String) -> Option<Self> {
        let mut toks = request_line.split(SP);
        let method = match toks.next() {
            Some(v) => v,
            None => return None
        };
        let path = match toks.next() {
            Some(v) => v,
            None => return None
        };
        let version = match toks.next() {
            Some(v) => v,
            None => return None
        };
        let (first, ver) = version.split_at(5);
        if first != "HTTP/" || toks.next().is_some() {
            return None;
        }
        Some(RequestLine {
            method: method.to_string(),
            path: path.to_string(),
            version: ver.to_string(),
        })
    }
}

pub struct Header {
    pub name: String,
    pub value: String,
}

