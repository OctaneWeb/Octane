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
}

pub trait ToJSON
where
    Self: Sized,
{
    fn to_json(self) -> Option<Value>;
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
make_tryfrom!(f64, Number);
make_tryfrom!(String, String);

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

macro_rules! make_numeric_tojson {
    ($type: ty) => {
        impl ToJSON for $type {
            fn to_json(self) -> Option<Value> {
                (self as f64).to_json()
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

make_numeric_tojson!(u128);
make_numeric_tojson!(u64);
make_numeric_tojson!(u32);
make_numeric_tojson!(u16);
make_numeric_tojson!(u8);
make_numeric_tojson!(i128);
make_numeric_tojson!(i64);
make_numeric_tojson!(i32);
make_numeric_tojson!(i16);
make_numeric_tojson!(i8);
make_numeric_tojson!(f32);

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify(self))
    }
}

impl ToJSON for f64 {
    fn to_json(self) -> Option<Value> {
        Some(Value::Number(self))
    }
}

impl ToJSON for () {
    fn to_json(self) -> Option<Value> {
        Some(Value::Null)
    }
}

impl ToJSON for bool {
    fn to_json(self) -> Option<Value> {
        Some(Value::Boolean(self))
    }
}

impl ToJSON for String {
    fn to_json(self) -> Option<Value> {
        Some(Value::String(self))
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
        Value::Number(x) => x.to_string(),
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
