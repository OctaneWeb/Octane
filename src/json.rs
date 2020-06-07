use std::collections::HashMap;
use std::char;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Boolean(bool),
    Null,
}

pub fn consume_ws(dat: &str) -> &str {
    dat.trim_start()
}

pub fn parse_string(dat: &str) -> Option<(String, &str)> {
    // Function assumes that the starting quote is valid.
    let mut ret = String::with_capacity(dat.len());
    let mut cur = &dat[1..];
    while !cur.is_empty() {
        let x = cur.find('\\');
        if let Some(i) = cur.find('\\') {
            ret.push_str(&cur[..i]);
            let chr = cur.as_bytes()[i + 1];
            if chr == b'u' {
                let hex = &cur[i + 2..i + 6];
                if hex.len() != 4 {
                    return None;
                }
                if let Ok(v) = u16::from_str_radix(hex, 16) {
                    ret.push(char::from_u32(v as u32)?);
                    cur = &cur[i + 6..];
                } else {
                    return None;
                }
            } else {
                let parsed = match chr {
                    b'"'  => '"',
                    b'\\' => '\\',
                    b'/'  => '/',
                    b'b'  => '\x08',
                    b'f'  => '\x0c',
                    b'n'  => '\n',
                    b'r'  => '\r',
                    b't'  => '\t',
                    _     => return None
                };
                ret.push(parsed);
                cur = &cur[i + 2..];
            }
        } else {
            let v = cur.find('"')?;
            ret.push_str(&cur[..v]);
            return Some((ret, &cur[v + 1..]));
        }
    }
    None
}

pub fn parse_bool(dat: &str) -> Option<(bool, &str)> {
    if dat.starts_with("true") {
        return Some((true, &dat[4..]));
    } else if dat.starts_with("false") {
        return Some((false, &dat[5..]));
    }
    None
}

pub fn parse_null(dat: &str) -> Option<((), &str)> {
    if dat.starts_with("null") {
        return Some(((), &dat[4..]));
    }
    None
}

