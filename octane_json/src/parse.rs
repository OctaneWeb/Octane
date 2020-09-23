use crate::value::Value;
use std::char;
use std::collections::HashMap;

pub fn consume_ws(dat: &str) -> &str {
    dat.trim_start()
}

pub fn parse_string(dat: &str) -> Option<(String, &str)> {
    // Function assumes that the first character is a double quote.
    let mut ret = String::with_capacity(dat.len());
    let mut cur = &dat[1..];
    while !cur.is_empty() {
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
                    b'"' => '"',
                    b'\\' => '\\',
                    b'/' => '/',
                    b'b' => '\x08',
                    b'f' => '\x0c',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    _ => return None,
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

pub fn parse_int_or_float(dat: &str) -> Option<(Value, &str)> {
    let mut end = dat.len();
    let dat_bytes = dat.as_bytes();
    let mut is_float = false;
    for (i, v) in dat_bytes.iter().enumerate() {
        match v {
            b'0'..=b'9' | b'-' | b'+' => {}
            b'e' | b'.' => {
                is_float = true;
            }
            _ => {
                end = i;
                break;
            }
        };
    }
    Some((
        if is_float {
            Value::Float(dat[..end].parse().ok()?)
        } else {
            Value::Integer(dat[..end].parse().ok()?)
        },
        &dat[end..],
    ))
}

pub fn parse_object(dat: &str) -> Option<(HashMap<String, Value>, &str)> {
    // This function assumes that the first character is {.
    let mut cur = consume_ws(&dat[1..]);
    let mut ret = HashMap::<String, Value>::new();
    if *cur.as_bytes().get(0)? == b'}' {
        return Some((ret, &cur[1..]));
    }
    while !cur.is_empty() {
        let (key, rest) = parse_string(cur)?;
        cur = consume_ws(rest);
        if *cur.as_bytes().get(0)? != b':' {
            return None;
        }
        let (val, remainder) = parse_element(&cur[1..])?;
        ret.insert(key, val);
        cur = remainder;
        match *cur.as_bytes().get(0)? {
            b',' => {
                cur = consume_ws(&cur[1..]);
            }
            b'}' => return Some((ret, &cur[1..])),
            _ => return None,
        }
    }
    None
}

pub fn parse_array(dat: &str) -> Option<(Vec<Value>, &str)> {
    // This function assumes that the first character is [.
    let mut cur = consume_ws(&dat[1..]);
    let mut ret = Vec::<Value>::new();
    if *cur.as_bytes().get(0)? == b']' {
        return Some((ret, &cur[1..]));
    }
    while !cur.is_empty() {
        let (val, rest) = parse_element(cur)?;
        ret.push(val);
        match *rest.as_bytes().get(0)? {
            b',' => {
                cur = consume_ws(&rest[1..]);
            }
            b']' => {
                cur = &rest[1..];
                break;
            }
            _ => return None,
        };
    }
    Some((ret, cur))
}

macro_rules! do_fst {
    (|$name:pat| $body:expr) => {
        |($name, snd)| ($body, snd)
    };
    ($func:path) => {
        |(fst, snd)| ($func(fst), snd)
    };
}

pub fn parse_value(dat: &str) -> Option<(Value, &str)> {
    match dat.as_bytes()[0] {
        b'{' => parse_object(dat).map(do_fst!(Value::Object)),
        b'-' | b'0'..=b'9' => parse_int_or_float(dat),
        b'"' => parse_string(dat).map(do_fst!(Value::String)),
        b't' | b'f' => parse_bool(dat).map(do_fst!(Value::Boolean)),
        b'n' => parse_null(dat).map(do_fst!(|_| Value::Null)),
        b'[' => parse_array(dat).map(do_fst!(Value::Array)),
        _ => None,
    }
}

pub fn parse_element(dat: &str) -> Option<(Value, &str)> {
    let trimmed = consume_ws(dat);
    let (val, rest) = parse_value(trimmed)?;
    Some((val, consume_ws(rest)))
}
