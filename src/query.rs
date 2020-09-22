use crate::util::{from_hex, DoublePeek};
use std::collections::HashMap;

pub(crate) fn unescape_hex(string: &str) -> String {
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

#[allow(dead_code)]
pub(crate) fn parse_query(query: &str) -> HashMap<String, String> {
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
pub(crate) fn parse_extended_query(query: &str) -> HashMap<String, QueryValue> {
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
                            if let None = ret.get(&outside) {
                                ret.insert(outside.clone(), QueryValue::Arr(Vec::new()));
                            }
                            match ret.get_mut(&outside) {
                                Some(QueryValue::Arr(v)) => {
                                    v.push(unescaped_val);
                                    continue;
                                }
                                _ => continue,
                            }
                        } else {
                            if let None = ret.get(&outside) {
                                ret.insert(outside.clone(), QueryValue::Obj(HashMap::new()));
                            }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg(feature = "extended_queries")]
    fn success_standard_extend_queries() {
        // Parsing should work as expected.
        let query = parse_extended_query(
            "abc=def&ab=ab%20cd%4A&arr[]=x&arr[]=y&arr[]=z&obj[a]=x&obj[b]=y&obj[c]=z",
        );
        assert_eq!(query["abc"], QueryValue::Str("def".to_string()));
        assert_eq!(query["ab"], QueryValue::Str("ab cdJ".to_string()));
        assert_eq!(
            query["arr"],
            QueryValue::Arr(["x", "y", "z"].iter().map(|v| v.to_string()).collect())
        );
        let obj = match query["obj"].clone() {
            QueryValue::Obj(v) => v,
            _ => panic!("Query string did not have an object where expected."),
        };
        assert_eq!(obj["a"], "x");
        assert_eq!(obj["b"], "y");
        assert_eq!(obj["c"], "z");
    }

    #[test]
    #[cfg(feature = "extended_queries")]
    fn success_ignore_incompatible() {
        // If different types are specified, strings take precedence, then the one coming first takes precedence.
        let query = parse_extended_query("a[]=2&a[x]=3&a=1&b[]=1&b[x]=2&c[x]=1&c[]=2");
        assert_eq!(query["a"], QueryValue::Str("1".to_string()));
        assert_eq!(query["b"], QueryValue::Arr(vec!["1".to_string()]));
        let obj = match query["c"].clone() {
            QueryValue::Obj(v) => v,
            _ => panic!("Query string did not have an object where expected."),
        };
        assert_eq!(obj["x"], "1".to_string());
    }

    #[test]
    fn success_standard() {
        // Parsing should work as expected.
        let query = parse_query("abc=def&ab=ab%20cd%4A");
        assert_eq!(query["abc"], "def");
        assert_eq!(query["ab"], "ab cdJ");
    }

    #[test]
    fn success_blank_query() {
        // Queries without an "=" should be blank.
        let query = parse_query("a&abc=def");
        assert_eq!(query["a"], "");
        assert_eq!(query["abc"], "def");
    }

    #[test]
    fn success_weird_hex() {
        // Queries should be able to handle weird hex escapes.
        let query = parse_query("a=%0%41&b=%41%0&c=%%41&d=A%5");
        assert_eq!(query["a"], "%0A");
        assert_eq!(query["b"], "A%0");
        assert_eq!(query["c"], "%A");
        assert_eq!(query["d"], "A%5");
    }

    #[test]
    #[cfg_attr(not(feature = "faithful"), ignore)]
    fn success_no_name() {
        // Queries with no name should not insert anything into the hashmap.
        assert_eq!(parse_query("").len(), 0);
        assert_eq!(parse_query("=x").len(), 0);
        assert_eq!(parse_query("=x&=y").len(), 0);
    }
}
