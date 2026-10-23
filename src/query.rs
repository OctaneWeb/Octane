use std::collections::HashMap;

// Decode a query component the way browsers encode one (the
// application/x-www-form-urlencoded format): `+` is a space, `%XX` is the
// byte XX, and the bytes are read as UTF-8 with invalid sequences replaced by
// U+FFFD. A `%` that isn't followed by two hex digits is kept as it is.
pub fn unescape_hex(string: &str) -> String {
    let bytes = string.as_bytes();
    let mut ret = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => ret.push(b' '),
            b'%' => {
                let hex = |at: usize| bytes.get(at).and_then(|&b| from_hex(b as char));
                match (hex(i + 1), hex(i + 2)) {
                    (Some(h1), Some(h2)) => {
                        ret.push(h1 * 16 + h2);
                        i += 2;
                    }
                    _ => ret.push(b'%'),
                }
            }
            b => ret.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&ret).into_owned()
}

pub fn from_hex(chr: char) -> Option<u8> {
    if chr > 'f' {
        return None;
    }
    let c = chr as u8;
    if c >= b'0' && c <= b'9' {
        return Some(c - b'0');
    }
    if c >= b'A' && c <= b'F' {
        return Some(c - b'A' + 10);
    }
    if c >= b'a' && c <= b'f' {
        return Some(c - b'a' + 10);
    }
    None
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
/// An parsed unit from an `extended_query`. We catch a request
/// which has an extended query as the path, we call `get_extended_query()`
/// method on the [`Request`](./request/struct.Request.html#method.get_extended_query) struct.
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
///
/// app.get("/", route_next!(|req, res| {
///     let parsed_extended_query = req.get_extended_query();
///     let value = parsed_extended_query.get("value");
/// }));
/// ```
/// An example of an extended query could be `a[]=2&a[x]=3&a=1&b[]=1&b[x]=2&c[x]=1&c[]=2`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryValue {
    /// A parsed String
    Str(String),
    /// An array of parsed String
    Arr(Vec<String>),
    /// A parsed object that is key and value represented
    Obj(HashMap<String, String>),
}

#[cfg(feature = "extended_queries")]
impl QueryValue {
    /// Get the string from a `QueryValue` instance, else None
    pub fn get_str(&self) -> Option<&String> {
        if let QueryValue::Str(x) = self {
            Some(x)
        } else {
            None
        }
    }
    /// Get the vector (array) from a `QueryValue` instance, else None
    pub fn get_arr(&self) -> Option<&Vec<String>> {
        if let QueryValue::Arr(x) = self {
            Some(x)
        } else {
            None
        }
    }
    /// Get the object (key and value seperated items) from a `QueryValue` instance, else None
    pub fn get_obj(&self) -> Option<&HashMap<String, String>> {
        if let QueryValue::Obj(x) = self {
            Some(x)
        } else {
            None
        }
    }
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
                            if ret.get(&outside).is_none() {
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
                            if ret.get(&outside).is_none() {
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
    fn success_standard_extended_queries() {
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
    fn success_utf8_and_plus() {
        // escapes are UTF-8 bytes and `+` is a space, like browsers send them
        let query = parse_query("q=caf%C3%A9+au+lait&p=%2B1&bad=%FF");
        assert_eq!(query["q"], "café au lait");
        assert_eq!(query["p"], "+1");
        assert_eq!(query["bad"], "\u{FFFD}");
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
