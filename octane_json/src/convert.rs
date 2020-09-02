use crate::value::Value;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTypeError;

pub trait FromJSON
where
    Self: Sized,
{
    fn from_json(val: Value) -> Option<Self>;

    fn from_json_string(s: &str) -> Option<Self> {
        Value::parse(s).and_then(Self::from_json)
    }
}

pub trait ToJSON
where
    Self: Sized,
{
    fn to_json(self) -> Option<Value>;

    fn to_json_string(self) -> Option<String> {
        self.to_json().map(|v| v.to_string())
    }
}

impl<T> FromJSON for T
where
    T: TryFrom<Value, Error = InvalidTypeError>,
{
    fn from_json(val: Value) -> Option<Self> {
        Self::try_from(val).ok()
    }
}

impl FromJSON for Value {
    fn from_json(val: Value) -> Option<Self> {
        Some(val)
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
make_tryfrom!(i64, Integer);
make_tryfrom!(String, String);

macro_rules! make_from_integer {
    ($type: ty) => {
        impl TryFrom<Value> for $type {
            type Error = InvalidTypeError;

            fn try_from(v: Value) -> Result<Self, Self::Error> {
                if let Value::Integer(x) = v {
                    if let Ok(n) = x.try_into() {
                        return Ok(n);
                    }
                }
                Err(InvalidTypeError)
            }
        }
    }
}

make_from_integer!(u8);
make_from_integer!(u16);
make_from_integer!(u32);
make_from_integer!(u64);
make_from_integer!(i8);
make_from_integer!(i16);
make_from_integer!(i32);

impl TryFrom<Value> for f32 {
    type Error = InvalidTypeError;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Float(x) => Ok(x as f32),
            Value::Integer(x) => Ok(x as f32),
            _ => Err(InvalidTypeError)
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = InvalidTypeError;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        match v {
            Value::Float(x) => Ok(x),
            Value::Integer(x) => Ok(x as f64),
            _ => Err(InvalidTypeError)
        }
    }
}

impl<T> TryFrom<Value> for Vec<T>
where
    T: FromJSON,
{
    type Error = InvalidTypeError;

    fn try_from(v: Value) -> Result<Self, Self::Error> {
        if let Value::Array(arr) = v {
            arr.into_iter()
                .map(T::from_json)
                .collect::<Option<_>>()
                .ok_or(InvalidTypeError)
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

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify(self))
    }
}

macro_rules! make_to_json {
    ($type: ty, $variant: ident) => {
        impl ToJSON for $type {
            fn to_json(self) -> Option<Value> {
                Some(Value::$variant(self))
            }
        }
    }
}

make_to_json!(i64, Integer);
make_to_json!(f64, Float);
make_to_json!(bool, Boolean);
make_to_json!(String, String);

macro_rules! make_to_integer {
    ($type: ty) => {
        impl ToJSON for $type {
            fn to_json(self) -> Option<Value> {
                Some(Value::Integer(self as i64))
            }
        }
    }
}

make_to_integer!(u8);
make_to_integer!(u16);
make_to_integer!(u32);
make_to_integer!(i8);
make_to_integer!(i16);
make_to_integer!(i32);

impl ToJSON for u64 {
    fn to_json(self) -> Option<Value> {
        Some(Value::Integer(self.try_into().ok()?))
    }
}

impl ToJSON for f32 {
    fn to_json(self) -> Option<Value> {
        Some(Value::Float(self as f64))
    }
}

impl ToJSON for () {
    fn to_json(self) -> Option<Value> {
        Some(Value::Null)
    }
}

impl<T: ToJSON> ToJSON for Vec<T> {
    fn to_json(self) -> Option<Value> {
        Some(Value::Array(
            self.into_iter().map(T::to_json).collect::<Option<_>>()?,
        ))
    }
}

impl<T: ToJSON> ToJSON for HashMap<String, T> {
    fn to_json(self) -> Option<Value> {
        Some(Value::Object(
            self.into_iter()
                .map(|(k, v)| Some((k, v.to_json()?)))
                .collect::<Option<_>>()?,
        ))
    }
}

impl ToJSON for Value {
    fn to_json(self) -> Option<Value> {
        Some(self)
    }
}

fn stringify(val: &Value) -> String {
    match val {
        Value::Null => "null".to_string(),
        Value::Float(x) => format!("{:?}", x),
        Value::Integer(x) => x.to_string(),
        Value::String(x) => format!("{:?}", x),
        Value::Boolean(x) => format!("{}", x),
        Value::Array(x) => {
            let mut ret = "[".to_string();
            let len = x.len();
            for (i, v) in x.iter().enumerate() {
                ret.push_str(&stringify(v));
                ret.push(if i < len - 1 { ',' } else { ']' });
            }
            ret
        }
        Value::Object(x) => {
            let mut ret = "{".to_string();
            let len = x.len();
            for (i, (k, v)) in x.iter().enumerate() {
                ret.push_str(&format!("{:?}", k));
                ret.push(':');
                ret.push_str(&stringify(v));
                ret.push(if i < len - 1 { ',' } else { '}' });
            }
            ret
        }
    }
}
