use std::borrow::Cow;
#[cfg(feature = "extended_queries")]
use std::collections::HashMap;

/// The query of a request or a form body, its `name=value` pairs in order,
/// decoded the way browsers encode them (`application/x-www-form-urlencoded`:
/// `+` is a space and `%XX` the byte XX, read as UTF-8), see [`decode`]. A
/// pair without `=` has an empty value. Names and values with nothing to
/// decode are borrowed from the request rather than copied.
///
/// ```
/// use octane::prelude::*;
///
/// let mut app = Octane::new();
/// app.get("/", route_stop!(|req, res| {
///     let query = req.get_query();
///     res.send(query.get("q").unwrap_or("nothing asked"));
/// }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Query<'a> {
    pairs: Vec<(Cow<'a, str>, Cow<'a, str>)>,
}

impl<'a> Query<'a> {
    /// Parse a query string, without its leading `?`
    pub fn parse(query: &'a str) -> Self {
        let mut pairs = Vec::new();
        for tok in query.split('&') {
            if cfg!(feature = "faithful") && tok.is_empty() {
                continue;
            }
            let (name, value) = tok.split_once('=').unwrap_or((tok, ""));
            if cfg!(feature = "faithful") && name.is_empty() && tok.contains('=') {
                continue;
            }
            pairs.push((decode(name), decode(value)));
        }
        Self { pairs }
    }

    /// The value of `name`, the last one when it's repeated
    pub fn get(&self, name: &str) -> Option<&str> {
        self.pairs
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_ref())
    }

    /// Every value of `name`, in order
    pub fn get_all<'s>(&'s self, name: &'s str) -> impl Iterator<Item = &'s str> + 's {
        self.pairs
            .iter()
            .filter(move |(key, _)| key == name)
            .map(|(_, value)| value.as_ref())
    }

    /// Whether `name` is in the query
    pub fn contains(&self, name: &str) -> bool {
        self.pairs.iter().any(|(key, _)| key == name)
    }

    /// Every pair, in order
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.pairs
            .iter()
            .map(|(key, value)| (key.as_ref(), value.as_ref()))
    }

    /// How many pairs there are
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

/// Decode a query string or form body component, the way browsers encode
/// them: `+` is a space and `%XX` the byte XX, read as UTF-8. A `%` that
/// isn't followed by two hex digits is kept as it is, and invalid UTF-8
/// becomes U+FFFD. It is borrowed when there is nothing to decode.
///
/// ```
/// use octane::query::decode;
///
/// assert_eq!(decode("Ada+Lovelace%21"), "Ada Lovelace!");
/// ```
pub fn decode(string: &str) -> Cow<'_, str> {
    crate::decode::unescape(string, true)
}

/// Decode the `%XX` escapes of a url path segment. Unlike [`decode`] a
/// `+` stays a `+`, it only means a space in queries and forms. Url
/// variables are already decoded with it.
///
/// ```
/// use octane::query::percent_decode;
///
/// assert_eq!(percent_decode("caf%C3%A9+cr%C3%A8me"), "café+crème");
/// ```
pub fn percent_decode(string: &str) -> Cow<'_, str> {
    crate::decode::unescape(string, false)
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
                let unescaped_val = decode(&val[1..]).into_owned();
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
                ret.insert(decode(name).into_owned(), QueryValue::Str(unescaped_val));
            }
            None => {
                ret.insert(decode(tok).into_owned(), QueryValue::Str("".to_owned()));
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
        let query = Query::parse("abc=def&ab=ab%20cd%4A");
        assert_eq!(query.get("abc").unwrap(), "def");
        assert_eq!(query.get("ab").unwrap(), "ab cdJ");
    }

    #[test]
    fn success_blank_query() {
        // Queries without an "=" should be blank.
        let query = Query::parse("a&abc=def");
        assert_eq!(query.get("a").unwrap(), "");
        assert_eq!(query.get("abc").unwrap(), "def");
    }

    #[test]
    fn success_weird_hex() {
        // Queries should be able to handle weird hex escapes.
        let query = Query::parse("a=%0%41&b=%41%0&c=%%41&d=A%5");
        assert_eq!(query.get("a").unwrap(), "%0A");
        assert_eq!(query.get("b").unwrap(), "A%0");
        assert_eq!(query.get("c").unwrap(), "%A");
        assert_eq!(query.get("d").unwrap(), "A%5");
    }

    #[test]
    fn success_utf8_and_plus() {
        // escapes are UTF-8 bytes and `+` is a space, like browsers send them
        let query = Query::parse("q=caf%C3%A9+au+lait&p=%2B1&bad=%FF");
        assert_eq!(query.get("q").unwrap(), "café au lait");
        assert_eq!(query.get("p").unwrap(), "+1");
        assert_eq!(query.get("bad").unwrap(), "\u{FFFD}");
    }

    #[test]
    #[cfg_attr(not(feature = "faithful"), ignore)]
    fn success_no_name() {
        // Queries with no name should not insert anything into the hashmap.
        assert_eq!(Query::parse("").len(), 0);
        assert_eq!(Query::parse("=x").len(), 0);
        assert_eq!(Query::parse("=x&=y").len(), 0);
    }
}
