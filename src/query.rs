#![cfg(feature = "query_strings")]
use crate::util::{from_hex, DoublePeek};
use std::collections::HashMap;

pub fn unescape_hex(string: &str) -> String {
    let mut ret = "".to_owned();
    let mut chars = string.chars();
    let mut peekable = DoublePeek::new(&mut chars);
    while let Some(val) = peekable.next() {
        if val != '%' {
            ret.push(val);
        } else {
            peekable.unpeek = true;
            let c1 = match peekable.peek() {
                Some(&v) => v,
                None => {
                    ret.push('%');
                    continue;
                }
            };
            let c2 = match peekable.peek() {
                Some(&v) => v,
                None => {
                    ret.push('%');
                    continue;
                }
            };
            let (h1, h2) = (from_hex(c1), from_hex(c2));
            if h1.is_none() || h2.is_none() {
                ret.push('%');
                continue;
            }
            ret.push((h1.unwrap() * 16 + h2.unwrap()) as char);
            peekable.next();
            peekable.next();
        }
    }
    ret
}

pub fn parse_query(query: &str) -> HashMap<String, String> {
    let toks = query.split('&');
    let mut ret: HashMap<String, String> = HashMap::new();
    for tok in toks {
        if cfg!(feature = "faithful") && tok.is_empty() {
            continue;
        }
        match tok.find('=') {
            Some(v) => {
                let (name, val) = tok.split_at(v);
                if cfg!(feature = "faithful") && name.is_empty() {
                    continue;
                }
                ret.insert(unescape_hex(name), unescape_hex(&val[1..]));
            }
            None => {
                ret.insert(unescape_hex(tok), "".to_owned());
            }
        }
    }
    ret
}

#[cfg(feature = "extended_queries")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryValue {
    Str(String),
    Arr(Vec<String>),
    Obj(HashMap<String, String>),
}

#[cfg(feature = "extended_queries")]
pub fn parse_extended_query(query: &str) -> HashMap<String, QueryValue> {
    let toks = query.split('&');
    let mut ret: HashMap<String, QueryValue> = HashMap::new();
    for tok in toks {
        if cfg!(feature = "faithful") && tok.is_empty() {
            continue;
        }
        match tok.find('=') {
            Some(v) => {
                let (name, val) = tok.split_at(v);
                let unescaped_val = unescape_hex(&val[1..]);
                if name.is_empty() {
                    continue;
                }
                let bytes = name.as_bytes();
                let bytelen = bytes.len();
                if bytes[bytelen - 1] == b']' {
                    if let Some(i) = bytes.iter().rev().position(|&v| v == b'[') {
                        let ind = bytelen - i - 1;
                        let outside = match String::from_utf8((&bytes[0..ind]).to_vec()) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let inside =
                            match String::from_utf8((&bytes[ind + 1..bytelen - 1]).to_vec()) {
                                Ok(v) => v,
                                Err(_) => continue,
                            };
                        if inside.is_empty() {
                            ret.entry(outside.clone())
                                .or_insert_with(|| QueryValue::Arr(Vec::new()));
                            match ret.get_mut(&outside) {
                                Some(QueryValue::Arr(v)) => {
                                    v.push(unescaped_val);
                                    continue;
                                }
                                _ => continue,
                            }
                        } else {
                            ret.entry(outside.clone())
                                .or_insert_with(|| QueryValue::Obj(HashMap::new()));
                            match ret.get_mut(&outside) {
                                Some(QueryValue::Obj(v)) => {
                                    v.insert(inside, unescaped_val);
                                    continue;
                                }
                                _ => continue,
                            }
                        }
                    }
                }
                ret.insert(unescape_hex(name), QueryValue::Str(unescaped_val));
            }
            None => {
                ret.insert(unescape_hex(tok), QueryValue::Str("".to_owned()));
            }
        }
    }
    ret
}
