pub use octane_macros::FromJSON;
use std::char;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Boolean(bool),
    Null,
}

macro_rules! make_as_func {
    ($name: ident, $type: ty, $variant: ident) => {
        pub fn $name(&self) -> Option<&$type> {
            if let Value::$variant(x) = self {
                Some(x)
            } else {
                None
            }
        }
    };
}

macro_rules! make_is_func {
    ($name: ident, $variant: ident) => {
        pub fn $name(&self) -> bool {
            if let Value::$variant(_) = self {
                true
            } else {
                false
            }
        }
    };
}

macro_rules! make_pe {
    ($type: ident, $variant: ident) => {
        impl PartialEq<$type> for Value {
            fn eq(&self, other: &$type) -> bool {
                if let Value::$variant(x) = self {
                    x.eq(other)
                } else {
                    false
                }
            }
        }
    };
}

impl Value {
    make_as_func!(as_number, f64, Number);
    make_as_func!(as_boolean, bool, Boolean);
    make_as_func!(as_string, String, String);
    make_as_func!(as_array, Vec<Value>, Array);
    make_as_func!(as_object, HashMap<String, Value>, Object);

    pub fn as_null(&self) -> Option<()> {
        if let Value::Null = self {
            Some(())
        } else {
            None
        }
    }

    make_is_func!(is_number, Number);
    make_is_func!(is_boolean, Boolean);
    make_is_func!(is_string, String);
    make_is_func!(is_array, Array);
    make_is_func!(is_object, Object);

    pub fn is_null(&self) -> bool {
        if let Value::Null = self {
            true
        } else {
            false
        }
    }

    pub fn parse(dat: &str) -> Option<Self> {
        let (val, rest) = parse_element(dat)?;
        if !rest.is_empty() {
            return None;
        }
        Some(val)
    }
}

impl Eq for Value {}

make_pe!(f64, Number);
make_pe!(String, String);
make_pe!(bool, Boolean);

impl<T> PartialEq<Vec<T>> for Value
where
    Value: PartialEq<T>
{
    fn eq(&self, other: &Vec<T>) -> bool {
        if let Value::Array(x) = self {
            x.eq(other)
        } else {
            false
        }
    }
}

impl<T> PartialEq<HashMap<String, T>> for Value
where
    Value: PartialEq<T>
{
    fn eq(&self, other: &HashMap<String, T>) -> bool {
        if let Value::Object(x) = self {
            if x.len() != other.len() {
                return false;
            }
            for (k, v1) in x {
                if let Some(v2) = other.get(k) {
                    if v1.ne(v2) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        false
    }
}

impl PartialEq<()> for Value {
    fn eq(&self, _: &()) -> bool {
        self.is_null()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Number(x), Value::Number(y)) => x.eq(y),
            (Value::String(x), Value::String(y)) => x.eq(y),
            (Value::Boolean(x), Value::Boolean(y)) => x.eq(y),
            (Value::Array(x), Value::Array(y)) => x.eq(y),
            (Value::Object(x), Value::Object(y)) => x.eq(y),
            (Value::Null, Value::Null) => true,
            _ => false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTypeError;

pub trait FromJSON
where
    Self: Sized,
{
    fn from_json(val: Value) -> Option<Self>;
}

pub trait ToJSON
where
    Self: Sized,
{
    fn to_json(&self) -> Option<String>;
}

impl<T> FromJSON for T
where
    T: TryFrom<Value>,
{
    fn from_json(val: Value) -> Option<Self> {
        Self::try_from(val).ok()
    }
}

macro_rules! make_tryfrom {
    ($type: ty, $variant: ident) => {
        impl TryFrom<Value> for $type {
            type Error = InvalidTypeError;

            fn try_from(v: Value) -> Result<Self, Self::Error> {
                if let Value::$variant(x) = v {
                    Ok(x)
                } else {
                    Err(InvalidTypeError)
                }
            }
        }
    };
}

make_tryfrom!(bool, Boolean);
make_tryfrom!(f64, Number);
make_tryfrom!(String, String);

impl<T> TryFrom<Value> for Vec<T>
where
    T: FromJSON,
{
    type Error = InvalidTypeError;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        if let Value::Array(arr) = v {
            arr.into_iter().map(T::from_json).collect::<Option<_>>().ok_or(InvalidTypeError)
        } else {
            Err(InvalidTypeError)
        }
    }
}

impl<T> TryFrom<Value> for HashMap<String, T>
where
    T: FromJSON,
{
    type Error = InvalidTypeError;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        if let Value::Object(map) = v {
            map.into_iter()
                .map(|(k, v)| Ok((k, T::from_json(v).ok_or(InvalidTypeError)?)))
                .collect::<Result<_, _>>()
        } else {
            Err(InvalidTypeError)
        }
    }
}

impl TryFrom<Value> for () {
    type Error = InvalidTypeError;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        if let Value::Null = v {
            Ok(())
        } else {
            Err(InvalidTypeError)
        }
    }
}

macro_rules! make_numeric_tryfrom {
    ($type: ty) => {
        impl TryFrom<Value> for $type {
            type Error = InvalidTypeError;

            #[allow(clippy::float_cmp)]
            fn try_from(v: Value) -> Result<Self, Self::Error> {
                let num: f64 = v.try_into()?;
                let conved = num as $type;
                if num == conved as f64 {
                    Ok(conved)
                } else {
                    Err(InvalidTypeError)
                }
            }
        }
    };
}

make_numeric_tryfrom!(u128);
make_numeric_tryfrom!(u64);
make_numeric_tryfrom!(u32);
make_numeric_tryfrom!(u16);
make_numeric_tryfrom!(u8);
make_numeric_tryfrom!(i128);
make_numeric_tryfrom!(i64);
make_numeric_tryfrom!(i32);
make_numeric_tryfrom!(i16);
make_numeric_tryfrom!(i8);
make_numeric_tryfrom!(f32);

impl ToJSON for f64 {
    fn to_json(&self) -> Option<String> {
        Some(self.to_string())
    }
}

impl ToJSON for () {
    fn to_json(&self) -> Option<String> {
        Some("null".to_string())
    }
}

impl ToJSON for bool {
    fn to_json(&self) -> Option<String> {
        Some(if *self {
            "true".to_string()
        } else {
            "false".to_string()
        })
    }
}

impl ToJSON for String {
    fn to_json(&self) -> Option<String> {
        Some(format!("{:?}", self))
    }
}

impl<T: ToJSON> ToJSON for Vec<T> {
    fn to_json(&self) -> Option<String> {
        let mut ret = "[".to_string();
        let len = self.len();
        for (i, v) in self.iter().enumerate() {
            ret.push_str(&v.to_json()?);
            ret.push(if i < len - 1 {','} else {']'});
        }
        Some(ret)
    }
}

impl<T: ToJSON> ToJSON for HashMap<String, T> {
    fn to_json(&self) -> Option<String> {
        let mut ret = "{".to_string();
        let len = self.len();
        for (i, (k, v)) in self.iter().enumerate() {
            ret.push_str(&k.to_json()?);
            ret.push(':');
            ret.push_str(&v.to_json()?);
            ret.push(if i < len - 1 {','} else {'}'});
        }
        Some(ret)
    }
}

impl ToJSON for Value {
    fn to_json(&self) -> Option<String> {
        match self {
            Value::Number(x) => x.to_json(),
            Value::String(x) => x.to_json(),
            Value::Boolean(x) => x.to_json(),
            Value::Array(x) => x.to_json(),
            Value::Object(x) => x.to_json(),
            Value::Null => ().to_json()
        }
    }
}

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

pub fn parse_number(dat: &str) -> Option<(f64, &str)> {
    let mut end = dat.len();
    let dat_bytes = dat.as_bytes();
    for (i, v) in dat_bytes.iter().enumerate() {
        match v {
            b'0'..=b'9' | b'e' | b'.' | b'-' | b'+' => {}
            _ => {
                end = i;
                break;
            }
        };
    }
    dat[..end].parse::<f64>().ok().map(|v| (v, &dat[end..]))
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
        b'-' | b'0'..=b'9' => parse_number(dat).map(do_fst!(Value::Number)),
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
