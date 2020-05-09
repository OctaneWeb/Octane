use crate::constants::*;

pub struct RequestLine {
    method: String,
    path: String,
    version: String,
}

pub fn parse_request_line(request_line: String) -> Option<RequestLine> {
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
    if first != "HTTP/" {
        return None;
    }
    Some(RequestLine {
        method: method.to_string(),
        path: path.to_string(),
        version: ver.to_string(),
    })
}